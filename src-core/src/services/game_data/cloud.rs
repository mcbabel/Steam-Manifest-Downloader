use std::collections::BTreeSet;

use crate::services::steam_pics::{RootOverride, SaveFile};

fn sanitize(path: &str) -> String {
    let mut p = path.trim_matches('/').to_string();
    while let Some(rest) = p.strip_suffix("/.") {
        p = rest.to_string();
    }
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_string();
    }
    while let Some(idx) = p.find("/./") {
        p.replace_range(idx..idx + 2, "");
    }
    if p == "." {
        String::new()
    } else {
        p
    }
}

fn fixup(path: &str) -> String {
    path.replace("{64BitSteamID}", "{::64BitSteamID::}")
        .replace("{Steam3AccountID}", "{::Steam3AccountID::}")
}

fn join(base: String, rest: &str) -> String {
    if rest.is_empty() {
        base
    } else {
        format!("{}/{}", base, rest)
    }
}

pub fn dirs_for(platform: &str, files: &[SaveFile], overrides: &[RootOverride]) -> Vec<String> {
    let files: Vec<&SaveFile> = files
        .iter()
        .filter(|f| {
            f.platforms.is_empty()
                || f.platforms
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case("all") || p.eq_ignore_ascii_case(platform))
        })
        .collect();
    if files.is_empty() {
        return Vec::new();
    }
    let overrides: Vec<&RootOverride> = overrides.iter().filter(|o| o.os.eq_ignore_ascii_case(platform)).collect();
    let mut out = BTreeSet::new();
    if overrides.is_empty() {
        for f in &files {
            let base = format!("{{::{}::}}", f.root.trim());
            out.insert(fixup(&join(base, &sanitize(&f.path.replace('\\', "/")))));
        }
    } else {
        for o in &overrides {
            let base = join(
                format!("{{::{}::}}", o.use_instead.trim()),
                &sanitize(&o.add_path.replace('\\', "/")),
            );
            for f in files.iter().filter(|f| f.root.eq_ignore_ascii_case(&o.root)) {
                let mut path = f.path.replace('\\', "/");
                for (find, replace) in &o.transforms {
                    let find = find.replace('\\', "/");
                    let replace = replace.replace('\\', "/");
                    if !find.is_empty() && !path.is_empty() {
                        path = path.replace(&find, &replace);
                    } else if find.is_empty() && path.is_empty() {
                        path = replace;
                    }
                }
                out.insert(fixup(&join(base.clone(), &sanitize(&path))));
            }
        }
    }
    out.into_iter().collect()
}

const SECTION_PREFIX: &str = "[app::cloud_save::";

pub fn merge_into_app_ini(existing: &str, win: &[String], linux: &[String]) -> String {
    let mut kept = String::new();
    let mut skipping = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            skipping = trimmed.starts_with(SECTION_PREFIX);
        }
        if !skipping {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    if win.is_empty() && linux.is_empty() {
        return kept;
    }
    let mut out = kept.trim_end().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str("[app::cloud_save::general]\ncreate_specific_dirs=1\n");
    for (name, list) in [("win", win), ("linux", linux)] {
        if list.is_empty() {
            continue;
        }
        out.push_str(&format!("\n[app::cloud_save::{}]\n", name));
        for (i, dir) in list.iter().enumerate() {
            out.push_str(&format!("dir{}={}\n", i + 1, dir));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cloud_dirs_like_the_generator() {
        let files = vec![
            SaveFile { root: "WinAppDataLocal".into(), path: "Studio/Game/{64BitSteamID}/.".into(), platforms: vec!["Windows".into()] },
            SaveFile { root: "gameinstall".into(), path: "./saves".into(), platforms: vec![] },
            SaveFile { root: "MacHome".into(), path: "x".into(), platforms: vec!["MacOS".into()] },
        ];
        let win = dirs_for("Windows", &files, &[]);
        assert_eq!(win, vec!["{::WinAppDataLocal::}/Studio/Game/{::64BitSteamID::}", "{::gameinstall::}/saves"]);
        let overrides = vec![RootOverride {
            root: "WinAppDataLocal".into(),
            use_instead: "LinuxXdgDataHome".into(),
            os: "Linux".into(),
            add_path: "".into(),
            transforms: vec![("Studio".into(), "studio".into())],
        }];
        let files_all: Vec<SaveFile> = files.iter().cloned().map(|mut f| { f.platforms.clear(); f }).collect();
        assert_eq!(
            dirs_for("Linux", &files_all, &overrides),
            vec!["{::LinuxXdgDataHome::}/studio/Game/{::64BitSteamID::}"]
        );
        let ini = merge_into_app_ini("[app::dlcs]\nunlock_all=0\n\n[app::cloud_save::win]\ndir1=old\n", &win, &[]);
        assert!(ini.starts_with("[app::dlcs]\nunlock_all=0\n\n[app::cloud_save::general]"));
        assert!(ini.contains("dir2={::gameinstall::}/saves"));
        assert!(!ini.contains("old"));
        assert_eq!(merge_into_app_ini("[app::cloud_save::win]\ndir1=old\n", &[], &[]), "");
    }
}
