use crate::backend::{parse, reply, Cx};
use smd_core::ops::emulator::{self as emu_ops, DlcMergePlan};
use smd_core::services::emulator::{
    self, EmuSettings, Platform, ReleaseInfo, ReplaceResult, ScannedFile, Variant,
};


pub async fn emu_release_info(app: &Cx) -> Result<ReleaseInfo, String> {
    let state = app.state();
    emulator::fetch_release_info(&state.http_client, &app.data_dir()).await
}

pub async fn emu_ensure_cached(app: &Cx, platform: Platform) -> Result<ReleaseInfo, String> {
    let state = app.state();
    emu_ops::ensure_cached(&state, &app.data_dir(), platform).await
}

pub async fn emu_scan_game_dir(game_dir: String) -> Result<Vec<ScannedFile>, String> {
    emu_ops::scan_game_dir(&game_dir).await
}

pub async fn emu_scan_for_dlc_merge(app: &Cx, game_dir: String, app_id: Option<String>) -> Result<Option<DlcMergePlan>, String> {
    let state = app.state();
    emu_ops::scan_for_dlc_merge(&state, game_dir, app_id).await
}

pub async fn emu_merge_dlc_depots(main_depot_dir: String, dlc_depot_dirs: Vec<String>) -> Result<u64, String> {
    emu_ops::merge_dlc_depots(main_depot_dir, dlc_depot_dirs).await
}

#[allow(clippy::too_many_arguments)]
pub async fn emu_apply_replacement(
    app: &Cx,
    targets: Vec<ScannedFile>,
    variant: Variant,
    app_id: String,
    installed_app_ids: Vec<String>,
    emu_settings: Option<EmuSettings>,
    allow_download: bool,
    prefer_local: Option<bool>,
    coldclient_exe: Option<String>,
) -> Result<Vec<ReplaceResult>, String> {
    let state = app.state();
    emu_ops::apply_replacement(
        &app.sink(),
        &state,
        &app.data_dir(),
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

pub async fn emu_coldclient_shortcut(game_dir: String, back: bool, close_steam: bool) -> Result<Vec<String>, String> {
    Ok(emu_ops::coldclient_shortcut(game_dir, back, close_steam).await)
}

pub async fn emu_generate_game_data(app: &Cx, targets: Vec<String>, app_id: String, language: Option<String>) -> Result<smd_core::services::game_data::GameDataResult, String> {
    let state = app.state();
    emu_ops::generate_game_data(&state, &app.data_dir(), targets, app_id, language).await
}

pub async fn emu_patch_info(target: String) -> Result<smd_core::services::emulator::PatchInfo, String> {
    Ok(emu_ops::patch_info(&target))
}

pub async fn emu_drm_backups(game_dir: String) -> Result<Vec<smd_core::services::steamless::DrmBackup>, String> {
    Ok(emu_ops::drm_backups(game_dir).await)
}

pub async fn emu_restore_drm(exe: String) -> Result<(), String> {
    emu_ops::restore_drm(&exe)
}

pub async fn emu_controller_info(app: &Cx, app_id: String) -> Result<smd_core::services::game_data::controller::ControllerInfo, String> {
    let state = app.state();
    emu_ops::controller_info(&state, app_id).await
}

pub async fn emu_write_controller(app: &Cx, targets: Vec<String>, app_id: String, layout_file: Option<String>) -> Result<smd_core::services::game_data::controller::ControllerResult, String> {
    let state = app.state();
    emu_ops::write_controller(&state, &app.data_dir(), targets, app_id, layout_file).await
}

pub async fn emu_read_emu_settings(target_path: String) -> Result<EmuSettings, String> {
    emu_ops::read_emu_settings(&target_path)
}

pub async fn emu_write_emu_settings(target_path: String, settings: EmuSettings) -> Result<(), String> {
    emu_ops::write_emu_settings(&target_path, &settings)
}

pub async fn emu_revert_replacement(targets: Vec<String>, close_steam: Option<bool>) -> Result<Vec<ReplaceResult>, String> {
    emu_ops::revert_replacement(targets, close_steam.unwrap_or(false)).await
}

use smd_core::services::emu_extras as extras;
use std::path::PathBuf;

pub async fn emu_save_folders(target: String, app_id: String) -> Result<Vec<extras::SaveFolder>, String> {
    emu_ops::blocking(move || extras::save_folders(&PathBuf::from(target), &app_id)).await
}

pub async fn emu_backup_saves(folder: String, dest: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::backup_saves(&PathBuf::from(folder), &PathBuf::from(dest))).await?
}

pub async fn emu_restore_saves(folder: String, archive: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::restore_saves(&PathBuf::from(folder), &PathBuf::from(archive))).await?
}

