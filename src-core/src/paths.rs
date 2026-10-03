use std::path::PathBuf;

pub const APP_IDENTIFIER: &str = "com.steammanifestdownloader.desktop";

pub const DATA_DIR_ENV: &str = "SMD_DATA_DIR";

pub fn default_app_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .map(|d| d.join(APP_IDENTIFIER))
        .unwrap_or_else(|| PathBuf::from(".").join(APP_IDENTIFIER))
}
