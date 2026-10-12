use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

#[cfg(not(target_os = "windows"))]
use crate::services::steam_library;

pub const NEEDS_PROTON: &str = "WINDOWS_GAME_NEEDS_PROTON";
pub const NEEDS_WINE_MAC: &str = "WINDOWS_GAME_NEEDS_WINE_MAC";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMethod {
    Direct,
    Steam,
    Wine,
}

fn prefs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("launch.json")
}

fn load_prefs(app_data_dir: &Path) -> HashMap<String, String> {
    std::fs::read(prefs_path(app_data_dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn saved_exe(app_data_dir: &Path, game_dir: &str) -> Option<String> {
    load_prefs(app_data_dir)
        .remove(game_dir)
        .filter(|exe| Path::new(exe).is_file())
}

pub fn remember_exe(app_data_dir: &Path, game_dir: &str, exe: &str) -> Result<(), String> {
    let mut prefs = load_prefs(app_data_dir);
    prefs.insert(game_dir.to_string(), exe.to_string());
    std::fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(prefs_path(app_data_dir), json).map_err(|e| e.to_string())
}

#[cfg_attr(target_os = "windows", allow(dead_code))]
fn is_windows_binary(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("bat"))
}

#[cfg(not(target_os = "windows"))]
fn find_program(program: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    dirs.into_iter().map(|dir| dir.join(program)).find(|p| p.is_file())
}

fn spawn(mut cmd: Command, cwd: &Path) -> Result<(), String> {
    cmd.current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn launch(exe: &str) -> Result<LaunchMethod, String> {
    let path = PathBuf::from(exe);
    if !path.is_file() {
        return Err("Executable not found".to_string());
    }
    let cwd = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "windows")]
    {
        spawn(Command::new(&path), &cwd)?;
        Ok(LaunchMethod::Direct)
    }

    #[cfg(not(target_os = "windows"))]
    {
        if !is_windows_binary(&path) {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&path) {
                let mut perms = meta.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(perms.mode() | 0o755);
                    let _ = std::fs::set_permissions(&path, perms);
                }
            }
            spawn(Command::new(&path), &cwd)?;
            return Ok(LaunchMethod::Direct);
        }
        if let Some(appid) = steam_library::find_shortcut_appid(exe) {
            steam_library::run_shortcut(appid)?;
            return Ok(LaunchMethod::Steam);
        }
        if let Some(wine) = find_program("wine").or_else(|| find_program("wine64")) {
            let mut cmd = Command::new(wine);
            cmd.arg(&path);
            spawn(cmd, &cwd)?;
            return Ok(LaunchMethod::Wine);
        }
        if cfg!(target_os = "macos") {
            Err(NEEDS_WINE_MAC.to_string())
        } else {
            Err(NEEDS_PROTON.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_the_chosen_executable_per_game_folder() {
        let dir = std::env::temp_dir().join(format!("smd-launch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("game")).unwrap();
        let exe = dir.join("game/run.exe");
        std::fs::write(&exe, b"x").unwrap();
        let game = dir.join("game").to_string_lossy().to_string();
        assert_eq!(saved_exe(&dir, &game), None);
        remember_exe(&dir, &game, &exe.to_string_lossy()).unwrap();
        assert_eq!(saved_exe(&dir, &game), Some(exe.to_string_lossy().to_string()));
        std::fs::remove_file(&exe).unwrap();
        assert_eq!(saved_exe(&dir, &game), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_executables_are_reported() {
        assert_eq!(launch("/no/such/game.exe"), Err("Executable not found".to_string()));
    }

    #[test]
    fn windows_binaries_are_recognised() {
        assert!(is_windows_binary(Path::new("Game.EXE")));
        assert!(!is_windows_binary(Path::new("game.x86_64")));
    }
}
