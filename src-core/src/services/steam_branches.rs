//! Branches of an app and their manifests, with the user's own account.
//!
//! Anonymous sessions only ever see the public branch. Logged on as the
//! account, PICS also shows branches the account was granted (owned games,
//! playtests, beta groups). Password branches need the branch password:
//! Steam checks it and returns a key for the branch, and with that key the
//! private depot section of the branch can be read. This mirrors what
//! DepotDownloader does with `-branch` / `-branchpassword`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use steam_vent::ConnectionTrait;
use steam_vent_proto::steammessages_clientserver_2::{
    CMsgClientCheckAppBetaPassword, CMsgClientCheckAppBetaPasswordResponse,
    CMsgClientGetDepotDecryptionKey, CMsgClientGetDepotDecryptionKeyResponse,
};
use steam_vent_proto::steammessages_clientserver_appinfo::{
    CMsgClientPICSPrivateBetaRequest, CMsgClientPICSPrivateBetaResponse,
};

use crate::services::steam_pics::{self, Branch};
use crate::services::steam_session::{call_with_retry, SteamSession};

const TIMEOUT: Duration = Duration::from_secs(20);
pub const PUBLIC: &str = "public";

pub fn is_public(branch: &str) -> bool {
    branch.is_empty() || branch.eq_ignore_ascii_case(PUBLIC)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchError {
    /// The branch is not public: sign in with an account that has access.
    NeedsLogin,
    /// The branch is protected by a password and none was given.
    NeedsPassword,
    /// Steam did not accept the password for this branch.
    WrongPassword,
    /// The branch does not exist, or the account has no access to it.
    NotFound,
    /// None of the selected depots has a build on this branch.
    NoBuild(Vec<String>),
    Steam(String),
}

impl BranchError {
    /// Stable key for translations and telemetry.
    pub fn key(&self) -> &'static str {
        match self {
            BranchError::NeedsLogin => "needs_login",
            BranchError::NeedsPassword => "needs_password",
            BranchError::WrongPassword => "wrong_password",
            BranchError::NotFound => "not_found",
            BranchError::NoBuild(_) => "no_build",
            BranchError::Steam(_) => "steam",
        }
    }
}

impl std::fmt::Display for BranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchError::NeedsLogin => f.write_str("This branch is not public. Sign in with a Steam account that has access to it."),
            BranchError::NeedsPassword => f.write_str("This branch is protected by a password. Enter the branch password."),
            BranchError::WrongPassword => f.write_str("Steam did not accept the password for this branch."),
            BranchError::NotFound => f.write_str("Steam does not show this branch to your account. It may not exist, or your account has no access to it."),
            BranchError::NoBuild(depots) => write!(f, "These depots have no build on this branch: {}", depots.join(", ")),
            BranchError::Steam(e) => write!(f, "Steam error: {}", e),
        }
    }
}

/// A branch as frontends list it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchView {
    pub name: String,
    pub description: String,
    pub password_required: bool,
    pub build_id: u64,
    pub time_updated: u64,
    /// The current session sees manifests on this branch (no password or
    /// sign-in needed beyond what it already has).
    pub available: bool,
}

/// Branches of `app_id` as the session sees them. With an account this
/// includes the branches the account was granted.
pub async fn list(session: Arc<SteamSession>, app_id: u32) -> Result<Vec<BranchView>, String> {
    let app = steam_pics::fetch_app_info(session, app_id).await?;
    Ok(steam_pics::branches(&app)
        .into_iter()
        .map(|b: Branch| {
            let available = !steam_pics::branch_gids(&app, &b.name).is_empty();
            BranchView {
                available,
                name: b.name,
                description: b.description,
                password_required: b.protected,
                build_id: b.build_id,
                time_updated: b.time_updated,
            }
        })
        .collect())
}

