use std::path::Path;

use crate::services::settings::{self as settings_service, TelemetryConsent};

fn label(consent: &TelemetryConsent) -> &'static str {
    match consent {
        TelemetryConsent::Pending => "pending",
        TelemetryConsent::Accepted => "accepted",
        TelemetryConsent::Declined => "declined",
    }
}

pub async fn telemetry_status(app_data_dir: &Path) -> serde_json::Value {
    let settings = settings_service::load_settings(app_data_dir).await;
    let consent = match settings.telemetry_consent {
        TelemetryConsent::Pending => {
            if cfg!(debug_assertions) {
                "declined"
            } else {
                "pending"
            }
        }
        TelemetryConsent::Accepted => "accepted",
        TelemetryConsent::Declined => "declined",
    };
    serde_json::json!({
        "consent": consent,
        "behavior": label(&settings.telemetry_behavior),
        "installation_id": settings.installation_id,
    })
}

pub async fn set_behavior_consent(app_data_dir: &Path, accept: bool) -> Result<(), String> {
    let mut settings = settings_service::load_settings(app_data_dir).await;
    settings.telemetry_behavior = if accept {
        TelemetryConsent::Accepted
    } else {
        TelemetryConsent::Declined
    };
    settings_service::save_settings(app_data_dir, &settings).await
}

pub async fn set_telemetry_consent(app_data_dir: &Path, accept: bool) -> Result<(), String> {
    let mut settings = settings_service::load_settings(app_data_dir).await;
    settings.telemetry_consent = if accept {
        TelemetryConsent::Accepted
    } else {
        TelemetryConsent::Declined
    };
    if accept && settings.installation_id.is_empty() {
        settings.installation_id = uuid::Uuid::new_v4().to_string();
    }
    settings_service::save_settings(app_data_dir, &settings).await
}
