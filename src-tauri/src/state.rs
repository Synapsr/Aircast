use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::audio::capture::{AudioFormat, CaptureSession};
use crate::audio::playback::{self, MonitorSession};
use crate::error::AppResult;
use crate::presets::store::PresetStore;
use crate::stream::StreamHandle;
use crate::studio::Mixer;

const DEFAULT_TARGET_RATE: u32 = 48_000;
const DEFAULT_TARGET_CHANNELS: u16 = 2;

/// Shared context passed to the capture consumer and the streaming pipeline so
/// they can talk without restarting the audio device.
#[derive(Clone)]
pub struct CaptureContext {
    pub current_format: Arc<Mutex<Option<AudioFormat>>>,
    pub stream_tx: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
}

impl CaptureContext {
    pub fn new() -> Self {
        Self {
            current_format: Arc::new(Mutex::new(None)),
            stream_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn format(&self) -> Option<AudioFormat> {
        *self.current_format.lock().unwrap()
    }

    pub fn set_format(&self, format: Option<AudioFormat>) {
        *self.current_format.lock().unwrap() = format;
    }

    pub fn set_stream_tx(&self, tx: Option<mpsc::Sender<Vec<u8>>>) {
        *self.stream_tx.lock().unwrap() = tx;
    }
}

pub struct AppState {
    pub mixer: Arc<Mixer>,
    pub capture: Mutex<Option<CaptureSession>>,
    pub capture_ctx: CaptureContext,
    pub stream: tokio::sync::Mutex<Option<StreamHandle>>,
    pub presets: PresetStore,
    #[allow(dead_code)]
    monitor: Mutex<Option<MonitorSession>>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let mixer = Arc::new(Mixer::new(DEFAULT_TARGET_RATE, DEFAULT_TARGET_CHANNELS));

        let capture_ctx = CaptureContext::new();

        // Best-effort monitor: if no output device or it errors, app still works.
        let monitor = match playback::start_monitor(
            mixer.monitor_muted.clone(),
            mixer.monitor_ring.clone(),
            capture_ctx.current_format.clone(),
        ) {
            Ok(s) => Some(s),
            Err(e) => {
                log::warn!("monitor disabled: {e}");
                None
            }
        };

        Ok(Self {
            mixer,
            capture: Mutex::new(None),
            capture_ctx,
            stream: tokio::sync::Mutex::new(None),
            presets: PresetStore::new(app)?,
            monitor: Mutex::new(monitor),
        })
    }
}
