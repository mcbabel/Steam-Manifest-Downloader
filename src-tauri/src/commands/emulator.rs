use smd_core::ops::emulator::{self as emu_ops, DlcMergePlan};
use smd_core::services::emulator::{
    self, EmuSettings, Platform, ReleaseInfo, ReplaceResult, ScannedFile, Variant,
};
use smd_core::services::AppState;
use tauri::{command, AppHandle};

use super::{app_data_dir, sink};

#[command]
pub async fn emu_release_info(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<ReleaseInfo, String> {
    emulator::fetch_release_info(&state.http_client, &app_data_dir(&app)).await
}

#[command]
pub async fn emu_ensure_cached(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    platform: Platform,
) -> Result<ReleaseInfo, String> {
    emu_ops::ensure_cached(&state, &app_data_dir(&app), platform).await
}

#[command]
pub async fn emu_scan_game_dir(game_dir: String) -> Result<Vec<ScannedFile>, String> {
    emu_ops::scan_game_dir(&game_dir).await
}

#[command]
pub async fn emu_scan_for_dlc_merge(
    state: tauri::State<'_, AppState>,
    game_dir: String,
    app_id: Option<String>,
) -> Result<Option<DlcMergePlan>, String> {
    emu_ops::scan_for_dlc_merge(&state, game_dir, app_id).await
}

#[command]
pub async fn emu_merge_dlc_depots(
    main_depot_dir: String,
    dlc_depot_dirs: Vec<String>,
) -> Result<u64, String> {
    emu_ops::merge_dlc_depots(main_depot_dir, dlc_depot_dirs).await
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn emu_apply_replacement(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<ScannedFile>,
    variant: Variant,
    app_id: String,
    installed_app_ids: Vec<String>,
    emu_settings: Option<EmuSettings>,
    allow_download: bool,
) -> Result<Vec<ReplaceResult>, String> {
    emu_ops::apply_replacement(
        &sink(&app),
        &state,
        &app_data_dir(&app),
        targets,
        variant,
        app_id,
        installed_app_ids,
        emu_settings,
        allow_download,
    )
    .await
}

#[command]
pub async fn emu_read_emu_settings(target_path: String) -> Result<EmuSettings, String> {
    emu_ops::read_emu_settings(&target_path)
}

#[command]
pub async fn emu_write_emu_settings(
    target_path: String,
    settings: EmuSettings,
) -> Result<(), String> {
    emu_ops::write_emu_settings(&target_path, &settings)
}

#[command]
pub async fn emu_revert_replacement(targets: Vec<String>) -> Result<Vec<ReplaceResult>, String> {
    emu_ops::revert_replacement(targets).await
}
