use serde::Serialize;
use tauri::{AppHandle, Emitter};

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

pub fn emit(app: &AppHandle, status: StreamStatus) {
    let _ = app.emit("stream-status", status);
}
