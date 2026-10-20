//! The signed-in Steam account: where its refresh token is kept.
//!
//! In order of preference:
//! 1. the system keychain (Windows Credential Manager, macOS Keychain, the
//!    Secret Service on Linux),
//! 2. a file readable only by the user, but only when the user asked for it
//!    (`smd login --store-file`, for servers and Docker); it is not encrypted,
//! 3. memory only: the sign-in lasts until the app closes.
//!
//! The token never goes into settings.json, the history, events or telemetry.
//! Frontends only ever get an [`AccountInfo`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::services::steam_auth::{token_info, Secret};

const KEYRING_SERVICE: &str = "steam-manifest-downloader";
const KEYRING_USER: &str = "steam-login";
const FILE_NAME: &str = "steam-login.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    Keychain,
    File,
    Session,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub account_name: String,
    pub steam_id: u64,
    pub refresh_token: Secret,
    pub expires_at: Option<i64>,
    pub storage: Storage,
}

/// What frontends may see about the account. No token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub account_name: String,
    pub steam_id: String,
    pub expires_at: Option<i64>,
    pub storage: Storage,
}

impl Account {
    pub fn info(&self) -> AccountInfo {
        AccountInfo {
            account_name: self.account_name.clone(),
            steam_id: self.steam_id.to_string(),
            expires_at: self.expires_at,
            storage: self.storage,
        }
    }

    pub fn is_expired(&self, now: i64) -> bool {
        self.expires_at.is_some_and(|exp| exp <= now)
    }
}

/// How a stored login looks on disk / in the keychain.
#[derive(Serialize, Deserialize)]
struct Stored {
    account_name: String,
    refresh_token: String,
}

fn from_stored(s: Stored, storage: Storage) -> Option<Account> {
    let token = Secret::new(s.refresh_token);
    let info = token_info(&token)?;
    Some(Account {
        account_name: s.account_name,
        steam_id: info.steam_id,
        refresh_token: token,
        expires_at: info.expires_at,
        storage,
    })
}

fn to_stored(account: &Account) -> String {
    serde_json::to_string(&Stored {
        account_name: account.account_name.clone(),
        refresh_token: account.refresh_token.expose().to_string(),
    })
    .unwrap_or_default()
}

fn file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

// ---- keychain -------------------------------------------------------------

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod keychain {
    use super::{KEYRING_SERVICE, KEYRING_USER};

    fn entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())
    }

    pub fn load() -> Result<Option<String>, String> {
        match entry()?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn save(value: &str) -> Result<(), String> {
        entry()?.set_password(value).map_err(|e| e.to_string())
    }

    pub fn delete() -> Result<(), String> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod keychain {
    pub fn load() -> Result<Option<String>, String> {
        Err("no keychain on this system".into())
    }
    pub fn save(_: &str) -> Result<(), String> {
        Err("no keychain on this system".into())
    }
    pub fn delete() -> Result<(), String> {
        Ok(())
    }
}

/// Keychain calls can block (D-Bus, a password prompt), so they run off the
/// async threads.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(f).await.ok()
}

// ---- file -----------------------------------------------------------------

async fn write_private(path: &Path, content: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let tmp2 = tmp.clone();
        let content = content.to_string();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let _ = std::fs::remove_file(&tmp2);
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp2)?;
            f.write_all(content.as_bytes())?;
            f.sync_all()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    tokio::fs::write(&tmp, content)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| e.to_string())
}

// ---- store ----------------------------------------------------------------

/// The account of this app instance. Holds the session copy so a login
/// without a keychain survives until the app closes.
pub struct AccountStore {
    current: Mutex<Option<Option<Account>>>,
}

impl Default for AccountStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountStore {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// The signed-in account, loading it from the keychain or the file the
    /// first time.
    pub async fn get(&self, data_dir: &Path) -> Option<Account> {
        let mut guard = self.current.lock().await;
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
        let loaded = load(data_dir).await;
        *guard = Some(loaded.clone());
        loaded
    }