/// Manifest gid per depot on `branch`. `password` is only used for password
/// branches; it is sent to Steam and nowhere else.
pub async fn manifests(
    session: Arc<SteamSession>,
    app_id: u32,
    branch: &str,
    password: Option<&str>,
) -> Result<HashMap<String, String>, BranchError> {
    let branch = if is_public(branch) { PUBLIC } else { branch };
    let app = steam_pics::fetch_app_info(session.clone(), app_id)
        .await
        .map_err(BranchError::Steam)?;
    let mut gids = steam_pics::branch_gids(&app, branch);
    for (depot, other_app) in steam_pics::depots_from_other_apps(&app) {
        if other_app == app_id || gids.contains_key(&depot) {
            continue;
        }
        if let Ok(other) = steam_pics::fetch_app_info(session.clone(), other_app).await {
            if let Some(gid) = steam_pics::branch_gids(&other, branch).remove(&depot) {
                gids.insert(depot, gid);
            }
        }
    }
    if !gids.is_empty() || branch == PUBLIC {
        return Ok(gids);
    }

    let listed = steam_pics::branches(&app)
        .into_iter()
        .find(|b| b.name == branch);
    if !session.uses_account() {
        return Err(BranchError::NeedsLogin);
    }
    let Some(listed) = listed else {
        return Err(BranchError::NotFound);
    };
    if !listed.protected {
        // Listed, but the account sees no manifests on it.
        return Err(BranchError::NotFound);
    }
    let password = password
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or(BranchError::NeedsPassword)?;
    let key = branch_key(session.clone(), app_id, branch, password).await?;
    let section = private_depot_section(session, app_id, branch, &key).await?;
    let gids = gids_from_private_section(&section, branch);
    if gids.is_empty() {
        return Err(BranchError::NotFound);
    }
    Ok(gids)
}

/// Asks Steam for the key of `branch` with its password.
async fn branch_key(
    session: Arc<SteamSession>,
    app_id: u32,
    branch: &str,
    password: &str,
) -> Result<Vec<u8>, BranchError> {
    let password = password.to_string();
    let resp: CMsgClientCheckAppBetaPasswordResponse = call_with_retry(
        &session,
        2,
        TIMEOUT,
        "Branch password check timed out",
        |conn| {
            let password = password.clone();
            async move {
                let req = CMsgClientCheckAppBetaPassword {
                    app_id: Some(app_id),
                    betapassword: Some(password),
                    ..Default::default()
                };
                conn.job(req)
                    .await
                    .map_err(|e| format!("CheckAppBetaPassword failed: {}", e))
            }
        },
    )
    .await
    .map_err(BranchError::Steam)?;
    if resp.eresult.unwrap_or(2) != 1 {
        return Err(BranchError::WrongPassword);
    }
    resp.betapasswords
        .iter()
        .find(|b| b.betaname.as_deref() == Some(branch))
        .and_then(|b| hex::decode(b.betapassword.as_deref()?).ok())
        .ok_or(BranchError::WrongPassword)
}

// CMsgClientPICSPrivateBeta has no EMsg in steam-vent-proto. The numbers are
// SteamKit2's EMsg.ClientPICSPrivateBetaRequest / Response.
mod private_beta {
    use steam_vent_proto::steammessages_clientserver_appinfo::{
        CMsgClientPICSPrivateBetaRequest, CMsgClientPICSPrivateBetaResponse,
    };
    use steam_vent_proto::{MsgKindEnum, RpcMessage, RpcMessageWithKind};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Kind {
        #[default]
        Request,
        Response,
    }

    impl steam_vent_proto::protobuf::Enum for Kind {
        const NAME: &'static str = "EMsgPICSPrivateBeta";
        fn value(&self) -> i32 {
            match self {
                Kind::Request => 8907,
                Kind::Response => 8908,
            }
        }
        fn from_i32(v: i32) -> Option<Self> {
            match v {
                8907 => Some(Kind::Request),
                8908 => Some(Kind::Response),
                _ => None,
            }
        }
        fn from_str(s: &str) -> Option<Self> {
            match s {
                "k_EMsgClientPICSPrivateBetaRequest" => Some(Kind::Request),
                "k_EMsgClientPICSPrivateBetaResponse" => Some(Kind::Response),
                _ => None,
            }
        }
        const VALUES: &'static [Self] = &[Kind::Request, Kind::Response];
    }

    impl MsgKindEnum for Kind {}

    macro_rules! wrap {
        ($name:ident, $inner:ty, $kind:expr) => {
            #[derive(Debug)]
            pub struct $name(pub $inner);

            impl RpcMessage for $name {
                fn parse(
                    reader: &mut dyn std::io::Read,
                ) -> steam_vent_proto::protobuf::Result<Self> {
                    <$inner as RpcMessage>::parse(reader).map($name)
                }
                fn write(
                    &self,
                    writer: &mut dyn std::io::Write,
                ) -> steam_vent_proto::protobuf::Result<()> {
                    <$inner as RpcMessage>::write(&self.0, writer)
                }
                fn encode_size(&self) -> usize {
                    <$inner as RpcMessage>::encode_size(&self.0)
                }
            }

            impl RpcMessageWithKind for $name {
                type KindEnum = Kind;
                const KIND: Kind = $kind;
            }
        };
    }

