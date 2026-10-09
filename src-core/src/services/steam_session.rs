use std::sync::Arc;
use std::time::Duration;

use steam_vent::{Connection, ConnectionTrait, ServerList};
use steam_vent_proto::steammessages_contentsystem_steamclient::{
    CContentServerDirectory_GetCDNAuthToken_Request,
    CContentServerDirectory_GetManifestRequestCode_Request,
    CContentServerDirectory_GetServersForSteamPipe_Request,
    CContentServerDirectory_ServerInfo,
};
use tokio::sync::Mutex;
use tokio::time::timeout;

const CDN_TIMEOUT: Duration = Duration::from_secs(15);

pub struct SteamSession {
    inner: Mutex<Option<Connection>>,
}

impl SteamSession {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub async fn connection(&self) -> Result<Connection, String> {
        let mut guard = self.inner.lock().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
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

#[derive(Debug, Clone)]
pub struct CdnServer {
    pub host: String,
    pub https_support: String,
}

pub async fn discover_cdn_servers(
    session: Arc<SteamSession>,
    cell_id: u32,
    max_servers: u32,
) -> Result<Vec<CdnServer>, String> {
    call_with_retry(&session, 3, CDN_TIMEOUT, "Steam CDN server discovery timed out", |conn| async move {
        let req = CContentServerDirectory_GetServersForSteamPipe_Request {
            cell_id: Some(cell_id),
            max_servers: Some(max_servers),
            ..Default::default()
        };
        let resp = conn
            .service_method(req)
            .await
            .map_err(|e| format!("GetServersForSteamPipe failed: {}", e))?;
        Ok(resp
            .servers
            .into_iter()
            .filter_map(server_info_to_cdn)
            .collect())
    })
    .await
}

fn server_info_to_cdn(mut s: CContentServerDirectory_ServerInfo) -> Option<CdnServer> {
    let type_str = s.type_().to_string();
    if type_str != "CDN" && type_str != "SteamCache" {
        return None;
    }
    let host = s.host.take()?;
    Some(CdnServer {
        host,
        https_support: s.https_support.unwrap_or_default(),
    })
}

pub async fn fetch_manifest_request_code(
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
) -> Result<u64, String> {
    call_with_retry(&session, 2, CDN_TIMEOUT, "GetManifestRequestCode timed out", |conn| async move {
        let req = CContentServerDirectory_GetManifestRequestCode_Request {
            app_id: Some(app_id),
            depot_id: Some(depot_id),
            manifest_id: Some(manifest_id),
            ..Default::default()
        };
        let resp = conn
            .service_method(req)
            .await
            .map_err(|e| format!("GetManifestRequestCode failed: {}", e))?;
        Ok(resp.manifest_request_code.unwrap_or(0))
    })
    .await
}

pub async fn fetch_cdn_auth_token(
    session: Arc<SteamSession>,
    host: &str,
    app_id: u32,
    depot_id: u32,
) -> Result<Option<String>, String> {
    call_with_retry(&session, 2, CDN_TIMEOUT, "GetCDNAuthToken timed out", |conn| {
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
    })
    .await
}
