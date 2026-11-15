use crate::backend::{parse, reply, Cx};
use smd_core::ops::account::{self, AccountDepot, BranchList, QrStatus, QrView};
use smd_core::services::steam_account::AccountInfo;


fn parse_app(app_id: &str) -> Result<u32, String> {
    app_id
        .trim()
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))
}

pub async fn account_status(app: &Cx) -> Result<Option<AccountInfo>, String> {
    let state = app.state();
    Ok(account::status(&state, &app.data_dir()).await)
}

pub async fn account_check(app: &Cx) -> Result<Option<AccountInfo>, String> {
    let state = app.state();
    account::check(&state, &app.data_dir()).await
}

pub async fn account_login_start(app: &Cx) -> Result<QrView, String> {
    let state = app.state();
    account::start_qr(&state).await
}

pub async fn account_login_poll(app: &Cx) -> Result<QrStatus, String> {
    let state = app.state();
    account::poll_qr(&state, &app.data_dir(), false).await
}

pub async fn account_login_cancel(app: &Cx) -> Result<(), String> {
    let state = app.state();
    account::cancel_qr(&state).await;
    Ok(())
}

pub async fn account_logout(app: &Cx) -> Result<(), String> {
    let state = app.state();
    account::logout(&state, &app.data_dir()).await;
    Ok(())
}

pub async fn list_branches(app: &Cx, app_id: String) -> Result<BranchList, String> {
    let state = app.state();
    account::branches(&state, &app.data_dir(), parse_app(&app_id)?).await
}

pub async fn account_app_depots(app: &Cx, app_id: String, branch: Option<String>, branch_password: Option<String>) -> Result<Vec<AccountDepot>, String> {
    let state = app.state();
    account::app_depots(
        &state,
        &app.data_dir(),
        parse_app(&app_id)?,
        branch.as_deref(),
        branch_password.as_deref(),
    )
    .await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "account_status" => reply(account_status(app).await),
        "account_check" => reply(account_check(app).await),
        "account_login_start" => reply(account_login_start(app).await),
        "account_login_poll" => reply(account_login_poll(app).await),
        "account_login_cancel" => reply(account_login_cancel(app).await),
        "account_logout" => reply(account_logout(app).await),
        "list_branches" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(list_branches(app, a.app_id).await)
        }
        "account_app_depots" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, branch: Option<String>, branch_password: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(account_app_depots(app, a.app_id, a.branch, a.branch_password).await)
        }
        _ => return None,
    })
}
