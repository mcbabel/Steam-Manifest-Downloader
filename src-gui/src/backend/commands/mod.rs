pub mod account;
pub mod download;
pub mod emulator;
pub mod file_ops;
pub mod history;
pub mod native_download;
pub mod queue;
pub mod search;
pub mod session;
pub mod settings;
pub mod shortcuts;
pub mod steam_api_bypass;
pub mod steam_library;
pub mod steamless;
pub mod system;
pub mod telemetry;
pub mod updater;

use super::{Cx, Reply};

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Reply {
    macro_rules! route {
        ($($module:ident),* $(,)?) => {
            $(
                if let Some(reply) = $module::dispatch(app, name, args.clone()).await {
                    return reply;
                }
            )*
        };
    }
    route!(
        settings,
        session,
        system,
        search,
        file_ops,
        download,
        native_download,
        history,
        queue,
        shortcuts,
        telemetry,
        emulator,
        steam_library,
        steamless,
        steam_api_bypass,
        account,
        updater,
    );
    Err(format!("unknown command: {}", name))
}
