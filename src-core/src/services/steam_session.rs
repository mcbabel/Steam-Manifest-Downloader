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
    timeout(CDN_TIMEOUT, async move {
        let conn = session.connection().await?;
        let req = CContentServerDirectory_GetServersForSteamPipe_Request {
            cell_id: Some(cell_id),
            max_servers: Some(max_servers),
            ..Default::default()
        };
        let resp = conn
            .service_method(req)
            .await
            .map_err(|e| format!("GetServersForSteamPipe failed: {}", e))?;
        Ok::<_, String>(
            resp.servers
                .into_iter()
                .filter_map(server_info_to_cdn)
                .collect(),
        )
    })
    .await
    .map_err(|_| "Steam CDN server discovery timed out".to_string())?
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
    timeout(CDN_TIMEOUT, async move {
        let conn = session.connection().await?;
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
        Ok::<_, String>(resp.manifest_request_code.unwrap_or(0))
    })
    .await
    .map_err(|_| "GetManifestRequestCode timed out".to_string())?
}

pub async fn fetch_cdn_auth_token(
    session: Arc<SteamSession>,
    host: &str,
    app_id: u32,
    depot_id: u32,
) -> Result<Option<String>, String> {
    let host = host.to_string();
    timeout(CDN_TIMEOUT, async move {
        let conn = session.connection().await?;
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
        Ok::<_, String>(resp.token)
    })
    .await
    .map_err(|_| "GetCDNAuthToken timed out".to_string())?
}
