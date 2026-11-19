//! The crosshair model shared by the renderer, the editor and the importers.
//! Sizes are pixels at 1080p; the renderer scales them to the current mode.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Lines {
    pub enabled: bool,
    pub length: f32,
    /// Separate length for the vertical arms (Valorant "unlinked" lines); `None` = same as `length`.
    pub vertical_length: Option<f32>,
    pub thickness: f32,
    /// Distance from the center to where the arms start.
    pub gap: f32,
    pub opacity: f32,
}

impl Default for Lines {
    fn default() -> Self {
        Lines { enabled: true, length: 6.0, vertical_length: None, thickness: 2.0, gap: 3.0, opacity: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Outline {
    pub enabled: bool,
    pub thickness: f32,
    pub opacity: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Outline { enabled: true, thickness: 1.0, opacity: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Dot {
    pub enabled: bool,
    pub size: f32,
    pub opacity: f32,
}

impl Default for Dot {
    fn default() -> Self {
        Dot { enabled: false, size: 2.0, opacity: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Circle {
    pub enabled: bool,
    pub radius: f32,
    pub thickness: f32,
    pub opacity: f32,
}

impl Default for Circle {
    fn default() -> Self {
        Circle { enabled: false, radius: 12.0, thickness: 2.0, opacity: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Crosshair {
    pub id: String,
    pub name: String,
    /// `#RRGGBB`.
    pub color: String,
    pub inner: Lines,
    pub outer: Lines,
    /// No top arm.
    pub t_style: bool,
    pub outline: Outline,
    pub dot: Dot,
    pub circle: Circle,
    /// PNG file name in the crosshairs folder; drawn instead of the shapes.
    pub image: Option<String>,
    pub image_scale: f32,
    pub image_opacity: f32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Default for Crosshair {
    fn default() -> Self {
        Crosshair {
            id: String::new(),
            name: String::new(),
            color: "#00FF00".into(),
            inner: Lines::default(),
            outer: Lines { enabled: false, ..Lines::default() },
            t_style: false,
            outline: Outline::default(),
            dot: Dot::default(),
            circle: Circle::default(),
            image: None,
            image_scale: 1.0,
            image_opacity: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// Upper bound for any size in the model (px at 1080p). Imported or edited values beyond it
/// are clamped, so a hostile code can never make the renderer allocate a huge canvas.
pub const MAX_PX: f32 = 200.0;

fn px(v: f32, lo: f32) -> f32 {
    if v.is_nan() { 0f32.clamp(lo, MAX_PX) } else { v.clamp(lo, MAX_PX) }
}

fn unit(v: f32) -> f32 {
    if v.is_nan() { 1.0 } else { v.clamp(0.0, 1.0) }
}

impl Lines {
    fn sanitized(&self) -> Lines {
        Lines {
            enabled: self.enabled,
            length: px(self.length, 0.0),
            vertical_length: self.vertical_length.map(|v| px(v, 0.0)),
            thickness: px(self.thickness, 0.0),
            gap: px(self.gap, -50.0),
            opacity: unit(self.opacity),
        }
    }
}

impl Crosshair {
    /// Same crosshair with every number finite and within sane bounds.
    pub fn sanitized(&self) -> Crosshair {
        Crosshair {
            inner: self.inner.sanitized(),
            outer: self.outer.sanitized(),
            outline: Outline { enabled: self.outline.enabled, thickness: px(self.outline.thickness, 0.0).min(10.0), opacity: unit(self.outline.opacity) },
            dot: Dot { enabled: self.dot.enabled, size: px(self.dot.size, 0.0), opacity: unit(self.dot.opacity) },
            circle: Circle {
                enabled: self.circle.enabled,
                radius: px(self.circle.radius, 0.0),
                thickness: px(self.circle.thickness, 0.0),
                opacity: unit(self.circle.opacity),
            },
            image_scale: if self.image_scale.is_nan() { 1.0 } else { self.image_scale.clamp(0.01, 10.0) },
            image_opacity: unit(self.image_opacity),
            offset_x: self.offset_x.clamp(-500, 500),
            offset_y: self.offset_y.clamp(-500, 500),
            ..self.clone()
        }
    }

    /// Color as RGB; an invalid value falls back to white.
    pub fn rgb(&self) -> (u8, u8, u8) {
        let hex = self.color.trim_start_matches('#');
        if hex.len() == 6
            && let Ok(v) = u32::from_str_radix(hex, 16) {
                return ((v >> 16) as u8, (v >> 8) as u8, v as u8);
            }
        (255, 255, 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_clamps_hostile_values() {
        let mut c = Crosshair::default();
        c.inner.length = f32::INFINITY;
        c.inner.gap = f32::NAN;
        c.inner.thickness = 1e30;
        c.inner.opacity = -3.0;
        c.inner.vertical_length = Some(f32::NEG_INFINITY);
        c.circle.radius = 1e9;
        c.dot.size = -5.0;
        c.image_scale = 1e6;
        c.offset_x = i32::MAX;
        let s = c.sanitized();
        assert_eq!(s.inner.length, MAX_PX);
        assert_eq!(s.inner.gap, 0.0);
        assert_eq!(s.inner.thickness, MAX_PX);
        assert_eq!(s.inner.opacity, 0.0);
        assert_eq!(s.inner.vertical_length, Some(0.0));
        assert_eq!(s.circle.radius, MAX_PX);
        assert_eq!(s.dot.size, 0.0);
        assert!(s.image_scale <= 10.0);
        assert!(s.offset_x <= 500);
    }

    #[test]
    fn color_hex_parses() {
        let mut c = Crosshair { color: "#FF4FD8".into(), ..Default::default() };
        assert_eq!(c.rgb(), (255, 79, 216));
        c.color = "bad".into();
        assert_eq!(c.rgb(), (255, 255, 255));
    }
}
