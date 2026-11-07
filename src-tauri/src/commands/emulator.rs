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
    prefer_local: Option<bool>,
    coldclient_exe: Option<String>,
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
        prefer_local.unwrap_or(false),
        coldclient_exe,
    )
    .await
}

#[command]
pub async fn emu_coldclient_shortcut(game_dir: String, back: bool, close_steam: bool) -> Result<Vec<String>, String> {
    Ok(emu_ops::coldclient_shortcut(game_dir, back, close_steam).await)
}

#[command]
pub async fn emu_generate_game_data(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<String>,
    app_id: String,
    language: Option<String>,
) -> Result<smd_core::services::game_data::GameDataResult, String> {
    emu_ops::generate_game_data(&state, &app_data_dir(&app), targets, app_id, language).await
}

#[command]
pub async fn emu_patch_info(target: String) -> Result<smd_core::services::emulator::PatchInfo, String> {
    Ok(emu_ops::patch_info(&target))
}

#[command]
pub async fn emu_drm_backups(game_dir: String) -> Result<Vec<smd_core::services::steamless::DrmBackup>, String> {
    Ok(emu_ops::drm_backups(game_dir).await)
}

#[command]
pub async fn emu_restore_drm(exe: String) -> Result<(), String> {
    emu_ops::restore_drm(&exe)
}

#[command]
pub async fn emu_controller_info(
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<smd_core::services::game_data::controller::ControllerInfo, String> {
    emu_ops::controller_info(&state, app_id).await
}

#[command]
pub async fn emu_write_controller(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<String>,
    app_id: String,
    layout_file: Option<String>,
) -> Result<smd_core::services::game_data::controller::ControllerResult, String> {
    emu_ops::write_controller(&state, &app_data_dir(&app), targets, app_id, layout_file).await
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
pub async fn emu_revert_replacement(targets: Vec<String>, close_steam: Option<bool>) -> Result<Vec<ReplaceResult>, String> {
    emu_ops::revert_replacement(targets, close_steam.unwrap_or(false)).await
}

use smd_core::services::emu_extras as extras;
use std::path::PathBuf;

#[command]
pub async fn emu_save_folders(target: String, app_id: String) -> Result<Vec<extras::SaveFolder>, String> {
    emu_ops::blocking(move || extras::save_folders(&PathBuf::from(target), &app_id)).await
}

#[command]
pub async fn emu_backup_saves(folder: String, dest: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::backup_saves(&PathBuf::from(folder), &PathBuf::from(dest))).await?
}

#[command]
pub async fn emu_restore_saves(folder: String, archive: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::restore_saves(&PathBuf::from(folder), &PathBuf::from(archive))).await?
}

#[command]
pub async fn emu_achievements(target: String, app_id: String, lang: String) -> Result<extras::AchievementView, String> {
    emu_ops::blocking(move || extras::achievements(&PathBuf::from(target), &app_id, &lang)).await
}

#[command]
pub async fn emu_reset_achievements(target: String, app_id: String, names: Option<Vec<String>>) -> Result<usize, String> {
    emu_ops::blocking(move || extras::reset_achievements(&PathBuf::from(target), &app_id, names.as_deref())).await?
}

#[command]
pub async fn emu_reset_stats(target: String, app_id: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::reset_stats(&PathBuf::from(target), &app_id)).await?
}

#[command]
pub async fn emu_image(path: String) -> Result<String, String> {
    emu_ops::blocking(move || extras::image_data_url(&PathBuf::from(path))).await?
}

#[command]
pub async fn emu_overlay_assets(target: String) -> Result<extras::OverlayAssets, String> {
    emu_ops::blocking(move || extras::overlay_assets(&PathBuf::from(target))).await
}

#[command]
pub async fn emu_installed_fonts() -> Result<Vec<extras::FontEntry>, String> {
    emu_ops::blocking(extras::installed_fonts).await
}

#[command]
pub async fn emu_set_font(targets: Vec<String>, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_font(&targets, &PathBuf::from(src))).await?
}

#[command]
pub async fn emu_remove_font(targets: Vec<String>) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_font(&targets)).await?
}

#[command]
pub async fn emu_set_avatar(targets: Vec<String>, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_avatar(&targets, &PathBuf::from(src))).await?
}

#[command]
pub async fn emu_remove_avatar(targets: Vec<String>) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_avatar(&targets)).await?
}

#[command]
pub async fn emu_set_sound(targets: Vec<String>, kind: String, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_sound(&targets, &kind, &PathBuf::from(src))).await?
}

#[command]
pub async fn emu_remove_sound(targets: Vec<String>, kind: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_sound(&targets, &kind)).await?
}

#[command]
pub async fn emu_broadcasts(target: String) -> Result<Vec<String>, String> {
    emu_ops::blocking(move || extras::broadcasts(&PathBuf::from(target))).await
}

#[command]
pub async fn emu_set_broadcasts(targets: Vec<String>, list: Vec<String>) -> Result<Vec<String>, String> {
    emu_ops::blocking(move || extras::set_broadcasts(&targets, &list)).await?
}

#[command]
pub async fn emu_lobby_connect(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    game_dir: String,
    target: String,
    allow_download: bool,
) -> Result<String, String> {
    let _ = game_dir;
    emu_ops::lobby_connect(&sink(&app), &state, &app_data_dir(&app), target, allow_download).await
}

#[command]
pub async fn emu_launch_console(path: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::launch_console(&PathBuf::from(path))).await?
}

#[command]
pub async fn emu_mods(target: String) -> Result<Vec<extras::ModEntry>, String> {
    emu_ops::blocking(move || extras::mods(&PathBuf::from(target))).await
}

#[command]
pub async fn emu_add_mod(
    state: tauri::State<'_, AppState>,
    targets: Vec<String>,
    app_id: String,
    id: String,
    src: String,
) -> Result<extras::AddedMod, String> {
    extras::add_mod(&state.http_client, &targets, &app_id, &id, &PathBuf::from(src)).await
}

#[command]
pub async fn emu_remove_mod(targets: Vec<String>, id: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_mod(&targets, &id)).await?
}
