//! Tauri commands used by the UI. Thin wrappers: the logic lives in `config`, `display` and `runtime`.

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::config::{Config, ConfigError, GameEntry};
use crate::crosshair::model::Crosshair;
use base64::Engine as _;

use crate::display::{resolution_options, DisplayApi, DisplayMode};
use crate::engine::StatusEvent;
use crate::runtime::Input;
use crate::{store, AppState};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub device: String,
    pub label: String,
    pub primary: bool,
    pub native: DisplayMode,
    /// One entry per resolution, with the refresh rate REZ would use there.
    pub resolutions: Vec<DisplayMode>,
}

/// Game visuals as data URLs: Steam logo + hero art when available, and the exe icon.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameArt {
    pub logo: Option<String>,
    pub hero: Option<String>,
    pub icon: Option<String>,
}

fn data_url(bytes: &[u8], mime: &str) -> String {
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn file_data_url(path: &std::path::Path) -> Option<String> {
    let mime = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) { "image/png" } else { "image/jpeg" };
    std::fs::read(path).ok().map(|b| data_url(&b, mime))
}

/// Applies `f` to the config, saves it and pushes the new rules to the runtime.
fn mutate<T>(state: &AppState, f: impl FnOnce(&mut Config) -> Result<T, ConfigError>) -> Result<T, String> {
    let mut config = state.config.lock().map_err(|_| "configuration verrouillée".to_string())?;
    let out = f(&mut config).map_err(|e| e.to_string())?;
    store::save(&state.config_path, &config).map_err(|e| format!("Impossible d'enregistrer la configuration : {e}"))?;
    let _ = state.input.send(Input::RulesChanged(config.rules()));
    let _ = state.input.send(AppState::crosshair_input(&config));
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
            let resolutions = resolution_options(&modes, native);
            Ok(DisplayInfo { device: m.device, label: m.label, primary: m.primary, native, resolutions })
        })
        .collect()
}

#[tauri::command]
pub async fn get_game_art(state: State<'_, AppState>, id: String) -> Result<GameArt, String> {
    let exe_path = {
        let config = state.config.lock().map_err(|_| "configuration verrouillée".to_string())?;
        config.games.iter().find(|g| g.id == id).map(|g| g.exe_path.clone()).ok_or("Jeu introuvable")?
    };
    let steam = crate::art::steam_art_for_exe(&exe_path, &crate::win::steam::steam_roots());
    let icon = crate::win::icon::exe_icon_png(&exe_path, 256)
        .map_err(|e| log::info!("no icon for {exe_path}: {e}"))
        .ok()
        .map(|png| data_url(&png, "image/png"));
    Ok(GameArt {
        logo: steam.logo.as_deref().and_then(file_data_url),
        hero: steam.hero.as_deref().and_then(file_data_url),
        icon,
    })
}

#[tauri::command]
pub fn list_crosshairs(state: State<AppState>) -> Result<Vec<Crosshair>, String> {
    state.config.lock().map(|c| c.crosshairs.clone()).map_err(|_| "configuration verrouillée".to_string())
}

/// Creates (empty id) or updates a crosshair; returns it with its id.
#[tauri::command]
pub fn save_crosshair(state: State<AppState>, crosshair: Crosshair) -> Result<Crosshair, String> {
    let mut crosshair = crosshair;
    if crosshair.id.is_empty() {
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        crosshair.id = format!("custom-{ms}");
    }
    let saved = crosshair.clone();
    mutate(&state, |c| {
        c.upsert_crosshair(crosshair);
        Ok(())
    })?;
    Ok(saved)
}

#[tauri::command]
pub fn delete_crosshair(state: State<AppState>, id: String) -> Result<(), String> {
    mutate(&state, |c| c.delete_crosshair(&id))
}

/// PNG preview (data URL) drawn by the same renderer as the overlay.
#[tauri::command]
pub async fn preview_crosshair(state: State<'_, AppState>, crosshair: Crosshair) -> Result<String, String> {
    let png = crate::crosshair::render::render_png(&crosshair, 2.0, &state.crosshair_dir);
    Ok(data_url(&png, "image/png"))
}

/// Decodes a pasted code (Valorant, CS2, REZ). Nothing is saved: the UI shows the notes first.
#[tauri::command]
pub fn import_crosshair_code(code: String) -> Result<crate::crosshair::import::Imported, String> {
    crate::crosshair::import::import_code(&code)
}

/// Imports a PNG as a new crosshair (image stored in the crosshairs folder) and saves it.
#[tauri::command]
pub fn import_crosshair_image(state: State<AppState>, path: String) -> Result<Crosshair, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Image illisible : {e}"))?;
    let file = crate::crosshair::import::import_png(&bytes, &state.crosshair_dir)?;
    let name = std::path::Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Image".into());
    let crosshair = Crosshair { name, image: Some(file), ..Crosshair::default() };
    save_crosshair(state, crosshair)
}

/// REZ share code of a crosshair (for "Copier le code").
#[tauri::command]
pub fn crosshair_share_code(crosshair: Crosshair) -> String {
    crate::crosshair::share::encode(&crosshair)
}

#[tauri::command]
pub fn set_crosshair_settings(state: State<AppState>, undistort: bool, hide_with_faceit: bool) -> Result<(), String> {
    mutate(&state, |c| {
        c.settings.crosshair_undistort = undistort;
        c.settings.hide_crosshair_with_faceit = hide_with_faceit;
        Ok(())
    })
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
