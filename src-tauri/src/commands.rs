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
    // Entering Simple/Studio: ensure any Relay input is stopped so the cpal
    // mic owns the audio source exclusively.
    *state.relay.lock().unwrap() = None;

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

    let consumer = make_passthrough_consumer(state.mixer.clone(), state.capture_ctx.clone(), &app);
    let session = capture::start_capture(&device_id, consumer)?;

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

/// Build the consumer closure shared by the cpal capture (Simple/Studio) and
/// the relay url-input (Relay). Both produce mono f32 PCM that goes through
/// the mixer to the monitor ring and the active ffmpeg encoder.
fn make_passthrough_consumer(
    mixer: std::sync::Arc<crate::studio::Mixer>,
    ctx: crate::state::CaptureContext,
    app: &AppHandle,
) -> impl FnMut(&[f32]) + Send + 'static {
    let mut vu = VuEmitter::new(app.clone());
    let mut output_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut music_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut cart_buf: Vec<f32> = Vec::with_capacity(4096);
    let mut bytes_buf: Vec<u8> = Vec::with_capacity(16_384);
    move |samples: &[f32]| {
        mixer.process(samples, &mut output_buf, &mut music_buf, &mut cart_buf);
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
    }
}

// ──────────────────── relay (URL input → Icecast) ────────────────────

#[tauri::command]
pub fn list_relay_sources(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<crate::presets::RelaySource>> {
    Ok(state.presets.relay_sources())
}

#[tauri::command]
pub fn upsert_relay_source(
    source: crate::presets::RelaySource,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    state.presets.upsert_relay_source(source)
}

#[tauri::command]
pub fn delete_relay_source(name: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.presets.delete_relay_source(&name)
}

#[tauri::command]
pub fn rename_relay_source(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    state.presets.rename_relay_source(&old_name, &new_name)
}

#[tauri::command]
pub fn set_active_relay_source(
    name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    state.presets.set_active_relay_source(name)
}

/// Spawn the upstream URL decoder for Relay mode. Mutually exclusive with
/// the cpal mic capture: starting a relay input stops any active mic session
/// (and vice versa). Looks up the URL from the named relay source persisted
/// in settings.
#[tauri::command]
pub fn start_relay_input(
    source_name: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    // Find the URL.
    let source = state
        .presets
        .relay_sources()
        .into_iter()
        .find(|s| s.name == source_name)
        .ok_or_else(|| AppError::Stream(format!("relay source '{source_name}' not found")))?;

    log::info!(
        "start_relay_input: source='{}' url='{}'",
        source.name,
        source.url
    );

    // Tear down any cpal mic session — relay owns the input now.
    *state.capture.lock().unwrap() = None;
    let mut guard = state.relay.lock().unwrap();
    *guard = None;

    // Announce the format we're going to feed the mixer at.
    let format = crate::audio::capture::AudioFormat {
        sample_rate: crate::audio::url_input::RELAY_RATE,
        channels: crate::audio::url_input::RELAY_CHANNELS,
    };
    state.capture_ctx.set_format(Some(format));
    state
        .mixer
        .set_target_format(format.sample_rate, format.channels);

    // Persist the selection so a restart restores it.
    state
        .presets
        .set_active_relay_source(Some(source.name.clone()))?;

    let consumer = make_passthrough_consumer(state.mixer.clone(), state.capture_ctx.clone(), &app);
    let session = crate::audio::url_input::start_relay_input(app.clone(), source.url, consumer)?;
    *guard = Some(session);
    Ok(())
}

#[tauri::command]
pub fn stop_relay_input(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    *state.relay.lock().unwrap() = None;
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
        log::info!("start_stream: stopping previous stream first");
        old.stop().await;
    }

    let format = state.capture_ctx.format().ok_or_else(|| {
        log::error!("start_stream rejected: no input device active");
        AppError::Stream("No input device active. Pick a microphone first.".into())
    })?;

    let target_display = match config.transport {
        crate::presets::Transport::Icecast => format!(
            "{}:{}{}",
            config.host,
            config.port,
            config.normalized_mount()
        ),
        crate::presets::Transport::Webcast => config.webcast_url(),
    };
    log::info!(
        "start_stream: transport={:?} target={} codec={:?}@{}kbps source={}Hz/{}ch",
        config.transport,
        target_display,
        config.format,
        config.bitrate,
        format.sample_rate,
        format.channels,
    );

    let settings = state.presets.settings();

    // Shared slot the webcast transport fills once its socket is up, so the
    // metadata updater can send in-band frames without knowing anything about
    // connection lifecycles. Unused by the Icecast transport.
    let webcast_sink = stream::MetadataSink::new();

    let handle = stream::start(
        app.clone(),
        config.clone(),
        settings,
        format,
        state.capture_ctx.clone(),
        webcast_sink.clone(),
    );
    *stream_guard = Some(handle);

    // Tell the metadata updater where to push titles. Pushes are gated on
    // `stream_live`, so the updater stays quiet until the pipeline reaches
    // the Live state.
    let target = stream::metadata::PushTarget::from_config(&config, &webcast_sink);
    let _ = state
        .metadata_tx
        .try_send(stream::metadata::Command::SetTarget(Some(target)));

    Ok(())
}