    wrap!(Request, CMsgClientPICSPrivateBetaRequest, Kind::Request);
    wrap!(Response, CMsgClientPICSPrivateBetaResponse, Kind::Response);
}

/// The depot section of a password branch (binary KeyValues).
async fn private_depot_section(
    session: Arc<SteamSession>,
    app_id: u32,
    branch: &str,
    key: &[u8],
) -> Result<Vec<u8>, BranchError> {
    let branch = branch.to_string();
    let key = key.to_vec();
    let resp: CMsgClientPICSPrivateBetaResponse = call_with_retry(
        &session,
        2,
        TIMEOUT,
        "Private branch query timed out",
        |conn| {
            let branch = branch.clone();
            let key = key.clone();
            async move {
                let token = steam_pics::pics_access_tokens(&conn, &[app_id])
                    .await
                    .get(&app_id)
                    .copied();
                let req = private_beta::Request(CMsgClientPICSPrivateBetaRequest {
                    appid: Some(app_id),
                    access_token: token,
                    beta_name: Some(branch),
                    password_hash: Some(key),
                    ..Default::default()
                });
                conn.job::<_, private_beta::Response>(req)
                    .await
                    .map(|r| r.0)
                    .map_err(|e| format!("PICS private branch request failed: {}", e))
            }
        },
    )
    .await
    .map_err(BranchError::Steam)?;
    match resp.eresult.unwrap_or(2) {
        1 => resp.depot_section.ok_or(BranchError::NotFound),
        5 | 15 => Err(BranchError::WrongPassword),
        _ => Err(BranchError::NotFound),
    }
}

/// Depot key for `depot_id` from the account. Only asked for depots of a
/// download that uses the account; the error says what Steam answered.
pub async fn depot_key(session: Arc<SteamSession>, app_id: u32, depot_id: u32) -> Result<[u8; 32], String> {
    if !session.uses_account() {
        return Err("not signed in".into());
    }
    let resp: CMsgClientGetDepotDecryptionKeyResponse = call_with_retry(
        &session,
        2,
        TIMEOUT,
        "Depot key request timed out",
        |conn| async move {
            let req = CMsgClientGetDepotDecryptionKey {
                depot_id: Some(depot_id),
                app_id: Some(app_id),
                ..Default::default()
            };
            conn.job(req)
                .await
                .map_err(|e| format!("GetDepotDecryptionKey failed: {}", e))
        },
    )
    .await?;
    let code = resp.eresult.unwrap_or(2);
    if code != 1 {
        return Err(match steam_vent::EResult::try_from(code) {
            Ok(e) => format!("Steam answered {:?} ({})", e, code),
            Err(_) => format!("Steam answered {}", code),
        });
    }
    resp.depot_encryption_key
        .as_deref()
        .and_then(|k| k.try_into().ok())
        .ok_or_else(|| "Steam sent no key".to_string())
}

// ---- binary KeyValues -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Kv {
    Str(String),
    Num(i64),
    Table(Vec<(String, Kv)>),
}

