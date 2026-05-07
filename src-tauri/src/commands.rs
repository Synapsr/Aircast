use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, capture, AudioDevice};
use crate::error::{AppError, AppResult};
use crate::presets::{Mode, Preset, Settings, StreamConfig};
use crate::state::AppState;
use crate::stream;
use crate::studio::music::scan_full_duration;
use crate::studio::{CartSlot, CartSnapshot, MusicSnapshot, TrackInfo};
use crate::vu::{self, VuEmitter};

// ──────────────────── devices ────────────────────

#[tauri::command]
pub fn list_audio_devices() -> AppResult<Vec<AudioDevice>> {
    audio::list_input_devices()
}

// ──────────────────── audio capture (always-on while a device is selected) ────────────────────

#[tauri::command]
pub fn start_audio_preview(
    device_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    // Hold the capture mutex for the entire operation so two parallel
    // invocations can't race and leave duplicate cpal streams running.
    let mut guard = state.capture.lock().unwrap();
    // Drop the previous session in place — its `Drop` joins the worker
    // thread, which drops the cpal stream synchronously.
    *guard = None;

    let format = capture::get_input_format(&device_id)?;
    state.capture_ctx.set_format(Some(format));
    state
        .mixer
        .set_target_format(format.sample_rate, format.channels);

    let mixer = state.mixer.clone();
    let ctx = state.capture_ctx.clone();
    let mut vu = VuEmitter::new(app.clone());

    let mut output_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut music_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut cart_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut bytes_buf: Vec<u8> = Vec::with_capacity(16_384);

    let session = capture::start_capture(&device_id, move |mic_samples| {
        mixer.process(mic_samples, &mut output_buf, &mut music_buf, &mut cart_buf);
        vu.push(&output_buf);

        if let Ok(slot) = ctx.stream_tx.try_lock() {
            if let Some(tx) = slot.as_ref() {
                bytes_buf.clear();
                bytes_buf.reserve(output_buf.len() * 4);
                for s in output_buf.iter() {
                    bytes_buf.extend_from_slice(&s.to_le_bytes());
                }
                let _ = tx.try_send(std::mem::take(&mut bytes_buf));
                bytes_buf = Vec::with_capacity(16_384);
            }
        }
    })?;

    *guard = Some(session);
    Ok(())
}

#[tauri::command]
pub fn stop_audio_preview(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    *state.capture.lock().unwrap() = None;
    state.capture_ctx.set_format(None);
    vu::emit_zero(&app);
    Ok(())
}

// ──────────────────── streaming ────────────────────

#[tauri::command]
pub async fn start_stream(
    config: StreamConfig,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    let mut stream_guard = state.stream.lock().await;
    if let Some(old) = stream_guard.take() {
        old.stop().await;
    }

    let format = state.capture_ctx.format().ok_or_else(|| {
        AppError::Stream("No input device active. Pick a microphone first.".into())
    })?;

    let settings = state.presets.settings();
    let handle = stream::start(
        app.clone(),
        config,
        settings,
        format,
        state.capture_ctx.clone(),
    );
    *stream_guard = Some(handle);

    Ok(())
}

#[tauri::command]
pub async fn stop_stream(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let mut stream_guard = state.stream.lock().await;
    if let Some(handle) = stream_guard.take() {
        handle.stop().await;
    }
    state.capture_ctx.set_stream_tx(None);
    stream::emit_status(&app, stream::StreamStatus::Idle);
    Ok(())
}

// ──────────────────── presets / settings ────────────────────

#[tauri::command]
pub fn load_presets(state: tauri::State<'_, AppState>) -> AppResult<Vec<Preset>> {
    Ok(state.presets.list_presets())
}

#[tauri::command]
pub fn save_preset(
    name: String,
    config: StreamConfig,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::Preset("preset name cannot be empty".into()));
    }
    state.presets.upsert_preset(name, config)
}

#[tauri::command]
pub fn delete_preset(name: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.presets.delete_preset(&name)
}

#[tauri::command]
pub fn rename_preset(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::Preset("preset name cannot be empty".into()));
    }
    state.presets.rename_preset(&old_name, trimmed)
}

#[tauri::command]
pub fn load_settings(state: tauri::State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.presets.settings())
}

#[tauri::command]
pub fn save_settings(settings: Settings, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .mixer
        .set_duck_amount(1.0 - settings.music_volume_when_mic_open.clamp(0.0, 1.0));
    state
        .mixer
        .set_crossfade_seconds(settings.crossfade_seconds);
    state.presets.save_settings(settings)
}

#[tauri::command]
pub fn load_current_config(state: tauri::State<'_, AppState>) -> AppResult<Option<StreamConfig>> {
    Ok(state.presets.current_config())
}

#[tauri::command]
pub fn save_current_config(
    config: StreamConfig,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    state.presets.save_current_config(config)
}

// ──────────────────── mode ────────────────────

#[tauri::command]
pub fn get_mode(state: tauri::State<'_, AppState>) -> AppResult<Mode> {
    Ok(state.presets.mode())
}

#[tauri::command]
pub fn set_mode(mode: Mode, state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    match mode {
        Mode::Simple => state.mixer.disable_studio(),
        Mode::Studio => state.mixer.enable_studio(),
    }
    state.presets.save_mode(mode)?;
    let _ = app.emit("studio-state-changed", ());
    let _ = app.emit("music-state-changed", ());
    let _ = app.emit("cart-state-changed", ());
    Ok(())
}

