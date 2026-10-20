//! Steam sign-in with a QR code, over the IAuthenticationService Web API.
//!
//! SMD never sees the Steam password: the user scans the code with the Steam
//! mobile app and approves the sign-in there. Steam then hands out a refresh
//! token for this device, which `steam_account` stores and `steam_session`
//! uses to log on to Steam.

use std::fmt;

use serde::Deserialize;

const API: &str = "https://api.steampowered.com/IAuthenticationService";
/// Shown in the Steam app when approving, and in the account's device list.
pub const DEVICE_NAME: &str = "Steam Manifest Downloader";
/// k_EAuthTokenPlatformType_SteamClient: a token a CM client logon accepts.
const PLATFORM_STEAM_CLIENT: &str = "1";

/// A token that must never end up in logs, events, settings or telemetry.
/// It has no `Serialize`, and `Debug` prints `***`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// The code timed out or was replaced; start a new one.
    Expired,
    /// Steam answered with an EResult other than OK.
    Steam(i32),
    Network(String),
    Invalid(String),
}

impl AuthError {
    /// Stable key for translations and telemetry.
    pub fn key(&self) -> &'static str {
        match self {
            AuthError::Expired => "expired",
            AuthError::Steam(_) => "steam",
            AuthError::Network(_) => "network",
            AuthError::Invalid(_) => "invalid",
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Expired => f.write_str("The QR code expired. Start the sign-in again."),
            AuthError::Steam(code) => write!(f, "Steam refused the sign-in (EResult {}).", code),
            AuthError::Network(e) => write!(f, "Could not reach Steam: {}", e),
            AuthError::Invalid(e) => write!(f, "Unexpected answer from Steam: {}", e),
        }
    }
}

/// A running QR sign-in. `challenge_url` is what the QR code encodes; Steam
/// may replace it while the sign-in runs.
#[derive(Clone)]
pub struct QrChallenge {
    pub client_id: String,
    request_id: String,
    pub challenge_url: String,
    pub interval_secs: f32,
}

impl fmt::Debug for QrChallenge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrChallenge")
            .field("client_id", &self.client_id)
            .field("interval_secs", &self.interval_secs)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrPoll {
    /// Not approved yet. `scanned` turns true once the code was scanned.
    Waiting { scanned: bool, new_url: bool },
    Approved {
        account_name: String,
        refresh_token: Secret,
    },
}

#[derive(Deserialize)]
struct Envelope<T> {
    response: T,
}

#[derive(Deserialize)]
struct BeginResponse {
    client_id: Option<String>,
    challenge_url: Option<String>,
    request_id: Option<String>,
    interval: Option<f32>,
}

#[derive(Deserialize, Default)]
struct PollResponse {
    new_client_id: Option<String>,
    new_challenge_url: Option<String>,
    refresh_token: Option<String>,
    had_remote_interaction: Option<bool>,
    account_name: Option<String>,
}

async fn post<T: for<'de> Deserialize<'de>>(
    method: &str,
    form: &[(&str, &str)],
) -> Result<T, AuthError> {
    let url = format!("{}/{}/v1/", API, method);
    let resp = crate::services::net::client()
        .post(&url)
        .form(form)
        .send()
        .await
        .map_err(|e| AuthError::Network(crate::services::net::describe(&e)))?;
    let eresult = resp
        .headers()
        .get("x-eresult")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    if !resp.status().is_success() {
        return Err(AuthError::Network(format!(
            "HTTP {}",
            resp.status().as_u16()
        )));
    }
    match eresult {
        1 => {}
        // FileNotFound / Expired / AccessDenied: the session is gone.
        9 | 27 | 15 => return Err(AuthError::Expired),
        other => return Err(AuthError::Steam(other)),
    }
    resp.json::<Envelope<T>>()
        .await
        .map(|e| e.response)
        .map_err(|e| AuthError::Invalid(e.to_string()))
}

/// Starts a QR sign-in.
pub async fn begin_qr() -> Result<QrChallenge, AuthError> {
    let r: BeginResponse = post(
        "BeginAuthSessionViaQR",
        &[
            ("device_friendly_name", DEVICE_NAME),
            ("platform_type", PLATFORM_STEAM_CLIENT),
        ],
    )
    .await?;
    match (r.client_id, r.challenge_url, r.request_id) {
        (Some(client_id), Some(challenge_url), Some(request_id)) => Ok(QrChallenge {
            client_id,
            request_id,
            challenge_url,
            interval_secs: r.interval.unwrap_or(5.0).clamp(1.0, 30.0),
        }),
        _ => Err(AuthError::Invalid("incomplete QR session".into())),
    }
}

/// Asks Steam whether the code was approved. Updates `challenge` when Steam
/// hands out a new code.
pub async fn poll_qr(challenge: &mut QrChallenge) -> Result<QrPoll, AuthError> {
    let client_id = challenge.client_id.clone();
    let request_id = challenge.request_id.clone();
    let r: PollResponse = post(
        "PollAuthSessionStatus",
        &[
            ("client_id", client_id.as_str()),
            ("request_id", request_id.as_str()),
        ],
    )
    .await?;
    Ok(apply_poll(challenge, r))
}

