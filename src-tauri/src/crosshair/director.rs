//! Pure decision of what the overlay shows, plus the geometry helpers. No Win32 here.

use super::model::Crosshair;

/// An enabled game with an assigned crosshair.
#[derive(Clone, Debug, PartialEq)]
pub struct CrosshairRule {
    pub exe_name_lower: String,
    pub crosshair: Crosshair,
}

/// What the overlay must display: whose window (pid), which crosshair, and the data needed
/// to scale it (native size of the game's monitor, anti-distortion setting).
#[derive(Clone, Debug, PartialEq)]
pub struct OverlaySpec {
    pub pid: u32,
    pub crosshair: Crosshair,
    pub native: (u32, u32),
    pub undistort: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayCommand {
    Show(Box<OverlaySpec>),
    Hide,
}

/// Decides show/hide from the foreground window, the per-game rules, the session toggle and
/// FACEIT. Returns a command only when the overlay must change.
pub struct CrosshairDirector {
    rules: Vec<CrosshairRule>,
    undistort: bool,
    hide_with_faceit: bool,
    visible: bool,
    last_fg: Option<(u32, String, (u32, u32))>,
    shown: Option<OverlaySpec>,
}

impl CrosshairDirector {
    pub fn new(rules: Vec<CrosshairRule>, undistort: bool, hide_with_faceit: bool) -> Self {
        CrosshairDirector { rules, undistort, hide_with_faceit, visible: true, last_fg: None, shown: None }
    }

    pub fn on_foreground(&mut self, pid: u32, exe_name: &str, native: (u32, u32), faceit_running: bool) -> Option<OverlayCommand> {
        self.last_fg = Some((pid, exe_name.to_lowercase(), native));
        self.reconcile(faceit_running)
    }

    pub fn on_rules(
        &mut self,
        rules: Vec<CrosshairRule>,
        undistort: bool,
        hide_with_faceit: bool,
        faceit_running: bool,
    ) -> Option<OverlayCommand> {
        self.rules = rules;
        self.undistort = undistort;
        self.hide_with_faceit = hide_with_faceit;
        self.reconcile(faceit_running)
    }

    /// Session show/hide toggle (hotkey).
    pub fn toggle(&mut self, faceit_running: bool) -> Option<OverlayCommand> {
        self.visible = !self.visible;
        self.reconcile(faceit_running)
    }

    fn desired(&self, faceit_running: bool) -> Option<OverlaySpec> {
        if !self.visible || (faceit_running && self.hide_with_faceit) {
            return None;
        }
        let (pid, exe, native) = self.last_fg.as_ref()?;
        if exe.is_empty() {
            return None;
        }
        let rule = self.rules.iter().find(|r| &r.exe_name_lower == exe)?;
        Some(OverlaySpec { pid: *pid, crosshair: rule.crosshair.clone(), native: *native, undistort: self.undistort })
    }

