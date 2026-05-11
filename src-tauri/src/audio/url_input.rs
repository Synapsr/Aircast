//! Spawns ffmpeg to decode an upstream stream URL (HTTP, HTTPS, HLS, Icecast,
//! local file...) into raw f32 PCM at a fixed sample rate, then hands each
//! decoded chunk to a caller-provided consumer at real-time pace.
//!
//! The consumer signature mirrors `audio::capture::start_capture` so the rest
//! of the audio path (mixer → monitor + encoder) doesn't need to know whether
//! the bytes came from a microphone or from a relay URL.
//!
//! ## Real-time pacing
//!
//! We pass `-re` to ffmpeg so it reads its input at real-time speed. Audio
//! decoders naturally output at the file's playback rate, so ffmpeg pushes
//! one second of PCM per second of wall-clock. Reading its stdout blocks
//! Aircast at that same pace — no manual sleeping needed.
//!
//! ## Reconnect
//!
//! If ffmpeg exits (network blip, server reboot, source ends), the session's
//! manager thread respawns it after a linear delay (5 s default). Each
//! transition emits a frontend event (`relay-upstream-changed`) so the UI
//! can show "Reconnecting…" with a countdown.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, AppResult};
use crate::stream::ffmpeg_path;

/// PCM format we ask ffmpeg to produce. Mono keeps the relay pipeline
/// symmetric with the existing capture pipeline (default mic = mono).
pub const RELAY_RATE: u32 = 48_000;
pub const RELAY_CHANNELS: u16 = 1;

const RECONNECT_DELAY_SECS: u64 = 5;
const READ_CHUNK_FRAMES: usize = 1024;
const READ_CHUNK_BYTES: usize = READ_CHUNK_FRAMES * RELAY_CHANNELS as usize * 4; // f32 = 4 bytes

/// Status the UI displays for the upstream connection. Variants are emitted
/// to the frontend via `relay-upstream-changed` events; clippy can't see that
/// they're constructed (we do it in the spawn loop), hence the allow.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpstreamStatus {
    Idle,
    Connecting,
    Streaming,
    Reconnecting,
    Stopped,
}

pub struct RelayInputSession {
    stop_flag: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for RelayInputSession {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        log::info!("relay input session dropped");
    }
}

/// Spawn the relay input task. `consumer` is called with each decoded chunk
/// of `f32` samples (mono, 48 kHz). It runs on a dedicated thread — keep it
/// non-blocking and fast (push to a ring / channel; don't do IO).
pub fn start_relay_input<F>(
    app: AppHandle,
    url: String,
    mut consumer: F,
) -> AppResult<RelayInputSession>
where
    F: FnMut(&[f32]) + Send + 'static,
{
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop_flag.clone();

    let handle = thread::Builder::new()
        .name("aircast-relay-input".into())
        .spawn(move || {
            let mut first_attempt = true;
            while !stop_for_thread.load(Ordering::SeqCst) {
                if first_attempt {
                    emit(&app, UpstreamStatus::Connecting);
                    first_attempt = false;
                } else {
                    emit(&app, UpstreamStatus::Reconnecting);
                    // Linear backoff between attempts.
                    let until = Instant::now() + Duration::from_secs(RECONNECT_DELAY_SECS);
                    while Instant::now() < until {
                        if stop_for_thread.load(Ordering::SeqCst) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(200));
                    }
                    if stop_for_thread.load(Ordering::SeqCst) {
                        break;
                    }
                }

                match run_one_attempt(&app, &url, &stop_for_thread, &mut consumer) {
                    Ok(()) => {
                        // ffmpeg exited cleanly (EOF, server closed) — try
                        // again so a partner station that briefly drops mid-
                        // programme isn't permanently lost.
                        log::warn!(
                            "relay upstream ended; will reconnect in {RECONNECT_DELAY_SECS}s"
                        );
                    }
                    Err(e) => {
                        log::warn!("relay upstream attempt failed: {e}");
                    }
                }
            }
            emit(&app, UpstreamStatus::Stopped);
        })
        .map_err(|e| AppError::Audio(format!("spawn relay thread: {e}")))?;

    Ok(RelayInputSession {
        stop_flag,
        handle: Some(handle),
    })
}

/// One ffmpeg lifetime: spawn → stream PCM into the consumer → exit.
/// Returns Ok when ffmpeg exited naturally, Err with a description otherwise.
fn run_one_attempt<F>(
    app: &AppHandle,
    url: &str,
    stop_flag: &Arc<AtomicBool>,
    consumer: &mut F,
) -> Result<(), String>
where
    F: FnMut(&[f32]),
{
    let binary = ffmpeg_path::resolve(app);
    log::info!("relay ffmpeg binary: {}", binary.display());

    let mut child = std::process::Command::new(&binary)
        .args([
            "-hide_banner",
            "-loglevel",
            "warning",
            "-nostats",
            "-re", // read input at real-time speed
            "-reconnect",
            "1",
            "-reconnect_streamed",
            "1",
            "-reconnect_delay_max",
            "5",
            "-i",
            url,
            "-f",
            "f32le",
            "-ar",
            &RELAY_RATE.to_string(),
            "-ac",
            &RELAY_CHANNELS.to_string(),
            "pipe:1",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn ffmpeg: {e}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "ffmpeg stdout missing".to_string())?;
    let stderr = child.stderr.take();

    // Pump stderr in a worker so it never blocks ffmpeg via full pipe and we
    // get diagnostic lines in the log file.
    if let Some(mut e) = stderr {
        thread::Builder::new()
            .name("aircast-relay-stderr".into())
            .spawn(move || {
                use std::io::BufRead;
                let reader = std::io::BufReader::new(&mut e);
                for line in reader.lines().map_while(Result::ok) {
                    log::debug!("relay ffmpeg: {line}");
                }
            })
            .ok();
    }

    let mut buf = vec![0u8; READ_CHUNK_BYTES];
    let mut announced_streaming = false;

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            let _ = child.kill();
            return Ok(());
        }

        // `read_exact` guarantees we consume entire 4-byte samples per call.
        // A plain `read()` could return a non-multiple-of-4 length and we'd
        // drop the trailing bytes, which shifts every subsequent sample
        // out of alignment — producing audible clicks/pops in the monitor
        // and the relayed stream. `read_exact` loops internally until the
        // buffer is full (or EOF).
        match stdout.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // ffmpeg closed stdout mid-buffer — treat as natural end.
                let _ = child.wait();
                return Ok(());
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("stdout read: {e}"));
            }
        }

        if !announced_streaming {
            announced_streaming = true;
            log::info!("relay upstream streaming");
            emit(app, UpstreamStatus::Streaming);
        }

        let samples = bytes_to_f32(&buf);
        consumer(&samples);
    }
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn emit(app: &AppHandle, status: UpstreamStatus) {
    if app.try_state::<crate::state::AppState>().is_some() {
        let _ = app.emit("relay-upstream-changed", status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_to_f32_basic_roundtrip() {
        let original: Vec<f32> = vec![0.0, 0.5, -0.5, 1.0];
        let bytes: Vec<u8> = original.iter().flat_map(|f| f.to_le_bytes()).collect();
        let decoded = bytes_to_f32(&bytes);
        assert_eq!(decoded, original);
    }

    #[test]
    fn bytes_to_f32_truncates_partial_trailing_bytes() {
        // 9 bytes — last byte is junk and should be dropped.
        let bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 128, 63, 99];
        let decoded = bytes_to_f32(&bytes);
        assert_eq!(decoded, vec![0.0, 1.0]); // 0x3F800000 = 1.0
    }
}
