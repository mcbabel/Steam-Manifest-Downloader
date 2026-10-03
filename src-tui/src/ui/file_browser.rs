use std::path::{Path, PathBuf};

use super::input::TextInput;
use crate::app::action::BrowsePurpose;

#[derive(Debug, Clone)]
pub enum BrowseMode {
    File { exts: Vec<&'static str> },
    Dir,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub parent: bool,
}

#[derive(Debug, Clone)]
pub struct FileBrowser {
    pub purpose: BrowsePurpose,
    pub mode: BrowseMode,
    pub title: String,
    pub cwd: Option<PathBuf>,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub offset: usize,
    pub path_input: TextInput,
    pub show_hidden: bool,
    pub error: Option<String>,
}

impl FileBrowser {
    pub fn open(
        purpose: BrowsePurpose,
        mode: BrowseMode,
        title: String,
        start: Option<&Path>,
    ) -> Self {
        let start = start
            .and_then(|p| {
                if p.is_dir() {
                    Some(p.to_path_buf())
                } else {
                    p.parent().filter(|d| d.is_dir()).map(Path::to_path_buf)
                }
            })
            .or_else(home_dir)
            .or_else(|| std::env::current_dir().ok());
        let mut fb = FileBrowser {
            purpose,
            mode,
            title,
            cwd: start,
            entries: Vec::new(),
            selected: 0,
            offset: 0,
            path_input: TextInput::default(),
            show_hidden: false,
            error: None,
        };
        fb.reload();
        fb
    }

    pub fn is_dir_mode(&self) -> bool {
        matches!(self.mode, BrowseMode::Dir)
    }

    pub fn reload(&mut self) {
        self.error = None;
        self.entries.clear();
        self.selected = 0;
        self.offset = 0;
        match self.cwd.clone() {
            None => {
                self.path_input.clear();
                for drive in list_drives() {
                    self.entries.push(Entry {
                        name: drive.to_string_lossy().to_string(),
                        path: drive,
                        is_dir: true,
                        size: 0,
                        parent: false,
                    });
                }
            }
            Some(dir) => {
                self.path_input.set(dir.to_string_lossy().to_string());
                if dir.parent().is_some() || cfg!(windows) {
                    self.entries.push(Entry {
                        name: "..".into(),
                        path: dir.parent().map(Path::to_path_buf).unwrap_or_default(),
                        is_dir: true,
                        size: 0,
                        parent: true,
                    });
                }
                match std::fs::read_dir(&dir) {
                    Ok(rd) => {
                        let mut dirs = Vec::new();
                        let mut files = Vec::new();
                        for e in rd.flatten() {
                            let name = e.file_name().to_string_lossy().to_string();
                            if !self.show_hidden && name.starts_with('.') {
                                continue;
                            }
                            let path = e.path();
                            let meta = std::fs::metadata(&path).ok();
                            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                            if is_dir {
                                dirs.push(Entry {
                                    name,
                                    path,
                                    is_dir,
                                    size: 0,
                                    parent: false,
                                });
                            } else if let BrowseMode::File { exts } = &self.mode {
                                let ext = path
                                    .extension()
                                    .and_then(|x| x.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                if exts.is_empty() || exts.iter().any(|x| *x == ext) {
                                    let size = meta.map(|m| m.len()).unwrap_or(0);
                                    files.push(Entry {
                                        name,
                                        path,
                                        is_dir,
                                        size,
                                        parent: false,
                                    });
                                }
                            }
                        }
                        let key = |e: &Entry| e.name.to_lowercase();
                        dirs.sort_by_key(key);
                        files.sort_by_key(key);
                        self.entries.extend(dirs);
                        self.entries.extend(files);
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        }
    }

    pub fn cd(&mut self, dir: Option<PathBuf>) {
        let prev = self.cwd.clone();
        self.cwd = dir;
        self.reload();
        if let (Some(prev), Some(_)) = (prev, &self.cwd) {
            if let Some(i) = self.entries.iter().position(|e| e.path == prev) {
                self.selected = i;
            }
        }
    }

    pub fn up(&mut self) {
        if let Some(dir) = self.cwd.clone() {
            match dir.parent() {
                Some(p) => self.cd(Some(p.to_path_buf())),
                None if cfg!(windows) => self.cd(None),
                None => {}
            }
        }
    }

    pub fn home(&mut self) {
        if let Some(h) = home_dir() {
            self.cd(Some(h));
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        let keep = self.cwd.clone();
        self.cd(keep);
    }

    pub fn activate(&mut self, idx: usize) -> Option<PathBuf> {
        let entry = self.entries.get(idx)?.clone();
        if entry.parent {
            self.up();
            None
        } else if entry.is_dir {
            self.cd(Some(entry.path));
            None
        } else {
            Some(entry.path)
        }
    }

    pub fn choose(&mut self) -> Option<PathBuf> {
        match self.mode {
            BrowseMode::Dir => {
                let highlighted = self
                    .entries
                    .get(self.selected)
                    .filter(|e| e.is_dir && !e.parent)
                    .map(|e| e.path.clone());
                highlighted.or_else(|| self.cwd.clone())
            }
            BrowseMode::File { .. } => {
                let idx = self.selected;
                self.activate(idx)
            }
        }
    }

    pub fn submit_path(&mut self) -> Option<PathBuf> {
        let raw = self.path_input.trimmed();
        if raw.is_empty() {
            return None;
        }
        let p = expand_tilde(&raw);
        if p.is_dir() {
            self.cd(Some(p));
            None
        } else if p.is_file() && !self.is_dir_mode() {
            Some(p)
        } else {
            self.error = Some(format!("{}: not found", p.display()));
            None
        }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

pub fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix('~') {
        if let Some(home) = home_dir() {
            return home.join(rest.trim_start_matches(['/', '\\']));
        }
    }
    PathBuf::from(raw)
}

fn list_drives() -> Vec<PathBuf> {
    if cfg!(windows) {
        (b'A'..=b'Z')
            .map(|l| PathBuf::from(format!("{}:\\", l as char)))
            .filter(|p| p.exists())
            .collect()
    } else {
        vec![PathBuf::from("/")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_dirs_first_and_filters_extensions() {
        let dir = std::env::temp_dir().join(format!("smd-fb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("game.lua"), "x").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.join(".hidden.lua"), "x").unwrap();

        let mut fb = FileBrowser::open(
            BrowsePurpose::UploadFile,
            BrowseMode::File {
                exts: vec!["lua", "st"],
            },
            String::new(),
            Some(&dir),
        );
        let names: Vec<&str> = fb.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["..", "sub", "game.lua"]);

        fb.toggle_hidden();
        assert!(fb.entries.iter().any(|e| e.name == ".hidden.lua"));

        let picked = fb.activate(
            fb.entries
                .iter()
                .position(|e| e.name == "game.lua")
                .unwrap(),
        );
        assert_eq!(picked, Some(dir.join("game.lua")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
