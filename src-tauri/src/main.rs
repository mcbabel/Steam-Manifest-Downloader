#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use smd_core::services;
use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();

            let app_version = app.config().version.clone().unwrap_or_default();
            let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local").to_string();

            let telemetry = services::telemetry::Telemetry::new(
                app_data.clone(),
                app_version,
                channel,
            );
            tauri::async_runtime::spawn(telemetry.clone().run_background_flush());

            let mut state = services::AppState::new();
            state.telemetry = Some(telemetry);
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // File operations
            commands::parse_lua_file,
            commands::parse_lua_content,
            // Search
            commands::search_repos,
            commands::get_repo_manifests,
            commands::search_steam_games,
            commands::fetch_depot_metadata,
            commands::fetch_depot_metadata_steam,
            commands::fetch_latest_manifest_id,
            // Steam
            commands::get_steam_app_info,
            // Download
            commands::start_download,
            commands::cancel_download,
            commands::pause_download,
            // Settings
            commands::get_settings,
            commands::save_settings,
            // System
            commands::check_dotnet,
            commands::get_disk_space,
            commands::check_free_space,
            commands::set_download_progress,
            commands::get_installed_depots,
            commands::check_game_updates,
            commands::get_launch_exe,
            commands::launch_game,
            commands::power_off_system,
            commands::get_build_info,
            commands::get_debug_log_path,
            // Window
            commands::minimize_window,
            commands::maximize_window,
            commands::close_window,
            commands::restart_app,
            // Updater
            commands::check_for_updates,
            commands::install_update,
            commands::get_auto_update_enabled,
            // History
            commands::get_history,
            commands::remove_history_entry,
            commands::clear_history,
            commands::record_history_entry,
            commands::save_pending_followup,
            commands::get_pending_followup,
            commands::clear_pending_followup,
            commands::open_folder,
            // Shortcuts
            commands::is_shortcut_supported,
            commands::detect_executables,
            commands::create_shortcuts,
            // Telemetry
            commands::get_telemetry_status,
            commands::set_telemetry_consent,
            commands::emit_telemetry_event,
            // Emulator (gbe_fork)
            commands::emu_release_info,
            commands::emu_ensure_cached,
            commands::emu_scan_game_dir,
            commands::emu_scan_for_dlc_merge,
            commands::emu_merge_dlc_depots,
            commands::emu_apply_replacement,
            commands::emu_revert_replacement,
            commands::emu_read_emu_settings,
            commands::emu_write_emu_settings,
            // Steam library (non-Steam shortcut + grid art)
            commands::steam_library_detect,
            commands::steam_library_add,
            // Steamless (DRM detection + removal)
            commands::steamless_scan,
            commands::steamless_unpack,
            // Steam API Check Bypass (version.dll DLL-hijack)
            commands::steam_api_bypass_apply,
            commands::steam_api_bypass_revert,
            commands::steam_api_bypass_status,
            // Native (pure-Rust) downloader — experimental, side-by-side with DepotDownloaderMod
            commands::native_download_depot,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<services::AppState>();
                if state.has_active_downloads() {
                    api.prevent_close();
                    let window = window.clone();
                    window.emit("close-requested", ()).ok();
                    return;
                }

                if state
                    .shutdown_flush_done
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    return;
                }

                let Some(telemetry) = state.telemetry.clone() else {
                    return;
                };
                api.prevent_close();
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        telemetry.flush(),
                    )
                    .await;
                    let _ = window.close();
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
