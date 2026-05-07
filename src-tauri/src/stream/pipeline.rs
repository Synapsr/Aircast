use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::AppHandle;
use tokio::sync::oneshot;

use crate::audio::capture::AudioFormat;
use crate::presets::{Settings, StreamConfig};
use crate::state::CaptureContext;
use crate::stream::ffmpeg::{is_fatal_error, FfmpegProcess};
use crate::stream::status::{emit, StreamStatus};

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
) -> StreamHandle {
    let (stop_tx, stop_rx) = oneshot::channel();
    let app_clone = app.clone();
    let join = tauri::async_runtime::spawn(run(
        app_clone,
        config,
        settings,
        format,
        capture_ctx,
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
    },
}

async fn run(
    app: AppHandle,
    config: StreamConfig,
    settings: Settings,
    format: AudioFormat,
    capture_ctx: CaptureContext,
    mut stop_rx: oneshot::Receiver<()>,
) {
    loop {
        emit(&app, StreamStatus::Connecting);

        match run_one_attempt(&app, &config, format, &capture_ctx, &mut stop_rx).await {
            AttemptOutcome::Stopped => {
                emit(&app, StreamStatus::Idle);
                return;
            }
            AttemptOutcome::Failed { message, details } => {
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

                if is_fatal_error(&message) || settings.reconnect_interval_seconds == 0 {
                    return;
                }

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
    let mut ffmpeg = match FfmpegProcess::spawn(app, config, format) {
        Ok(f) => f,
        Err(e) => {
            return AttemptOutcome::Failed {
                message: e.to_string(),
                details: None,
            }
        }
    };

    let stdin_tx = match ffmpeg.stdin_sender() {
        Some(tx) => tx,
        None => {
            return AttemptOutcome::Failed {
                message: "ffmpeg stdin missing".into(),
                details: None,
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
                outcome = AttemptOutcome::Failed { message, details };
                break;
            }
            _ = &mut live_check => {
                if !announced_live && status.became_live.load(Ordering::Acquire) {
                    announced_live = true;
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
