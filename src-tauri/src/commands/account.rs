//! Steam account (QR sign-in) and branches. The token never reaches the
//! webview: these commands only return `AccountInfo`, QR codes and status.

use smd_core::ops::account::{self, AccountDepot, BranchList, QrStatus, QrView};
use smd_core::services::steam_account::AccountInfo;
use smd_core::services::AppState;
use tauri::{command, AppHandle};

use super::app_data_dir;

fn parse_app(app_id: &str) -> Result<u32, String> {
    app_id
        .trim()
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))
}

#[command]
pub async fn account_status(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Option<AccountInfo>, String> {
    Ok(account::status(&state, &app_data_dir(&app)).await)
}

#[command]
pub async fn account_check(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Option<AccountInfo>, String> {
    account::check(&state, &app_data_dir(&app)).await
}

#[command]
pub async fn account_login_start(state: tauri::State<'_, AppState>) -> Result<QrView, String> {
    account::start_qr(&state).await
}

/// The desktop app never falls back to the unencrypted file: without a
/// keychain the sign-in lasts until the app closes.
#[command]
pub async fn account_login_poll(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<QrStatus, String> {
    account::poll_qr(&state, &app_data_dir(&app), false).await
}

#[command]
pub async fn account_login_cancel(state: tauri::State<'_, AppState>) -> Result<(), String> {
    account::cancel_qr(&state).await;
    Ok(())
}

#[command]
pub async fn account_logout(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    account::logout(&state, &app_data_dir(&app)).await;
    Ok(())
}

#[command]
pub async fn list_branches(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<BranchList, String> {
    account::branches(&state, &app_data_dir(&app), parse_app(&app_id)?).await
}

#[command]
pub async fn account_app_depots(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
    branch: Option<String>,
    branch_password: Option<String>,
) -> Result<Vec<AccountDepot>, String> {
    account::app_depots(
        &state,
        &app_data_dir(&app),
        parse_app(&app_id)?,
        branch.as_deref(),
        branch_password.as_deref(),
    )
    .await
}
