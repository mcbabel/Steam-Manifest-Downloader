#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Stdio;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DotnetStatus {
    pub installed: bool,
    pub version: Option<String>,
}

pub fn check_dotnet() -> DotnetStatus {
    #[cfg(target_os = "linux")]
    {
        DotnetStatus {
            installed: true,
            version: Some("self-contained".to_string()),
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("dotnet");
        cmd.args(["--list-runtimes"]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut found_version: Option<String> = None;

                for line in stdout.lines() {
                    if line.contains("Microsoft.NETCore.App 9.") {
                        if let Some(version_part) = line.strip_prefix("Microsoft.NETCore.App ") {
                            if let Some(ver) = version_part.split_whitespace().next() {
                                found_version = Some(ver.to_string());
                            }
                        }
                        break;
                    }
                }

                DotnetStatus {
                    installed: found_version.is_some(),
                    version: found_version,
                }
            }
            Err(_) => DotnetStatus {
                installed: false,
                version: None,
            },
        }
    }
}

pub fn open_folder(path: &str) -> Result<(), String> {
    let dir = std::path::Path::new(path);
    if !dir.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(target_os = "macos")]
    let program = "open";

    std::process::Command::new(program)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(())
}
