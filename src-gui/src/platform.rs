use std::path::PathBuf;

use crate::backend::BoxFuture;

#[derive(Clone, Debug, Default)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

pub trait Platform: Send + Sync + 'static {
    fn open_url(&self, url: &str);
    fn pick_file(&self, start: Option<PathBuf>, filters: Vec<FileFilter>) -> BoxFuture<Option<PathBuf>>;
    fn pick_folder(&self, start: Option<PathBuf>) -> BoxFuture<Option<PathBuf>>;
    fn save_file(&self, name: String, filters: Vec<FileFilter>) -> BoxFuture<Option<PathBuf>>;
    fn notify(&self, _title: &str, _body: &str) {}
    fn beep(&self) {}
    fn set_progress(&self, _percent: Option<u8>, _paused: bool) {}
}

pub struct Native {
    window: Option<isize>,
    runtime: tokio::runtime::Handle,
}

impl Native {
    pub fn new(window: Option<isize>, runtime: tokio::runtime::Handle) -> Self {
        Self { window, runtime }
    }
}

fn beep_wav() -> &'static [u8] {
    static WAV: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    WAV.get_or_init(|| {
        const RATE: u32 = 44_100;
        let samples = RATE / 2;
        let mut out = Vec::with_capacity(44 + samples as usize * 2);
        let data_len = samples * 2;
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for index in 0..samples {
            let time = index as f32 / RATE as f32;
            let gain = 0.3 * (0.01f32 / 0.3).powf(time / 0.5);
            let value = (time * 800.0 * std::f32::consts::TAU).sin() * gain;
            out.extend_from_slice(&((value * i16::MAX as f32) as i16).to_le_bytes());
        }
        out
    })
}

#[cfg(target_os = "windows")]
mod system {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList, TBPF_NOPROGRESS, TBPF_NORMAL, TBPF_PAUSED};

    pub fn set_progress(window: Option<isize>, percent: Option<u8>, paused: bool) {
        let Some(window) = window else { return };
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let Ok(taskbar) = CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) else { return };
            let handle = HWND(window as *mut core::ffi::c_void);
            match percent {
                None => {
                    let _ = taskbar.SetProgressState(handle, TBPF_NOPROGRESS);
                }
                Some(percent) => {
                    let _ = taskbar.SetProgressState(handle, if paused { TBPF_PAUSED } else { TBPF_NORMAL });
                    let _ = taskbar.SetProgressValue(handle, u64::from(percent), 100);
                }
            }
        }
    }

    pub fn beep(_runtime: &tokio::runtime::Handle) {
        let wav = super::beep_wav();
        unsafe {
            let _ = PlaySoundW(PCWSTR(wav.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod system {
    pub fn set_progress(_window: Option<isize>, percent: Option<u8>, _paused: bool) {
        #[cfg(target_os = "linux")]
        linux_progress(percent);
        #[cfg(not(target_os = "linux"))]
        let _ = percent;
    }

    #[cfg(target_os = "linux")]
    fn linux_progress(percent: Option<u8>) {
        use std::collections::HashMap;
        use zbus::zvariant::Value;
        let Ok(handle) = tokio::runtime::Handle::try_current() else { return };
        handle.spawn(async move {
            let Ok(connection) = zbus::Connection::session().await else { return };
            let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
            properties.insert("progress", Value::from(f64::from(percent.unwrap_or(0)) / 100.0));
            properties.insert("progress-visible", Value::from(percent.is_some()));
            let _ = connection
                .emit_signal(None::<&str>, "/com/canonical/unity/launcherentry/smd", "com.canonical.Unity.LauncherEntry", "Update", &(format!("application://{}.desktop", super::app_id()), properties))
                .await;
        });
    }

    pub fn beep(runtime: &tokio::runtime::Handle) {
        let wav = super::beep_wav();
        runtime.spawn(async move {
            let path = std::env::temp_dir().join("smd-notify.wav");
            if tokio::fs::write(&path, wav).await.is_err() {
                return;
            }
            let players: &[&str] = if cfg!(target_os = "macos") { &["afplay"] } else { &["pw-play", "paplay", "aplay"] };
            for player in players {
                let mut command = tokio::process::Command::new(player);
                if *player == "aplay" {
                    command.arg("-q");
                }
                if let Ok(status) = command.arg(&path).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().await {
                    if status.success() {
                        break;
                    }
                }
            }
        });
    }
}

pub fn app_id() -> String {
    std::env::var("FLATPAK_ID").ok().filter(|id| !id.is_empty()).unwrap_or_else(|| "steam-manifest-downloader".to_string())
}

impl Platform for Native {
    fn open_url(&self, url: &str) {
        let _ = open::that_detached(url);
    }

    fn notify(&self, title: &str, body: &str) {
        let (title, body) = (title.to_string(), body.to_string());
        self.runtime.spawn_blocking(move || {
            let mut notification = notify_rust::Notification::new();
            notification.appname("Steam Manifest Downloader").summary(&title).body(&body);
            #[cfg(all(unix, not(target_os = "macos")))]
            notification.hint(notify_rust::Hint::DesktopEntry(app_id()));
            let _ = notification.show();
        });
    }

    fn beep(&self) {
        system::beep(&self.runtime);
    }

    fn set_progress(&self, percent: Option<u8>, paused: bool) {
        let _guard = self.runtime.enter();
        system::set_progress(self.window, percent, paused);
    }

    fn pick_file(&self, start: Option<PathBuf>, filters: Vec<FileFilter>) -> BoxFuture<Option<PathBuf>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new();
            if let Some(start) = start.filter(|p| p.is_dir()) {
                dialog = dialog.set_directory(start);
            }
            for filter in &filters {
                dialog = dialog.add_filter(&filter.name, &filter.extensions);
            }
            dialog.pick_file().await.map(|file| file.path().to_path_buf())
        })
    }

    fn pick_folder(&self, start: Option<PathBuf>) -> BoxFuture<Option<PathBuf>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new();
            if let Some(start) = start.filter(|p| p.is_dir()) {
                dialog = dialog.set_directory(start);
            }
            dialog.pick_folder().await.map(|folder| folder.path().to_path_buf())
        })
    }

    fn save_file(&self, name: String, filters: Vec<FileFilter>) -> BoxFuture<Option<PathBuf>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_file_name(name);
            for filter in &filters {
                dialog = dialog.add_filter(&filter.name, &filter.extensions);
            }
            dialog.save_file().await.map(|file| file.path().to_path_buf())
        })
    }
}
