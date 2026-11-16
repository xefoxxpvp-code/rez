//! User configuration: the games REZ manages and global settings.

use serde::{Deserialize, Serialize};

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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub autostart: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub games: Vec<GameEntry>,
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Config { version: 1, games: Vec::new(), settings: Settings::default() }
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
    fn update_and_remove_unknown_id_fail() {
        let mut c = Config::default();
        let g = GameEntry { id: "x".into(), ..Default::default() };
        assert_eq!(c.update_game(g), Err(ConfigError::NotFound("x".into())));
        assert_eq!(c.remove_game("x"), Err(ConfigError::NotFound("x".into())));
    }
}
