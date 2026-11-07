use std::sync::Arc;
use std::time::Duration;

use steam_vent::{Connection, ConnectionTrait, ServerList};
use steam_vent_proto::steammessages_contentsystem_steamclient::{
    CContentServerDirectory_GetCDNAuthToken_Request,
    CContentServerDirectory_GetManifestRequestCode_Request,
    CContentServerDirectory_GetServersForSteamPipe_Request, CContentServerDirectory_ServerInfo,
};
use tokio::sync::Mutex;
use tokio::time::timeout;

const CDN_TIMEOUT: Duration = Duration::from_secs(15);

pub struct SteamSession {
    inner: Mutex<Option<Connection>>,
    account: Mutex<Option<(u64, Connection)>>,
    login: Option<crate::services::steam_account::Account>,
    branch: Option<String>,
    key_errors: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountLogonError {
    Rejected(String),
    Other(String),
}

impl std::fmt::Display for AccountLogonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountLogonError::Rejected(e) => write!(
                f,
                "Steam no longer accepts this sign-in ({}). Sign in again.",
                e
            ),
            AccountLogonError::Other(e) => {
                write!(f, "Could not log on to Steam with your account: {}", e)
            }
        }
    }
}

fn classify_logon(err: &steam_vent::ConnectionError) -> AccountLogonError {
    use steam_vent::{ConnectionError, EResult, LoginError};
    let text = err.to_string();
    match err {
        ConnectionError::AccessToken(_)
        | ConnectionError::LoginError(LoginError::InvalidCredentials)
        | ConnectionError::LoginError(LoginError::InvalidSteamId)
        | ConnectionError::LoginError(LoginError::UnavailableAccount) => {
            AccountLogonError::Rejected(text)
        }
        ConnectionError::LoginError(LoginError::Unknown(
            EResult::AccessDenied | EResult::Expired | EResult::Revoked | EResult::InvalidPassword,
        )) => AccountLogonError::Rejected(text),
        _ => AccountLogonError::Other(text),
    }
}

impl SteamSession {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            account: Mutex::new(None),
            login: None,
            branch: None,
            key_errors: Default::default(),
        }
    }

    pub fn for_account(
        account: crate::services::steam_account::Account,
        branch: Option<String>,
    ) -> Self {
        Self {
            inner: Mutex::new(None),
            account: Mutex::new(None),
            login: Some(account),
            branch: branch.filter(|b| !b.is_empty() && !b.eq_ignore_ascii_case("public")),
            key_errors: Default::default(),
        }
    }

    pub fn note_key_error(&self, depot: &str, reason: String) {
        if let Ok(mut map) = self.key_errors.lock() {
            map.insert(depot.to_string(), reason);
        }
    }

    pub fn key_error(&self, depot: &str) -> Option<String> {
        self.key_errors.lock().ok()?.get(depot).cloned()
    }

    pub fn uses_account(&self) -> bool {
        self.login.is_some()
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub async fn account_connection(
        &self,
        account: &crate::services::steam_account::Account,
    ) -> Result<Connection, AccountLogonError> {
        let mut guard = self.account.lock().await;
        if let Some((steam_id, conn)) = guard.as_ref() {
            if *steam_id == account.steam_id {
                return Ok(conn.clone());
            }
        }
        *guard = None;
        let mut last: Option<AccountLogonError> = None;
        for attempt in 0..3u32 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(1000 * u64::from(attempt))).await;
            }
            let server_list = match ServerList::discover().await {
                Ok(list) => list,
                Err(e) => {
                    last = Some(AccountLogonError::Other(format!(
                        "Steam server discovery failed: {}",
                        e
                    )));
                    continue;
                }
            };
            match Connection::access(
                &server_list,
                &account.account_name,
                account.refresh_token.expose(),
            )
            .await
            {
                Ok(conn) => {
                    crate::dlog!("steam", "logged on with the signed-in account");
                    *guard = Some((account.steam_id, conn.clone()));
                    return Ok(conn);
                }
                Err(e) => {
                    let err = classify_logon(&e);
                    crate::dlog!("steam", "account logon failed: {}", err);
                    if matches!(err, AccountLogonError::Rejected(_)) {
                        return Err(err);
                    }
                    last = Some(err);
                }
            }
        }
        Err(last.unwrap_or_else(|| AccountLogonError::Other("Steam logon failed".into())))
    }

    pub async fn reset_account(&self) {
        *self.account.lock().await = None;
    }

    pub async fn connection(&self) -> Result<Connection, String> {
        let mut guard = self.inner.lock().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        if let Some(account) = self.login.as_ref() {
            let conn = self
                .account_connection(account)
                .await
                .map_err(|e| e.to_string())?;
            *guard = Some(conn.clone());
            return Ok(conn);
        }
        let mut last_err: Option<String> = None;
        for attempt in 0..4u32 {
            if attempt > 0 {
                let backoff_ms = 500u64 * (1u64 << attempt.min(4));
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
            let server_list = match ServerList::discover().await {
                Ok(list) => list,
                Err(e) => {
                    last_err = Some(format!("Steam server discovery failed: {}", e));
                    continue;
                }
            };
            match Connection::anonymous(&server_list).await {
                Ok(conn) => {
                    *guard = Some(conn.clone());
                    return Ok(conn);
                }
                Err(e) => {
                    last_err = Some(format!("Anonymous Steam login failed: {}", e));
                    continue;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| "Steam session bootstrap failed".to_string()))
    }

    pub async fn reset(&self) {
        *self.inner.lock().await = None;
        if self.login.is_some() {
            *self.account.lock().await = None;
        }
    }
}

pub async fn call_with_retry<T, F, Fut>(
    session: &SteamSession,
    attempts: u32,
    limit: Duration,
    timeout_msg: &str,
    f: F,
) -> Result<T, String>
where
    F: Fn(Connection) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last_err = String::new();
    for attempt in 0..attempts.max(1) {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(1000 * u64::from(attempt))).await;
        }
        let conn = match session.connection().await {
            Ok(conn) => conn,
            Err(e) => {
                last_err = e;
                continue;
            }
        };
        match timeout(limit, f(conn)).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(e)) => last_err = e,
            Err(_) => last_err = timeout_msg.to_string(),
        }
        session.reset().await;
    }
    Err(last_err)
}

