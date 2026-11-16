//! Atomic JSON persistence of [`Config`] in `%APPDATA%\REZ\config.json`.

use std::path::{Path, PathBuf};

use crate::config::Config;

/// `%APPDATA%\REZ\config.json`; creates the directory when missing.
pub fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("REZ");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("cannot create {}: {e}", dir.display());
    }
    dir.join("config.json")
}

/// Loads the config. Missing file → default. Unparsable file → backed up as
/// `config.json.corrupt-<unix secs>` and default returned.
pub fn load(path: &Path) -> Config {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Config::default(),
    };
    match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(e) => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let name = format!(
                "{}.corrupt-{secs}",
                path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "config.json".into())
            );
            let backup = path.with_file_name(name);
            log::warn!("config unreadable ({e}); backing up to {}", backup.display());
            if let Err(e) = std::fs::rename(path, &backup) {
                log::warn!("backup failed: {e}");
            }
            Config::default()
        }
    }
}

/// Writes `<path>.tmp` then renames it over `path` (replace is atomic on NTFS).
pub fn save(path: &Path, config: &Config) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameEntry;

    #[test]
    fn missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("config.json")), Config::default());
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut c = Config::default();
        c.games.push(GameEntry { id: "cs2.exe".into(), exe_name: "cs2.exe".into(), width: 1280, height: 960, enabled: true, ..Default::default() });
        c.settings.autostart = true;
        save(&path, &c).unwrap();
        assert_eq!(load(&path), c);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn corrupt_file_is_backed_up_and_default_returned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(load(&path), Config::default());
        let backed_up = std::fs::read_dir(dir.path()).unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().starts_with("config.json.corrupt-"));
        assert!(backed_up);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"games":[{"id":"cs2.exe","exeName":"cs2.exe","width":1280,"height":960}]}"#).unwrap();
        let c = load(&path);
        assert_eq!(c.version, 1);
        assert_eq!(c.games.len(), 1);
        assert!(!c.games[0].enabled);
        assert!(!c.games[0].keep_stretched);
        assert!(!c.settings.autostart);
    }
}
