use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

pub const MARKER: &str = ".smd-coldclient.json";
const INI: &str = "ColdClientLoader.ini";
const CLIENT_FILES: [&str; 4] = [
    "steamclient.dll",
    "steamclient64.dll",
    "GameOverlayRenderer.dll",
    "GameOverlayRenderer64.dll",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ColdInstall {
    pub exe: String,
    pub loader: String,
    pub files: Vec<String>,
}

pub fn loader_source(platform_cache: &Path) -> Option<PathBuf> {
    WalkDir::new(platform_cache)
        .max_depth(5)
        .into_iter()
        .flatten()
        .find(|e| {
            e.file_type().is_dir()
                && e.file_name().to_string_lossy().eq_ignore_ascii_case("steamclient_experimental")
                && e.path().join("steamclient_loader_x64.exe").is_file()
        })
        .map(|e| e.path().to_path_buf())
}

pub fn exe_is_64(exe: &Path) -> Option<bool> {
    let data = fs::read(exe).ok()?;
    let pe = u32::from_le_bytes(data.get(0x3c..0x40)?.try_into().ok()?) as usize;
    if data.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let machine = u16::from_le_bytes(data.get(pe + 4..pe + 6)?.try_into().ok()?);
    match machine {
        0x8664 | 0xaa64 => Some(true),
        0x014c => Some(false),
        _ => None,
    }
}

pub fn loader_name(x64: bool) -> &'static str {
    if x64 {
        "steamclient_loader_x64.exe"
    } else {
        "steamclient_loader_x86.exe"
    }
}

