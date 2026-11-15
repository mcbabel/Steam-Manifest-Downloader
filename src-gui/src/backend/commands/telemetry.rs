use crate::backend::{parse, reply, Cx};

use smd_core::ops::consent;
use smd_core::services::telemetry::{self as telemetry_service, Event};


pub async fn get_telemetry_status(app: &Cx) -> Result<serde_json::Value, String> {
    Ok(consent::telemetry_status(&app.data_dir()).await)
}

pub async fn set_telemetry_consent(app: &Cx, accept: bool) -> Result<(), String> {
    consent::set_telemetry_consent(&app.data_dir(), accept).await?;

    if accept {
        if let Some(telemetry) = app.telemetry() {
            telemetry.emit(Event::new("consent_accepted")).await;
        }
    }
    Ok(())
}

pub async fn set_behavior_consent(app: &Cx, accept: bool) -> Result<(), String> {
    consent::set_behavior_consent(&app.data_dir(), accept).await?;
    if let Some(telemetry) = app.telemetry() {
        telemetry
            .emit(Event::new("behavior_consent").with_props(serde_json::json!({ "accept": accept })))
            .await;
    }
    Ok(())
}

pub async fn set_telemetry_focus(app: &Cx, focused: bool) -> Result<(), String> {
    if let Some(telemetry) = app.telemetry() {
        telemetry.set_focused(focused).await;
    }
    Ok(())
}

pub async fn emit_telemetry_event(app: &Cx, kind: String, props: Option<serde_json::Value>) -> Result<(), String> {
    if !telemetry_service::is_safe_kind(&kind) {
        return Err("unknown event kind".to_string());
    }
    if let Some(telemetry) = app.telemetry() {
        let mut event = Event::new(kind.clone());
        if let Some(p) = props {
            event = event.with_props(telemetry_service::sanitize_props(&kind, p));
        }
        telemetry.emit(event).await;
    }
    Ok(())
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "get_telemetry_status" => reply(get_telemetry_status(app).await),
        "set_telemetry_consent" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { accept: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(set_telemetry_consent(app, a.accept).await)
        }
        "set_behavior_consent" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { accept: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(set_behavior_consent(app, a.accept).await)
        }
        "set_telemetry_focus" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { focused: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(set_telemetry_focus(app, a.focused).await)
        }
        "emit_telemetry_event" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { kind: String, props: Option<serde_json::Value>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(emit_telemetry_event(app, a.kind, a.props).await)
        }
        _ => return None,
    })
}
