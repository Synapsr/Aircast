use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;

use crate::error::{AppError, AppResult};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
static ALIVE_SESSIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct CaptureSession {
    id: u64,
    stop_flag: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        // Mark dead first — the cpal callback checks this flag every time and
        // becomes a no-op once we drop. This protects us from the situation
        // (observed on macOS) where dropping a `cpal::Stream` doesn't actually
        // stop the OS audio thread for several seconds, so leftover callbacks
        // keep pulling music/cart samples and the audio sounds doubled.
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        let alive = ALIVE_SESSIONS
            .fetch_sub(1, Ordering::SeqCst)
            .saturating_sub(1);
        log::info!(
            "capture session #{} dropped (alive now: {})",
            self.id,
            alive
        );
    }
}

pub fn get_input_format(device_id: &str) -> AppResult<AudioFormat> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| AppError::Audio(format!("enumerate input devices: {e}")))?;
    for device in devices {
        if device.name().map(|n| n == device_id).unwrap_or(false) {
            let cfg = device
                .default_input_config()
                .map_err(|e| AppError::Audio(format!("default input config: {e}")))?;
            return Ok(AudioFormat {
                sample_rate: cfg.sample_rate().0,
                channels: cfg.channels(),
            });
        }
    }
    Err(AppError::Audio(format!("device not found: {device_id}")))
}

pub fn start_capture<F>(device_id: &str, mut consumer: F) -> AppResult<CaptureSession>
where
    F: FnMut(&[f32]) + Send + 'static,
{
    let device_id = device_id.to_string();
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<AudioFormat, String>>(1);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_thread = stop_flag.clone();
    let stop_flag_callback = stop_flag.clone();

    let handle = thread::spawn(move || {
        let host = cpal::default_host();

        let device = match find_input_device(&host, &device_id) {
            Ok(d) => d,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("default input config: {e}")));
                return;
            }
        };

        let format = AudioFormat {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        };

        log::info!(
            "capture input: {} Hz, {} ch, {:?}",
            format.sample_rate,
            format.channels,
            config.sample_format()
        );

        let sample_format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let err_fn = |err| log::error!("cpal stream error: {err}");

        // Wrap the consumer so the cpal callback is a no-op the moment our
        // CaptureSession is dropped. This is essential on macOS where the
        // OS audio thread can keep firing callbacks for several seconds
        // after the `cpal::Stream` is dropped.
        let stop_for_f32 = stop_flag_callback.clone();
        let stop_for_i16 = stop_flag_callback.clone();
        let stop_for_u16 = stop_flag_callback.clone();

        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => {
                let mut cb_count: u64 = 0;
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        if stop_for_f32.load(Ordering::Relaxed) {
                            return;
                        }
                        cb_count = cb_count.wrapping_add(1);
                        // Debug-level so the audio thread doesn't pay
                        // formatting cost in production. Kept for diagnostic
                        // purposes (RUST_LOG=debug).
                        if cb_count <= 3 || cb_count % 1000 == 0 {
                            log::debug!("cpal mic cb #{}: data.len()={}", cb_count, data.len());
                        }
                        consumer(data);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut buf = Vec::<f32>::new();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        if stop_for_i16.load(Ordering::Relaxed) {
                            return;
                        }
                        buf.clear();
                        buf.reserve(data.len());
                        for s in data {
                            buf.push(*s as f32 / i16::MAX as f32);
                        }
                        consumer(&buf);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut buf = Vec::<f32>::new();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        if stop_for_u16.load(Ordering::Relaxed) {
                            return;
                        }
                        buf.clear();
                        buf.reserve(data.len());
                        for s in data {
                            buf.push((*s as f32 - 32768.0) / 32768.0);
                        }
                        consumer(&buf);
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                let _ = ready_tx.send(Err(format!("unsupported sample format: {other:?}")));
                return;
            }
        };

        let stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("build input stream: {e}")));
                return;
            }
        };

        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(format!("play stream: {e}")));
            return;
        }

        let _ = ready_tx.send(Ok(format));

        while !stop_flag_thread.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
        // Pause first (synchronous on most hosts), then drop. Even if the OS
        // audio thread is slow to release, our callback no-ops via stop_flag.
        let _ = stream.pause();
        drop(stream);
    });

    let format = ready_rx
        .recv()
        .map_err(|e| AppError::Audio(format!("capture thread died before ready: {e}")))?
        .map_err(AppError::Audio)?;

    let _ = format;
    let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    let alive = ALIVE_SESSIONS.fetch_add(1, Ordering::SeqCst) + 1;
    log::info!("capture session #{} created (alive now: {})", id, alive);
    Ok(CaptureSession {
        id,
        stop_flag,
        handle: Some(handle),
    })
}

fn find_input_device(host: &cpal::Host, name: &str) -> Result<cpal::Device, String> {
    let devices = host
        .input_devices()
        .map_err(|e| format!("enumerate input devices: {e}"))?;

    for device in devices {
        if device.name().map(|n| n == name).unwrap_or(false) {
            return Ok(device);
        }
    }

    Err(format!("input device not found: {name}"))
}
