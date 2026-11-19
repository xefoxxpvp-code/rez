//! Valorant crosshair profile codes (`0;P;…`). Only the primary section is drawn; movement
//! and firing error are dynamic and cannot be reproduced by a static overlay.
//! Defaults and key meanings come from the community parsers (genesy/crosshair-codes, MIT).

use std::collections::HashMap;

use super::import::Imported;
use super::model::{Crosshair, Dot, Lines, Outline};

const PALETTE: [&str; 8] = ["#FFFFFF", "#00FF00", "#7FFF00", "#DFFF00", "#FFFF00", "#00FFFF", "#FF00FF", "#FF0000"];

const KNOWN: [&str; 30] = [
    "c", "u", "b", "h", "t", "o", "d", "z", "a", "f", "s", "m", //
    "0b", "0t", "0l", "0g", "0v", "0o", "0a", "0m", "0s", "0f", "0e", //
    "1b", "1t", "1l", "1g", "1v", "1o", "1a",
];
const KNOWN_DYNAMIC: [&str; 4] = ["1m", "1s", "1f", "1e"];

struct Keys<'a>(HashMap<&'a str, &'a str>);

impl Keys<'_> {
    fn num(&self, key: &str, default: f32) -> f32 {
        self.0.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
    fn flag(&self, key: &str, default: bool) -> bool {
        self.0.get(key).map(|v| *v != "0").unwrap_or(default)
    }
    fn lines(&self, prefix: char, length: f32, gap: f32, opacity: f32) -> Lines {
        let k = |s: &str| format!("{prefix}{s}");
        let length = self.num(&k("l"), length);
        let separate = self.flag(&k("g"), false);
        Lines {
            enabled: self.flag(&k("b"), true),
            length,
            vertical_length: separate.then(|| self.num(&k("v"), length)),
            thickness: self.num(&k("t"), 2.0),
            gap: self.num(&k("o"), gap),
            opacity: self.num(&k("a"), opacity),
        }
    }
}

pub fn parse(code: &str) -> Result<Imported, String> {
    let tokens: Vec<&str> = code.trim().split(';').filter(|t| !t.is_empty()).collect();
    if tokens.first() != Some(&"0") {
        return Err("Code Valorant invalide : il doit commencer par 0;".into());
    }
    let mut primary = HashMap::new();
    let mut section = "";
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i];
        if t.len() == 1 && t.chars().all(|c| c.is_ascii_uppercase()) {
            section = t;
            i += 1;
            continue;
        }
        let value = *tokens.get(i + 1).ok_or("Code Valorant incomplet")?;
        if section == "P" {
            primary.insert(t, value);
        }
        i += 2;
    }
    let keys = Keys(primary);

    let color_index = keys.num("c", 0.0) as usize;
    let color = if color_index == 8 {
        keys.0
            .get("u")
            .and_then(|u| u.get(..6))
            .filter(|h| h.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|h| format!("#{}", h.to_uppercase()))
            .unwrap_or_else(|| "#FFFFFF".into())
    } else {
        PALETTE.get(color_index).copied().unwrap_or("#FFFFFF").to_string()
    };
    let crosshair = Crosshair {
        name: "Import Valorant".into(),
        color,
        inner: keys.lines('0', 6.0, 3.0, 0.8),
        outer: keys.lines('1', 2.0, 10.0, 0.35),
        outline: Outline { enabled: keys.flag("h", true), thickness: keys.num("t", 1.0), opacity: keys.num("o", 0.5) },
        dot: Dot { enabled: keys.flag("d", false), size: keys.num("z", 2.0), opacity: keys.num("a", 1.0) },
        ..Crosshair::default()
    }
    .sanitized();

    let mut notes = Vec::new();
    if crosshair.inner.enabled || crosshair.outer.enabled {
        notes.push("Crosshair fixe : l'écartement au tir et en mouvement n'est pas reproduit.".to_string());
    }
    let unknown: Vec<&str> =
        keys.0.keys().copied().filter(|k| !KNOWN.contains(k) && !KNOWN_DYNAMIC.contains(k)).collect();
    if !unknown.is_empty() {
        notes.push(format!("Réglages ignorés : {}", unknown.join(", ")));
    }
    Ok(Imported { crosshair, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_cyan_code() {
        let imported = parse("0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0").unwrap();
        let c = imported.crosshair;
        assert_eq!(c.color, "#00FFFF");
        assert!(!c.outline.enabled && !c.outer.enabled && !c.dot.enabled);
        assert!(c.inner.enabled);
        assert_eq!((c.inner.length, c.inner.vertical_length, c.inner.thickness), (4.0, None, 2.0));
        assert_eq!((c.inner.gap, c.inner.opacity), (2.0, 1.0));
    }

    #[test]
    fn parses_custom_color_dot_and_separate_vertical() {
        let c = parse("0;s;1;P;c;8;u;FF00AAFF;d;1;z;3;0g;1;0v;8;1b;0;S;c;1").unwrap().crosshair;
        assert_eq!(c.color, "#FF00AA");
        assert!(c.dot.enabled);
        assert_eq!(c.dot.size, 3.0);
        assert_eq!((c.inner.length, c.inner.vertical_length), (6.0, Some(8.0)));
        assert_eq!((c.inner.thickness, c.inner.gap), (2.0, 3.0));
        assert!((c.inner.opacity - 0.8).abs() < 1e-6);
        assert!(c.outline.enabled);
        assert_eq!((c.outline.thickness, c.outline.opacity), (1.0, 0.5));
        assert!(!c.outer.enabled);
    }

    #[test]
    fn defaults_show_outer_lines() {
        let c = parse("0;P;c;1").unwrap().crosshair;
        assert!(c.outer.enabled);
        assert_eq!((c.outer.length, c.outer.gap), (2.0, 10.0));
        assert!((c.outer.opacity - 0.35).abs() < 1e-6);
        assert_eq!(c.color, "#00FF00");
    }

    #[test]
    fn notes_static_limitation() {
        let imported = parse("0;P;c;5;0l;4").unwrap();
        assert!(imported.notes.iter().any(|n| n.contains("tir")));
    }

    #[test]
    fn hostile_values_are_clamped_not_trusted() {
        let c = parse("0;P;0l;1e30;0o;NaN;0t;inf;z;-9").unwrap().crosshair;
        assert!(c.inner.length <= crate::crosshair::model::MAX_PX);
        assert!(c.inner.gap.is_finite());
        assert!(c.inner.thickness <= crate::crosshair::model::MAX_PX);
        assert!(c.dot.size >= 0.0);
    }

    #[test]
    fn non_ascii_custom_color_falls_back_to_white() {
        let c = parse("0;P;c;8;u;éééééé").unwrap().crosshair;
        assert_eq!(c.color, "#FFFFFF");
        let c = parse("0;P;c;8;u;GGGGGGFF").unwrap().crosshair;
        assert_eq!(c.color, "#FFFFFF");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("").is_err());
        assert!(parse("hello").is_err());
        assert!(parse("1;P;c;5").is_err());
        assert!(parse("0;P;c").is_err(), "dangling key");
    }
}
