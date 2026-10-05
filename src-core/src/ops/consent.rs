use std::path::Path;

use crate::services::settings::{self as settings_service, TelemetryConsent};

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
        "installation_id": settings.installation_id,
    })
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
