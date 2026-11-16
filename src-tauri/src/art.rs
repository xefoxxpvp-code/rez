//! Game artwork. For a Steam install, the Steam client keeps the game's transparent logo
//! and wide hero art in `appcache/librarycache/<appid>/`; other games fall back to the
//! executable's own icon (see `win::icon`).

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArtPaths {
    pub logo: Option<PathBuf>,
    pub hero: Option<PathBuf>,
}

/// Value of `"key"  "value"` in a Steam KeyValues (.acf / .vdf) text.
pub fn acf_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(&needle)?;
        let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
        Some(value.replace("\\\\", "\\"))
    })
}

/// For `<library>\steamapps\common\<installdir>\...`, returns the library root and installdir.
pub fn steam_install(exe_path: &str) -> Option<(PathBuf, String)> {
    let lower = exe_path.to_lowercase();
    let marker = "\\steamapps\\common\\";
    let at = lower.find(marker)?;
    let library = PathBuf::from(&exe_path[..at]);
    let rest = &exe_path[at + marker.len()..];
    let installdir = rest.split('\\').next().filter(|s| !s.is_empty())?;
    Some((library, installdir.to_string()))
}

/// Finds the app id whose manifest in `<library>\steamapps` has this installdir (case-insensitive).
pub fn steam_appid(library: &Path, installdir: &str) -> Option<u32> {
    let entries = std::fs::read_dir(library.join("steamapps")).ok()?;
    entries.filter_map(|e| e.ok()).find_map(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
            return None;
        }
        let text = std::fs::read_to_string(e.path()).ok()?;
        let dir = acf_value(&text, "installdir")?;
        if !dir.eq_ignore_ascii_case(installdir) {
            return None;
        }
        acf_value(&text, "appid")?.parse().ok()
    })
}

/// Searches `<steam_root>\appcache\librarycache\<appid>` (and its hash subfolders) for the
/// transparent logo and the wide hero art.
pub fn find_steam_art(steam_root: &Path, appid: u32) -> ArtPaths {
    let dir = steam_root.join("appcache").join("librarycache").join(appid.to_string());
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let path = e.path();
            if path.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&path) {
                    files.extend(sub.filter_map(|s| s.ok()).map(|s| s.path()));
                }
            } else {
                files.push(path);
            }
        }
    }
    let find = |names: &[&str]| {
        names.iter().find_map(|n| {
            files
                .iter()
                .find(|p| p.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(n)))
                .cloned()
        })
    };
    ArtPaths {
        logo: find(&["logo.png"]),
        hero: find(&["library_hero.jpg", "library_header.jpg", "header.jpg"]),
    }
}

/// Steam artwork for an executable, trying each candidate Steam root (registry path, the
/// game's own library, the default install folder).
pub fn steam_art_for_exe(exe_path: &str, steam_roots: &[PathBuf]) -> ArtPaths {
    let Some((library, installdir)) = steam_install(exe_path) else { return ArtPaths::default() };
    let Some(appid) = steam_appid(&library, &installdir) else { return ArtPaths::default() };
    let mut roots = steam_roots.to_vec();
    roots.push(library);
    roots
        .iter()
        .map(|root| find_steam_art(root, appid))
        .find(|art| art.logo.is_some() || art.hero.is_some())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACF: &str = r#""AppState"
{
	"appid"		"252490"
	"name"		"Rust"
	"installdir"		"Rust"
}"#;

    #[test]
    fn acf_value_reads_quoted_fields() {
        assert_eq!(acf_value(ACF, "appid").as_deref(), Some("252490"));
        assert_eq!(acf_value(ACF, "installdir").as_deref(), Some("Rust"));
        assert_eq!(acf_value(ACF, "missing"), None);
    }

    #[test]
    fn steam_install_splits_library_and_installdir_case_insensitively() {
        let (lib, dir) = steam_install(r"D:\SteamLibrary\SteamApps\Common\Rust\RustClient.exe").unwrap();
        assert_eq!(lib, PathBuf::from(r"D:\SteamLibrary"));
        assert_eq!(dir, "Rust");
        assert_eq!(steam_install(r"C:\Riot Games\VALORANT\live\VALORANT.exe"), None);
    }

    #[test]
    fn steam_appid_matches_manifest_installdir() {
        let lib = tempfile::tempdir().unwrap();
        let apps = lib.path().join("steamapps");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(apps.join("appmanifest_252490.acf"), ACF).unwrap();
        std::fs::write(apps.join("appmanifest_730.acf"), ACF.replace("252490", "730").replace("\"Rust\"", "\"Counter-Strike Global Offensive\"")).unwrap();
        assert_eq!(steam_appid(lib.path(), "rust"), Some(252490));
        assert_eq!(steam_appid(lib.path(), "Counter-Strike Global Offensive"), Some(730));
        assert_eq!(steam_appid(lib.path(), "Nope"), None);
    }

    #[test]
    fn find_steam_art_searches_hash_subfolders() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("appcache").join("librarycache").join("252490");
        std::fs::create_dir_all(cache.join("0a1b2c")).unwrap();
        std::fs::write(cache.join("logo.png"), b"png").unwrap();
        std::fs::write(cache.join("0a1b2c").join("library_hero.jpg"), b"jpg").unwrap();
        let art = find_steam_art(root.path(), 252490);
        assert_eq!(art.logo, Some(cache.join("logo.png")));
        assert_eq!(art.hero, Some(cache.join("0a1b2c").join("library_hero.jpg")));
        assert_eq!(find_steam_art(root.path(), 1), ArtPaths::default());
    }
}
