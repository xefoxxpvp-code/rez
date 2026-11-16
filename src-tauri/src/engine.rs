//! Pure foreground/display state machine (no Win32).
//!
//! The runtime feeds it [`Event`]s with an explicit `now`, and executes the returned
//! [`Action`]s. Time-based behavior (the leave debounce) is driven by [`Engine::tick`]
//! and [`Engine::next_deadline`], so everything here is deterministic and testable.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::display::DisplayMode;

pub const LEAVE_DEBOUNCE: Duration = Duration::from_millis(250);

/// An enabled game the engine watches for.
#[derive(Clone, Debug, PartialEq)]
pub struct GameRule {
    pub id: String,
    pub exe_name_lower: String,
    pub width: u32,
    pub height: u32,
    pub keep_stretched: bool,
}

pub enum Event {
    Foreground { pid: u32, exe_name: String, monitor: String, modes: Vec<DisplayMode>, native: DisplayMode },
    ProcessExited { pid: u32 },
    DisplayChanged { monitor: String, current: DisplayMode },
    RestoreRequested,
    Shutdown,
    RulesChanged(Vec<GameRule>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SaveDesktopLayout { monitor: String },
    Apply { monitor: String, mode: DisplayMode },
    Restore { monitor: String },
    RestoreDesktopLayout { monitor: String },
    WatchProcess { pid: u32 },
    Status(StatusEvent),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StatusEvent {
    Idle,
    Active { game_id: String, width: u32, height: u32 },
    ModeUnavailable { game_id: String, width: u32, height: u32 },
    Error { message: String },
}

struct ActiveState {
    game_id: String,
    pid: u32,
    monitor: String,
    mode: DisplayMode,
    keep_stretched: bool,
    leave_deadline: Option<Instant>,
    /// Modes and native mode of `monitor`, kept so a rules change can re-pick a mode.
    modes: Vec<DisplayMode>,
    native: DisplayMode,
}

pub struct Engine {
    rules: Vec<GameRule>,
    active: Option<ActiveState>,
}

impl Engine {
    pub fn new(rules: Vec<GameRule>) -> Self {
        Engine { rules, active: None }
    }

    pub fn active_monitor(&self) -> Option<&str> {
        self.active.as_ref().map(|a| a.monitor.as_str())
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.active.as_ref().and_then(|a| a.leave_deadline)
    }

    pub fn status(&self) -> StatusEvent {
        match &self.active {
            Some(a) => active_status(a),
            None => StatusEvent::Idle,
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Action> {
        match self.active.as_ref().and_then(|a| a.leave_deadline) {
            Some(deadline) if now >= deadline => self.exit(),
            _ => Vec::new(),
        }
    }

    pub fn handle(&mut self, event: Event, now: Instant) -> Vec<Action> {
        match event {
            Event::Foreground { pid, exe_name, monitor, modes, native } => {
                self.on_foreground(pid, &exe_name, monitor, modes, native, now)
            }
            Event::ProcessExited { pid } => match &self.active {
                Some(a) if a.pid == pid => self.exit(),
                _ => Vec::new(),
            },
            Event::DisplayChanged { monitor, current } => match &self.active {
                Some(a) if a.monitor == monitor && a.leave_deadline.is_none() && current != a.mode => {
                    vec![Action::Apply { monitor, mode: a.mode }]
                }
                _ => Vec::new(),
            },
            Event::RestoreRequested | Event::Shutdown => self.exit(),
            Event::RulesChanged(rules) => {
                self.rules = rules;
                self.on_rules_changed()
            }
        }
    }

    fn on_foreground(
        &mut self,
        pid: u32,
        exe_name: &str,
        monitor: String,
        modes: Vec<DisplayMode>,
        native: DisplayMode,
        now: Instant,
    ) -> Vec<Action> {
        let exe_lower = exe_name.to_lowercase();
        let rule = if exe_lower.is_empty() {
            None
        } else {
            self.rules.iter().find(|r| r.exe_name_lower == exe_lower).cloned()
        };
        let Some(rule) = rule else {
            self.start_leave(now);
            return Vec::new();
        };

        // Back to the game that is already applied on this monitor: cancel any pending leave.
        if let Some(a) = self.active.as_mut()
            && a.game_id == rule.id && a.monitor == monitor {
                a.leave_deadline = None;
                if a.pid != pid {
                    a.pid = pid;
                    return vec![Action::WatchProcess { pid }];
                }
                return Vec::new();
            }

        let Some(mode) = crate::display::pick_mode(&modes, rule.width, rule.height, native.refresh_hz) else {
            self.start_leave(now);
            return vec![Action::Status(StatusEvent::ModeUnavailable {
                game_id: rule.id,
                width: rule.width,
                height: rule.height,
            })];
        };

        let mut actions = Vec::new();
        match self.active.take() {
            Some(prev) if prev.monitor == monitor => {}
            Some(prev) => {
                actions.push(Action::Restore { monitor: prev.monitor.clone() });
                actions.push(Action::RestoreDesktopLayout { monitor: prev.monitor });
                actions.push(Action::SaveDesktopLayout { monitor: monitor.clone() });
            }
            None => actions.push(Action::SaveDesktopLayout { monitor: monitor.clone() }),
        }
        actions.push(Action::Apply { monitor: monitor.clone(), mode });
        actions.push(Action::WatchProcess { pid });
        let state = ActiveState {
            game_id: rule.id,
            pid,
            monitor,
            mode,
            keep_stretched: rule.keep_stretched,
            leave_deadline: None,
            modes,
            native,
        };
        actions.push(Action::Status(active_status(&state)));
        self.active = Some(state);
        actions
    }

    fn start_leave(&mut self, now: Instant) {
        if let Some(a) = self.active.as_mut()
            && !a.keep_stretched && a.leave_deadline.is_none() {
                a.leave_deadline = Some(now + LEAVE_DEBOUNCE);
            }
    }

    fn on_rules_changed(&mut self) -> Vec<Action> {
        let Some(a) = self.active.as_mut() else { return Vec::new() };
        let Some(rule) = self.rules.iter().find(|r| r.id == a.game_id).cloned() else {
            return self.exit();
        };
        a.keep_stretched = rule.keep_stretched;
        if (rule.width, rule.height) == (a.mode.width, a.mode.height) {
            return Vec::new();
        }
        match crate::display::pick_mode(&a.modes, rule.width, rule.height, a.native.refresh_hz) {
            Some(mode) => {
                a.mode = mode;
                let monitor = a.monitor.clone();
                let status = active_status(a);
                vec![Action::Apply { monitor, mode }, Action::Status(status)]
            }
            None => {
                let mut actions = self.exit();
                actions.push(Action::Status(StatusEvent::ModeUnavailable {
                    game_id: rule.id,
                    width: rule.width,
                    height: rule.height,
                }));
                actions
            }
        }
    }

    fn exit(&mut self) -> Vec<Action> {
        match self.active.take() {
            Some(a) => vec![
                Action::Restore { monitor: a.monitor.clone() },
                Action::RestoreDesktopLayout { monitor: a.monitor },
                Action::Status(StatusEvent::Idle),
            ],
            None => Vec::new(),
        }
    }
}

fn active_status(a: &ActiveState) -> StatusEvent {
    StatusEvent::Active { game_id: a.game_id.clone(), width: a.mode.width, height: a.mode.height }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Action::*;

    const D1: &str = r"\\.\DISPLAY1";
    const D2: &str = r"\\.\DISPLAY2";
    const VAL: &str = "valorant-win64-shipping.exe";

    fn m(w: u32, h: u32, hz: u32) -> DisplayMode {
        DisplayMode { width: w, height: h, refresh_hz: hz }
    }
    fn native() -> DisplayMode {
        m(1920, 1080, 240)
    }
    fn modes() -> Vec<DisplayMode> {
        vec![m(1920, 1080, 240), m(1440, 1080, 240), m(1280, 960, 240)]
    }
    fn rule(id: &str, exe: &str, w: u32, h: u32, keep: bool) -> GameRule {
        GameRule { id: id.into(), exe_name_lower: exe.into(), width: w, height: h, keep_stretched: keep }
    }
    fn rules() -> Vec<GameRule> {
        vec![rule("valorant", VAL, 1440, 1080, false), rule("cs2", "cs2.exe", 1280, 960, false)]
    }
    fn fg(pid: u32, exe: &str, monitor: &str) -> Event {
        Event::Foreground { pid, exe_name: exe.into(), monitor: monitor.into(), modes: modes(), native: native() }
    }
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn exit(monitor: &str) -> Vec<Action> {
        vec![
            Restore { monitor: monitor.into() },
            RestoreDesktopLayout { monitor: monitor.into() },
            Status(StatusEvent::Idle),
        ]
    }
    fn active_valorant(t0: Instant) -> Engine {
        let mut e = Engine::new(rules());
        e.handle(fg(10, VAL, D1), t0);
        e
    }

    #[test]
    fn enter_game_applies_mode_with_native_refresh() {
        let mut e = Engine::new(rules());
        let a = e.handle(fg(10, VAL, D1), Instant::now());
        assert_eq!(
            a,
            vec![
                SaveDesktopLayout { monitor: D1.into() },
                Apply { monitor: D1.into(), mode: m(1440, 1080, 240) },
                WatchProcess { pid: 10 },
                Status(StatusEvent::Active { game_id: "valorant".into(), width: 1440, height: 1080 }),
            ]
        );
        assert_eq!(e.active_monitor(), Some(D1));
    }

    #[test]
    fn exe_match_is_case_insensitive() {
        let mut e = Engine::new(rules());
        let a = e.handle(fg(10, "VALORANT-Win64-Shipping.exe", D1), Instant::now());
        assert!(a.contains(&Apply { monitor: D1.into(), mode: m(1440, 1080, 240) }));
    }

    #[test]
    fn leave_restores_after_debounce() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        let t1 = t0 + ms(1000);
        assert_eq!(e.handle(fg(99, "chrome.exe", D1), t1), vec![]);
        assert_eq!(e.next_deadline(), Some(t1 + LEAVE_DEBOUNCE));
        assert_eq!(e.tick(t1 + ms(249)), vec![]);
        assert_eq!(e.tick(t1 + ms(250)), exit(D1));
        assert_eq!(e.active_monitor(), None);
        assert_eq!(e.next_deadline(), None);
    }

    #[test]
    fn quick_return_cancels_restore() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        let t1 = t0 + ms(1000);
        e.handle(fg(99, "chrome.exe", D1), t1);
        assert_eq!(e.handle(fg(10, VAL, D1), t1 + ms(100)), vec![]);
        assert_eq!(e.tick(t1 + ms(300)), vec![]);
        assert_eq!(e.next_deadline(), None);
    }

    #[test]
    fn switch_game_to_game_same_monitor_applies_directly() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(
            e.handle(fg(20, "cs2.exe", D1), t0 + ms(10)),
            vec![
                Apply { monitor: D1.into(), mode: m(1280, 960, 240) },
                WatchProcess { pid: 20 },
                Status(StatusEvent::Active { game_id: "cs2".into(), width: 1280, height: 960 }),
            ]
        );
    }

    #[test]
    fn switch_across_monitors_restores_first() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(
            e.handle(fg(20, "cs2.exe", D2), t0 + ms(10)),
            vec![
                Restore { monitor: D1.into() },
                RestoreDesktopLayout { monitor: D1.into() },
                SaveDesktopLayout { monitor: D2.into() },
                Apply { monitor: D2.into(), mode: m(1280, 960, 240) },
                WatchProcess { pid: 20 },
                Status(StatusEvent::Active { game_id: "cs2".into(), width: 1280, height: 960 }),
            ]
        );
        assert_eq!(e.active_monitor(), Some(D2));
    }

