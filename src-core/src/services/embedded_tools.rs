use std::path::PathBuf;
use tokio::fs;

#[cfg(target_os = "windows")]
mod platform {
    pub const DDM_FILES: &[(&str, &[u8])] = &[
        ("DepotDownloaderMod.exe", include_bytes!("../../vendor/ddm-windows/DepotDownloaderMod.exe")),
        ("DepotDownloaderMod.dll", include_bytes!("../../vendor/ddm-windows/DepotDownloaderMod.dll")),
        ("DepotDownloaderMod.deps.json", include_bytes!("../../vendor/ddm-windows/DepotDownloaderMod.deps.json")),
        ("DepotDownloaderMod.runtimeconfig.json", include_bytes!("../../vendor/ddm-windows/DepotDownloaderMod.runtimeconfig.json")),
        ("SteamKit2.dll", include_bytes!("../../vendor/ddm-windows/SteamKit2.dll")),
        ("protobuf-net.Core.dll", include_bytes!("../../vendor/ddm-windows/protobuf-net.Core.dll")),
        ("protobuf-net.dll", include_bytes!("../../vendor/ddm-windows/protobuf-net.dll")),
        ("QRCoder.dll", include_bytes!("../../vendor/ddm-windows/QRCoder.dll")),
        ("System.IO.Hashing.dll", include_bytes!("../../vendor/ddm-windows/System.IO.Hashing.dll")),
        ("ZstdSharp.dll", include_bytes!("../../vendor/ddm-windows/ZstdSharp.dll")),
    ];
    pub const EXE_NAME: &str = "DepotDownloaderMod.exe";
}

#[cfg(target_os = "linux")]
mod platform {
    pub const DDM_FILES: &[(&str, &[u8])] = &[
        ("DepotDownloaderMod", include_bytes!("../../vendor/ddm-linux/DepotDownloaderMod")),
    ];
    pub const EXE_NAME: &str = "DepotDownloaderMod";
}

pub async fn ensure_extracted() -> Result<PathBuf, String> {
    let base_dir = std::env::temp_dir().join("SteamManifestDownloader").join("DepotDownloaderMod");
    let marker_file = base_dir.join(".extracted");
    let exe_path = base_dir.join(platform::EXE_NAME);

    // Marker + exe both present → skip the write churn.
    if marker_file.exists() && exe_path.exists() {
        return Ok(exe_path);
    }

    eprintln!("[EmbeddedTools] Extracting DepotDownloaderMod to {:?}", base_dir);

    fs::create_dir_all(&base_dir)
        .await
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

    for (name, data) in platform::DDM_FILES {
        let file_path = base_dir.join(name);
        fs::write(&file_path, data)
            .await
            .map_err(|e| format!("Failed to extract {}: {}", name, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&exe_path).await
            .map_err(|e| format!("Failed to get metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&exe_path, perms).await
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    fs::write(&marker_file, "extracted")
        .await
        .map_err(|e| format!("Failed to write marker file: {}", e))?;

    eprintln!("[EmbeddedTools] Extraction complete");
    Ok(exe_path)
}
