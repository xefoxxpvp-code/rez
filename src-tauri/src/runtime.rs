//! Runtime: feeds OS inputs to the [`Engine`] and executes its actions through
//! [`DisplayApi`] and [`LayoutApi`], on a dedicated thread.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::display::{DisplayApi, DisplayMode};
use crate::engine::{Action, Engine, Event, GameRule, StatusEvent};

/// Inputs coming from Win32 event threads, Tauri commands and the tray.
#[derive(Debug)]
pub enum Input {
    Foreground { pid: u32, exe_name: String, monitor: String },
    ProcessExited { pid: u32 },
    DisplayMaybeChanged,
    RestoreRequested,
    RulesChanged(Vec<GameRule>),
    Shutdown { ack: std::sync::mpsc::Sender<()> },
}

/// How long after a mode change the runtime keeps checking that no monitor left the desktop.
pub const TOPOLOGY_GUARD: Duration = Duration::from_secs(5);

const TOPOLOGY_LOST_MESSAGE: &str = "Un écran s'est déconnecté pendant le changement de résolution. REZ a remis ta configuration d'écrans et a mis les bascules en pause : modifie un réglage d'un jeu pour les réactiver.";

/// Shows/hides the crosshair overlay (Win32 window in production, a recorder in tests).
pub trait OverlayApi: Send {
    fn apply(&mut self, cmd: crate::crosshair::director::OverlayCommand);
}

/// Saves and restores desktop icon positions and window placements around a mode switch.
pub trait LayoutApi: Send {
    fn capture(&mut self, monitor: &str);
    /// May complete asynchronously (Explorer settles after a mode change).
    fn restore(&mut self, monitor: &str);
    /// Completes every pending restore synchronously (app shutdown).
    fn flush(&mut self) {}
}

pub struct Runtime {
    engine: Engine,
    display: Arc<dyn DisplayApi>,
    layout: Box<dyn LayoutApi>,
    watch_process: Box<dyn Fn(u32) + Send>,
    on_status: Box<dyn Fn(StatusEvent) + Send>,
    /// Per-monitor (modes, registry mode), read once per session.
    modes_cache: HashMap<String, (Vec<DisplayMode>, DisplayMode)>,
    watched: HashSet<u32>,
    stopped: bool,
    /// Devices attached to the desktop before the first mode change of a session.
    guard_devices: Vec<String>,
    /// Final topology check time, armed after each Apply/Restore.
    guard_until: Option<Instant>,
    /// Set after a topology incident; cleared by the next rules change.
    suspended: bool,
    /// Every monitor REZ applied a mode to this session (emergency restore targets).
    touched: HashSet<String>,
}

impl Runtime {
    pub fn new(
        rules: Vec<GameRule>,
        display: Arc<dyn DisplayApi>,
        layout: Box<dyn LayoutApi>,
        watch_process: Box<dyn Fn(u32) + Send>,
        on_status: Box<dyn Fn(StatusEvent) + Send>,
    ) -> Self {
        Runtime {
            engine: Engine::new(rules),
            display,
            layout,
            watch_process,
            on_status,
            modes_cache: HashMap::new(),
            watched: HashSet::new(),
            stopped: false,
            guard_devices: Vec::new(),
            guard_until: None,
            suspended: false,
            touched: HashSet::new(),
        }
    }