    #[test]
    fn mode_unavailable_reports_and_stays_idle() {
        let mut e = Engine::new(rules());
        let ev = Event::Foreground {
            pid: 10,
            exe_name: VAL.into(),
            monitor: D1.into(),
            modes: vec![m(1920, 1080, 240), m(1280, 960, 240)],
            native: native(),
        };
        assert_eq!(
            e.handle(ev, Instant::now()),
            vec![Status(StatusEvent::ModeUnavailable { game_id: "valorant".into(), width: 1440, height: 1080 })]
        );
        assert_eq!(e.active_monitor(), None);
    }

    #[test]
    fn keep_stretched_ignores_leave_and_restores_on_exit() {
        let t0 = Instant::now();
        let mut e = Engine::new(vec![rule("valorant", VAL, 1440, 1080, true)]);
        e.handle(fg(10, VAL, D1), t0);
        assert_eq!(e.handle(fg(99, "chrome.exe", D1), t0 + ms(10)), vec![]);
        assert_eq!(e.tick(t0 + ms(1010)), vec![]);
        assert_eq!(e.handle(Event::ProcessExited { pid: 10 }, t0 + ms(2000)), exit(D1));
    }

    #[test]
    fn process_exit_restores_immediately() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(e.handle(Event::ProcessExited { pid: 10 }, t0 + ms(10)), exit(D1));
    }

    #[test]
    fn exit_of_other_pid_is_ignored() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(e.handle(Event::ProcessExited { pid: 777 }, t0 + ms(10)), vec![]);
        assert_eq!(e.active_monitor(), Some(D1));
    }

    #[test]
    fn display_changed_while_active_reapplies() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(
            e.handle(Event::DisplayChanged { monitor: D1.into(), current: native() }, t0 + ms(10)),
            vec![Apply { monitor: D1.into(), mode: m(1440, 1080, 240) }]
        );
        assert_eq!(
            e.handle(Event::DisplayChanged { monitor: D1.into(), current: m(1440, 1080, 240) }, t0 + ms(20)),
            vec![]
        );
    }

    #[test]
    fn display_changed_while_idle_or_leaving_is_ignored() {
        let t0 = Instant::now();
        let mut idle = Engine::new(rules());
        assert_eq!(idle.handle(Event::DisplayChanged { monitor: D1.into(), current: native() }, t0), vec![]);
        let mut e = active_valorant(t0);
        e.handle(fg(99, "chrome.exe", D1), t0 + ms(10));
        assert_eq!(e.handle(Event::DisplayChanged { monitor: D1.into(), current: native() }, t0 + ms(20)), vec![]);
    }

    #[test]
    fn restore_requested_goes_idle() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(e.handle(Event::RestoreRequested, t0 + ms(10)), exit(D1));
        assert_eq!(e.handle(Event::RestoreRequested, t0 + ms(20)), vec![]);
    }

    #[test]
    fn shutdown_restores_when_active() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        assert_eq!(e.handle(Event::Shutdown, t0 + ms(10)), exit(D1));
        let mut idle = Engine::new(rules());
        assert_eq!(idle.handle(Event::Shutdown, t0), vec![]);
    }

    #[test]
    fn rules_changed_removing_active_game_restores() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        let only_cs2 = vec![rule("cs2", "cs2.exe", 1280, 960, false)];
        assert_eq!(e.handle(Event::RulesChanged(only_cs2), t0 + ms(10)), exit(D1));
    }

    #[test]
    fn rules_changed_new_resolution_reapplies() {
        let t0 = Instant::now();
        let mut e = active_valorant(t0);
        let new_rules = vec![rule("valorant", VAL, 1280, 960, false), rule("cs2", "cs2.exe", 1280, 960, false)];
        assert_eq!(
            e.handle(Event::RulesChanged(new_rules), t0 + ms(10)),
            vec![
                Apply { monitor: D1.into(), mode: m(1280, 960, 240) },
                Status(StatusEvent::Active { game_id: "valorant".into(), width: 1280, height: 960 }),
            ]
        );
        assert_eq!(e.status(), StatusEvent::Active { game_id: "valorant".into(), width: 1280, height: 960 });
    }
}
