//! Tauri commands used by the UI. Thin wrappers: the logic lives in `config`, `display` and `runtime`.

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::config::{Config, ConfigError, GameEntry};
use crate::display::{distinct_resolutions, DisplayApi, DisplayMode};
use crate::engine::StatusEvent;
use crate::runtime::Input;
use crate::{store, AppState};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub device: String,
    pub label: String,
    pub primary: bool,
    pub native: DisplayMode,
    pub resolutions: Vec<Resolution>,
}

/// Applies `f` to the config, saves it and pushes the new rules to the runtime.
fn mutate<T>(state: &AppState, f: impl FnOnce(&mut Config) -> Result<T, ConfigError>) -> Result<T, String> {
    let mut config = state.config.lock().map_err(|_| "configuration verrouillée".to_string())?;
    let out = f(&mut config).map_err(|e| e.to_string())?;
    store::save(&state.config_path, &config).map_err(|e| format!("Impossible d'enregistrer la configuration : {e}"))?;
    let _ = state.input.send(Input::RulesChanged(config.rules()));
    Ok(out)
}

fn primary_modes(state: &AppState) -> Result<(Vec<DisplayMode>, DisplayMode), String> {
    let monitors = state.display.list_monitors().map_err(|e| e.to_string())?;
    let primary = monitors.iter().find(|m| m.primary).or(monitors.first()).ok_or("aucun écran détecté")?;
    let modes = state.display.list_modes(&primary.device).map_err(|e| e.to_string())?;
    let native = state.display.registry_mode(&primary.device).map_err(|e| e.to_string())?;
    Ok((modes, native))
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, String> {
    state.config.lock().map(|c| c.clone()).map_err(|_| "configuration verrouillée".to_string())
}

#[tauri::command]
pub fn list_displays(state: State<AppState>) -> Result<Vec<DisplayInfo>, String> {
    let monitors = state.display.list_monitors().map_err(|e| e.to_string())?;
    monitors
        .into_iter()
        .map(|m| {
            let modes = state.display.list_modes(&m.device).map_err(|e| e.to_string())?;
            let native = state.display.registry_mode(&m.device).map_err(|e| e.to_string())?;
            let resolutions = distinct_resolutions(&modes).into_iter().map(|(width, height)| Resolution { width, height }).collect();
            Ok(DisplayInfo { device: m.device, label: m.label, primary: m.primary, native, resolutions })
        })
        .collect()
}

#[tauri::command]
pub fn add_game(state: State<AppState>, exe_path: String) -> Result<GameEntry, String> {
    let (modes, native) = primary_modes(&state)?;
    let game = GameEntry::from_exe_path(&exe_path, &modes, native);
    mutate(&state, |c| c.add_game(game.clone()))?;
    Ok(game)
}

#[tauri::command]
pub fn update_game(state: State<AppState>, game: GameEntry) -> Result<(), String> {
    mutate(&state, |c| c.update_game(game))
}

#[tauri::command]
pub fn remove_game(state: State<AppState>, id: String) -> Result<(), String> {
    mutate(&state, |c| c.remove_game(&id))
}

#[tauri::command]
pub fn restore_native(state: State<AppState>) {
    let _ = state.input.send(Input::RestoreRequested);
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> StatusEvent {
    state.status.lock().map(|s| s.clone()).unwrap_or(StatusEvent::Idle)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("Démarrage automatique : {e}"))?;
    mutate(&state, |c| {
        c.settings.autostart = enabled;
        Ok(())
    })
}
