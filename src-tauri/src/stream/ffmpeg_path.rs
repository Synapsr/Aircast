use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Resolve the ffmpeg binary path. Checks (in order):
/// 1. Next to the running executable (Tauri places sidecars here on macOS/Windows bundled apps,
///    and in `target/<profile>/` during dev when `bundle.externalBin` is configured).
/// 2. The Tauri resource directory (some Linux packagings).
/// 3. System PATH (development without sidecar).
pub fn resolve(app: &AppHandle) -> PathBuf {
    let bin_name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join(bin_name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join(bin_name);
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from(bin_name)
}
