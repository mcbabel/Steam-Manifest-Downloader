use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use super::steam_manifest::DecodedManifest;

const DIRECTORY_FLAG: u32 = 0x40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotInstall {
    pub depot_id: String,
    pub manifest_id: String,
    pub files: Vec<String>,
}

pub fn record_path(work_dir: &Path, depot_id: &str) -> PathBuf {
    work_dir.join(format!(".smd-depot-{}.json", depot_id))
}

pub fn load(work_dir: &Path, depot_id: &str) -> Option<DepotInstall> {
    let bytes = std::fs::read(record_path(work_dir, depot_id)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save(work_dir: &Path, install: &DepotInstall) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(install).map_err(|e| e.to_string())?;
    std::fs::write(record_path(work_dir, &install.depot_id), json)
        .map_err(|e| format!("write install record: {}", e))
}

pub fn installed(work_dir: &Path) -> Vec<DepotInstall> {
    let Ok(dir) = std::fs::read_dir(work_dir) else {
        return Vec::new();
    };
    let mut list: Vec<DepotInstall> = dir
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let id = name.strip_prefix(".smd-depot-")?.strip_suffix(".json")?.to_string();
            id.chars().all(|c| c.is_ascii_digit()).then_some(id)
        })
        .filter_map(|id| load(work_dir, &id))
        .collect();
    list.sort_by(|a, b| a.depot_id.cmp(&b.depot_id));
    list
}

pub fn normalize(rel: &str) -> String {
    rel.replace('\\', "/").trim_start_matches('/').to_string()
}

pub fn manifest_files(manifest: &DecodedManifest) -> Vec<String> {
    manifest
        .payload
        .mappings
        .iter()
        .filter(|m| m.flags.unwrap_or(0) & DIRECTORY_FLAG == 0)
        .filter_map(|m| m.filename.as_deref())
        .map(normalize)
        .filter(|f| !f.is_empty())
        .collect()
}

pub fn stale_files(old: &[String], new: &[String]) -> Vec<String> {
    let keep: HashSet<String> = new.iter().map(|f| normalize(f).to_lowercase()).collect();
    let mut seen = HashSet::new();
    old.iter()
        .map(|f| normalize(f))
        .filter(|f| !keep.contains(&f.to_lowercase()))
        .filter(|f| seen.insert(f.to_lowercase()))
        .collect()
}

fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    if rel.is_absolute() {
        return None;
    }
    let mut out = root.to_path_buf();
    for part in rel.components() {
        match part {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (out != root).then_some(out)
}

pub fn remove_stale(depot_out: &Path, stale: &[String]) -> usize {
    let mut removed = 0;
    for rel in stale {
        let Some(path) = safe_join(depot_out, rel) else {
            continue;
        };
        let is_file = std::fs::symlink_metadata(&path)
            .map(|m| m.is_file())
            .unwrap_or(false);
        if !is_file || std::fs::remove_file(&path).is_err() {
            continue;
        }
        removed += 1;
        let mut dir = path.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d == depot_out || !d.starts_with(depot_out) || std::fs::remove_dir(&d).is_err() {
                break;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    removed
}

pub fn other_manifest_files(work_dir: &Path, depot_id: &str, keep_manifest_id: &str) -> Vec<PathBuf> {
    let prefix = format!("{}_", depot_id);
    let keep = format!("{}_{}.manifest", depot_id, keep_manifest_id);
    std::fs::read_dir(work_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.starts_with(&prefix) && n.ends_with(".manifest") && n != keep
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DlcChoice {
    pub dlcs: Vec<String>,
}

pub fn dlc_choice_path(work_dir: &Path) -> PathBuf {
    work_dir.join(".smd-dlcs.json")
}

pub fn save_dlc_choice(work_dir: &Path, dlcs: &[String]) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(&DlcChoice { dlcs: dlcs.to_vec() }).map_err(|e| e.to_string())?;
    std::fs::write(dlc_choice_path(work_dir), json).map_err(|e| format!("write DLC choice: {}", e))
}

pub fn load_dlc_choice(work_dir: &Path) -> Option<Vec<String>> {
    let bytes = std::fs::read(dlc_choice_path(work_dir)).ok()?;
    serde_json::from_slice::<DlcChoice>(&bytes).ok().map(|c| c.dlcs)
}

pub fn find_game_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .take(8)
        .find(|dir| dlc_choice_path(dir).is_file() || has_install_records(dir))
        .map(Path::to_path_buf)
}

pub fn has_depot_folders(work_dir: &Path) -> bool {
    std::fs::read_dir(work_dir.join("depots"))
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

pub fn has_install_records(work_dir: &Path) -> bool {
    std::fs::read_dir(work_dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with(".smd-depot-"))
        })
        .unwrap_or(false)
}

pub fn use_merged_layout(work_dir: &Path, preferred: bool) -> bool {
    if has_depot_folders(work_dir) {
        return false;
    }
    if has_install_records(work_dir) || work_dir.join(".DepotDownloader").is_dir() {
        return true;
    }
    preferred
}

pub fn files_of_other_depots(work_dir: &Path, depot_id: &str) -> HashSet<String> {
    installed(work_dir)
        .into_iter()
        .filter(|d| d.depot_id != depot_id)
        .flat_map(|d| d.files)
        .map(|f| normalize(&f).to_lowercase())
        .collect()
}

pub fn keep_shared(stale: Vec<String>, shared: &HashSet<String>) -> Vec<String> {
    stale
        .into_iter()
        .filter(|f| !shared.contains(&normalize(f).to_lowercase()))
        .collect()
}

pub fn lower_set(files: &[String]) -> HashSet<String> {
    files.iter().map(|f| normalize(f).to_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_installed_depots_and_ignores_checkpoints() {
        let dir = std::env::temp_dir().join(format!("smd-installed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (id, manifest) in [("482", "2"), ("481", "1")] {
            save(
                &dir,
                &DepotInstall {
                    depot_id: id.to_string(),
                    manifest_id: manifest.to_string(),
                    files: vec![],
                },
            )
            .unwrap();
        }
        std::fs::write(dir.join(".smd-depot-481.resume.json"), "{}").unwrap();
        let found: Vec<(String, String)> = installed(&dir)
            .into_iter()
            .map(|d| (d.depot_id, d.manifest_id))
            .collect();
        assert_eq!(
            found,
            vec![("481".to_string(), "1".to_string()), ("482".to_string(), "2".to_string())]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn stale_files_are_the_ones_missing_from_the_new_version() {
        let old = s(&["game.exe", "data\\old.pak", "data/keep.pak"]);
        let new = s(&["game.exe", "data/keep.pak", "data/new.pak"]);
        assert_eq!(stale_files(&old, &new), s(&["data/old.pak"]));
    }

    #[test]
    fn a_case_only_rename_is_never_deleted() {
        let old = s(&["Data/Level.pak"]);
        let new = s(&["data/level.pak"]);
        assert!(stale_files(&old, &new).is_empty());
    }

    #[test]
    fn paths_leaving_the_depot_folder_are_ignored() {
        let root = std::env::temp_dir().join(format!("smd-install-{}", std::process::id()));
        std::fs::create_dir_all(root.join("depot/sub")).unwrap();
        std::fs::write(root.join("outside.txt"), b"x").unwrap();
        std::fs::write(root.join("depot/sub/old.pak"), b"x").unwrap();
        std::fs::write(root.join("depot/keep.txt"), b"x").unwrap();
        let removed = remove_stale(
            &root.join("depot"),
            &s(&["../outside.txt", "sub/old.pak", "/etc/passwd"]),
        );
        assert_eq!(removed, 1);
        assert!(root.join("outside.txt").exists());
        assert!(!root.join("depot/sub").exists());
        assert!(root.join("depot/keep.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn records_round_trip() {
        let root = std::env::temp_dir().join(format!("smd-record-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let rec = DepotInstall {
            depot_id: "481".into(),
            manifest_id: "123".into(),
            files: s(&["a.bin"]),
        };
        save(&root, &rec).unwrap();
        assert_eq!(load(&root, "481"), Some(rec));
        std::fs::write(root.join("481_123.manifest"), b"").unwrap();
        std::fs::write(root.join("481_99.manifest"), b"").unwrap();
        std::fs::write(root.join("4810_1.manifest"), b"").unwrap();
        let others = other_manifest_files(&root, "481", "123");
        assert_eq!(others, vec![root.join("481_99.manifest")]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn picks_the_folder_layout() {
        let root = std::env::temp_dir().join(format!("smd-layout-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(use_merged_layout(&root, true));
        assert!(!use_merged_layout(&root, false));
        std::fs::create_dir_all(root.join("depots/.DepotDownloader")).unwrap();
        assert!(!has_depot_folders(&root));
        save(&root, &DepotInstall { depot_id: "221".into(), manifest_id: "1".into(), files: s(&["hl2.exe"]) }).unwrap();
        assert!(use_merged_layout(&root, false));
        std::fs::create_dir_all(root.join("depots/221 - Content")).unwrap();
        assert!(!use_merged_layout(&root, true));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_files_of_another_depot_are_kept() {
        let root = std::env::temp_dir().join(format!("smd-shared-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for (id, files) in [("221", s(&["a.pak", "Shared.dat"])), ("224", s(&["shared.dat", "de.pak"]))] {
            save(&root, &DepotInstall { depot_id: id.into(), manifest_id: "1".into(), files }).unwrap();
        }
        let shared = files_of_other_depots(&root, "221");
        assert_eq!(keep_shared(s(&["a.pak", "Shared.dat"]), &shared), s(&["a.pak"]));
        let _ = std::fs::remove_dir_all(&root);
    }
}
