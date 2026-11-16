//! The five crosshairs REZ offers on first launch (values from the crosshair spec).

use super::model::{Circle, Crosshair, Dot, Lines};

fn preset(id: &str, name: &str, color: &str) -> Crosshair {
    Crosshair { id: format!("preset-{id}"), name: name.into(), color: color.into(), ..Default::default() }
}

pub fn presets() -> Vec<Crosshair> {
    let classique = Crosshair {
        inner: Lines { length: 6.0, thickness: 2.0, gap: 3.0, ..Lines::default() },
        ..preset("classique", "Classique", "#00FF00")
    };
    let point = Crosshair {
        inner: Lines { enabled: false, ..Lines::default() },
        dot: Dot { enabled: true, size: 4.0, opacity: 1.0 },
        ..preset("point", "Point", "#FFFFFF")
    };
    let t = Crosshair {
        inner: Lines { length: 7.0, thickness: 2.0, gap: 4.0, ..Lines::default() },
        t_style: true,
        ..preset("t", "T", "#00FFFF")
    };
    let cercle = Crosshair {
        inner: Lines { enabled: false, ..Lines::default() },
        dot: Dot { enabled: true, size: 3.0, opacity: 1.0 },
        circle: Circle { enabled: true, radius: 14.0, thickness: 2.0, opacity: 1.0 },
        ..preset("cercle", "Cercle", "#FF4FD8")
    };
    let compact = Crosshair {
        inner: Lines { length: 4.0, thickness: 2.0, gap: 0.0, ..Lines::default() },
        ..preset("compact", "Compact", "#FFFF00")
    };
    vec![classique, point, t, cercle, compact]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_five_distinct_named_crosshairs() {
        let p = presets();
        let names: Vec<&str> = p.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Classique", "Point", "T", "Cercle", "Compact"]);
        let mut ids: Vec<&str> = p.iter().map(|c| c.id.as_str()).collect();
        ids.dedup();
        assert_eq!(ids.len(), 5);
        assert!(p.iter().find(|c| c.name == "T").unwrap().t_style);
        assert!(p.iter().find(|c| c.name == "Cercle").unwrap().circle.enabled);
        assert!(!p.iter().find(|c| c.name == "Point").unwrap().inner.enabled);
    }
}
