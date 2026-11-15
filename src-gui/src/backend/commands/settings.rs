use crate::backend::{parse, plain, reply, Cx};
use smd_core::services::settings as settings_service;

pub async fn get_settings(app: &Cx) -> Result<serde_json::Value, String> {
    let settings = settings_service::seed_defaults_if_needed(&app.data_dir()).await;
    serde_json::to_value(&settings).map_err(|e| format!("Failed to serialize settings: {}", e))
}

pub async fn save_settings(app: &Cx, settings: serde_json::Value) -> Result<(), String> {
    let new_settings: settings_service::Settings = serde_json::from_value(settings)
        .map_err(|e| format!("Invalid settings format: {}", e))?;

    settings_service::save_settings(&app.data_dir(), &new_settings).await
}

pub fn download_regions() -> Vec<serde_json::Value> {
    smd_core::services::cdn_servers::REGIONS
        .iter()
        .map(|(cell, name)| serde_json::json!({ "cell": cell, "name": name }))
        .collect()
}

pub async fn test_proxy(proxy: String) -> Result<smd_core::services::net::ProxyTest, String> {
    smd_core::services::net::test_proxy(&proxy).await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "get_settings" => reply(get_settings(app).await),
        "save_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { settings: serde_json::Value, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(save_settings(app, a.settings).await)
        }
        "download_regions" => plain(download_regions()),
        "test_proxy" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { proxy: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(test_proxy(a.proxy).await)
        }
        _ => return None,
    })
}
