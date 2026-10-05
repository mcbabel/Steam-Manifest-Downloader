use std::io::{self, Stdout};
use std::path::PathBuf;

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn stderr_log_path() -> Option<PathBuf> {
    smd_core::services::debug_log::log_path().map(|p| p.with_file_name("smd-tui-stderr.log"))
}

pub fn init() -> io::Result<Tui> {
    install_panic_hook();
    stderr_redirect::start();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        EnableFocusChange,
        SetTitle("Steam Manifest Downloader")
    )?;
    Terminal::new(CrosstermBackend::new(stdout))
}

pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        DisableFocusChange,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    stderr_redirect::stop();
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

pub fn redirect_stderr() {
    stderr_redirect::start();
}

pub fn write_original_stderr(msg: &str) {
    if !stderr_redirect::write_original(msg.as_bytes()) {
        eprint!("{}", msg);
    }
}

pub fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    let encoded = base64_encode(text.as_bytes());
    let mut out = io::stdout();
    let _ = write!(out, "\x1b]52;c;{}\x07", encoded);
    let _ = out.flush();
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(unix)]
mod stderr_redirect {
    use std::os::unix::io::AsRawFd;
    use std::sync::Mutex;

    static SAVED: Mutex<Option<i32>> = Mutex::new(None);

    pub fn start() {
        let Some(path) = super::stderr_log_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        else {
            return;
        };
        let mut saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        if saved.is_some() {
            return;
        }
        unsafe {
            let original = libc::dup(2);
            if original < 0 {
                return;
            }
            if libc::dup2(file.as_raw_fd(), 2) < 0 {
                libc::close(original);
                return;
            }
            *saved = Some(original);
        }
    }

    pub fn stop() {
        let mut saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(original) = saved.take() {
            unsafe {
                libc::dup2(original, 2);
                libc::close(original);
            }
        }
    }

    pub fn write_original(bytes: &[u8]) -> bool {
        use std::io::Write;
        use std::os::unix::io::FromRawFd;
        let saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        let Some(fd) = *saved else { return false };
        let file = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
        (&*file).write_all(bytes).is_ok()
    }
}

#[cfg(windows)]
mod stderr_redirect {
    use std::os::windows::io::IntoRawHandle;
    use std::sync::Mutex;

    type Handle = *mut std::ffi::c_void;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;

    extern "system" {
        fn GetStdHandle(n_std_handle: u32) -> Handle;
        fn SetStdHandle(n_std_handle: u32, handle: Handle) -> i32;
    }

    struct Saved(Handle);
    unsafe impl Send for Saved {}

    static SAVED: Mutex<Option<Saved>> = Mutex::new(None);

    pub fn start() {
        let Some(path) = super::stderr_log_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        else {
            return;
        };
        let mut saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        if saved.is_some() {
            return;
        }
        unsafe {
            let original = GetStdHandle(STD_ERROR_HANDLE);
            let handle = file.into_raw_handle();
            if SetStdHandle(STD_ERROR_HANDLE, handle) != 0 {
                *saved = Some(Saved(original));
            }
        }
    }

    pub fn stop() {
        let mut saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(Saved(original)) = saved.take() {
            unsafe {
                SetStdHandle(STD_ERROR_HANDLE, original);
            }
        }
    }

    pub fn write_original(bytes: &[u8]) -> bool {
        use std::io::Write;
        use std::os::windows::io::FromRawHandle;
        let saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
        let Some(Saved(handle)) = saved.as_ref() else {
            return false;
        };
        if handle.is_null() {
            return false;
        }
        let file = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_handle(*handle) });
        (&*file).write_all(bytes).is_ok()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_reference() {
        assert_eq!(super::base64_encode(b""), "");
        assert_eq!(super::base64_encode(b"f"), "Zg==");
        assert_eq!(super::base64_encode(b"fo"), "Zm8=");
        assert_eq!(super::base64_encode(b"foo"), "Zm9v");
        assert_eq!(
            super::base64_encode(b"Steam Manifest"),
            "U3RlYW0gTWFuaWZlc3Q="
        );
    }
}
