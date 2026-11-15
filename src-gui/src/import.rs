use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::store::LocalStore;

const IDENTIFIER: &str = smd_core::paths::APP_IDENTIFIER;

pub const KEYS: [&str; 9] =
    ["theme", "manifestHubApiKey", "lastEmuSettings_v1", "smd.historyView", "dismissedNotices_v1", "settingsSection", "skippedUpdateVersion", "dotnetWarningDismissed", "emuGameData"];

pub fn import_once(store: &LocalStore) {
    if store.exists() {
        return;
    }
    for (key, value) in web_view_values() {
        if KEYS.contains(&key.as_str()) && store.get(&key).is_none() {
            store.set(&key, &value);
        }
    }
    store.persist();
}

fn web_view_values() -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for root in roots() {
        collect(&root, &mut found);
    }
    found
}

fn roots() -> Vec<PathBuf> {
    [dirs::data_local_dir(), dirs::data_dir(), dirs::cache_dir()]
        .into_iter()
        .flatten()
        .map(|dir| dir.join(IDENTIFIER))
        .chain(dirs::home_dir().map(|home| home.join("Library").join("WebKit").join(IDENTIFIER)))
        .filter(|dir| dir.is_dir())
        .collect()
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
    String::from_utf16_lossy(&units)
}

#[cfg(any(windows, test))]
mod chromium {
    use std::collections::BTreeMap;
    use std::path::Path;

    use rusty_leveldb::{LdbIterator, Options, DB};

    fn text(bytes: &[u8]) -> Option<String> {
        match bytes.split_first()? {
            (0, rest) => Some(super::utf16(rest)),
            (1, rest) => Some(rest.iter().map(|byte| *byte as char).collect()),
            _ => None,
        }
    }

    fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && entry.file_name() != "LOCK" {
                std::fs::copy(entry.path(), to.join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    pub fn read(leveldb: &Path, found: &mut BTreeMap<String, String>) {
        let copy = std::env::temp_dir().join(format!("smd-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&copy);
        if copy_dir(leveldb, &copy).is_ok() {
            let options = Options { create_if_missing: false, ..Options::default() };
            if let Ok(mut db) = DB::open(&copy, options) {
                if let Ok(mut entries) = db.new_iter() {
                    while let Some((key, value)) = entries.next() {
                        let Some(rest) = key.strip_prefix(b"_") else { continue };
                        let Some(split) = rest.iter().position(|byte| *byte == 0) else { continue };
                        if let (Some(name), Some(value)) = (text(&rest[split + 1..]), text(&value)) {
                            found.insert(name, value);
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&copy);
    }
}

#[cfg(unix)]
mod webkit {
    use std::collections::BTreeMap;
    use std::path::Path;

    use rusqlite::{Connection, OpenFlags};

    pub fn read(file: &Path, found: &mut BTreeMap<String, String>) {
        let Ok(db) = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY) else { return };
        let Ok(mut query) = db.prepare("SELECT key, value FROM ItemTable") else { return };
        let Ok(rows) = query.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))) else { return };
        for (key, value) in rows.flatten() {
            found.insert(key, super::utf16(&value));
        }
    }
}

fn collect(dir: &Path, found: &mut BTreeMap<String, String>) {
    walk(dir, 0, found);
}

fn walk(dir: &Path, depth: usize, found: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            #[cfg(windows)]
            if name == "leveldb" && dir.file_name().is_some_and(|parent| parent == "Local Storage") {
                chromium::read(&path, found);
                continue;
            }
            if depth < 7 {
                walk(&path, depth + 1, found);
            }
        } else {
            #[cfg(unix)]
            if name.ends_with(".localstorage") || name == "localstorage.sqlite3" {
                webkit::read(&path, found);
            }
            let _ = &name;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smd-import-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn chromium_local_storage_is_read() {
        use rusty_leveldb::{Options, DB};
        let dir = scratch("leveldb");
        {
            let mut db = DB::open(&dir, Options::default()).unwrap();
            let mut key = b"_http://tauri.localhost\0\x01theme".to_vec();
            db.put(&key, b"\x01light").unwrap();
            key = b"_http://tauri.localhost\0\x01manifestHubApiKey".to_vec();
            let mut wide = vec![0u8];
            wide.extend("schl\u{fc}ssel".encode_utf16().flat_map(u16::to_le_bytes));
            db.put(&key, &wide).unwrap();
            db.put(b"META:http://tauri.localhost", b"x").unwrap();
            db.flush().unwrap();
        }
        let mut found = BTreeMap::new();
        chromium::read(&dir, &mut found);
        assert_eq!(found.get("theme").map(String::as_str), Some("light"));
        assert_eq!(found.get("manifestHubApiKey").map(String::as_str), Some("schl\u{fc}ssel"));
        assert_eq!(found.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn webkit_local_storage_is_read() {
        let dir = scratch("sqlite");
        let file = dir.join("tauri_localhost_0.localstorage");
        {
            let db = rusqlite::Connection::open(&file).unwrap();
            db.execute("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB NOT NULL ON CONFLICT FAIL)", []).unwrap();
            let value: Vec<u8> = "light".encode_utf16().flat_map(u16::to_le_bytes).collect();
            db.execute("INSERT INTO ItemTable (key, value) VALUES (?1, ?2)", rusqlite::params!["theme", value]).unwrap();
        }
        let mut found = BTreeMap::new();
        collect(&dir, &mut found);
        assert_eq!(found.get("theme").map(String::as_str), Some("light"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_known_values_are_taken_and_only_once() {
        let dir = scratch("store");
        let store = LocalStore::open(&dir);
        assert!(!store.exists());
        import_once(&store);
        assert!(store.exists(), "the store file is written even when nothing was found");
        store.set("theme", "light");
        let again = LocalStore::open(&dir);
        import_once(&again);
        assert_eq!(again.get("theme").as_deref(), Some("light"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
