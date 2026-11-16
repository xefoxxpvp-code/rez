//! Display mode model and the platform-neutral display API.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub device: String,
    pub label: String,
    pub primary: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("mode refusé par Windows (code {0})")]
    Rejected(i32),
    #[error("{0}")]
    Win32(String),
}

/// Abstraction over the OS display settings, so the engine/runtime can be tested with a mock.
pub trait DisplayApi: Send + Sync {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, DisplayError>;
    fn list_modes(&self, device: &str) -> Result<Vec<DisplayMode>, DisplayError>;
    fn registry_mode(&self, device: &str) -> Result<DisplayMode, DisplayError>;
    fn current_mode(&self, device: &str) -> Result<DisplayMode, DisplayError>;
    /// Applies a mode that is never persisted to the registry (`CDS_FULLSCREEN`).
    fn apply_temporary(&self, device: &str, mode: DisplayMode) -> Result<(), DisplayError>;
    /// Returns the device to its registry (native) mode.
    fn restore(&self, device: &str) -> Result<(), DisplayError>;
}

fn is_4_3(width: u32, height: u32) -> bool {
    width * 3 == height * 4
}

/// Picks the mode for `width`x`height`, preferring the native refresh rate,
/// otherwise the highest refresh rate available at that resolution.
pub fn pick_mode(modes: &[DisplayMode], width: u32, height: u32, native_hz: u32) -> Option<DisplayMode> {
    let candidates = modes.iter().filter(|m| m.width == width && m.height == height);
    candidates
        .clone()
        .find(|m| m.refresh_hz == native_hz)
        .or_else(|| candidates.max_by_key(|m| m.refresh_hz))
        .copied()
}

/// Default resolution for a newly added game: 1440x1080 if offered, else the largest 4:3, else native.
pub fn default_resolution(modes: &[DisplayMode], native: DisplayMode) -> (u32, u32) {
    if modes.iter().any(|m| m.width == 1440 && m.height == 1080) {
        return (1440, 1080);
    }
    modes
        .iter()
        .filter(|m| is_4_3(m.width, m.height))
        .max_by_key(|m| (m.width, m.height))
        .map(|m| (m.width, m.height))
        .unwrap_or((native.width, native.height))
}

/// Unique resolutions, sorted by width then height, descending.
pub fn distinct_resolutions(modes: &[DisplayMode]) -> Vec<(u32, u32)> {
    let mut res: Vec<(u32, u32)> = modes.iter().map(|m| (m.width, m.height)).collect();
    res.sort_unstable_by(|a, b| b.cmp(a));
    res.dedup();
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(w: u32, h: u32, hz: u32) -> DisplayMode {
        DisplayMode { width: w, height: h, refresh_hz: hz }
    }

    #[test]
    fn pick_mode_keeps_native_refresh() {
        let modes = [m(1920, 1080, 240), m(1440, 1080, 60), m(1440, 1080, 240)];
        assert_eq!(pick_mode(&modes, 1440, 1080, 240), Some(m(1440, 1080, 240)));
    }

    #[test]
    fn pick_mode_falls_back_to_highest_refresh() {
        let modes = [m(1440, 1080, 60), m(1440, 1080, 120)];
        assert_eq!(pick_mode(&modes, 1440, 1080, 240), Some(m(1440, 1080, 120)));
    }

    #[test]
    fn pick_mode_none_when_resolution_missing() {
        assert_eq!(pick_mode(&[m(1920, 1080, 144)], 1440, 1080, 144), None);
    }

    #[test]
    fn default_resolution_prefers_1440x1080() {
        let modes = [m(1920, 1080, 144), m(1440, 1080, 144), m(1280, 960, 144)];
        assert_eq!(default_resolution(&modes, m(1920, 1080, 144)), (1440, 1080));
    }

    #[test]
    fn default_resolution_largest_4_3_otherwise() {
        let modes = [m(1920, 1080, 144), m(1280, 960, 144), m(1024, 768, 144)];
        assert_eq!(default_resolution(&modes, m(1920, 1080, 144)), (1280, 960));
    }

    #[test]
    fn default_resolution_native_when_no_4_3() {
        assert_eq!(default_resolution(&[m(1920, 1080, 144)], m(1920, 1080, 144)), (1920, 1080));
    }

    #[test]
    fn distinct_resolutions_dedups_and_sorts() {
        let modes = [m(1280, 960, 60), m(1920, 1080, 60), m(1280, 960, 144)];
        assert_eq!(distinct_resolutions(&modes), vec![(1920, 1080), (1280, 960)]);
    }
}
