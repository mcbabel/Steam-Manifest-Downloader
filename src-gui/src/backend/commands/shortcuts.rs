use crate::backend::{parse, reply, Cx};
use smd_core::ops::shortcuts;

pub async fn is_shortcut_supported() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "supported": shortcuts::is_shortcut_supported() }))
}

pub async fn detect_executables(download_dir: String) -> Result<serde_json::Value, String> {
    let executables = shortcuts::detect_executables(&download_dir).await?;
    Ok(serde_json::json!({ "executables": executables }))
}

pub async fn create_shortcuts(
    exe_path: String,
    game_name: String,
    icon_path: Option<String>,
    create_desktop: bool,
    create_start_menu: bool,
) -> Result<serde_json::Value, String> {
    let result = shortcuts::create_shortcuts(
        exe_path,
        game_name,
        icon_path,
        create_desktop,
        create_start_menu,
    )
    .await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

pub(crate) async fn dispatch(_app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "is_shortcut_supported" => reply(is_shortcut_supported().await),
        "detect_executables" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { download_dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(detect_executables(a.download_dir).await)
        }
        "create_shortcuts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { exe_path: String, game_name: String, icon_path: Option<String>, create_desktop: bool, create_start_menu: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(create_shortcuts(a.exe_path, a.game_name, a.icon_path, a.create_desktop, a.create_start_menu).await)
        }
        _ => return None,
    })
}