    pub fn handle_input(&mut self, input: Input, now: Instant) {
        let mut ack = None;
        let event = match input {
            Input::Foreground { .. } if self.suspended => return,
            Input::Foreground { pid, exe_name, monitor } => {
                let (modes, native) = if exe_name.is_empty() {
                    (Vec::new(), DisplayMode { width: 0, height: 0, refresh_hz: 0 })
                } else {
                    self.modes_for(&monitor)
                };
                Event::Foreground { pid, exe_name, monitor, modes, native }
            }
            Input::ProcessExited { pid } => {
                self.watched.remove(&pid);
                Event::ProcessExited { pid }
            }
            Input::DisplayMaybeChanged => {
                if self.check_topology(now) {
                    return;
                }
                let Some(monitor) = self.engine.active_monitor().map(str::to_string) else {
                    // The user may have changed resolution/refresh in Windows: re-read modes next time.
                    self.modes_cache.clear();
                    return;
                };
                match self.display.current_mode(&monitor) {
                    Ok(current) => Event::DisplayChanged { monitor, current },
                    Err(e) => {
                        log::warn!("current_mode({monitor}) failed: {e}");
                        return;
                    }
                }
            }
            Input::RestoreRequested => Event::RestoreRequested,
            Input::RulesChanged(rules) => {
                self.suspended = false;
                Event::RulesChanged(rules)
            }
            Input::Shutdown { ack: a } => {
                ack = Some(a);
                self.stopped = true;
                Event::Shutdown
            }
        };
        let emergency = matches!(event, Event::RestoreRequested);
        let actions = self.engine.handle(event, now);
        let restored = self.execute(actions, now);
        if emergency {
            // Whatever the engine believes, put back every monitor REZ touched.
            let pending: Vec<String> = self.touched.iter().filter(|m| !restored.contains(*m)).cloned().collect();
            for monitor in pending {
                if let Err(e) = self.display.restore(&monitor) {
                    let message = format!("Impossible de revenir en résolution native : {e}");
                    log::warn!("{message}");
                    (self.on_status)(StatusEvent::Error { message });
                }
            }
        }
        if let Some(ack) = ack {
            self.layout.flush();
            let _ = ack.send(());
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let actions = self.engine.tick(now);
        let _ = self.execute(actions, now);
        if self.guard_until.is_some_and(|t| now >= t)
            && !self.check_topology(now) {
                self.guard_until = None;
                if self.engine.active_monitor().is_none() {
                    self.guard_devices.clear();
                }
            }
    }

    /// Earliest of the engine's leave deadline and the topology guard deadline.
    pub fn next_deadline(&self) -> Option<Instant> {
        match (self.engine.next_deadline(), self.guard_until) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Returns true when a monitor left the desktop since the session started; in that case
    /// restores native modes and the saved topology, goes idle and suspends switching.
    fn check_topology(&mut self, now: Instant) -> bool {
        // Only right after REZ's own mode changes: later changes are the user's business.
        if self.guard_devices.is_empty() || self.guard_until.is_none() {
            return false;
        }
        let current: Vec<String> = match self.display.list_monitors() {
            Ok(monitors) => monitors.into_iter().map(|m| m.device).collect(),
            Err(e) => {
                log::warn!("topology check failed: {e}");
                return false;
            }
        };
        let missing: Vec<&String> = self.guard_devices.iter().filter(|d| !current.contains(d)).collect();
        if missing.is_empty() {
            return false;
        }
        log::warn!("monitor(s) left the desktop after a mode change: {missing:?}");
        for device in current.iter().filter(|d| self.touched.contains(*d)) {
            if let Err(e) = self.display.restore(device) {
                log::warn!("restore {device} failed: {e}");
            }
        }
        if let Err(e) = self.display.restore_topology() {
            log::warn!("restore_topology failed: {e}");
        }
        let actions = self.engine.handle(Event::RestoreRequested, now);
        let _ = self.execute(actions, now);
        self.guard_devices.clear();
        self.guard_until = None;
        self.suspended = true;
        (self.on_status)(StatusEvent::Error { message: TOPOLOGY_LOST_MESSAGE.to_string() });
        true
    }

    /// Runs the runtime on its own thread until `Shutdown` or until every sender is dropped.
    pub fn spawn(mut self, rx: Receiver<Input>) -> JoinHandle<()> {
        std::thread::Builder::new()
            .name("rez-runtime".into())
            .spawn(move || {
                while !self.stopped {
                    let received = match self.next_deadline() {
                        Some(deadline) => rx.recv_timeout(deadline.saturating_duration_since(Instant::now())),
                        None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                    };
                    match received {
                        Ok(input) => self.handle_input(input, Instant::now()),
                        Err(RecvTimeoutError::Timeout) => self.tick(Instant::now()),
                        Err(RecvTimeoutError::Disconnected) => {
                            let now = Instant::now();
                            let actions = self.engine.handle(Event::Shutdown, now);
                            let _ = self.execute(actions, now);
                            break;
                        }
                    }
                }
            })
            .expect("spawn runtime thread")
    }

    fn modes_for(&mut self, monitor: &str) -> (Vec<DisplayMode>, DisplayMode) {
        if let Some(cached) = self.modes_cache.get(monitor) {
            return cached.clone();
        }
        let modes = self.display.list_modes(monitor);
        let native = self.display.registry_mode(monitor);
        match (modes, native) {
            (Ok(modes), Ok(native)) => {
                self.modes_cache.insert(monitor.to_string(), (modes.clone(), native));
                (modes, native)
            }
            (modes, native) => {
                log::warn!("cannot read modes of {monitor}: {:?} / {:?}", modes.err(), native.err());
                (Vec::new(), DisplayMode { width: 0, height: 0, refresh_hz: 0 })
            }
        }
    }

    /// Runs the actions; returns the monitors restored. A failed Apply/Restore replaces the
    /// status that follows it in the batch, so the UI never claims a state the screen is not in.
    fn execute(&mut self, actions: Vec<Action>, now: Instant) -> Vec<String> {
        let mut restored = Vec::new();
        let mut failure: Option<String> = None;
        for action in actions {
            match action {
                Action::SaveDesktopLayout { monitor } => {
                    if self.guard_devices.is_empty() {
                        if let Err(e) = self.display.save_topology() {
                            log::warn!("save_topology failed: {e}");
                        }
                        self.guard_devices = match self.display.list_monitors() {
                            Ok(monitors) => monitors.into_iter().map(|m| m.device).collect(),
                            Err(_) => Vec::new(),
                        };
                    }
                    self.layout.capture(&monitor);
                }
                Action::RestoreDesktopLayout { monitor } => self.layout.restore(&monitor),
                Action::Apply { monitor, mode } => {
                    self.guard_until = Some(now + TOPOLOGY_GUARD);
                    self.touched.insert(monitor.clone());
                    if let Err(e) = self.display.apply_temporary(&monitor, mode) {
                        let message = format!("Impossible de passer en {} × {} : {e}", mode.width, mode.height);
                        log::warn!("{message}");
                        failure = Some(message);
                    }
                }
                Action::Restore { monitor } => {
                    self.guard_until = Some(now + TOPOLOGY_GUARD);
                    if let Err(e) = self.display.restore(&monitor) {
                        let message = format!("Impossible de revenir en résolution native : {e}");
                        log::warn!("{message}");
                        failure = Some(message);
                    }
                    restored.push(monitor);
                }
                Action::WatchProcess { pid } => {
                    if self.watched.insert(pid) {
                        (self.watch_process)(pid);
                    }
                }
                Action::Status(status) => match failure.take() {
                    Some(message) => (self.on_status)(StatusEvent::Error { message }),
                    None => (self.on_status)(status),
                },
            }
        }
        if let Some(message) = failure {
            (self.on_status)(StatusEvent::Error { message });
        }
        restored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{DisplayError, MonitorInfo};
    use std::sync::Mutex;
    use std::time::Duration;

    const D1: &str = r"\\.\DISPLAY1";

    fn m(w: u32, h: u32, hz: u32) -> DisplayMode {
        DisplayMode { width: w, height: h, refresh_hz: hz }
    }

    const D2: &str = r"\\.\DISPLAY2";

    #[derive(Default)]
    struct MockDisplay {
        calls: Arc<Mutex<Vec<String>>>,
        current: Mutex<Option<DisplayMode>>,
        fail_apply: bool,
        fail_restore: Mutex<bool>,
        /// Overrides the registry (native) mode; default 1920x1080@240.
        registry: Mutex<Option<DisplayMode>>,
        /// Attached devices; tests remove one to simulate a monitor leaving the desktop.
        devices: Mutex<Vec<String>>,
    }
    impl DisplayApi for MockDisplay {
        fn list_monitors(&self) -> Result<Vec<MonitorInfo>, DisplayError> {
            Ok(self
                .devices
                .lock()
                .unwrap()
                .iter()
                .map(|d| MonitorInfo { device: d.clone(), label: "Mock".into(), primary: d == D1 })
                .collect())
        }
        fn save_topology(&self) -> Result<(), DisplayError> {
            self.calls.lock().unwrap().push("save_topology".into());
            Ok(())
        }
        fn restore_topology(&self) -> Result<(), DisplayError> {
            self.calls.lock().unwrap().push("restore_topology".into());
            Ok(())
        }
        fn list_modes(&self, _: &str) -> Result<Vec<DisplayMode>, DisplayError> {
            Ok(vec![m(1920, 1080, 240), m(1920, 1080, 144), m(1440, 1080, 240), m(1280, 960, 240), m(1280, 960, 144)])
        }
        fn registry_mode(&self, _: &str) -> Result<DisplayMode, DisplayError> {
            Ok(self.registry.lock().unwrap().unwrap_or(m(1920, 1080, 240)))
        }
        fn current_mode(&self, _: &str) -> Result<DisplayMode, DisplayError> {
            Ok(self.current.lock().unwrap().unwrap_or(m(1920, 1080, 240)))
        }
        fn apply_temporary(&self, device: &str, mode: DisplayMode) -> Result<(), DisplayError> {
            self.calls.lock().unwrap().push(format!("apply {device} {}x{}@{}", mode.width, mode.height, mode.refresh_hz));
            if self.fail_apply {
                return Err(DisplayError::Rejected(-2));
            }
            *self.current.lock().unwrap() = Some(mode);
            Ok(())
        }
        fn restore(&self, device: &str) -> Result<(), DisplayError> {
            self.calls.lock().unwrap().push(format!("restore {device}"));
            if *self.fail_restore.lock().unwrap() {
                return Err(DisplayError::Rejected(-1));
            }
            *self.current.lock().unwrap() = None;
            Ok(())
        }
    }

    struct MockLayout(Arc<Mutex<Vec<String>>>);
    impl LayoutApi for MockLayout {
        fn capture(&mut self, monitor: &str) {
            self.0.lock().unwrap().push(format!("capture {monitor}"));
        }
        fn restore(&mut self, monitor: &str) {
            self.0.lock().unwrap().push(format!("restore {monitor}"));
        }
        fn flush(&mut self) {
            self.0.lock().unwrap().push("flush".into());
        }
    }

    struct Harness {
        rt: Runtime,
        display: Arc<MockDisplay>,
        layout: Arc<Mutex<Vec<String>>>,
        watched: Arc<Mutex<Vec<u32>>>,
        statuses: Arc<Mutex<Vec<StatusEvent>>>,
    }

    fn harness(fail_apply: bool) -> Harness {
        let display = Arc::new(MockDisplay {
            fail_apply,
            devices: Mutex::new(vec![D1.into(), D2.into()]),
            ..Default::default()
        });
        let layout = Arc::new(Mutex::new(Vec::new()));
        let watched = Arc::new(Mutex::new(Vec::new()));
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let rules = vec![GameRule { id: "cs2".into(), exe_name_lower: "cs2.exe".into(), width: 1280, height: 960, keep_stretched: false }];
        let (w, s) = (watched.clone(), statuses.clone());
        let rt = Runtime::new(
            rules,
            display.clone(),
            Box::new(MockLayout(layout.clone())),
            Box::new(move |pid| w.lock().unwrap().push(pid)),
            Box::new(move |st| s.lock().unwrap().push(st)),
        );
        Harness { rt, display, layout, watched, statuses }
    }

    fn fg(pid: u32, exe: &str) -> Input {
        Input::Foreground { pid, exe_name: exe.into(), monitor: D1.into() }
    }

    #[test]
    fn runtime_applies_and_restores_via_apis() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        assert_eq!(*h.display.calls.lock().unwrap(), vec!["save_topology".to_string(), format!("apply {D1} 1280x960@240")]);
        assert_eq!(*h.layout.lock().unwrap(), vec![format!("capture {D1}")]);
        assert_eq!(*h.watched.lock().unwrap(), vec![10]);

        let t1 = t0 + Duration::from_millis(1000);
        h.rt.handle_input(fg(99, "chrome.exe"), t1);
        h.rt.tick(t1 + Duration::from_millis(250));
        assert_eq!(h.display.calls.lock().unwrap().last().unwrap(), &format!("restore {D1}"));
        assert_eq!(h.layout.lock().unwrap().last().unwrap(), &format!("restore {D1}"));
        assert_eq!(
            *h.statuses.lock().unwrap(),
            vec![StatusEvent::Active { game_id: "cs2".into(), width: 1280, height: 960, refresh_hz: 240 }, StatusEvent::Idle]
        );
    }

    #[test]
    fn runtime_ignores_unreadable_exe() {
        let mut h = harness(false);
        h.rt.handle_input(fg(10, ""), Instant::now());
        assert!(h.display.calls.lock().unwrap().is_empty());
        assert!(h.statuses.lock().unwrap().is_empty());
    }

    #[test]
    fn runtime_reports_apply_error() {
        let mut h = harness(true);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        // The error must be the last status: the screen is NOT stretched, the UI must not say it is.
        let last = h.statuses.lock().unwrap().last().cloned();
        assert!(matches!(last, Some(StatusEvent::Error { .. })), "last status: {last:?}");
        let (ack, _ack_rx) = std::sync::mpsc::channel();
        h.rt.handle_input(Input::Shutdown { ack }, t0 + Duration::from_millis(10));
        assert_eq!(h.display.calls.lock().unwrap().last().unwrap(), &format!("restore {D1}"));
    }

    #[test]
    fn runtime_reapplies_on_display_change() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        *h.display.current.lock().unwrap() = Some(m(1920, 1080, 240)); // e.g. UAC reset the mode
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_millis(10));
        let applies = h.display.calls.lock().unwrap().iter().filter(|c| c.starts_with("apply")).count();
        assert_eq!(applies, 2);
    }

