mod audio;
mod commands;
mod error;
mod presets;
mod state;
mod stream;
mod studio;
mod vu;

use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::presets::Mode;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let handle = app.handle();

            // Register the log plugin first so AppState::new boot logs are captured.
            if cfg!(debug_assertions) {
                handle.plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

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

            app.manage(app_state);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
