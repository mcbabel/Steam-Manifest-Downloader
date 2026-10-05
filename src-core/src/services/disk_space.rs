use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct SpaceCheck {
    pub path: String,
    pub free: Option<u64>,
    pub needed: u64,
    pub enough: bool,
}

fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(p) = current {
        if p.exists() {
            return Some(p.to_path_buf());
        }
        current = p.parent();
    }
    None
}

#[cfg(target_os = "windows")]
fn free_at(path: &Path) -> Option<u64> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide = HSTRING::from(path.as_os_str());
    let mut available: u64 = 0;
    unsafe { GetDiskFreeSpaceExW(&wide, Some(&mut available), None, None) }.ok()?;
    Some(available)
}

#[cfg(target_os = "linux")]
fn free_at(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    Some((stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn free_at(_path: &Path) -> Option<u64> {
    None
}

pub fn free_bytes(path: &Path) -> Option<u64> {
    free_at(&existing_ancestor(path)?)
}

pub fn check(path: &Path, needed: u64) -> SpaceCheck {
    let free = free_bytes(path);
    SpaceCheck {
        path: path.to_string_lossy().to_string(),
        free,
        needed,
        enough: free.is_none_or(|f| f >= needed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_folders_use_the_nearest_existing_parent() {
        let missing = std::env::temp_dir().join("smd-space-test/does/not/exist");
        assert!(free_bytes(&missing).is_some() || cfg!(not(any(target_os = "windows", target_os = "linux"))));
    }

    #[test]
    fn reports_when_space_is_short() {
        let dir = std::env::temp_dir();
        assert!(check(&dir, 0).enough);
        if let Some(free) = free_bytes(&dir) {
            assert!(!check(&dir, free.saturating_add(1 << 40)).enough);
        }
    }
}