pub async fn emu_achievements(target: String, app_id: String, lang: String) -> Result<extras::AchievementView, String> {
    emu_ops::blocking(move || extras::achievements(&PathBuf::from(target), &app_id, &lang)).await
}

pub async fn emu_reset_achievements(target: String, app_id: String, names: Option<Vec<String>>) -> Result<usize, String> {
    emu_ops::blocking(move || extras::reset_achievements(&PathBuf::from(target), &app_id, names.as_deref())).await?
}

pub async fn emu_reset_stats(target: String, app_id: String) -> Result<usize, String> {
    emu_ops::blocking(move || extras::reset_stats(&PathBuf::from(target), &app_id)).await?
}

pub async fn emu_image(path: String) -> Result<String, String> {
    emu_ops::blocking(move || extras::image_data_url(&PathBuf::from(path))).await?
}

pub async fn emu_overlay_assets(target: String) -> Result<extras::OverlayAssets, String> {
    emu_ops::blocking(move || extras::overlay_assets(&PathBuf::from(target))).await
}

pub async fn emu_installed_fonts() -> Result<Vec<extras::FontEntry>, String> {
    emu_ops::blocking(extras::installed_fonts).await
}

pub async fn emu_set_font(targets: Vec<String>, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_font(&targets, &PathBuf::from(src))).await?
}

pub async fn emu_remove_font(targets: Vec<String>) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_font(&targets)).await?
}

pub async fn emu_set_avatar(targets: Vec<String>, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_avatar(&targets, &PathBuf::from(src))).await?
}

pub async fn emu_remove_avatar(targets: Vec<String>) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_avatar(&targets)).await?
}

pub async fn emu_set_sound(targets: Vec<String>, kind: String, src: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::set_sound(&targets, &kind, &PathBuf::from(src))).await?
}

pub async fn emu_remove_sound(targets: Vec<String>, kind: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_sound(&targets, &kind)).await?
}

pub async fn emu_broadcasts(target: String) -> Result<Vec<String>, String> {
    emu_ops::blocking(move || extras::broadcasts(&PathBuf::from(target))).await
}

pub async fn emu_set_broadcasts(targets: Vec<String>, list: Vec<String>) -> Result<Vec<String>, String> {
    emu_ops::blocking(move || extras::set_broadcasts(&targets, &list)).await?
}

pub async fn emu_lobby_connect(app: &Cx, game_dir: String, target: String, allow_download: bool) -> Result<String, String> {
    let state = app.state();
    let _ = game_dir;
    emu_ops::lobby_connect(&app.sink(), &state, &app.data_dir(), target, allow_download).await
}

pub async fn emu_launch_console(path: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::launch_console(&PathBuf::from(path))).await?
}

pub async fn emu_mods(target: String) -> Result<Vec<extras::ModEntry>, String> {
    emu_ops::blocking(move || extras::mods(&PathBuf::from(target))).await
}

pub async fn emu_add_mod(app: &Cx, targets: Vec<String>, app_id: String, id: String, src: String) -> Result<extras::AddedMod, String> {
    let state = app.state();
    extras::add_mod(&state.http_client, &targets, &app_id, &id, &PathBuf::from(src)).await
}

