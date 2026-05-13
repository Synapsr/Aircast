mod audio;
mod commands;
mod error;
mod presets;
mod state;
mod stream;
mod studio;
mod vu;

use std::time::Duration;

use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::presets::Mode;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK 2.42+ uses a DMA-BUF renderer that fails to initialize EGL
    // on several Ubuntu 24.10+ / mesa combinations, especially inside an
    // AppImage. The crash is `Could not create default EGL display:
    // EGL_BAD_PARAMETER. Aborting...` before the first window appears.
    // Upstream WebKit ships this env var as the official fallback to the
    // older shared-memory renderer — visually identical for an app like
    // Aircast (no WebGL, no HTML5 video). Honour any explicit user choice.
    //
    // Single-threaded at this point (no Tokio, no Tauri builder, no
    // WebKit), so `set_var` is sound even if glibc's setenv is not
    // thread-safe.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let handle = app.handle();

            // Register the log plugin first so AppState::new boot logs are captured.
            // We write to BOTH stdout (visible during dev) and a rotating file
            // in the OS log directory (always available in release for support).
            //
            // macOS:   ~/Library/Logs/com.aircast.app/Aircast.log
            // Windows: %LOCALAPPDATA%\com.aircast.app\logs\Aircast.log
            // Linux:   ~/.local/share/com.aircast.app/logs/Aircast.log
            //
            // The "Copy diagnostic" button in Setup → Advanced reads the latest
            // file content, so users can paste a clean report into an issue.
            handle.plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .targets([
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("Aircast".into()),
                        }),
                    ])
                    .max_file_size(5 * 1024 * 1024) // 5 MB per file
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .build(),
            )?;
            log::info!(
                "Aircast {} starting ({})",
                env!("CARGO_PKG_VERSION"),
                if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                }
            );

            let app_state = AppState::new(handle)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

            // Restore persisted mode + cart slots + ducking.
            let restored_mode = app_state.presets.mode();
            if matches!(restored_mode, Mode::Studio) {
                app_state.mixer.enable_studio();
            }
            let s = app_state.presets.settings();
            app_state
                .mixer
                .set_duck_amount(1.0 - s.music_volume_when_mic_open.clamp(0.0, 1.0));
            app_state.mixer.set_crossfade_seconds(s.crossfade_seconds);
            for cart in app_state.presets.carts() {
                if let Ok(mut bank) = app_state.mixer.carts.lock() {
                    let _ = bank.assign(cart.slot, cart.name, cart.path);
                }
            }

            // Push restored metadata settings to the updater before any
            // stream may start. SetTarget(None) keeps it dormant.
            let _ = app_state
                .metadata_tx
                .try_send(stream::metadata::Command::SetSettings(s.metadata.clone()));

            // If the user persisted File mode with a path, boot the watcher
            // immediately so the file content is already available when the
            // user clicks Go Live.
            if matches!(s.metadata.mode, presets::MetadataMode::File) {
                if let Some(path) = s.metadata.file_path.clone() {
                    let handle = stream::metadata::spawn_file_watcher(
                        path,
                        s.metadata.file_poll_secs,
                        app_state.metadata_file_content.clone(),
                    );
                    *app_state.metadata_file_watcher.lock().unwrap() = Some(handle);
                }
            }

            app.manage(app_state);

            // ── State poller for the metadata updater ─────────────────────
            // Every ~750 ms, snapshot music + cart + mic + file content and
            // send a Tick so the updater can recompute and push if changed.
            // 750 ms is the longest a listener tolerates between a track
            // change and seeing the new title without feeling sluggish.
            let app_for_poller = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(750));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    let Some(state) = app_for_poller.try_state::<AppState>() else {
                        continue;
                    };
                    let stream_live = state
                        .stream_status
                        .lock()
                        .map(|s| s.is_live())
                        .unwrap_or(false);
                    let music = state.mixer.music.lock().map(|m| m.snapshot()).ok();
                    let carts = state
                        .mixer
                        .carts
                        .lock()
                        .map(|c| c.snapshot())
                        .unwrap_or_default();
                    let mic_open = state.mixer.is_mic_open();
                    let file_content = state.metadata_file_content.lock().await.clone();
                    if let Some(music) = music {
                        let input = stream::metadata::build_compose_input(
                            &music,
                            &carts,
                            mic_open,
                            file_content,
                            stream_live,
                        );
                        let _ = state
                            .metadata_tx
                            .try_send(stream::metadata::Command::Tick(Box::new(input)));
                    }
                }
            });

            // Forward deep-link URLs to the frontend.
            let handle_for_links = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    log::info!("deep link received: {}", url);
                    let _ = handle_for_links.emit("deep-link-url", url.to_string());
                }
            });

            // In dev mode, try to register the URL scheme dynamically. On
            // macOS this only works for bundled apps — Launch Services can't
            // register a bare debug binary, so users testing in dev should
            // paste links via the Setup modal instead.
            #[cfg(any(target_os = "linux", debug_assertions))]
            {
                match app.deep_link().register("aircast") {
                    Ok(_) => log::info!("deep link scheme 'aircast' registered"),
                    Err(e) => log::info!(
                        "deep link runtime register failed (expected in dev on macOS): {e}"
                    ),
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::start_audio_preview,
            commands::stop_audio_preview,
            commands::start_stream,
            commands::stop_stream,
            commands::load_presets,
            commands::save_preset,
            commands::delete_preset,
            commands::rename_preset,
            commands::load_settings,
            commands::save_settings,
            commands::load_current_config,
            commands::save_current_config,
            commands::get_mode,
            commands::set_mode,
            commands::set_mic_open,
            commands::get_mic_open,
            commands::set_monitor_muted,
            commands::get_monitor_muted,
            commands::music_enqueue,
            commands::music_remove,
            commands::music_move,
            commands::music_play,
            commands::music_pause,
            commands::music_stop,
            commands::music_next,
            commands::music_snapshot,
            commands::cart_assign,
            commands::cart_remove,
            commands::cart_play,
            commands::cart_stop,
            commands::cart_snapshot,
            commands::open_external,
            commands::get_diagnostic_bundle,
            commands::push_metadata_now,
            commands::list_relay_sources,
            commands::upsert_relay_source,
            commands::delete_relay_source,
            commands::rename_relay_source,
            commands::set_active_relay_source,
            commands::start_relay_input,
            commands::stop_relay_input,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