#[tauri::command]
pub async fn stop_stream(state: tauri::State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    log::info!("stop_stream: requested");
    let mut stream_guard = state.stream.lock().await;
    if let Some(handle) = stream_guard.take() {
        handle.stop().await;
    }
    state.capture_ctx.set_stream_tx(None);
    stream::emit_status(&app, stream::StreamStatus::Idle);
    let _ = state
        .metadata_tx
        .try_send(stream::metadata::Command::SetTarget(None));
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

    // Side-effects on metadata config: push to updater and (re)start the
    // file watcher if mode changed to/from File or the path/poll changed.
    let _ = state
        .metadata_tx
        .try_send(crate::stream::metadata::Command::SetSettings(
            settings.metadata.clone(),
        ));
    sync_metadata_file_watcher(&state, &settings.metadata);

    state.presets.save_settings(settings)
}

/// Aborts any running file watcher and spawns a fresh one if the user is
/// in File mode with a path set. Idempotent — safe to call on every settings
/// save even when the file mode hasn't changed.
fn sync_metadata_file_watcher(
    state: &tauri::State<'_, AppState>,
    settings: &crate::presets::MetadataSettings,
) {
    let mut slot = state.metadata_file_watcher.lock().unwrap();
    if let Some(handle) = slot.take() {
        handle.abort();
    }
    if settings.mode == crate::presets::MetadataMode::File {
        if let Some(path) = settings.file_path.clone() {
            let handle = crate::stream::metadata::spawn_file_watcher(
                path,
                settings.file_poll_secs,
                state.metadata_file_content.clone(),
            );
            *slot = Some(handle);
        } else {
            // Mode is File but no path: clear stale content so we don't
            // keep pushing the previous file's last value.
            let content = state.metadata_file_content.clone();
            tauri::async_runtime::spawn(async move {
                *content.lock().await = None;
            });
        }
    } else {
        // Switching out of File mode: also clear content cache.
        let content = state.metadata_file_content.clone();
        tauri::async_runtime::spawn(async move {
            *content.lock().await = None;
        });
    }
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
        // Simple and Relay both want passthrough mixing: the mic (Simple) or
        // the decoded upstream stream (Relay) flows straight to the output.
        Mode::Simple | Mode::Relay => state.mixer.disable_studio(),
        Mode::Studio => state.mixer.enable_studio(),
    }
    state.presets.save_mode(mode)?;
    let _ = app.emit("studio-state-changed", ());
    let _ = app.emit("music-state-changed", ());
    let _ = app.emit("cart-state-changed", ());
    // Mic gating semantics differ between modes (Simple = passthrough, Studio
    // = gated by mic_gain). enable/disable_studio touches mic_gain_target
    // directly, so we must broadcast the new mic state ourselves — otherwise
    // the StatusBar shows stale info after a mode flip.
    let _ = app.emit("mic-state-changed", state.mixer.is_mic_open());
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

// ──────────────────── metadata broadcaster ────────────────────

/// Push the given title (or, if None, the title currently composed from
/// state + settings) to Icecast immediately, bypassing the dedup. Used by
/// the "Test now" button in Setup → Advanced so the user can verify the
/// pipeline end-to-end.
#[tauri::command]
pub async fn push_metadata_now(
    title: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let title = match title.map(|s| s.trim().to_string()) {
        Some(s) if !s.is_empty() => s,
        _ => {
            // Compose from current state.
            let settings = state.presets.settings();
            let music = state
                .mixer
                .music
                .lock()
                .map(|m| m.snapshot())
                .map_err(|_| AppError::Stream("music lock poisoned".into()))?;
            let carts = state
                .mixer
                .carts
                .lock()
                .map(|c| c.snapshot())
                .map_err(|_| AppError::Stream("carts lock poisoned".into()))?;
            let mic_open = state.mixer.is_mic_open();
            let file_content = state.metadata_file_content.lock().await.clone();
            let stream_live = state
                .stream_status
                .lock()
                .map(|s| s.is_live())
                .unwrap_or(false);
            let input = crate::stream::metadata::build_compose_input(
                &music,
                &carts,
                mic_open,
                file_content,
                stream_live,
            );
            crate::stream::metadata::compose_title(&input, &settings.metadata)
        }
    };

    if title.is_empty() {
        return Err(AppError::Stream(
            "Nothing to push: composed title is empty.".into(),
        ));
    }
    state
        .metadata_tx
        .send(crate::stream::metadata::Command::PushNow(title))
        .await
        .map_err(|e| AppError::Stream(format!("metadata channel closed: {e}")))?;
    Ok(())
}

// ──────────────────── diagnostics ────────────────────