    fn reconcile(&mut self, faceit_running: bool) -> Option<OverlayCommand> {
        let want = self.desired(faceit_running);
        if want == self.shown {
            return None;
        }
        self.shown = want.clone();
        Some(match want {
            Some(spec) => OverlayCommand::Show(Box::new(spec)),
            None => OverlayCommand::Hide,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub scale: f32,
    pub x_factor: f32,
}

/// Render scale (model sizes are px at 1080p) and the horizontal pre-squeeze that keeps the
/// crosshair square once the GPU stretches `mode` to the `native` panel.
pub fn geometry(mode: (u32, u32), native: (u32, u32), undistort: bool) -> Geometry {
    let scale = mode.1 as f32 / 1080.0;
    let x_factor = if undistort && native.0 > 0 && native.1 > 0 && mode.0 > 0 && mode.1 > 0 {
        (mode.0 as f32 / native.0 as f32) / (mode.1 as f32 / native.1 as f32)
    } else {
        1.0
    };
    Geometry { scale, x_factor }
}

/// Center of a client rectangle (screen coordinates), plus the crosshair offset.
pub fn client_center(left: i32, top: i32, right: i32, bottom: i32, offset: (i32, i32)) -> (i32, i32) {
    ((left + right) / 2 + offset.0, (top + bottom) / 2 + offset.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::presets::presets;

    const NATIVE: (u32, u32) = (1920, 1080);

    fn rules() -> Vec<CrosshairRule> {
        vec![CrosshairRule { exe_name_lower: "cs2.exe".into(), crosshair: presets()[0].clone() }]
    }

    fn shown(cmd: Option<OverlayCommand>) -> OverlaySpec {
        match cmd {
            Some(OverlayCommand::Show(spec)) => *spec,
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn shows_for_configured_game() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        let spec = shown(d.on_foreground(10, "CS2.EXE", NATIVE, false));
        assert_eq!(spec.pid, 10);
        assert_eq!(spec.crosshair.name, presets()[0].name);
        assert_eq!(spec.native, NATIVE);
        assert!(spec.undistort);
    }

    #[test]
    fn hides_for_other_apps() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        assert_eq!(d.on_foreground(99, "chrome.exe", NATIVE, false), Some(OverlayCommand::Hide));
        assert_eq!(d.on_foreground(98, "discord.exe", NATIVE, false), None, "no redundant Hide");
    }

    #[test]
    fn same_game_again_sends_nothing() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        assert_eq!(d.on_foreground(10, "cs2.exe", NATIVE, false), None);
    }

    #[test]
    fn toggle_hides_and_shows_back() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        assert_eq!(d.toggle(false), Some(OverlayCommand::Hide));
        shown(d.toggle(false));
    }

    #[test]
    fn toggle_off_stays_hidden_on_next_game_focus() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.toggle(false); // off before any game
        assert_eq!(d.on_foreground(10, "cs2.exe", NATIVE, false), None);
    }

    #[test]
    fn faceit_hides_when_enabled() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        assert_eq!(d.on_foreground(10, "cs2.exe", NATIVE, true), None);
        let mut lax = CrosshairDirector::new(rules(), true, false);
        shown(lax.on_foreground(10, "cs2.exe", NATIVE, true));
    }

    #[test]
    fn rules_change_updates_shown_crosshair() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        let new_rules = vec![CrosshairRule { exe_name_lower: "cs2.exe".into(), crosshair: presets()[2].clone() }];
        assert_eq!(shown(d.on_rules(new_rules, true, true, false)).crosshair.name, presets()[2].name);
    }

    #[test]
    fn rules_change_removing_crosshair_hides() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        assert_eq!(d.on_rules(Vec::new(), true, true, false), Some(OverlayCommand::Hide));
    }

    #[test]
    fn rules_change_while_in_other_app_sends_nothing() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(99, "chrome.exe", NATIVE, false);
        assert_eq!(d.on_rules(rules(), false, true, false), None);
    }

    #[test]
    fn empty_exe_hides() {
        let mut d = CrosshairDirector::new(rules(), true, true);
        d.on_foreground(10, "cs2.exe", NATIVE, false);
        assert_eq!(d.on_foreground(11, "", NATIVE, false), Some(OverlayCommand::Hide));
    }

    #[test]
    fn geometry_stretched_undistorts() {
        let g = geometry((1440, 1080), (1920, 1080), true);
        assert!((g.scale - 1.0).abs() < 1e-4 && (g.x_factor - 0.75).abs() < 1e-4);
        let g = geometry((1280, 960), (1920, 1080), true);
        assert!((g.scale - 0.8889).abs() < 1e-3 && (g.x_factor - 0.75).abs() < 1e-4);
        let g = geometry((1440, 1080), (1920, 1080), false);
        assert!((g.x_factor - 1.0).abs() < 1e-6);
        let g = geometry((1920, 1080), (1920, 1080), true);
        assert!((g.x_factor - 1.0).abs() < 1e-6);
    }

    #[test]
    fn geometry_centers_on_client_rect() {
        assert_eq!(client_center(0, 0, 1440, 1080, (0, 0)), (720, 540));
        assert_eq!(client_center(0, 0, 1440, 1080, (2, -3)), (722, 537));
        assert_eq!(client_center(1920, 0, 3840, 1080, (0, 0)), (2880, 540));
    }
}