    fn calls(h: &Harness) -> Vec<String> {
        h.display.calls.lock().unwrap().clone()
    }

    #[test]
    fn topology_loss_restores_topology_and_suspends() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.display.devices.lock().unwrap().retain(|d| d != D2); // DISPLAY2 left the desktop
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_millis(500));

        let c = calls(&h);
        assert!(c.contains(&"restore_topology".to_string()));
        assert!(c.contains(&format!("restore {D1}")));
        // The error is the last status, so the UI keeps showing it after the engine went idle.
        let last = h.statuses.lock().unwrap().last().cloned();
        assert!(matches!(last, Some(StatusEvent::Error { .. })), "last status: {last:?}");

        // Suspended: the game coming back to the foreground does not switch again...
        let applies_before = calls(&h).iter().filter(|c| c.starts_with("apply")).count();
        h.rt.handle_input(fg(99, "chrome.exe"), t0 + Duration::from_millis(600));
        h.rt.handle_input(fg(10, "cs2.exe"), t0 + Duration::from_millis(700));
        assert_eq!(calls(&h).iter().filter(|c| c.starts_with("apply")).count(), applies_before);

        // ...until the user changes the configuration.
        h.display.devices.lock().unwrap().push(D2.into());
        let rules = vec![GameRule { id: "cs2".into(), exe_name_lower: "cs2.exe".into(), width: 1280, height: 960, keep_stretched: false }];
        h.rt.handle_input(Input::RulesChanged(rules), t0 + Duration::from_millis(800));
        h.rt.handle_input(fg(10, "cs2.exe"), t0 + Duration::from_millis(900));
        assert_eq!(calls(&h).iter().filter(|c| c.starts_with("apply")).count(), applies_before + 1);
    }

    #[test]
    fn topology_loss_detected_on_guard_deadline() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.display.devices.lock().unwrap().retain(|d| d != D2);
        assert_eq!(h.rt.next_deadline(), Some(t0 + TOPOLOGY_GUARD));
        h.rt.tick(t0 + TOPOLOGY_GUARD);
        assert!(calls(&h).contains(&"restore_topology".to_string()));
    }

    #[test]
    fn unchanged_topology_does_nothing() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_millis(500));
        h.rt.tick(t0 + TOPOLOGY_GUARD);
        assert!(!calls(&h).contains(&"restore_topology".to_string()));
        assert_eq!(h.rt.next_deadline(), None);
    }

    fn restores(h: &Harness, device: &str) -> usize {
        calls(h).iter().filter(|c| **c == format!("restore {device}")).count()
    }

    #[test]
    fn runtime_reports_restore_error_last() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        *h.display.fail_restore.lock().unwrap() = true;
        h.rt.handle_input(fg(99, "chrome.exe"), t0 + Duration::from_millis(10));
        h.rt.tick(t0 + Duration::from_millis(260));
        let last = h.statuses.lock().unwrap().last().cloned();
        assert!(matches!(last, Some(StatusEvent::Error { .. })), "last status: {last:?}");
    }

    #[test]
    fn emergency_restore_works_even_when_idle() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        *h.display.fail_restore.lock().unwrap() = true;
        h.rt.handle_input(Input::ProcessExited { pid: 10 }, t0 + Duration::from_millis(10)); // restore fails, engine idle
        *h.display.fail_restore.lock().unwrap() = false;
        h.rt.handle_input(Input::RestoreRequested, t0 + Duration::from_millis(20));
        assert_eq!(restores(&h, D1), 2, "the emergency path must retry every monitor REZ touched");
    }

    #[test]
    fn native_mode_is_reread_after_a_display_change_while_idle() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.rt.handle_input(Input::ProcessExited { pid: 10 }, t0 + Duration::from_millis(10));
        h.rt.tick(t0 + TOPOLOGY_GUARD + Duration::from_millis(20)); // guard window over
        *h.display.registry.lock().unwrap() = Some(m(1920, 1080, 144)); // user switched to 144 Hz in Windows
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_secs(10));
        h.rt.handle_input(fg(11, "cs2.exe"), t0 + Duration::from_secs(11));
        assert_eq!(calls(&h).last().unwrap(), &format!("apply {D1} 1280x960@144"));
    }

    #[test]
    fn topology_change_after_the_guard_window_is_left_alone() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.rt.tick(t0 + TOPOLOGY_GUARD); // REZ's own change settled fine
        h.display.devices.lock().unwrap().retain(|d| d != D2); // later, the user disables DISPLAY2
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_secs(30));
        assert!(!calls(&h).contains(&"restore_topology".to_string()));
    }

    #[test]
    fn topology_incident_restores_only_monitors_rez_touched() {
        const D3: &str = r"\.\DISPLAY3";
        let mut h = harness(false);
        h.display.devices.lock().unwrap().push(D3.into());
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        h.display.devices.lock().unwrap().retain(|d| d != D3);
        h.rt.handle_input(Input::DisplayMaybeChanged, t0 + Duration::from_millis(500));
        assert!(calls(&h).contains(&"restore_topology".to_string()));
        assert_eq!(restores(&h, D2), 0, "DISPLAY2 was never stretched by REZ");
    }

    #[test]
    fn runtime_shutdown_acks() {
        let mut h = harness(false);
        let t0 = Instant::now();
        h.rt.handle_input(fg(10, "cs2.exe"), t0);
        let (ack, ack_rx) = std::sync::mpsc::channel();
        h.rt.handle_input(Input::Shutdown { ack }, t0 + Duration::from_millis(10));
        assert_eq!(h.display.calls.lock().unwrap().last().unwrap(), &format!("restore {D1}"));
        // Pending layout restores are completed before acknowledging, so quitting never loses them.
        assert_eq!(h.layout.lock().unwrap().last().map(String::as_str), Some("flush"));
        assert!(ack_rx.try_recv().is_ok());
    }
}
