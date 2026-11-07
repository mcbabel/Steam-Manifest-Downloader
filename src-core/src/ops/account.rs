use std::path::Path;

use serde::Serialize;

use crate::services::steam_account::AccountInfo;
use crate::services::steam_auth::{self, AuthError, QrPoll};
use crate::services::steam_session::AccountLogonError;
use crate::services::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrView {
    pub challenge_url: String,
    pub svg: String,
    pub interval_secs: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum QrStatus {
    #[serde(rename_all = "camelCase")]
    Waiting { scanned: bool, qr: Option<QrView> },
    #[serde(rename_all = "camelCase")]
    Done { account: AccountInfo },
    Expired,
    Idle,
}

fn view(challenge: &steam_auth::QrChallenge) -> Result<QrView, String> {
    Ok(QrView {
        challenge_url: challenge.challenge_url.clone(),
        svg: steam_auth::qr_svg(&challenge.challenge_url)?,
        interval_secs: challenge.interval_secs,
    })
}

pub async fn status(state: &AppState, data_dir: &Path) -> Option<AccountInfo> {
    state.account.get(data_dir).await.map(|a| a.info())
}

pub async fn start_qr(state: &AppState) -> Result<QrView, String> {
    let challenge = steam_auth::begin_qr().await.map_err(|e| e.to_string())?;
    let v = view(&challenge)?;
    *state.pending_qr.lock().await = Some(challenge);
    Ok(v)
}

pub async fn cancel_qr(state: &AppState) {
    *state.pending_qr.lock().await = None;
}

pub async fn poll_qr(
    state: &AppState,
    data_dir: &Path,
    allow_file: bool,
) -> Result<QrStatus, String> {
    let mut guard = state.pending_qr.lock().await;
    let Some(challenge) = guard.as_mut() else {
        return Ok(QrStatus::Idle);
    };
    let polled = steam_auth::poll_qr(challenge).await;
    match polled {
        Ok(QrPoll::Waiting { scanned, new_url }) => Ok(QrStatus::Waiting {
            scanned,
            qr: if new_url {
                Some(view(challenge)?)
            } else {
                None
            },
        }),
        Ok(QrPoll::Approved {
            account_name,
            refresh_token,
        }) => {
            *guard = None;
            drop(guard);
            let account = state
                .account
                .set(data_dir, account_name, refresh_token, allow_file)
                .await?;
            state.steam_session.reset_account().await;
            if let Err(e) = state.steam_session.account_connection(&account).await {
                if matches!(e, AccountLogonError::Rejected(_)) {
                    state.account.clear(data_dir).await;
                }
                return Err(e.to_string());
            }
            Ok(QrStatus::Done {
                account: account.info(),
            })
        }
        Err(AuthError::Expired) => {
            *guard = None;
            Ok(QrStatus::Expired)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn check(state: &AppState, data_dir: &Path) -> Result<Option<AccountInfo>, String> {
    let Some(account) = state.account.get(data_dir).await else {
        return Ok(None);
    };
    if account.is_expired(chrono::Utc::now().timestamp()) {
        state.account.clear(data_dir).await;
        return Err(AccountLogonError::Rejected("token expired".into()).to_string());
    }
    match state.steam_session.account_connection(&account).await {
        Ok(_) => Ok(Some(account.info())),
        Err(e) => {
            if matches!(e, AccountLogonError::Rejected(_)) {
                state.account.clear(data_dir).await;
            }
            Err(e.to_string())
        }
    }
}

pub async fn logout(state: &AppState, data_dir: &Path) {
    if let Some(account) = state.account.get(data_dir).await {
        revoke(state, &account).await;
    }
    state.steam_session.reset_account().await;
    state.account.clear(data_dir).await;
}

async fn revoke(state: &AppState, account: &crate::services::steam_account::Account) {
    use steam_vent::ConnectionTrait;
    use steam_vent_proto::steammessages_auth_steamclient::{
        CAuthentication_Token_Revoke_Request, EAuthTokenRevokeAction,
    };
    let Ok(conn) = state.steam_session.account_connection(account).await else {
        return;
    };
    let req = CAuthentication_Token_Revoke_Request {
        token: Some(account.refresh_token.expose().to_string()),
        revoke_action: Some(EAuthTokenRevokeAction::k_EAuthTokenRevokePermanent.into()),
        ..Default::default()
    };
    let call = conn.service_method(req);
    if let Err(e) = tokio::time::timeout(std::time::Duration::from_secs(10), call).await {
        crate::dlog!("Account", "token revoke timed out: {}", e);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchList {
    pub signed_in: bool,
    pub branches: Vec<crate::services::steam_branches::BranchView>,
}

pub async fn branches(
    state: &AppState,
    data_dir: &Path,
    app_id: u32,
) -> Result<BranchList, String> {
    use crate::services::{steam_branches, steam_session::SteamSession};
    let account = state.account.get(data_dir).await;
    let signed_in = account.is_some();
    let session = match account {
        Some(a) => std::sync::Arc::new(SteamSession::for_account(a, None)),
        None => state.steam_session.clone(),
    };
    Ok(BranchList {
        signed_in,
        branches: steam_branches::list(session, app_id).await?,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDepot {
    pub depot_id: String,
    pub manifest_id: String,
}

pub async fn app_depots(
    state: &AppState,
    data_dir: &Path,
    app_id: u32,
    branch: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<AccountDepot>, String> {
    use crate::services::{steam_branches, steam_pics, steam_session::SteamSession};
    let account = state
        .account
        .get(data_dir)
        .await
        .ok_or_else(|| "Sign in with your Steam account first.".to_string())?;
    let branch = branch.unwrap_or(steam_branches::PUBLIC);
    let session = std::sync::Arc::new(SteamSession::for_account(account, Some(branch.to_string())));
    let gids = steam_branches::manifests(session.clone(), app_id, branch, password)
        .await
        .map_err(|e| e.to_string())?;
    let meta = steam_pics::fetch_depots_with_names(session, app_id).await?;
    let mut out: Vec<AccountDepot> = meta
        .iter()
        .filter_map(|m| {
            Some(AccountDepot {
                depot_id: m.depot_id.clone(),
                manifest_id: gids.get(&m.depot_id)?.clone(),
            })
        })
        .collect();
    out.sort_by_key(|d| d.depot_id.parse::<u64>().unwrap_or(u64::MAX));
    Ok(out)
}