    /// Saves a fresh login. `allow_file` is the user's explicit choice to
    /// use the unencrypted file when there is no keychain.
    pub async fn set(
        &self,
        data_dir: &Path,
        account_name: String,
        refresh_token: Secret,
        allow_file: bool,
    ) -> Result<Account, String> {
        let info = token_info(&refresh_token).ok_or("Steam returned a token SMD cannot read")?;
        let mut account = Account {
            account_name,
            steam_id: info.steam_id,
            refresh_token,
            expires_at: info.expires_at,
            storage: Storage::Session,
        };
        let value = to_stored(&account);
        let saved = {
            let value = value.clone();
            blocking(move || keychain::save(&value)).await
        };
        match saved {
            Some(Ok(())) => {
                account.storage = Storage::Keychain;
                let _ = tokio::fs::remove_file(file_path(data_dir)).await;
            }
            Some(Err(e)) => {
                crate::dlog!("Account", "keychain unavailable: {}", e);
                if allow_file {
                    write_private(&file_path(data_dir), &value).await?;
                    account.storage = Storage::File;
                }
            }
            None => {}
        }
        *self.current.lock().await = Some(Some(account.clone()));
        Ok(account)
    }

    /// Forgets the login everywhere it might be stored.
    pub async fn clear(&self, data_dir: &Path) {
        let _ = blocking(keychain::delete).await;
        let _ = tokio::fs::remove_file(file_path(data_dir)).await;
        *self.current.lock().await = Some(None);
    }
}

async fn load(data_dir: &Path) -> Option<Account> {
    if let Some(Ok(Some(value))) = blocking(keychain::load).await {
        if let Some(a) = serde_json::from_str::<Stored>(&value)
            .ok()
            .and_then(|s| from_stored(s, Storage::Keychain))
        {
            return Some(a);
        }
    }
    let content = tokio::fs::read_to_string(file_path(data_dir)).await.ok()?;
    serde_json::from_str::<Stored>(&content)
        .ok()
        .and_then(|s| from_stored(s, Storage::File))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn token(sub: &str) -> Secret {
        let enc = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        let claims = serde_json::json!({ "iss": "steam", "sub": sub, "exp": 1900000000 });
        Secret::new(format!(
            "{}.{}.sig",
            enc(b"{}"),
            enc(claims.to_string().as_bytes())
        ))
    }

    #[test]
    fn info_carries_no_token() {
        let a = from_stored(
            Stored {
                account_name: "me".into(),
                refresh_token: token("76561197960287930").expose().into(),
            },
            Storage::File,
        )
        .unwrap();
        let json = serde_json::to_string(&a.info()).unwrap();
        assert!(
            json.contains("\"accountName\":\"me\"")
                && json.contains("\"steamId\":\"76561197960287930\"")
        );
        assert!(!json.contains(a.refresh_token.expose()));
        assert!(!format!("{:?}", a).contains(a.refresh_token.expose()));
    }

    #[test]
    fn expiry_is_checked_against_now() {
        let mut a = from_stored(
            Stored {
                account_name: "me".into(),
                refresh_token: token("1").expose().into(),
            },
            Storage::Session,
        )
        .unwrap();
        assert!(!a.is_expired(1_800_000_000));
        assert!(a.is_expired(1_900_000_000));
        a.expires_at = None;
        assert!(!a.is_expired(i64::MAX));
    }

    #[tokio::test]
    async fn file_login_round_trips_and_is_private() {
        let dir = std::env::temp_dir().join(format!("smd-acct-{}", uuid::Uuid::new_v4()));
        let a = from_stored(
            Stored {
                account_name: "me".into(),
                refresh_token: token("42").expose().into(),
            },
            Storage::File,
        )
        .unwrap();
        write_private(&file_path(&dir), &to_stored(&a))
            .await
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(file_path(&dir))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let content = tokio::fs::read_to_string(file_path(&dir)).await.unwrap();
        let back = from_stored(serde_json::from_str(&content).unwrap(), Storage::File).unwrap();
        assert_eq!(back.account_name, "me");
        assert_eq!(back.steam_id, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
