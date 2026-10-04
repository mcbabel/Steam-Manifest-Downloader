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

pub fn power_off() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("shutdown")
            .args(["/s", "/t", "0"])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to run shutdown: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("shutdown exited with {}", status))
        }
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [&[&str]; 3] = [
            &["systemctl", "poweroff"],
            &["loginctl", "poweroff"],
            &["shutdown", "-h", "now"],
        ];
        let mut last_error = String::from("no shutdown command available");
        for cmd in attempts {
            match std::process::Command::new(cmd[0])
                .args(&cmd[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
            {
                Ok(out) if out.status.success() => return Ok(()),
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    last_error = if stderr.is_empty() {
                        format!("{} exited with {}", cmd.join(" "), out.status)
                    } else {
                        format!("{}: {}", cmd.join(" "), stderr)
                    };
                }
                Err(e) => last_error = format!("{}: {}", cmd[0], e),
            }
        }
        Err(last_error)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Shutting down is not supported on this system".to_string())
    }
}
