#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use smd_core::services::{self, telemetry::Telemetry, AppState};
use smd_gui::app::{App, Boot};
use smd_gui::backend::{Backend, CoreBackend, Cx, Events};
use smd_gui::store::LocalStore;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> eframe::Result {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("tokio runtime");
    let data_dir = smd_core::paths::default_app_data_dir();
    std::fs::create_dir_all(&data_dir).ok();

    let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local").to_string();
    services::telemetry::install_crash_hook(data_dir.clone());
    let (events, events_rx) = Events::channel();
    let (backend, settings) = {
        let _guard = runtime.enter();
        let telemetry = Telemetry::new(data_dir.clone(), VERSION.to_string(), channel, "gui");
        runtime.spawn(telemetry.clone().run_background_flush());
        let mut state = AppState::new();
        state.telemetry = Some(telemetry);
        let cx = Cx::new(Arc::new(state), data_dir.clone(), events.clone(), VERSION);
        let backend: Arc<dyn Backend> = Arc::new(CoreBackend::new(cx));
        let settings = runtime.block_on(backend.invoke("get_settings", serde_json::Value::Null)).unwrap_or(serde_json::Value::Null);
        (backend, settings)
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Steam Manifest Downloader")
            .with_app_id(smd_gui::platform::app_id())
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png")).expect("window icon"))
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_decorations(false)
            .with_transparent(true),
        centered: true,
        ..Default::default()
    };
    let handle = runtime.handle().clone();
    let store = LocalStore::open(&data_dir);
    smd_gui::import::import_once(&store);
    let system_locale = sys_locale::get_locale().unwrap_or_default().chars().take(2).collect::<String>().to_lowercase();
    eframe::run_native(
        "Steam Manifest Downloader",
        options,
        Box::new(move |cc| {
            let wake = cc.egui_ctx.clone();
            events.set_wake(Arc::new(move || wake.request_repaint()));
            let window = window_handle(cc);
            let platform = Arc::new(smd_gui::platform::Native::new(window, handle.clone()));
            let boot = Boot { backend, events: events_rx, runtime: handle, settings, store, system_locale, platform };
            Ok(Box::new(App::new(&cc.egui_ctx, boot)))
        }),
    )
}

fn window_handle(cc: &eframe::CreationContext<'_>) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match cc.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}
