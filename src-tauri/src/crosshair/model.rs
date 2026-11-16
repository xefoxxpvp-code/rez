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

impl Crosshair {
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
    fn color_hex_parses() {
        let mut c = Crosshair { color: "#FF4FD8".into(), ..Default::default() };
        assert_eq!(c.rgb(), (255, 79, 216));
        c.color = "bad".into();
        assert_eq!(c.rgb(), (255, 255, 255));
    }
}
