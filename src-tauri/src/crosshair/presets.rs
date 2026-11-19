//! The five crosshairs REZ offers on first launch: the styles pro players use most
//! (small, static, no outline; cyan and green lead). Sizes are px at 1080p.

use super::model::{Crosshair, Dot, Lines, Outline};

fn preset(id: &str, name: &str, color: &str) -> Crosshair {
    Crosshair { id: format!("preset-{id}"), name: name.into(), color: color.into(), ..Default::default() }
}

fn no_outline() -> Outline {
    Outline { enabled: false, ..Outline::default() }
}

pub fn presets() -> Vec<Crosshair> {
    // Valorant pro staple: small cyan cross, no outline (0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0).
    let cyan_pro = Crosshair {
        inner: Lines { length: 4.0, thickness: 2.0, gap: 2.0, opacity: 1.0, ..Lines::default() },
        outline: no_outline(),
        ..preset("cyan-pro", "Cyan pro", "#00FFFF")
    };
    // Borderless "plus" with no gap, also very common in Valorant pro play.
    let plus = Crosshair {
        inner: Lines { length: 4.0, thickness: 2.0, gap: 0.0, opacity: 1.0, ..Lines::default() },
        outline: no_outline(),
        ..preset("plus", "Plus sans écart", "#FFFFFF")
    };
    // CS2 consensus: static, size 2, thickness 1, gap -3, no outline, classic green.
    let vert = Crosshair {
        inner: Lines { length: 4.0, thickness: 2.0, gap: 2.0, opacity: 1.0, ..Lines::default() },
        outline: no_outline(),
        ..preset("vert-statique", "Vert statique", "#32FA32")
    };
    // Center dot only, with a thin outline so it stays visible on bright walls.
    let point = Crosshair {
        inner: Lines { enabled: false, ..Lines::default() },
        dot: Dot { enabled: true, size: 4.0, opacity: 1.0 },
        ..preset("point", "Point", "#FFFFFF")
    };
    // CS2 thin lines (thickness 0 = 1 px), small negative gap, yellow.
    let fin = Crosshair {
        inner: Lines { length: 5.0, thickness: 1.0, gap: 2.0, opacity: 1.0, ..Lines::default() },
        outline: no_outline(),
        ..preset("fin-jaune", "Fin jaune", "#FAFA32")
    };
    vec![cyan_pro, plus, vert, point, fin]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_five_distinct_named_crosshairs() {
        let p = presets();
        let names: Vec<&str> = p.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Cyan pro", "Plus sans écart", "Vert statique", "Point", "Fin jaune"]);
        let mut ids: Vec<&str> = p.iter().map(|c| c.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
        assert!(ids.contains(&"preset-point") && ids.contains(&"preset-plus"));
    }

    #[test]
    fn presets_follow_pro_habits() {
        let p = presets();
        // Pros favour small static crosshairs without outline (cyan / green most common).
        let cyan = &p[0];
        assert_eq!(cyan.color, "#00FFFF");
        assert!(!cyan.outline.enabled && !cyan.outer.enabled);
        assert_eq!((cyan.inner.length, cyan.inner.thickness, cyan.inner.gap), (4.0, 2.0, 2.0));
        assert_eq!(p[1].inner.gap, 0.0);
        assert!(p[3].dot.enabled && !p[3].inner.enabled);
        assert_eq!(p[4].inner.thickness, 1.0);
        assert!(p.iter().all(|c| !c.circle.enabled && !c.t_style));
    }
}
