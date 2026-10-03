#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn is_shortcut_supported() -> bool {
    cfg!(target_os = "windows")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectedExecutable {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub recommended: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ShortcutResult {
    pub desktop: bool,
    #[serde(rename = "startMenu")]
    pub start_menu: bool,
    pub errors: Vec<String>,
}

pub async fn detect_executables(download_dir: &str) -> Result<Vec<DetectedExecutable>, String> {
    let dir_path = std::path::PathBuf::from(download_dir);
    if !dir_path.exists() {
        return Err("Download directory does not exist".to_string());
    }

    let mut exes: Vec<(String, String, u64)> = Vec::new();
    scan_dir_recursive(&dir_path, &mut exes, 0, 5).await;

    if exes.is_empty() {
        return Ok(Vec::new());
    }

    // Folder names look like "220 - Half-Life 2"; pull the name half as a hint.
    let folder_name = dir_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let game_hint = folder_name
        .split(" - ")
        .nth(1)
        .unwrap_or(&folder_name)
        .to_lowercase();
    let hint_words: Vec<&str> = game_hint
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .collect();

    let mut scored: Vec<(i64, String, String, u64)> = exes
        .into_iter()
        .map(|(path, name, size)| {
            let name_lower = name.to_lowercase();
            let mut score: i64 = 0;

            if is_blacklisted(&name_lower) {
                score -= 10000;
            }

            for word in &hint_words {
                if name_lower.contains(word) {
                    score += 500;
                }
            }

            score += (size / (1024 * 1024)) as i64;

            (score, path, name, size)
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let executables: Vec<DetectedExecutable> = scored
        .into_iter()
        .enumerate()
        .map(|(i, (_, path, name, size))| DetectedExecutable {
            path,
            name,
            size,
            recommended: i == 0,
        })
        .collect();

    Ok(executables)
}

async fn scan_dir_recursive(
    dir: &std::path::Path,
    results: &mut Vec<(String, String, u64)>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || results.len() >= 1000 {
        return;
    }

    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if let Ok(metadata) = entry.metadata().await {
            if metadata.is_dir() {
                Box::pin(scan_dir_recursive(&path, results, depth + 1, max_depth)).await;
            } else if metadata.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                if is_executable_candidate(&path, &name).await {
                    results.push((path.to_string_lossy().to_string(), name, metadata.len()));
                }
            }
        }

        if results.len() >= 1000 {
            break;
        }
    }
}

async fn is_executable_candidate(path: &std::path::Path, name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.ends_with(".exe") {
        return true;
    }
    if lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".manifest")
        || lower.ends_with(".vdf")
        || lower.ends_with(".txt")
        || lower.ends_with(".json")
        || lower.ends_with(".ini")
        || lower.ends_with(".pdb")
        || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".dat")
        || lower.ends_with(".pak")
        || lower.ends_with(".lua")
    {
        return false;
    }
    is_native_executable(path).await
}

#[cfg(unix)]
async fn is_native_executable(path: &std::path::Path) -> bool {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return false;
    };
    use tokio::io::AsyncReadExt;
    let mut header = [0u8; 20];
    if file.read_exact(&mut header).await.is_err() {
        return false;
    }
    if &header[..4] != b"\x7fELF" {
        return false;
    }
    let e_type = u16::from_le_bytes([header[16], header[17]]);
    e_type == 2 || e_type == 3
}

#[cfg(not(unix))]
async fn is_native_executable(_path: &std::path::Path) -> bool {
    false
}

fn is_blacklisted(name_lower: &str) -> bool {
    const BLACKLIST_EXACT: &[&str] = &[
        "unins000.exe",
        "unins001.exe",
        "uninstall.exe",
        "setup.exe",
        "installer.exe",
        "dxsetup.exe",
        "dxwebsetup.exe",
    ];

    const BLACKLIST_PREFIX: &[&str] = &[
        "unitycrashhandler",
        "ue4prereqsetup",
        "vcredist",
        "dotnetfx",
        "directx",
        "crashreporter",
        "crashhandler",
        "bugreporter",
    ];

    const BLACKLIST_CONTAINS: &[&str] = &["redist", "prerequisite", "dotnet"];

    if BLACKLIST_EXACT.contains(&name_lower) {
        return true;
    }

    for prefix in BLACKLIST_PREFIX {
        if name_lower.starts_with(prefix) {
            return true;
        }
    }

    for needle in BLACKLIST_CONTAINS {
        if name_lower.contains(needle) {
            return true;
        }
    }

    false
}

