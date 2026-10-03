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

#[cfg(test)]
mod tests {
    use super::*;

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
}
