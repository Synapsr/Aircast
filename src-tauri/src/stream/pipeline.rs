use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;

use crate::audio::capture::AudioFormat;
use crate::presets::{Settings, StreamConfig, Transport};
use crate::state::{AppState, CaptureContext, LastStreamError};
use crate::stream::ffmpeg::{is_fatal_error, EncoderOutput, FfmpegProcess};
use crate::stream::status::{emit, StreamStatus};
use crate::stream::webcast::{self, MetadataSink, SessionEnd};

pub struct StreamHandle {
    stop_tx: Option<oneshot::Sender<()>>,
    join: Option<tauri::async_runtime::JoinHandle<()>>,
}

impl StreamHandle {
    pub async fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.join.take() {
            let _ = j.await;
        }
    }
}

pub fn start(
    app: AppHandle,
    config: StreamConfig,
    settings: Settings,
    format: AudioFormat,
    capture_ctx: CaptureContext,
    meta_sink: MetadataSink,
) -> StreamHandle {
    let (stop_tx, stop_rx) = oneshot::channel();
    let app_clone = app.clone();
    let join = tauri::async_runtime::spawn(run(
        app_clone,
        config,
        settings,
        format,
        capture_ctx,
        meta_sink,
        stop_rx,
    ));
    StreamHandle {
        stop_tx: Some(stop_tx),
        join: Some(join),
    }
}

enum AttemptOutcome {
    Stopped,
    Failed {
        message: String,
        details: Option<String>,
        /// `None` means "decide from the message text" — the Icecast path,
        /// where ffmpeg's stderr is the only signal. The webcast transport
        /// knows the answer exactly and says so.
        fatal: Option<bool>,
    },
}

/// The webcast transport owns its own attempt loop; this maps its outcome onto
/// the pipeline's, so the reconnect/backoff logic below stays transport-agnostic.
fn from_session_end(end: SessionEnd) -> AttemptOutcome {
    match end {
        SessionEnd::Stopped => AttemptOutcome::Stopped,
        SessionEnd::Failed {
            message,
            details,
            fatal,
        } => AttemptOutcome::Failed {
            message,
            details,
            fatal: Some(fatal),
        },
    }
}

async fn run(
    app: AppHandle,
    config: StreamConfig,
    settings: Settings,
    format: AudioFormat,
    capture_ctx: CaptureContext,
    meta_sink: MetadataSink,
    mut stop_rx: oneshot::Receiver<()>,
) {
    loop {
        log::info!(
            "stream pipeline: connecting ({:?} transport)",
            config.transport
        );
        emit(&app, StreamStatus::Connecting);

        let attempt = match config.transport {
            Transport::Icecast => {
                run_one_attempt(&app, &config, format, &capture_ctx, &mut stop_rx).await
            }
            Transport::Webcast => from_session_end(
                webcast::run_attempt(
                    &app,
                    &config,
                    format,
                    &capture_ctx,
                    &meta_sink,
                    &mut stop_rx,
                )
                .await,
            ),
        };

        match attempt {
            AttemptOutcome::Stopped => {
                log::info!("stream pipeline: stopped by user");
                emit(&app, StreamStatus::Idle);
                return;
            }
            AttemptOutcome::Failed {
                message,
                details,
                fatal,
            } => {
                let is_fatal = fatal.unwrap_or_else(|| is_fatal_error(&message));
                log::error!(
                    "stream pipeline: attempt failed — {message}{}",
                    details
                        .as_ref()
                        .map(|d| format!("\n--- ffmpeg tail ---\n{d}"))
                        .unwrap_or_default()
                );
                // Persist the last error in AppState so the diagnostic
                // bundle can include it even after the stream loop exits.
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut slot) = state.last_stream_error.lock() {
                        *slot = Some(LastStreamError {
                            message: message.clone(),
                            details: details.clone(),
                            at: std::time::SystemTime::now(),
                        });
                    }
                }
                // Always surface the actual ffmpeg/server message so the user
                // can see *why* the connection failed. The frontend keeps the
                // last error around in its own state, so a follow-up
                // Reconnecting status doesn't hide the dialog.
                emit(
                    &app,
                    StreamStatus::Error {
                        message: message.clone(),
                        details: details.clone(),
                    },
                );

                if is_fatal || settings.reconnect_interval_seconds == 0 {
                    log::info!(
                        "stream pipeline: not reconnecting ({})",
                        if is_fatal {
                            "fatal error"
                        } else {
                            "auto-reconnect disabled"
                        }
                    );
                    return;
                }

                log::info!(
                    "stream pipeline: reconnecting in {}s",
                    settings.reconnect_interval_seconds
                );
                emit(
                    &app,
                    StreamStatus::Reconnecting {
                        next_attempt_in_ms: settings.reconnect_interval_seconds * 1000,
                    },
                );

                let sleep =
                    tokio::time::sleep(Duration::from_secs(settings.reconnect_interval_seconds));
                tokio::pin!(sleep);

                tokio::select! {
                    _ = &mut stop_rx => {
                        log::info!("stream pipeline: stopped during reconnect wait");
                        emit(&app, StreamStatus::Idle);
                        return;
                    }
                    _ = &mut sleep => {}
                }
            }
        }
    }
}

