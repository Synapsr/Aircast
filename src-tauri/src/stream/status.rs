use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamStatus {
    Idle,
    Connecting,
    Live,
    #[serde(rename_all = "camelCase")]
    Reconnecting {
        next_attempt_in_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        message: String,
        /// Raw ffmpeg/server output for the technical-details section in the
        /// error dialog. Only present for connection failures, not for
        /// client-side validation errors.
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<String>,
    },
}

impl StreamStatus {
    pub fn is_live(&self) -> bool {
        matches!(self, StreamStatus::Live)
    }
}

pub fn emit(app: &AppHandle, status: StreamStatus) {
    // Mirror to AppState so the metadata updater (and any other consumer that
    // doesn't get the event stream) can read the current state synchronously.
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut slot) = state.stream_status.lock() {
            *slot = status.clone();
        }
    }
    let _ = app.emit("stream-status", status);
}
