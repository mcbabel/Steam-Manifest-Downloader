use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const FILE: &str = "gui-state.json";

#[derive(Default)]
struct Inner {
    path: Option<PathBuf>,
    values: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub struct LocalStore {
    inner: Arc<Mutex<Inner>>,
}

impl LocalStore {
    pub fn memory() -> Self {
        Self::default()
    }

    pub fn open(data_dir: &Path) -> Self {
        let path = data_dir.join(FILE);
        let values = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { inner: Arc::new(Mutex::new(Inner { path: Some(path), values })) }
    }

    pub fn exists(&self) -> bool {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).path.as_ref().is_some_and(|path| path.is_file())
    }

    pub fn persist(&self) {
        Self::flush(&self.inner.lock().unwrap_or_else(|e| e.into_inner()));
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).values.get(key).cloned()
    }

    pub fn set(&self, key: &str, value: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.values.get(key).map(String::as_str) == Some(value) {
            return;
        }
        inner.values.insert(key.to_string(), value.to_string());
        Self::flush(&inner);
    }

    pub fn remove(&self, key: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.values.remove(key).is_some() {
            Self::flush(&inner);
        }
    }

    fn flush(inner: &Inner) {
        let Some(path) = inner.path.as_ref() else { return };
        if let Ok(text) = serde_json::to_string_pretty(&inner.values) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("smd-gui-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = LocalStore::open(&dir);
        assert_eq!(store.get("theme"), None);
        store.set("theme", "light");
        store.set("other", "x");
        store.remove("other");
        let again = LocalStore::open(&dir);
        assert_eq!(again.get("theme").as_deref(), Some("light"));
        assert_eq!(again.get("other"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
