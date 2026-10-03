use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

static LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

fn log_file() -> &'static Mutex<Option<std::fs::File>> {
    LOG_FILE.get_or_init(|| Mutex::new(open_log()))
}

fn open_log() -> Option<std::fs::File> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("smd-debug.log"))
        .ok()
}

fn log_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().or_else(|| std::env::var("USERPROFILE").ok())?;
    Some(PathBuf::from(home).join(".cache").join("steam-manifest-downloader"))
}

pub fn log_path() -> Option<PathBuf> {
    Some(log_dir()?.join("smd-debug.log"))
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

pub fn write(scope: &str, msg: &str) {
    let line = format!("[{}] [{}] {}\n", timestamp(), scope, msg);
    eprint!("{}", line);
    if let Ok(mut guard) = log_file().lock() {
        if let Some(file) = guard.as_mut() {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

#[macro_export]
macro_rules! dlog {
    ($scope:expr, $($arg:tt)*) => {{
        $crate::services::debug_log::write($scope, &format!($($arg)*));
    }};
}