pub async fn create_shortcuts(
    exe_path: String,
    game_name: String,
    icon_path: Option<String>,
    create_desktop: bool,
    create_start_menu: bool,
) -> Result<ShortcutResult, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (
            &exe_path,
            &game_name,
            &icon_path,
            create_desktop,
            create_start_menu,
        );
        return Err("Shortcut creation is only supported on Windows".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let exe = std::path::PathBuf::from(&exe_path);
        if !exe.exists() {
            return Err("Executable not found".to_string());
        }

        let working_dir = exe
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let icon = icon_path.unwrap_or_else(|| exe_path.clone());

        let safe_name: String = game_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let mut desktop_ok = false;
        let mut start_menu_ok = false;
        let mut errors: Vec<String> = Vec::new();

        if create_desktop {
            match create_lnk_shortcut(
                &safe_name,
                &exe_path,
                &working_dir,
                &icon,
                &game_name,
                "Desktop",
            ) {
                Ok(_) => desktop_ok = true,
                Err(e) => errors.push(format!("Desktop: {}", e)),
            }
        }

        if create_start_menu {
            match create_lnk_shortcut(
                &safe_name,
                &exe_path,
                &working_dir,
                &icon,
                &game_name,
                "Programs",
            ) {
                Ok(_) => start_menu_ok = true,
                Err(e) => errors.push(format!("Start Menu: {}", e)),
            }
        }

        Ok(ShortcutResult {
            desktop: desktop_ok,
            start_menu: start_menu_ok,
            errors,
        })
    }
}

// User-derived values go via env vars, not string-interpolated into the script,
// so nothing the caller supplies can be parsed as PowerShell.
#[cfg(target_os = "windows")]
fn create_lnk_shortcut(
    safe_name: &str,
    exe_path: &str,
    working_dir: &str,
    icon_path: &str,
    description: &str,
    folder_type: &str, // "Desktop" or "Programs"
) -> Result<(), String> {
    let native = std::thread::scope(|s| {
        s.spawn(|| {
            create_lnk_native(
                safe_name,
                exe_path,
                working_dir,
                icon_path,
                description,
                folder_type,
            )
        })
        .join()
        .unwrap_or_else(|_| Err("shortcut thread panicked".to_string()))
    });
    match native {
        Ok(()) => Ok(()),
        Err(native_err) => create_lnk_powershell(
            safe_name,
            exe_path,
            working_dir,
            icon_path,
            description,
            folder_type,
        )
        .map_err(|ps_err| format!("{}; fallback: {}", native_err, ps_err)),
    }
}

#[cfg(target_os = "windows")]
fn create_lnk_native(
    safe_name: &str,
    exe_path: &str,
    working_dir: &str,
    icon_path: &str,
    description: &str,
    folder_type: &str,
) -> Result<(), String> {
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, SHGetKnownFolderPath, ShellLink,
        KF_FLAG_DEFAULT,
    };

    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| -> Result<(), String> {
            let folder_id = if folder_type == "Desktop" {
                &FOLDERID_Desktop
            } else {
                &FOLDERID_Programs
            };
            let raw = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None)
                .map_err(|e| format!("folder lookup failed: {}", e))?;
            let folder = raw.to_string();
            CoTaskMemFree(Some(raw.0 as *const _));
            let folder = folder.map_err(|e| format!("folder path invalid: {}", e))?;
            let target = std::path::Path::new(&folder).join(format!("{}.lnk", safe_name));

            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("ShellLink unavailable: {}", e))?;
            link.SetPath(&HSTRING::from(exe_path))
                .map_err(|e| e.to_string())?;
            link.SetWorkingDirectory(&HSTRING::from(working_dir))
                .map_err(|e| e.to_string())?;
            link.SetDescription(&HSTRING::from(description))
                .map_err(|e| e.to_string())?;
            link.SetIconLocation(&HSTRING::from(icon_path), 0)
                .map_err(|e| e.to_string())?;
            let file: IPersistFile = link.cast().map_err(|e| e.to_string())?;
            file.Save(&HSTRING::from(target.as_os_str()), true)
                .map_err(|e| format!("saving shortcut failed: {}", e))
        })();
        if initialized {
            CoUninitialize();
        }
        result
    }
}

#[cfg(target_os = "windows")]
fn create_lnk_powershell(
    safe_name: &str,
    exe_path: &str,
    working_dir: &str,
    icon_path: &str,
    description: &str,
    folder_type: &str,
) -> Result<(), String> {
    const SCRIPT: &str = "\
        $ErrorActionPreference = 'Stop';\
        $folder = [Environment]::GetFolderPath($env:SMD_FOLDER);\
        $target = Join-Path $folder ($env:SMD_NAME + '.lnk');\
        $ws = New-Object -ComObject WScript.Shell;\
        $s = $ws.CreateShortcut($target);\
        $s.TargetPath = $env:SMD_TARGET;\
        $s.WorkingDirectory = $env:SMD_WORKDIR;\
        $s.Description = $env:SMD_DESC;\
        $s.IconLocation = $env:SMD_ICON;\
        $s.Save()";

    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT]);
    cmd.env("SMD_FOLDER", folder_type);
    cmd.env("SMD_NAME", safe_name);
    cmd.env("SMD_TARGET", exe_path);
    cmd.env("SMD_WORKDIR", working_dir);
    cmd.env("SMD_DESC", description);
    cmd.env("SMD_ICON", icon_path);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("PowerShell error: {}", stderr.trim()))
            }
        }
        Err(e) => Err(format!("Failed to run PowerShell: {}", e)),
    }
}
