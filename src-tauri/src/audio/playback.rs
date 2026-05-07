use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::capture::AudioFormat;
use crate::error::{AppError, AppResult};

/// Maximum samples kept in the monitor ring (~500ms at 48kHz stereo, ~1s at
/// 48kHz mono). Big enough to absorb CPU jitter; small enough to keep the
/// monitor's perceived latency low.
const MAX_RING_LEN: usize = 48_000;

pub type MonitorRing = Arc<Mutex<VecDeque<f32>>>;
pub type InputFormatSlot = Arc<Mutex<Option<AudioFormat>>>;

pub fn make_ring() -> MonitorRing {
    Arc::new(Mutex::new(VecDeque::with_capacity(MAX_RING_LEN)))
}

/// Push samples to the monitor ring. Lock is held briefly. Drops oldest when
/// full so the audible signal stays live (capped delay above).
pub fn push_to_ring(ring: &MonitorRing, samples: &[f32]) {
    if let Ok(mut ring) = ring.try_lock() {
        let overflow = (ring.len() + samples.len()).saturating_sub(MAX_RING_LEN);
        for _ in 0..overflow {
            ring.pop_front();
        }
        ring.extend(samples.iter().copied());
    }
}

pub struct MonitorSession {
    stop_flag: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for MonitorSession {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Sample-and-hold rate converter + channel mapper. Drains the ring into a
/// local scratch buffer in a single short critical section so the input
/// callback can push samples without contending with the output callback.
struct MonitorPipeline {
    fractional: f64,
    last_frame: Vec<f32>,
    cached_in_channels: u16,
    drain_buf: Vec<f32>,
}

impl MonitorPipeline {
    fn new() -> Self {
        Self {
            fractional: 0.0,
            last_frame: Vec::new(),
            cached_in_channels: 0,
            drain_buf: Vec::with_capacity(8192),
        }
    }

    fn ensure_in_channels(&mut self, in_channels: u16) {
        if self.cached_in_channels != in_channels {
            let n = in_channels.max(1) as usize;
            self.last_frame.clear();
            self.last_frame.resize(n, 0.0);
            self.cached_in_channels = in_channels;
            self.fractional = 0.0;
        }
    }

    fn render_f32(
        &mut self,
        buf: &mut [f32],
        out_channels: u16,
        out_rate: u32,
        in_format: Option<AudioFormat>,
        ring: &MonitorRing,
    ) {
        let Some(fmt) = in_format else {
            buf.fill(0.0);
            return;
        };
        if fmt.channels == 0 || fmt.sample_rate == 0 {
            buf.fill(0.0);
            return;
        }

        self.ensure_in_channels(fmt.channels);
        let in_chs = fmt.channels as usize;
        let out_chs = out_channels.max(1) as usize;
        let out_frames = buf.len() / out_chs;
        let step = fmt.sample_rate as f64 / out_rate.max(1) as f64;

        // Worst-case input samples needed for this output buffer.
        let want_frames = (out_frames as f64 * step).ceil() as usize + 1;
        let want_samples = want_frames * in_chs;

        // 1) Drain the ring quickly. Hold the lock for as little as possible.
        self.drain_buf.clear();
        if self.drain_buf.capacity() < want_samples {
            self.drain_buf
                .reserve(want_samples - self.drain_buf.capacity());
        }
        {
            match ring.lock() {
                Ok(mut ring_guard) => {
                    let take = want_samples.min(ring_guard.len());
                    for _ in 0..take {
                        // unwrap is safe: take ≤ len
                        self.drain_buf.push(ring_guard.pop_front().unwrap());
                    }
                }
                Err(_) => {
                    buf.fill(0.0);
                    return;
                }
            }
        } // ring lock released here

        // 2) Resample + channel map from the drained scratch into the output.
        let mut idx = 0;
        for f in 0..out_frames {
            self.fractional += step;
            while self.fractional >= 1.0 {
                self.fractional -= 1.0;
                if idx + in_chs <= self.drain_buf.len() {
                    for c in 0..in_chs {
                        self.last_frame[c] = self.drain_buf[idx + c];
                    }
                    idx += in_chs;
                }
                // else: under-run for this frame; keep `last_frame` (sample-and-hold).
            }

            let out_off = f * out_chs;
            map_channels(
                &self.last_frame,
                in_chs,
                &mut buf[out_off..out_off + out_chs],
            );
        }
    }
}

fn map_channels(input: &[f32], in_ch: usize, output: &mut [f32]) {
    let out_ch = output.len();
    if in_ch == 0 || out_ch == 0 {
        for o in output.iter_mut() {
            *o = 0.0;
        }
        return;
    }
    if in_ch == 1 {
        for o in output.iter_mut() {
            *o = input[0];
        }
        return;
    }
    if out_ch == 1 {
        if in_ch >= 2 {
            output[0] = (input[0] + input[1]) * 0.5;
        } else {
            output[0] = input[0];
        }
        return;
    }
    for o in 0..out_ch {
        output[o] = input[o.min(in_ch - 1)];
    }
}

pub fn start_monitor(
    muted: Arc<AtomicBool>,
    ring: MonitorRing,
    input_format: InputFormatSlot,
) -> AppResult<MonitorSession> {
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_thread = stop_flag.clone();

    let handle = thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                let _ = ready_tx.send(Err("no default output device".into()));
                return;
            }
        };
        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("default output config: {e}")));
                return;
            }
        };

        let out_rate = config.sample_rate().0;
        let out_channels = config.channels();
        let sample_format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let err_fn = |err| log::error!("monitor stream error: {err}");

        log::info!(
            "monitor output: {} Hz, {} ch, {:?}",
            out_rate,
            out_channels,
            sample_format
        );

        // Each match arm owns its own pipeline (FnMut closure, no inner Mutex).
        let muted_cb = muted.clone();
        let ring_cb = ring.clone();
        let input_cb = input_format.clone();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let mut pipeline = MonitorPipeline::new();
                device.build_output_stream(
                    &stream_config,
                    move |buf: &mut [f32], _| {
                        if muted_cb.load(Ordering::Relaxed) {
                            buf.fill(0.0);
                            return;
                        }
                        let in_fmt = input_cb.lock().ok().and_then(|g| *g);
                        pipeline.render_f32(buf, out_channels, out_rate, in_fmt, &ring_cb);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut pipeline = MonitorPipeline::new();
                let mut scratch: Vec<f32> = Vec::with_capacity(8192);
                device.build_output_stream(
                    &stream_config,
                    move |buf: &mut [i16], _| {
                        if muted_cb.load(Ordering::Relaxed) {
                            buf.fill(0);
                            return;
                        }
                        scratch.clear();
                        scratch.resize(buf.len(), 0.0);
                        let in_fmt = input_cb.lock().ok().and_then(|g| *g);
                        pipeline.render_f32(&mut scratch, out_channels, out_rate, in_fmt, &ring_cb);
                        for (d, s) in buf.iter_mut().zip(scratch.iter()) {
                            *d = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut pipeline = MonitorPipeline::new();
                let mut scratch: Vec<f32> = Vec::with_capacity(8192);
                device.build_output_stream(
                    &stream_config,
                    move |buf: &mut [u16], _| {
                        if muted_cb.load(Ordering::Relaxed) {
                            buf.fill(u16::MAX / 2);
                            return;
                        }
                        scratch.clear();
                        scratch.resize(buf.len(), 0.0);
                        let in_fmt = input_cb.lock().ok().and_then(|g| *g);
                        pipeline.render_f32(&mut scratch, out_channels, out_rate, in_fmt, &ring_cb);
                        for (d, s) in buf.iter_mut().zip(scratch.iter()) {
                            *d = (((s.clamp(-1.0, 1.0) + 1.0) * 0.5) * u16::MAX as f32) as u16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                let _ = ready_tx.send(Err(format!("unsupported output format: {other:?}")));
                return;
            }
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("build output stream: {e}")));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(format!("play output stream: {e}")));
            return;
        }
        let _ = ready_tx.send(Ok(()));

        while !stop_flag_thread.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }
        drop(stream);
    });

    ready_rx
        .recv()
        .map_err(|e| AppError::Audio(format!("monitor thread died: {e}")))?
        .map_err(AppError::Audio)?;

    Ok(MonitorSession {
        stop_flag,
        handle: Some(handle),
    })
}