fn read_marker(dir: &Path) -> Option<ColdInstall> {
    fs::read_to_string(dir.join(MARKER)).ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn render_ini(exe_name: &str, app_id: &str) -> String {
    format!(
        "[SteamClient]\nExe={}\nExeRunDir=\nExeCommandLine=\nAppId={}\nSteamClientDll=steamclient.dll\nSteamClient64Dll=steamclient64.dll\n\n[Injection]\nForceInjectSteamClient=0\nForceInjectGameOverlayRenderer=0\nDllsToInjectFolder=\nIgnoreInjectionError=1\nIgnoreLoaderArchDifference=0\n\n[Persistence]\nMode=0\n\n[Debug]\nResumeByDebugger=0\n",
        exe_name,
        app_id.trim()
    )
}

pub fn install(src: &Path, exe: &Path, app_id: &str) -> Result<ColdInstall, String> {
    let is_exe = exe.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    if !is_exe || !exe.is_file() {
        return Err("Choose the game's Windows program (.exe)".into());
    }
    let dir = exe.parent().ok_or("target has no parent dir")?;
    let x64 = exe_is_64(exe).unwrap_or(true);
    let loader = loader_name(x64);
    let loader_src = src.join(loader);
    if !loader_src.is_file() || !src.join("steamclient64.dll").is_file() {
        return Err("This gbe_fork release has no ColdClientLoader files".into());
    }
    let previous = read_marker(dir).map(|m| m.files).unwrap_or_default();
    let mut files: Vec<String> = Vec::new();
    for name in CLIENT_FILES.iter().copied().chain([loader, INI]) {
        let ours = previous.iter().any(|p| p.eq_ignore_ascii_case(name));
        if dir.join(name).exists() && !ours {
            return Err(format!("The game already has its own {}, SMD won't overwrite it", name));
        }
    }
    for name in CLIENT_FILES.iter().copied().chain([loader]) {
        let from = src.join(name);
        if !from.is_file() {
            continue;
        }
        fs::copy(&from, dir.join(name)).map_err(|e| format!("copy {}: {}", name, e))?;
        files.push(name.to_string());
    }
    let exe_name = exe.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    fs::write(dir.join(INI), render_ini(&exe_name, app_id)).map_err(|e| format!("write {}: {}", INI, e))?;
    files.push(INI.to_string());
    let install = ColdInstall {
        exe: exe.to_string_lossy().to_string(),
        loader: dir.join(loader).to_string_lossy().to_string(),
        files,
    };
    fs::write(dir.join(MARKER), serde_json::to_string_pretty(&install).unwrap_or_default())
        .map_err(|e| format!("write {}: {}", MARKER, e))?;
    Ok(install)
}

pub fn installs_in(game_dir: &Path) -> Vec<ColdInstall> {
    let mut out: Vec<ColdInstall> = WalkDir::new(game_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file() && e.file_name() == MARKER)
        .filter_map(|e| e.path().parent().and_then(read_marker))
        .filter(|m| Path::new(&m.loader).is_file())
        .collect();
    out.sort_by(|a, b| a.loader.cmp(&b.loader));
    out
}

pub fn install_for_loader(loader: &Path) -> Option<ColdInstall> {
    loader.parent().and_then(read_marker).filter(|m| Path::new(&m.loader) == loader)
}

pub fn remove(loader: &Path) -> Result<ColdInstall, String> {
    let dir = loader.parent().ok_or("target has no parent dir")?;
    let install = read_marker(dir).ok_or_else(|| format!("no backup found at {}", dir.join(MARKER).display()))?;
    for name in &install.files {
        let p = dir.join(name);
        if p.exists() {
            fs::remove_file(&p).map_err(|e| format!("remove {}: {}", name, e))?;
        }
    }
    let _ = fs::remove_file(dir.join(MARKER));
    let settings = dir.join("steam_settings");
    for name in ["steam_appid.txt", "installed_app_ids.txt", "configs.app.ini", ".smd-emulator.json"] {
        let _ = fs::remove_file(settings.join(name));
    }
    let _ = fs::remove_dir(&settings);
    Ok(install)
}

pub fn retarget_links(from_exe: &str, to_exe: &str) -> usize {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const SCRIPT: &str = "\
            $ErrorActionPreference = 'SilentlyContinue';\
            $ws = New-Object -ComObject WScript.Shell;\
            $n = 0;\
            $dirs = @([Environment]::GetFolderPath('Desktop'), [Environment]::GetFolderPath('CommonDesktopDirectory'), [Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('CommonPrograms'));\
            foreach ($d in $dirs) { if (-not $d) { continue };\
              Get-ChildItem -LiteralPath $d -Filter *.lnk -Recurse -ErrorAction SilentlyContinue | ForEach-Object {\
                $s = $ws.CreateShortcut($_.FullName);\
                if ($s.TargetPath -and ($s.TargetPath -ieq $env:SMD_FROM)) {\
                  if (-not $s.IconLocation -or $s.IconLocation -like ',*') { $s.IconLocation = $env:SMD_FROM + ',0' };\
                  $s.TargetPath = $env:SMD_TO;\
                  $s.WorkingDirectory = Split-Path -Parent $env:SMD_TO;\
                  $s.Save(); $n++ } } };\
            Write-Output $n";
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
            .env("SMD_FROM", from_exe)
            .env("SMD_TO", to_exe)
            .creation_flags(0x08000000)
            .output();
        out.ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(0)
    }
    #[cfg(not(windows))]
    {
        let _ = (from_exe, to_exe);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe(machine: u16) -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[0] = b'M';
        d[1] = b'Z';
        d[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        d[0x80..0x84].copy_from_slice(b"PE\0\0");
        d[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
        d
    }

    #[test]
    fn installs_next_to_the_game_and_removes_cleanly() {
        let root = std::env::temp_dir().join(format!("smd-cold-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let src = root.join("cache/release/steamclient_experimental");
        fs::create_dir_all(&src).unwrap();
        for f in ["steamclient.dll", "steamclient64.dll", "steamclient_loader_x64.exe", "steamclient_loader_x86.exe"] {
            fs::write(src.join(f), b"x").unwrap();
        }
        assert_eq!(loader_source(&root.join("cache")), Some(src.clone()));
        let game = root.join("game/bin");
        fs::create_dir_all(&game).unwrap();
        let exe = game.join("Game.exe");
        fs::write(&exe, pe(0x014c)).unwrap();
        assert_eq!(exe_is_64(&exe), Some(false));
        let inst = install(&src, &exe, "480").unwrap();
        assert!(inst.loader.ends_with("steamclient_loader_x86.exe"));
        let ini = fs::read_to_string(game.join(INI)).unwrap();
        assert!(ini.contains("Exe=Game.exe") && ini.contains("AppId=480"));
        assert_eq!(installs_in(&root.join("game")).len(), 1);
        assert!(install(&src, &exe, "480").is_ok());
        remove(Path::new(&inst.loader)).unwrap();
        assert!(!game.join("steamclient.dll").exists() && !game.join(MARKER).exists());
        fs::write(game.join("steamclient64.dll"), b"own").unwrap();
        assert!(install(&src, &exe, "480").unwrap_err().contains("own steamclient64.dll"));
        let _ = fs::remove_dir_all(&root);
    }
}
