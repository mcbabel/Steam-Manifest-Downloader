use crate::backend::{parse, plain, Cx};

pub fn has_active_downloads(app: &Cx) -> bool {
    app.state().has_active_downloads()
}

pub async fn end_session(app: &Cx, reason: String, clear_downloads: Option<bool>) {
    if clear_downloads.unwrap_or(false) {
        smd_core::services::telemetry::clear_active_downloads(&app.data_dir());
    }
    app.end_session(&reason, 3).await;
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "has_active_downloads" => plain(has_active_downloads(app)),
        "end_session" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                reason: String,
                clear_downloads: Option<bool>,
            }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            end_session(app, a.reason, a.clear_downloads).await;
            plain(())
        }
        _ => return None,
    })
}
