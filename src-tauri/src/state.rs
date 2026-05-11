use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::audio::capture::{AudioFormat, CaptureSession};
use crate::audio::playback::{self, MonitorSession};
use crate::audio::url_input::RelayInputSession;
use crate::error::AppResult;
use crate::presets::store::PresetStore;
use crate::stream::metadata;
use crate::stream::status::StreamStatus;
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

/// The most recent stream error, kept around even after the stream stops so
/// the diagnostic bundle can include it. Cleared once a new live cycle
/// reaches the Live state (no point reporting a successfully-recovered
/// transient error).
#[derive(Debug, Clone)]
pub struct LastStreamError {
    pub message: String,
    pub details: Option<String>,
    pub at: std::time::SystemTime,
}

pub struct AppState {
    pub mixer: Arc<Mixer>,
    pub capture: Mutex<Option<CaptureSession>>,
    pub capture_ctx: CaptureContext,
    pub stream: tokio::sync::Mutex<Option<StreamHandle>>,
    /// Active Relay-mode input session (ffmpeg decoding an upstream URL).
    /// Mutually exclusive with `capture` — Relay mode keeps the cpal mic
    /// closed and feeds the mixer from this session instead.
    pub relay: Mutex<Option<RelayInputSession>>,
    pub presets: PresetStore,
    pub last_stream_error: Arc<Mutex<Option<LastStreamError>>>,
    /// Latest stream status, kept in sync by [`crate::stream::pipeline`] on
    /// every emission. Read-only for everyone else; lets the metadata
    /// updater tell whether the stream is actually live before pushing.
    pub stream_status: Arc<Mutex<StreamStatus>>,
    /// Sender to the long-lived metadata updater task. Components send
    /// `Tick`, `SetTarget`, `SetSettings`, `PushNow` commands here from
    /// any thread/runtime via `try_send` (non-blocking).
    pub metadata_tx: mpsc::Sender<metadata::Command>,
    /// Slot updated by the file-poll watcher (`MetadataMode::File`). The
    /// state poller reads this each tick to feed `ComposeInput.file_content`.
    pub metadata_file_content: Arc<tokio::sync::Mutex<Option<String>>>,
    /// Handle to the currently-running file-poll task, if any. Aborted
    /// when settings change to `Auto`/`Static` mode or the path changes.
    pub metadata_file_watcher: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
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

        // Metadata updater: long-lived task, lives for the whole process.
        // Channel is bounded — `Tick`s drop on overflow which is desirable
        // (one stale tick gets replaced by the next; we don't need a queue).
        let (metadata_tx, metadata_rx) = mpsc::channel::<metadata::Command>(16);
        metadata::spawn(app.clone(), metadata_rx);

        Ok(Self {
            mixer,
            capture: Mutex::new(None),
            capture_ctx,
            stream: tokio::sync::Mutex::new(None),
            relay: Mutex::new(None),
            presets: PresetStore::new(app)?,
            last_stream_error: Arc::new(Mutex::new(None)),
            stream_status: Arc::new(Mutex::new(StreamStatus::Idle)),
            metadata_tx,
            metadata_file_content: Arc::new(tokio::sync::Mutex::new(None)),
            metadata_file_watcher: Mutex::new(None),
            monitor: Mutex::new(monitor),
        })
    }
}