impl Kv {
    fn get(&self, key: &str) -> Option<&Kv> {
        match self {
            Kv::Table(items) => items
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn text(&self) -> Option<String> {
        match self {
            Kv::Str(s) => Some(s.clone()),
            Kv::Num(n) => Some(n.to_string()),
            Kv::Table(_) => None,
        }
    }
}

fn read_cstr(data: &[u8], pos: &mut usize) -> Option<String> {
    let end = data[*pos..].iter().position(|&b| b == 0)? + *pos;
    let s = String::from_utf8_lossy(&data[*pos..end]).into_owned();
    *pos = end + 1;
    Some(s)
}

fn take<const N: usize>(data: &[u8], pos: &mut usize) -> Option<[u8; N]> {
    let bytes: [u8; N] = data.get(*pos..*pos + N)?.try_into().ok()?;
    *pos += N;
    Some(bytes)
}

fn parse_table(data: &[u8], pos: &mut usize, depth: usize) -> Option<Vec<(String, Kv)>> {
    if depth > 32 {
        return None;
    }
    let mut items = Vec::new();
    while *pos < data.len() {
        let ty = data[*pos];
        *pos += 1;
        if ty == 0x08 || ty == 0x0B {
            return Some(items);
        }
        let name = read_cstr(data, pos)?;
        let value = match ty {
            0x00 => Kv::Table(parse_table(data, pos, depth + 1)?),
            0x01 => Kv::Str(read_cstr(data, pos)?),
            0x02 | 0x04 | 0x06 => Kv::Num(i32::from_le_bytes(take::<4>(data, pos)?) as i64),
            0x03 => Kv::Str(f32::from_le_bytes(take::<4>(data, pos)?).to_string()),
            0x07 => Kv::Num(u64::from_le_bytes(take::<8>(data, pos)?) as i64),
            0x0A => Kv::Num(i64::from_le_bytes(take::<8>(data, pos)?)),
            _ => return None,
        };
        items.push((name, value));
    }
    Some(items)
}

fn parse_binary_kv(data: &[u8]) -> Option<Kv> {
    let mut pos = 0;
    parse_table(data, &mut pos, 0).map(Kv::Table)
}

/// Manifest gids on `branch` from a private depot section. The section is
/// either the depots themselves or a single root key ("depots") holding them.
fn gids_from_private_section(section: &[u8], branch: &str) -> HashMap<String, String> {
    let Some(root) = parse_binary_kv(section) else {
        return HashMap::new();
    };
    let depots = match &root {
        Kv::Table(items) if items.len() == 1 && !items[0].0.chars().all(|c| c.is_ascii_digit()) => {
            &items[0].1
        }
        _ => &root,
    };
    let Kv::Table(items) = depots else {
        return HashMap::new();
    };
    items
        .iter()
        .filter(|(k, _)| k.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(depot, v)| {
            let entry = v.get("manifests")?.get(branch)?;
            let gid = entry
                .get("gid")
                .and_then(Kv::text)
                .or_else(|| entry.text())?;
            Some((depot.clone(), gid))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv_str(out: &mut Vec<u8>, k: &str, v: &str) {
        out.push(0x01);
        out.extend_from_slice(k.as_bytes());
        out.push(0);
        out.extend_from_slice(v.as_bytes());
        out.push(0);
    }

    fn kv_open(out: &mut Vec<u8>, k: &str) {
        out.push(0x00);
        out.extend_from_slice(k.as_bytes());
        out.push(0);
    }

    fn section(wrapped: bool) -> Vec<u8> {
        let mut b = Vec::new();
        if wrapped {
            kv_open(&mut b, "depots");
        }
        kv_open(&mut b, "481");
        kv_open(&mut b, "manifests");
        kv_open(&mut b, "beta");
        kv_str(&mut b, "gid", "1234567890123456789");
        b.push(0x07);
        b.extend_from_slice(b"size\0");
        b.extend_from_slice(&5_000_000u64.to_le_bytes());
        b.push(0x08); // beta
        b.push(0x08); // manifests
        b.push(0x08); // 481
        kv_open(&mut b, "482");
        kv_open(&mut b, "manifests");
        kv_str(&mut b, "other", "1");
        b.push(0x08);
        b.push(0x08);
        if wrapped {
            b.push(0x08);
        }
        b.push(0x08);
        b
    }

    #[test]
    fn reads_branch_gids_from_a_private_section() {
        for wrapped in [true, false] {
            let gids = gids_from_private_section(&section(wrapped), "beta");
            assert_eq!(gids.len(), 1, "wrapped={}", wrapped);
            assert_eq!(gids["481"], "1234567890123456789");
        }
    }

    #[test]
    fn broken_sections_give_nothing() {
        assert!(gids_from_private_section(&[0x01, b'a'], "beta").is_empty());
        assert!(gids_from_private_section(&[], "beta").is_empty());
        assert!(gids_from_private_section(&[0x42, b'x', 0], "beta").is_empty());
    }

    #[test]
    fn public_is_recognized_in_any_case() {
        assert!(is_public("public") && is_public("Public") && is_public(""));
        assert!(!is_public("beta"));
    }

    #[test]
    fn private_beta_kinds_match_steamkit() {
        use steam_vent_proto::protobuf::Enum;
        assert_eq!(private_beta::Kind::Request.value(), 8907);
        assert_eq!(private_beta::Kind::Response.value(), 8908);
    }
}
