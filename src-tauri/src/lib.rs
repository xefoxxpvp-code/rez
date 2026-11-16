pub mod commands;
pub mod config;
pub mod display;
pub mod engine;
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
}

/// Emergency shortcut: back to native resolution.
fn restore_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyR)
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
            StatusEvent::Active { game_id, width, height } => {
                let name = state
                    .config
                    .lock()
                    .ok()
                    .and_then(|c| c.games.iter().find(|g| &g.id == game_id).map(|g| g.name.clone()))
                    .unwrap_or_else(|| game_id.clone());
                format!("REZ — {name} {width}×{height}")
            }
            _ => "REZ — inactif".to_string(),
        };
        tray::set_tooltip(app, &tooltip);
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
                    if event.state == ShortcutState::Pressed && shortcut == &restore_shortcut()
                        && let Some(state) = app.try_state::<AppState>() {
                            let _ = state.input.send(Input::RestoreRequested);
                        }
                })
                .build(),
        )
        .setup(move |app| {
            let config_path = store::config_path();
            let config = store::load(&config_path);
            let display = Arc::new(Win32Display::new());
            let (tx, rx) = mpsc::channel::<Input>();

            let handle = app.handle().clone();
            let watch_tx = tx.clone();
            Runtime::new(
                config.rules(),
                display.clone(),
                Box::new(Win32Layout::new()),
                Box::new(move |pid| win::events::watch_process(pid, watch_tx.clone())),
                Box::new(move |status| on_status(&handle, status)),
            )
            .spawn(rx);
            win::events::start(tx.clone())?;

            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
                input: tx,
                status: Mutex::new(StatusEvent::Idle),
                display,
            });
            tray::create(app.handle())?;
            if let Err(e) = app.global_shortcut().register(restore_shortcut()) {
                log::warn!("emergency shortcut Ctrl+Alt+Shift+R unavailable: {e}");
            }
            if !minimized {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::list_displays,
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