async fn run_one_attempt(
    app: &AppHandle,
    config: &StreamConfig,
    format: AudioFormat,
    capture_ctx: &CaptureContext,
    stop_rx: &mut oneshot::Receiver<()>,
) -> AttemptOutcome {
    let mut ffmpeg = match FfmpegProcess::spawn(app, config, format, EncoderOutput::Icecast) {
        Ok(f) => f,
        Err(e) => {
            return AttemptOutcome::Failed {
                message: e.to_string(),
                details: None,
                fatal: None,
            }
        }
    };

    let stdin_tx = match ffmpeg.stdin_sender() {
        Some(tx) => tx,
        None => {
            return AttemptOutcome::Failed {
                message: "ffmpeg stdin missing".into(),
                details: None,
                fatal: None,
            }
        }
    };
    let status = ffmpeg.status.clone();

    // Attach to the live capture loop
    capture_ctx.set_stream_tx(Some(stdin_tx));

    let mut announced_live = false;
    let connection_start = std::time::Instant::now();
    const CONNECT_TIMEOUT_SECS: u64 = 30;
    let outcome;

    loop {
        let live_check = tokio::time::sleep(Duration::from_millis(150));
        tokio::pin!(live_check);

        tokio::select! {
            _ = &mut *stop_rx => {
                outcome = AttemptOutcome::Stopped;
                break;
            }
            exit = ffmpeg.wait() => {
                let tail = ffmpeg.status.tail();
                let classified = ffmpeg.status.last_error_message();
                let message = classified.unwrap_or_else(|| {
                    format!("ffmpeg exited unexpectedly ({:?})", exit.ok())
                });
                let details = if tail.is_empty() { None } else { Some(tail) };
                outcome = AttemptOutcome::Failed {
                    message,
                    details,
                    fatal: None,
                };
                break;
            }
            _ = &mut live_check => {
                if !announced_live && status.became_live.load(Ordering::Acquire) {
                    announced_live = true;
                    log::info!("stream pipeline: live");
                    // Clear any leftover error now that we're healthy again.
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(mut slot) = state.last_stream_error.lock() {
                            *slot = None;
                        }
                    }
                    emit(app, StreamStatus::Live);
                }
                if !announced_live
                    && connection_start.elapsed().as_secs() >= CONNECT_TIMEOUT_SECS
                {
                    let tail = ffmpeg.status.tail();
                    let details = if tail.is_empty() { None } else { Some(tail) };
                    outcome = AttemptOutcome::Failed {
                        message: format!(
                            "Connection timed out — the server didn't accept the stream within {}s.",
                            CONNECT_TIMEOUT_SECS
                        ),
                        details,
                        fatal: None,
                    };
                    break;
                }
            }
        }
    }

    capture_ctx.set_stream_tx(None);
    ffmpeg.shutdown().await;
    outcome
}
