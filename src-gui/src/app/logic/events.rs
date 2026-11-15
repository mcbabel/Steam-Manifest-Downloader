use serde_json::json;

use super::{sources, steps};
use crate::app::App;

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-minimize" => app.minimize(),
        "btn-maximize" => app.toggle_maximize(),
        "btn-close" => app.request_close(),
        "tab-upload" => steps::switch_tab(app, "upload"),
        "tab-search" => {
            steps::switch_tab(app, "search");
            sources::refresh_sources_ui(app);
        }
        "tab-patch-only" => steps::switch_tab(app, "patchOnly"),
        "btn-sources-empty-upload" => {
            app.emit_event("advice_action", json!({ "advice": "no_sources", "action": "upload" }));
            steps::switch_tab(app, "upload");
        }
        "btn-close-no" | "close-modal>modal__backdrop" => app.ui.hide("close-modal"),
        "behavior-modal>modal__backdrop" | "language-picker>modal__backdrop" | "telemetry-modal>modal__backdrop" => {}
        "btn-close-yes" => {
            app.ui.hide("close-modal");
            if app.state.job_id.is_some() {
                let props = super::telemetry::abandon_props(app);
                app.emit_event("download_abandoned", props);
            }
            app.close_now();
        }
        _ => return false,
    }
    true
}
