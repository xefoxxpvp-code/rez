//! Bookkeeping for desktop-layout snapshots, independent of Win32 so it can be tested.
//!
//! One *pristine* snapshot is kept per stretch session: a quick return to the game while a
//! delayed restore is still pending cancels that restore and reuses the pristine snapshot,
//! instead of capturing the half-settled layout. Icons are global: captured once when the
//! first monitor is stretched, restored only when the last one is back to native.

use std::collections::{HashMap, HashSet};

/// Identifies one scheduled restore; stale tickets are ignored by [`LayoutBook::fire`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    monitor_gen: u64,
}

pub struct LayoutBook<W, I> {
    windows: HashMap<String, W>,
    icons: Option<I>,
    active: HashSet<String>,
    /// monitor -> generation of its pending restore.
    pending: HashMap<String, u64>,
    next_gen: u64,
}

impl<W, I> Default for LayoutBook<W, I> {
    fn default() -> Self {
        LayoutBook { windows: HashMap::new(), icons: None, active: HashSet::new(), pending: HashMap::new(), next_gen: 0 }
    }
}

impl<W, I> LayoutBook<W, I> {
    /// Called when `monitor` is about to be stretched. A pending restore for it is cancelled
    /// and its pristine snapshot kept; otherwise `windows` is captured. Icons are captured
    /// only when no snapshot of them is held.
    pub fn capture(&mut self, monitor: &str, windows: impl FnOnce() -> W, icons: impl FnOnce() -> I) {
        let resumed = self.pending.remove(monitor).is_some();
        if !resumed || !self.windows.contains_key(monitor) {
            self.windows.insert(monitor.to_string(), windows());
        }
        if self.icons.is_none() {
            self.icons = Some(icons());
        }
        self.active.insert(monitor.to_string());
    }

    /// Called when `monitor` went back to native; the restore itself happens in [`Self::fire`].
    pub fn schedule_restore(&mut self, monitor: &str) -> Ticket {
        self.active.remove(monitor);
        self.next_gen += 1;
        self.pending.insert(monitor.to_string(), self.next_gen);
        Ticket { monitor_gen: self.next_gen }
    }

    /// Runs a scheduled restore: returns the windows snapshot, plus the icons if no other
    /// monitor is stretched or waiting. `None` when the ticket was cancelled or superseded.
    pub fn fire(&mut self, monitor: &str, ticket: Ticket) -> Option<(W, Option<I>)> {
        if self.pending.get(monitor) != Some(&ticket.monitor_gen) {
            return None;
        }
        self.pending.remove(monitor);
        let windows = self.windows.remove(monitor)?;
        let icons = if self.active.is_empty() && self.pending.is_empty() { self.icons.take() } else { None };
        Some((windows, icons))
    }

    /// Runs every pending restore now (app shutdown).
    pub fn take_all_pending(&mut self) -> Vec<(W, Option<I>)> {
        let monitors: Vec<String> = self.pending.keys().cloned().collect();
        let mut out = Vec::new();
        for (i, monitor) in monitors.iter().enumerate() {
            self.pending.remove(monitor);
            if let Some(windows) = self.windows.remove(monitor) {
                let last = i + 1 == monitors.len() && self.active.is_empty();
                out.push((windows, if last { self.icons.take() } else { None }));
            }
        }
        out
    }

    /// Icons snapshot currently held, if any (used to persist it).
    pub fn icons(&self) -> Option<&I> {
        self.icons.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D1: &str = "D1";
    const D2: &str = "D2";

    fn book() -> LayoutBook<&'static str, &'static str> {
        LayoutBook::default()
    }

    #[test]
    fn capture_then_restore_returns_snapshot_and_icons() {
        let mut b = book();
        b.capture(D1, || "win-d1", || "icons");
        let t = b.schedule_restore(D1);
        assert_eq!(b.fire(D1, t), Some(("win-d1", Some("icons"))));
    }

    #[test]
    fn quick_return_reuses_pristine_snapshot() {
        let mut b = book();
        b.capture(D1, || "pristine", || "icons");
        let t1 = b.schedule_restore(D1);
        // Back in the game before the delayed restore ran: nothing is captured again.
        b.capture(D1, || panic!("must not recapture windows"), || panic!("must not recapture icons"));
        assert_eq!(b.fire(D1, t1), None, "cancelled restore must do nothing");
        let t2 = b.schedule_restore(D1);
        assert_eq!(b.fire(D1, t2), Some(("pristine", Some("icons"))));
    }

    #[test]
    fn icons_are_restored_only_with_the_last_monitor() {
        let mut b = book();
        b.capture(D1, || "win-d1", || "icons");
        let t1 = b.schedule_restore(D1);
        b.capture(D2, || "win-d2", || panic!("icons already held"));
        assert_eq!(b.fire(D1, t1), Some(("win-d1", None)));
        let t2 = b.schedule_restore(D2);
        assert_eq!(b.fire(D2, t2), Some(("win-d2", Some("icons"))));
    }

    #[test]
    fn next_session_captures_fresh_icons() {
        let mut b = book();
        b.capture(D1, || "a", || "icons-1");
        let t = b.schedule_restore(D1);
        b.fire(D1, t);
        b.capture(D1, || "b", || "icons-2");
        let t = b.schedule_restore(D1);
        assert_eq!(b.fire(D1, t), Some(("b", Some("icons-2"))));
    }

    #[test]
    fn take_all_pending_flushes_every_scheduled_restore() {
        let mut b = book();
        b.capture(D1, || "win-d1", || "icons");
        b.schedule_restore(D1);
        let flushed = b.take_all_pending();
        assert_eq!(flushed, vec![("win-d1", Some("icons"))]);
        assert!(b.take_all_pending().is_empty());
    }
}
