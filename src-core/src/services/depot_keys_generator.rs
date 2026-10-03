use std::path::Path;
use tokio::fs;

use crate::services::lua_parser::DepotInfo;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepotKeysResult {
    pub output_path: String,
    pub depot_count: usize,
}

// Writes `{depotId};{hexKey}\n` lines to {base_dir}/{folder_name}/steam.keys.
pub async fn generate_depot_keys(
    app_id: u64,
    depots: &[DepotInfo],
    folder_name: Option<&str>,
    base_dir: &Path,
) -> Result<DepotKeysResult, String> {
    let folder = folder_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| app_id.to_string());
    let output_dir = base_dir.join(&folder);
    let output_path = output_dir.join("steam.keys");

    fs::create_dir_all(&output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let lines: Vec<String> = depots
        .iter()
        .filter_map(|depot| {
            depot
                .depot_key
                .as_ref()
                .map(|key| format!("{};{}", depot.depot_id, key))
        })
        .collect();

    let content = if lines.is_empty() {
        String::from("\n")
    } else {
        lines.join("\n") + "\n"
    };

    fs::write(&output_path, &content)
        .await
        .map_err(|e| format!("Failed to write steam.keys: {}", e))?;

    Ok(DepotKeysResult {
        output_path: output_path.to_string_lossy().to_string(),
        depot_count: lines.len(),
    })
}