fn apply_poll(challenge: &mut QrChallenge, r: PollResponse) -> QrPoll {
    if let (Some(token), Some(account_name)) =
        (r.refresh_token.filter(|t| !t.is_empty()), r.account_name)
    {
        return QrPoll::Approved {
            account_name,
            refresh_token: Secret::new(token),
        };
    }
    let mut new_url = false;
    if let Some(id) = r.new_client_id {
        challenge.client_id = id;
    }
    if let Some(url) = r.new_challenge_url.filter(|u| !u.is_empty()) {
        new_url = url != challenge.challenge_url;
        challenge.challenge_url = url;
    }
    QrPoll::Waiting {
        scanned: r.had_remote_interaction.unwrap_or(false),
        new_url,
    }
}

/// What a refresh token says about itself (it is a JWT). Read only for
/// display and for the logon; the signature is checked by Steam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    pub steam_id: u64,
    /// Unix seconds.
    pub expires_at: Option<i64>,
}

pub fn token_info(token: &Secret) -> Option<TokenInfo> {
    use base64::Engine;
    #[derive(Deserialize)]
    struct Claims {
        iss: Option<String>,
        sub: Option<String>,
        exp: Option<i64>,
    }
    let payload = token.expose().split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: Claims = serde_json::from_slice(&bytes).ok()?;
    if claims.iss.as_deref() != Some("steam") {
        return None;
    }
    Some(TokenInfo {
        steam_id: claims.sub?.parse().ok()?,
        expires_at: claims.exp,
    })
}

/// SVG of a QR code for `url`, sized by its container.
pub fn qr_svg(url: &str) -> Result<String, String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<svg::Color>()
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// The QR code as text for a terminal, two module rows per character.
/// Blocks are drawn for dark modules; with `blocks_are_light` they are drawn
/// for the light ones instead, which reads right on a dark terminal without
/// colors.
pub fn qr_text(url: &str, blocks_are_light: bool) -> Result<Vec<String>, String> {
    use qrcode::render::unicode::Dense1x2;
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
    let (dark, light) = if blocks_are_light {
        (Dense1x2::Light, Dense1x2::Dark)
    } else {
        (Dense1x2::Dark, Dense1x2::Light)
    };
    Ok(code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .dark_color(dark)
        .light_color(light)
        .build()
        .lines()
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn jwt(claims: serde_json::Value) -> Secret {
        let enc = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        Secret::new(format!(
            "{}.{}.sig",
            enc(b"{\"alg\":\"EdDSA\"}"),
            enc(claims.to_string().as_bytes())
        ))
    }

    fn challenge() -> QrChallenge {
        QrChallenge {
            client_id: "1".into(),
            request_id: "r".into(),
            challenge_url: "https://s.team/q/1/1".into(),
            interval_secs: 5.0,
        }
    }

    #[test]
    fn secret_debug_hides_the_value() {
        let s = Secret::new("eyJhbGciOi.secret.sig");
        assert_eq!(format!("{:?}", s), "***");
        assert!(!format!(
            "{:?}",
            QrPoll::Approved {
                account_name: "me".into(),
                refresh_token: s
            }
        )
        .contains("secret"));
    }

    #[test]
    fn reads_steam_id_and_expiry_from_the_token() {
        let t = jwt(
            serde_json::json!({ "iss": "steam", "sub": "76561197960287930", "exp": 1800000000 }),
        );
        assert_eq!(
            token_info(&t),
            Some(TokenInfo {
                steam_id: 76561197960287930,
                expires_at: Some(1800000000)
            })
        );
    }

    #[test]
    fn rejects_tokens_from_other_issuers_and_garbage() {
        assert_eq!(
            token_info(&jwt(serde_json::json!({ "iss": "x", "sub": "1" }))),
            None
        );
        assert_eq!(token_info(&Secret::new("not a jwt")), None);
    }

    #[test]
    fn poll_keeps_waiting_and_follows_a_new_code() {
        let mut c = challenge();
        let r = PollResponse {
            new_client_id: Some("2".into()),
            new_challenge_url: Some("https://s.team/q/1/2".into()),
            had_remote_interaction: Some(true),
            ..Default::default()
        };
        assert_eq!(
            apply_poll(&mut c, r),
            QrPoll::Waiting {
                scanned: true,
                new_url: true
            }
        );
        assert_eq!(c.client_id, "2");
        assert_eq!(c.challenge_url, "https://s.team/q/1/2");
    }

    #[test]
    fn poll_returns_the_token_once_approved() {
        let mut c = challenge();
        let r = PollResponse {
            refresh_token: Some("tok".into()),
            account_name: Some("me".into()),
            ..Default::default()
        };
        assert_eq!(
            apply_poll(&mut c, r),
            QrPoll::Approved {
                account_name: "me".into(),
                refresh_token: Secret::new("tok")
            }
        );
    }

    #[test]
    fn renders_the_code_as_svg_and_text() {
        let svg = qr_svg("https://s.team/q/1/13199529398481165711").unwrap();
        assert!(svg.starts_with("<?xml") && svg.contains("<svg"));
        let lines = qr_text("https://s.team/q/1/13199529398481165711", true).unwrap();
        assert!(
            lines.len() > 10
                && lines
                    .iter()
                    .all(|l| l.chars().count() == lines[0].chars().count())
        );
    }
}
