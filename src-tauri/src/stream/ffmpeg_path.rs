use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Resolve the ffmpeg binary path. The bundled sidecar is namespaced
/// `aircast-ffmpeg` (load-bearing on Linux: a bare `ffmpeg` in the deb would
/// land at `/usr/bin/ffmpeg` and collide with the system ffmpeg package).
///
/// Checks, in order:
/// 1. `aircast-ffmpeg` next to the running executable — where Tauri places
///    sidecars in bundled apps and copies them in dev when `externalBin`
///    is configured.
/// 2. `aircast-ffmpeg` in the Tauri resource directory (some Linux paths).
/// 3. System `ffmpeg` on PATH — the dev fallback for contributors who haven't
///    run `pnpm fetch-ffmpeg`.
pub fn resolve(app: &AppHandle) -> PathBuf {
    let sidecar_name = if cfg!(windows) {
        "aircast-ffmpeg.exe"
    } else {
        "aircast-ffmpeg"
    };

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join(sidecar_name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join(sidecar_name);
        if candidate.exists() {
            return candidate;
        }
    }

    PathBuf::from(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}