impl Default for SteamSession {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CdnServer {
    pub host: String,
    pub https_support: String,
    pub cell_id: u32,
    pub weighted_load: f32,
    pub allowed_app_ids: Vec<u32>,
}

impl CdnServer {
    pub fn use_https(&self) -> bool {
        !self.https_support.is_empty() && self.https_support != "none"
    }
}

pub async fn discover_cdn_servers(
    session: Arc<SteamSession>,
    cell_id: Option<u32>,
    max_servers: u32,
) -> Result<Vec<CdnServer>, String> {
    call_with_retry(
        &session,
        3,
        CDN_TIMEOUT,
        "Steam CDN server discovery timed out",
        |conn| async move {
            let cell = cell_id.unwrap_or_else(|| conn.cell_id());
            let req = CContentServerDirectory_GetServersForSteamPipe_Request {
                cell_id: Some(cell),
                max_servers: Some(max_servers),
                ..Default::default()
            };
            let resp = conn
                .service_method(req)
                .await
                .map_err(|e| format!("GetServersForSteamPipe failed: {}", e))?;
            crate::dlog!("native", "content servers asked for cell {}", cell);
            Ok(resp
                .servers
                .into_iter()
                .filter_map(server_info_to_cdn)
                .collect())
        },
    )
    .await
}

fn server_info_to_cdn(mut s: CContentServerDirectory_ServerInfo) -> Option<CdnServer> {
    let type_str = s.type_().to_string();
    if type_str != "CDN" && type_str != "SteamCache" {
        return None;
    }
    if s.use_as_proxy.unwrap_or(false) || s.steam_china_only.unwrap_or(false) {
        return None;
    }
    let host = s.host.take()?;
    Some(CdnServer {
        host,
        https_support: s.https_support.unwrap_or_default(),
        cell_id: s.cell_id.unwrap_or(0).max(0) as u32,
        weighted_load: s.weighted_load.unwrap_or(f32::MAX),
        allowed_app_ids: std::mem::take(&mut s.allowed_app_ids),
    })
}

pub async fn fetch_manifest_request_code(
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
) -> Result<u64, String> {
    let branch = session.branch().map(str::to_string);
    call_with_retry(
        &session,
        2,
        CDN_TIMEOUT,
        "GetManifestRequestCode timed out",
        |conn| {
            let branch = branch.clone();
            async move {
                let req = CContentServerDirectory_GetManifestRequestCode_Request {
                    app_id: Some(app_id),
                    depot_id: Some(depot_id),
                    manifest_id: Some(manifest_id),
                    app_branch: branch,
                    ..Default::default()
                };
                let resp = conn
                    .service_method(req)
                    .await
                    .map_err(|e| format!("GetManifestRequestCode failed: {}", e))?;
                Ok(resp.manifest_request_code.unwrap_or(0))
            }
        },
    )
    .await
}

pub async fn fetch_cdn_auth_token(
    session: Arc<SteamSession>,
    host: &str,
    app_id: u32,
    depot_id: u32,
) -> Result<Option<String>, String> {
    call_with_retry(
        &session,
        2,
        CDN_TIMEOUT,
        "GetCDNAuthToken timed out",
        |conn| {
            let host = host.to_string();
            async move {
                let req = CContentServerDirectory_GetCDNAuthToken_Request {
                    app_id: Some(app_id),
                    depot_id: Some(depot_id),
                    host_name: Some(host),
                    ..Default::default()
                };
                let resp = conn
                    .service_method(req)
                    .await
                    .map_err(|e| format!("GetCDNAuthToken failed: {}", e))?;
                Ok(resp.token)
            }
        },
    )
    .await
}
