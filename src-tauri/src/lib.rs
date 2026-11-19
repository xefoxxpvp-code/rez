pub mod art;
pub mod commands;
pub mod config;
pub mod crosshair;
pub mod display;
pub mod engine;
pub mod imaging;
pub mod layout_book;
pub mod runtime;
pub mod store;
pub mod tray;
pub mod win;

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::config::Config;
use crate::engine::StatusEvent;
use crate::runtime::{Input, Runtime};
use crate::win::desktop_layout::Win32Layout;
use crate::win::display::Win32Display;

pub const STATUS_EVENT: &str = "rez://status";

pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub input: mpsc::Sender<Input>,
    pub status: Mutex<StatusEvent>,
    pub display: Arc<Win32Display>,
    /// Imported crosshair images (`%APPDATA%\REZ\crosshairs`).
    pub crosshair_dir: PathBuf,
}

impl AppState {
    /// Message telling the runtime the current overlay rules and settings.
    pub fn crosshair_input(config: &Config) -> Input {
        Input::CrosshairRulesChanged {
            rules: config.crosshair_rules(),
            undistort: config.settings.crosshair_undistort,
            hide_with_faceit: config.settings.hide_crosshair_with_faceit,
        }
    }
}

/// Emergency shortcut: back to native resolution.
fn restore_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyR)
}

/// Crosshair show/hide shortcut.
fn crosshair_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyX)
}

/// Focuses the main window, or builds it (the WebView is destroyed when the window closes).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("REZ")
        .inner_size(960.0, 640.0)
        .min_inner_size(820.0, 560.0)
        .build();
    if let Err(e) = built {
        log::error!("cannot open main window: {e}");
    }
}

fn on_status(app: &AppHandle, status: StatusEvent) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut s) = state.status.lock() {
            *s = status.clone();
        }
        let tooltip = match &status {
            StatusEvent::Active { game_id, width, height, refresh_hz } => {
                let name = state
                    .config
                    .lock()
                    .ok()
                    .and_then(|c| c.games.iter().find(|g| &g.id == game_id).map(|g| g.name.clone()))
                    .unwrap_or_else(|| game_id.clone());
                format!("REZ — {name} {width}×{height} {refresh_hz} Hz")
            }
            _ => "REZ — inactif".to_string(),
        };
        // set_tooltip blocks until the main thread runs it; never block the runtime thread on
        // that (at exit the main thread waits for the runtime's ack).
        let app = app.clone();
        std::thread::spawn(move || tray::set_tooltip(&app, &tooltip));
    }
    let _ = app.emit(STATUS_EVENT, &status);
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        win::display::restore_all();
        previous(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();
    let minimized = std::env::args().any(|a| a == "--minimized");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(state) = app.try_state::<AppState>() else { return };
                    if shortcut == &restore_shortcut() {
                        let _ = state.input.send(Input::RestoreRequested);
                    } else if shortcut == &crosshair_shortcut() {
                        let _ = state.input.send(Input::ToggleCrosshair);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let config_path = store::config_path();
            let mut config = store::load(&config_path);
            if config.seed_crosshairs()
                && let Err(e) = store::save(&config_path, &config) {
                    log::warn!("cannot save seeded crosshairs: {e}");
                }
            let display = Arc::new(Win32Display::new());
            let (tx, rx) = mpsc::channel::<Input>();

            let handle = app.handle().clone();
            let watch_tx = tx.clone();
            let icons_snapshot = config_path.with_file_name("icons-snapshot.json");
            Win32Layout::recover_icons_from_disk(&icons_snapshot);
            let crosshair_dir = config_path.with_file_name("crosshairs");
            if let Err(e) = std::fs::create_dir_all(&crosshair_dir) {
                log::warn!("cannot create {}: {e}", crosshair_dir.display());
            }
            Runtime::new(
                config.rules(),
                display.clone(),
                Box::new(Win32Layout::new(icons_snapshot)),
                Box::new(move |pid| win::events::watch_process(pid, watch_tx.clone())),
                Box::new(move |status| on_status(&handle, status)),
            )
            .with_crosshair(
                Box::new(win::overlay::Win32Overlay::spawn(crosshair_dir.clone())),
                Box::new(|| win::process::is_any_running(&win::process::FACEIT_PROCESSES)),
            )
            .spawn(rx);
            let _ = tx.send(AppState::crosshair_input(&config));
            win::events::start(tx.clone())?;

            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
                input: tx,
                status: Mutex::new(StatusEvent::Idle),
                display,
                crosshair_dir,
            });
            tray::create(app.handle())?;
            if let Err(e) = app.global_shortcut().register(restore_shortcut()) {
                log::warn!("emergency shortcut Ctrl+Alt+Shift+R unavailable: {e}");
            }
            if let Err(e) = app.global_shortcut().register(crosshair_shortcut()) {
                log::warn!("crosshair shortcut Ctrl+Alt+Shift+X unavailable: {e}");
            }
            if !minimized {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::list_displays,
            commands::list_crosshairs,
            commands::save_crosshair,
            commands::delete_crosshair,
            commands::preview_crosshair,
            commands::set_crosshair_settings,
            commands::import_crosshair_code,
            commands::import_crosshair_image,
            commands::crosshair_share_code,
            commands::get_game_art,
            commands::add_game,
            commands::update_game,
            commands::remove_game,
            commands::restore_native,
            commands::get_status,
            commands::set_autostart,
        ])
        .build(tauri::generate_context!())
        .expect("error while building REZ");

    app.run(|app, event| match event {
        // Closing the last window keeps REZ running in the notification area.
        RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
        RunEvent::Exit => {
            if let Some(state) = app.try_state::<AppState>() {
                let (ack, ack_rx) = mpsc::channel();
                if state.input.send(Input::Shutdown { ack }).is_ok() {
                    let _ = ack_rx.recv_timeout(Duration::from_secs(2));
                }
            }
        }
        _ => {}
    });
}