// ──────────────────── mic gate ────────────────────

#[tauri::command]
pub fn set_mic_open(
    open: bool,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state.mixer.set_mic_open(open);
    let _ = app.emit("mic-state-changed", open);
    Ok(())
}

#[tauri::command]
pub fn get_mic_open(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.mixer.is_mic_open())
}

// ──────────────────── monitor (local speaker return) ────────────────────

#[tauri::command]
pub fn set_monitor_muted(
    muted: bool,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state.mixer.set_monitor_muted(muted);
    let _ = app.emit("monitor-state-changed", muted);
    Ok(())
}

#[tauri::command]
pub fn get_monitor_muted(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.mixer.is_monitor_muted())
}

// ──────────────────── music queue ────────────────────

#[tauri::command]
pub fn music_enqueue(
    paths: Vec<PathBuf>,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<Vec<TrackInfo>> {
    let mut added = Vec::new();
    {
        let mut music = state
            .mixer
            .music
            .lock()
            .map_err(|_| AppError::Stream("music lock poisoned".into()))?;
        for path in paths {
            if !path.exists() {
                continue;
            }
            let info = TrackInfo::from_path(path);
            music.enqueue(info.clone());
            added.push(info);
        }
    }
    // Spawn background scans for any track without a known duration. Each
    // scan decodes the file end-to-end to count samples, then sets a duration
    // override and re-emits the snapshot so the UI can show the progress bar.
    for info in &added {
        if info.duration_secs.is_some() {
            continue;
        }
        let app_clone = app.clone();
        let track_id = info.id.clone();
        let path = info.path.clone();
        std::thread::spawn(move || {
            let Some(dur) = scan_full_duration(&path) else {
                return;
            };
            let snap = {
                let Some(state) = app_clone.try_state::<AppState>() else {
                    return;
                };
                state.mixer.set_music_duration_override(&track_id, dur);
                log::info!("music duration scanned: {} → {:.2}s", path.display(), dur);
                state.mixer.music.lock().ok().map(|m| m.snapshot())
            };
            if let Some(snap) = snap {
                let _ = app_clone.emit("music-state-changed", snap);
            }
        });
    }
    emit_music_state(&app, &state);
    Ok(added)
}

#[tauri::command]
pub fn music_remove(
    id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .remove(&id);
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_move(
    id: String,
    delta: i32,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .move_track(&id, delta);
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_play(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .play()
        .map_err(AppError::Stream)?;
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_pause(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .pause();
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_stop(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .stop();
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_next(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .next_track()
        .map_err(AppError::Stream)?;
    emit_music_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn music_snapshot(state: tauri::State<'_, AppState>) -> AppResult<MusicSnapshot> {
    Ok(state
        .mixer
        .music
        .lock()
        .map_err(|_| AppError::Stream("music lock poisoned".into()))?
        .snapshot())
}

// ──────────────────── carts ────────────────────

#[tauri::command]
pub fn cart_assign(
    slot: u8,
    name: String,
    path: PathBuf,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<CartSlot> {
    let info = state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .assign(slot, name, path)
        .map_err(AppError::Stream)?;

    persist_carts(&state)?;
    emit_cart_state(&app, &state);
    Ok(info)
}

#[tauri::command]
pub fn cart_remove(slot: u8, state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .remove(slot);
    persist_carts(&state)?;
    emit_cart_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn cart_play(slot: u8, state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .play(slot);
    emit_cart_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn cart_stop(slot: u8, state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .stop(slot);
    emit_cart_state(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn cart_snapshot(state: tauri::State<'_, AppState>) -> AppResult<Vec<CartSnapshot>> {
    Ok(state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .snapshot())
}

// ──────────────────── external links ────────────────────

/// Open a URL in the user's default browser. We avoid pulling in
/// `tauri-plugin-shell` for a single use case — the cost of one tiny
/// platform-specific spawn is much less than dragging in another plugin
/// (and its capability surface) for a single call site.
#[tauri::command]
pub fn open_external(url: String) -> AppResult<()> {
    // Refuse anything that isn't an https/http URL so this command can never
    // be coerced into running an arbitrary local command.
    let lower = url.to_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(AppError::Stream(format!("refusing to open: {url}")));
    }

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&url).spawn();

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&url).spawn();

    result
        .map(|_| ())
        .map_err(|e| AppError::Stream(format!("open url: {e}")))
}

// ──────────────────── helpers ────────────────────

fn emit_music_state(app: &AppHandle, state: &tauri::State<'_, AppState>) {
    if let Ok(music) = state.mixer.music.lock() {
        let _ = app.emit("music-state-changed", music.snapshot());
    }
}

fn emit_cart_state(app: &AppHandle, state: &tauri::State<'_, AppState>) {
    if let Ok(carts) = state.mixer.carts.lock() {
        let _ = app.emit("cart-state-changed", carts.snapshot());
    }
}

fn persist_carts(state: &tauri::State<'_, AppState>) -> AppResult<()> {
    let snapshot = state
        .mixer
        .carts
        .lock()
        .map_err(|_| AppError::Stream("carts lock poisoned".into()))?
        .persisted();
    state.presets.save_carts(snapshot)
}
