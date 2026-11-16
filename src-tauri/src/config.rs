//! User configuration: the games REZ manages and global settings.

use serde::{Deserialize, Serialize};

use crate::crosshair::director::CrosshairRule;
use crate::crosshair::model::Crosshair;
use crate::crosshair::presets::presets;
use crate::display::{default_resolution, DisplayMode};
use crate::engine::GameRule;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameEntry {
    pub id: String,
    pub name: String,
    pub exe_name: String,
    pub exe_path: String,
    pub width: u32,
    pub height: u32,
    pub enabled: bool,
    pub keep_stretched: bool,
    /// Crosshair drawn over this game; `None` = no REZ crosshair.
    pub crosshair_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub autostart: bool,
    /// The five presets were offered once already (deleting them all must not bring them back).
    pub crosshairs_seeded: bool,
    /// Keep the crosshair square on screen when the game is stretched.
    pub crosshair_undistort: bool,
    pub hide_crosshair_with_faceit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { autostart: false, crosshairs_seeded: false, crosshair_undistort: true, hide_crosshair_with_faceit: true }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub games: Vec<GameEntry>,
    pub settings: Settings,
    pub crosshairs: Vec<Crosshair>,
}

impl Default for Config {
    fn default() -> Self {
        Config { version: 1, games: Vec::new(), settings: Settings::default(), crosshairs: Vec::new() }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfigError {
    #[error("Ce jeu est déjà dans la liste")]
    AlreadyExists(String),
    #[error("Jeu introuvable")]
    NotFound(String),
}

impl GameEntry {
    /// Builds a new enabled entry from an executable path, with the default resolution for `modes`.
    pub fn from_exe_path(path: &str, modes: &[DisplayMode], native: DisplayMode) -> GameEntry {
        let exe_name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
        let name = match exe_name.rsplit_once('.') {
            Some((stem, _)) if !stem.is_empty() => stem.to_string(),
            _ => exe_name.clone(),
        };
        let (width, height) = default_resolution(modes, native);
        GameEntry {
            id: exe_name.to_lowercase(),
            name,
            exe_name,
            exe_path: path.to_string(),
            width,
            height,
            enabled: true,
            keep_stretched: false,
            crosshair_id: None,
        }
    }
}

impl Config {
    pub fn add_game(&mut self, game: GameEntry) -> Result<(), ConfigError> {
        if self.games.iter().any(|g| g.id == game.id) {
            return Err(ConfigError::AlreadyExists(game.id));
        }
        self.games.push(game);
        Ok(())
    }

    pub fn update_game(&mut self, game: GameEntry) -> Result<(), ConfigError> {
        match self.games.iter_mut().find(|g| g.id == game.id) {
            Some(slot) => {
                *slot = game;
                Ok(())
            }
            None => Err(ConfigError::NotFound(game.id)),
        }
    }

    pub fn remove_game(&mut self, id: &str) -> Result<(), ConfigError> {
        let before = self.games.len();
        self.games.retain(|g| g.id != id);
        if self.games.len() == before { Err(ConfigError::NotFound(id.to_string())) } else { Ok(()) }
    }

    /// Adds the five presets the first time; returns true if the config changed.
    pub fn seed_crosshairs(&mut self) -> bool {
        if self.settings.crosshairs_seeded {
            return false;
        }
        self.crosshairs.extend(presets());
        self.settings.crosshairs_seeded = true;
        true
    }

    /// Replaces the crosshair with the same id, or appends it.
    pub fn upsert_crosshair(&mut self, crosshair: Crosshair) {
        match self.crosshairs.iter_mut().find(|c| c.id == crosshair.id) {
            Some(slot) => *slot = crosshair,
            None => self.crosshairs.push(crosshair),
        }
    }

    /// Deletes a crosshair; games that used it go back to "no crosshair".
    pub fn delete_crosshair(&mut self, id: &str) -> Result<(), ConfigError> {
        let before = self.crosshairs.len();
        self.crosshairs.retain(|c| c.id != id);
        if self.crosshairs.len() == before {
            return Err(ConfigError::NotFound(id.to_string()));
        }
        for g in self.games.iter_mut().filter(|g| g.crosshair_id.as_deref() == Some(id)) {
            g.crosshair_id = None;
        }
        Ok(())
    }

    /// Overlay rules: enabled games whose crosshair exists.
    pub fn crosshair_rules(&self) -> Vec<CrosshairRule> {
        self.games
            .iter()
            .filter(|g| g.enabled)
            .filter_map(|g| {
                let id = g.crosshair_id.as_deref()?;
                let crosshair = self.crosshairs.iter().find(|c| c.id == id)?.clone();
                Some(CrosshairRule { exe_name_lower: g.exe_name.to_lowercase(), crosshair })
            })
            .collect()
    }

    /// Engine rules for enabled games only.
    pub fn rules(&self) -> Vec<GameRule> {
        self.games
            .iter()
            .filter(|g| g.enabled)
            .map(|g| GameRule {
                id: g.id.clone(),
                exe_name_lower: g.exe_name.to_lowercase(),
                width: g.width,
                height: g.height,
                keep_stretched: g.keep_stretched,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(w: u32, h: u32, hz: u32) -> DisplayMode {
        DisplayMode { width: w, height: h, refresh_hz: hz }
    }

    #[test]
    fn from_exe_path_builds_entry() {
        let modes = [m(1920, 1080, 144), m(1440, 1080, 144)];
        let g = GameEntry::from_exe_path(r"C:\Games\Rust\RustClient.exe", &modes, m(1920, 1080, 144));
        assert_eq!(
            (g.id.as_str(), g.name.as_str(), g.exe_name.as_str()),
            ("rustclient.exe", "RustClient", "RustClient.exe")
        );
        assert_eq!(g.exe_path, r"C:\Games\Rust\RustClient.exe");
        assert_eq!((g.width, g.height, g.enabled, g.keep_stretched), (1440, 1080, true, false));
    }

    #[test]
    fn add_same_exe_twice_fails_case_insensitive() {
        let mut c = Config::default();
        c.add_game(GameEntry::from_exe_path(r"C:\A\cs2.exe", &[], m(1920, 1080, 60))).unwrap();
        let dup = GameEntry::from_exe_path(r"D:\B\CS2.EXE", &[], m(1920, 1080, 60));
        assert_eq!(c.add_game(dup), Err(ConfigError::AlreadyExists("cs2.exe".into())));
    }

    #[test]
    fn rules_exclude_disabled_games() {
        let mut c = Config::default();
        c.add_game(GameEntry::from_exe_path(r"C:\A\cs2.exe", &[], m(1920, 1080, 60))).unwrap();
        let mut off = GameEntry::from_exe_path(r"C:\B\RustClient.exe", &[], m(1920, 1080, 60));
        off.enabled = false;
        c.add_game(off).unwrap();
        let rules = c.rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].exe_name_lower, "cs2.exe");
        assert_eq!(rules[0].id, "cs2.exe");
    }

    #[test]
    fn seed_adds_presets_once() {
        let mut c = Config::default();
        assert!(c.seed_crosshairs());
        assert_eq!(c.crosshairs.len(), 5);
        let ids: Vec<String> = c.crosshairs.iter().map(|x| x.id.clone()).collect();
        for id in ids {
            c.delete_crosshair(&id).unwrap();
        }
        assert!(!c.seed_crosshairs());
        assert!(c.crosshairs.is_empty());
    }

    #[test]
    fn deleting_crosshair_unassigns_games() {
        let mut c = Config::default();
        c.seed_crosshairs();
        let mut g = GameEntry::from_exe_path(r"C:\A\cs2.exe", &[], m(1920, 1080, 60));
        g.crosshair_id = Some("preset-t".into());
        c.add_game(g).unwrap();
        c.delete_crosshair("preset-t").unwrap();
        assert_eq!(c.games[0].crosshair_id, None);
        assert_eq!(c.delete_crosshair("nope"), Err(ConfigError::NotFound("nope".into())));
    }

    #[test]
    fn crosshair_rules_resolve_ids() {
        let mut c = Config::default();
        c.seed_crosshairs();
        let mut on = GameEntry::from_exe_path(r"C:\A\CS2.exe", &[], m(1920, 1080, 60));
        on.crosshair_id = Some("preset-point".into());
        let mut off = GameEntry::from_exe_path(r"C:\B\rust.exe", &[], m(1920, 1080, 60));
        off.crosshair_id = Some("preset-point".into());
        off.enabled = false;
        let mut unknown = GameEntry::from_exe_path(r"C:\C\other.exe", &[], m(1920, 1080, 60));
        unknown.crosshair_id = Some("ghost".into());
        for g in [on, off, unknown] {
            c.add_game(g).unwrap();
        }
        let rules = c.crosshair_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].exe_name_lower, "cs2.exe");
        assert_eq!(rules[0].crosshair.name, "Point");
    }

    #[test]
    fn upsert_replaces_by_id() {
        let mut c = Config::default();
        c.seed_crosshairs();
        let mut x = c.crosshairs[0].clone();
        x.name = "Renommé".into();
        c.upsert_crosshair(x);
        assert_eq!(c.crosshairs.len(), 5);
        assert_eq!(c.crosshairs[0].name, "Renommé");
    }

    #[test]
    fn v1_config_gets_crosshair_defaults() {
        let json = r#"{"version":1,"games":[{"id":"cs2.exe","exeName":"cs2.exe","width":1280,"height":960,"enabled":true}],"settings":{"autostart":false}}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert!(c.crosshairs.is_empty());
        assert!(!c.settings.crosshairs_seeded);
        assert!(c.settings.crosshair_undistort);
        assert!(c.settings.hide_crosshair_with_faceit);
        assert_eq!(c.games[0].crosshair_id, None);
    }

    #[test]
    fn update_and_remove_unknown_id_fail() {
        let mut c = Config::default();
        let g = GameEntry { id: "x".into(), ..Default::default() };
        assert_eq!(c.update_game(g), Err(ConfigError::NotFound("x".into())));
        assert_eq!(c.remove_game("x"), Err(ConfigError::NotFound("x".into())));
    }
}
