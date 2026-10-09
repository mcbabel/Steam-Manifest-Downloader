use tauri::{command, AppHandle, Manager};

use smd_core::ops::consent;
use smd_core::services::{
    telemetry::{self as telemetry_service, Event},
    AppState,
};

use super::app_data_dir;

#[command]
pub async fn get_telemetry_status(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(consent::telemetry_status(&app_data_dir(&app)).await)
}

#[command]
pub async fn set_telemetry_consent(app: AppHandle, accept: bool) -> Result<(), String> {
    consent::set_telemetry_consent(&app_data_dir(&app), accept).await?;

    if accept {
        if let Some(state) = app.try_state::<AppState>() {
            if let Some(telemetry) = state.telemetry.clone() {
                telemetry
                    .emit(Event::new("consent_accepted"))
                    .await;
            }
        }
    }
    Ok(())
}

#[command]
pub async fn emit_telemetry_event(
    app: AppHandle,
    kind: String,
    props: Option<serde_json::Value>,
) -> Result<(), String> {
    if !telemetry_service::is_safe_kind(&kind) {
        return Err("unknown event kind".to_string());
    }
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(telemetry) = state.telemetry.clone() {
            let mut event = Event::new(kind.clone());
            if let Some(p) = props {
                event = event.with_props(telemetry_service::sanitize_props(&kind, p));
            }
            telemetry.emit(event).await;
        }
    }
    Ok(())
}