/// Build a single-shot diagnostic bundle: app metadata, host environment,
/// active audio/streaming context, the last classified stream error and the
/// tail of the rolling log file. The frontend exposes this as a "Copy
/// diagnostic" button so a non-technical user can paste a clean report
/// into a bug report or email.
///
/// Credentials are masked. The mount path is included verbatim because it
/// rarely contains a secret and it's load-bearing for diagnosing failures.
#[tauri::command]
pub fn get_diagnostic_bundle(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<String> {
    use std::fmt::Write;

    let mut s = String::new();

    // ── header ──
    let _ = writeln!(s, "Aircast {} diagnostic", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(
        s,
        "Build: {}  ·  Target: {}/{}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    let now = chrono_like_now();
    let _ = writeln!(s, "Generated: {now}");

    // ── audio capture ──
    s.push_str("\n=== Audio ===\n");
    if let Some(format) = state.capture_ctx.format() {
        let _ = writeln!(
            s,
            "Active format: {} Hz, {} ch",
            format.sample_rate, format.channels
        );
    } else {
        s.push_str("Active format: (none — no input device selected)\n");
    }

    // ── streaming config (sanitized) ──
    s.push_str("\n=== Active server (sanitized) ===\n");
    if let Some(cfg) = state.presets.current_config() {
        let _ = writeln!(s, "host: {}", cfg.host);
        let _ = writeln!(s, "port: {}", cfg.port);
        let _ = writeln!(s, "mount: {}", cfg.mount);
        let _ = writeln!(s, "username: {}", cfg.username);
        let _ = writeln!(
            s,
            "password: {}",
            if cfg.password.is_empty() {
                "(empty)"
            } else {
                "(set, masked)"
            }
        );
        let _ = writeln!(s, "format: {:?}", cfg.format);
        let _ = writeln!(s, "bitrate: {} kbps", cfg.bitrate);
        let _ = writeln!(s, "device id: {}", cfg.device_id);
    } else {
        s.push_str("(no active server config)\n");
    }

    // ── settings (relevant subset) ──
    s.push_str("\n=== Settings ===\n");
    let st = state.presets.settings();
    let _ = writeln!(
        s,
        "reconnect_interval_seconds: {}",
        st.reconnect_interval_seconds
    );
    let _ = writeln!(
        s,
        "music_volume_when_mic_open: {:.2}",
        st.music_volume_when_mic_open
    );
    let _ = writeln!(s, "crossfade_seconds: {:.2}", st.crossfade_seconds);
    let _ = writeln!(s, "language: {}", st.language);
    let _ = writeln!(s, "mode: {:?}", state.presets.mode());

    // ── last stream error (if any) ──
    s.push_str("\n=== Last stream error ===\n");
    if let Ok(slot) = state.last_stream_error.lock() {
        if let Some(err) = slot.as_ref() {
            let secs_ago = err.at.elapsed().map(|d| d.as_secs()).unwrap_or(0);
            let _ = writeln!(s, "{}s ago: {}", secs_ago, err.message);
            if let Some(d) = &err.details {
                s.push_str("--- ffmpeg tail ---\n");
                s.push_str(d);
                s.push('\n');
            }
        } else {
            s.push_str("(none)\n");
        }
    }

    // ── log tail ──
    s.push_str("\n=== Recent log (last 300 lines) ===\n");
    match read_log_tail(&app, 300) {
        Ok(lines) if !lines.is_empty() => s.push_str(&lines),
        Ok(_) => s.push_str("(log file is empty)\n"),
        Err(e) => {
            let _ = writeln!(s, "(could not read log file: {e})");
        }
    }

    Ok(s)
}

/// Read up to `max_lines` from the end of the rolling log file. Tries the
/// canonical Tauri log directory first; falls back gracefully when the
/// file doesn't exist yet (e.g. fresh install with no errors).
fn read_log_tail(app: &AppHandle, max_lines: usize) -> std::io::Result<String> {
    use tauri::Manager;
    let log_dir = app.path().app_log_dir().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("app_log_dir unavailable: {e}"),
        )
    })?;
    let log_file = log_dir.join("Aircast.log");
    if !log_file.exists() {
        return Ok(format!("(no log file yet at {})\n", log_file.display()));
    }
    let contents = std::fs::read_to_string(&log_file)?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n") + "\n")
}

/// Tiny ISO-8601-ish UTC formatter so we don't pull in chrono just for the
/// diagnostic header. Format: `2026-05-09 14:32:07 UTC`.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d, hh, mm, ss) = epoch_to_ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
}

/// Convert seconds-since-epoch to (year, month, day, hour, minute, second)
/// in UTC. Handles dates well past 2099.
fn epoch_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let mut day = (secs / 86_400) as i64;
    let ss = (secs % 60) as u32;
    let mm = ((secs / 60) % 60) as u32;
    let hh = ((secs / 3_600) % 24) as u32;
    // 1970-01-01 = day 0
    let mut year: i64 = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let len = if leap { 366 } else { 365 };
        if day < len {
            break;
        }
        day -= len;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month: i64 = 1;
    for &len in &months {
        if day < len {
            break;
        }
        day -= len;
        month += 1;
    }
    (year as u32, month as u32, (day + 1) as u32, hh, mm, ss)
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
