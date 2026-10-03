use std::path::Path;

use serde::{Deserialize, Serialize};
use smd_core::services::emulator::EmuSettings;

use crate::theme::ThemeMode;

const FILE: &str = "tui.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub theme: ThemeMode,
    pub skipped_update: Option<String>,
    pub mh_api_key: String,
    pub last_emu_settings: Option<EmuSettings>,
}

impl Prefs {
    pub async fn load(dir: &Path) -> Self {
        match tokio::fs::read_to_string(dir.join(FILE)).await {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Prefs::default(),
        }
    }

    pub async fn save(&self, dir: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| e.to_string())?;
        tokio::fs::write(dir.join(FILE), json)
            .await
            .map_err(|e| e.to_string())
    }
}

impl super::App {
    pub fn save_prefs(&self) {
        let prefs = self.prefs.clone();
        let dir = self.data_dir.clone();
        tokio::spawn(async move {
            let _ = prefs.save(&dir).await;
        });
    }
}
