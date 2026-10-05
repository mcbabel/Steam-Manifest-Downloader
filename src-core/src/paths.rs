use std::path::PathBuf;

pub const APP_IDENTIFIER: &str = "com.steammanifestdownloader.desktop";

pub const DATA_DIR_ENV: &str = "SMD_DATA_DIR";

pub const BUNDLED_DIR_ENV: &str = "SMD_BUNDLED_DIR";

pub const BUNDLED_DIR_NAME: &str = "third-party";

pub fn bundled_tools_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(BUNDLED_DIR_ENV).filter(|v| !v.is_empty()) {
        let dir = PathBuf::from(dir);
        return dir.is_dir().then_some(dir);
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join(BUNDLED_DIR_NAME);
    dir.is_dir().then_some(dir)
}

pub fn bundled_file(parts: &[&str]) -> Option<PathBuf> {
    let mut path = bundled_tools_dir()?;
    for part in parts {
        path.push(part);
    }
    path.is_file().then_some(path)
}

pub fn default_app_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .map(|d| d.join(APP_IDENTIFIER))
        .unwrap_or_else(|| PathBuf::from(".").join(APP_IDENTIFIER))
}