pub async fn emu_remove_mod(targets: Vec<String>, id: String) -> Result<(), String> {
    emu_ops::blocking(move || extras::remove_mod(&targets, &id)).await?
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "emu_release_info" => reply(emu_release_info(app).await),
        "emu_ensure_cached" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { platform: Platform, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_ensure_cached(app, a.platform).await)
        }
        "emu_scan_game_dir" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_scan_game_dir(a.game_dir).await)
        }
        "emu_scan_for_dlc_merge" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, app_id: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_scan_for_dlc_merge(app, a.game_dir, a.app_id).await)
        }
        "emu_merge_dlc_depots" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { main_depot_dir: String, dlc_depot_dirs: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_merge_dlc_depots(a.main_depot_dir, a.dlc_depot_dirs).await)
        }
        "emu_apply_replacement" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<ScannedFile>, variant: Variant, app_id: String, installed_app_ids: Vec<String>, emu_settings: Option<EmuSettings>, allow_download: bool, prefer_local: Option<bool>, coldclient_exe: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_apply_replacement(app, a.targets, a.variant, a.app_id, a.installed_app_ids, a.emu_settings, a.allow_download, a.prefer_local, a.coldclient_exe).await)
        }
        "emu_coldclient_shortcut" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, back: bool, close_steam: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_coldclient_shortcut(a.game_dir, a.back, a.close_steam).await)
        }
        "emu_generate_game_data" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, app_id: String, language: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_generate_game_data(app, a.targets, a.app_id, a.language).await)
        }
        "emu_patch_info" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_patch_info(a.target).await)
        }
        "emu_drm_backups" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_drm_backups(a.game_dir).await)
        }
        "emu_restore_drm" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { exe: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_restore_drm(a.exe).await)
        }
        "emu_controller_info" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_controller_info(app, a.app_id).await)
        }
        "emu_write_controller" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, app_id: String, layout_file: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_write_controller(app, a.targets, a.app_id, a.layout_file).await)
        }
        "emu_read_emu_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target_path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_read_emu_settings(a.target_path).await)
        }
        "emu_write_emu_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target_path: String, settings: EmuSettings, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_write_emu_settings(a.target_path, a.settings).await)
        }
        "emu_revert_replacement" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, close_steam: Option<bool>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_revert_replacement(a.targets, a.close_steam).await)
        }
        "emu_save_folders" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_save_folders(a.target, a.app_id).await)
        }
        "emu_backup_saves" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { folder: String, dest: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_backup_saves(a.folder, a.dest).await)
        }
        "emu_restore_saves" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { folder: String, archive: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_restore_saves(a.folder, a.archive).await)
        }
        "emu_achievements" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, app_id: String, lang: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_achievements(a.target, a.app_id, a.lang).await)
        }
        "emu_reset_achievements" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, app_id: String, names: Option<Vec<String>>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_reset_achievements(a.target, a.app_id, a.names).await)
        }
        "emu_reset_stats" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_reset_stats(a.target, a.app_id).await)
        }
        "emu_image" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_image(a.path).await)
        }
        "emu_overlay_assets" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_overlay_assets(a.target).await)
        }
        "emu_installed_fonts" => reply(emu_installed_fonts().await),
        "emu_set_font" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, src: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_set_font(a.targets, a.src).await)
        }
        "emu_remove_font" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_remove_font(a.targets).await)
        }
        "emu_set_avatar" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, src: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_set_avatar(a.targets, a.src).await)
        }
        "emu_remove_avatar" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_remove_avatar(a.targets).await)
        }
        "emu_set_sound" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, kind: String, src: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_set_sound(a.targets, a.kind, a.src).await)
        }
        "emu_remove_sound" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, kind: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_remove_sound(a.targets, a.kind).await)
        }
        "emu_broadcasts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_broadcasts(a.target).await)
        }
        "emu_set_broadcasts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, list: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_set_broadcasts(a.targets, a.list).await)
        }
        "emu_lobby_connect" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, target: String, allow_download: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_lobby_connect(app, a.game_dir, a.target, a.allow_download).await)
        }
        "emu_launch_console" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_launch_console(a.path).await)
        }
        "emu_mods" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { target: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_mods(a.target).await)
        }
        "emu_add_mod" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, app_id: String, id: String, src: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_add_mod(app, a.targets, a.app_id, a.id, a.src).await)
        }
        "emu_remove_mod" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emu_remove_mod(a.targets, a.id).await)
        }
        _ => return None,
    })
}
