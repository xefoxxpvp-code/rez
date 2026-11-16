//! Runtime: feeds OS inputs to the [`Engine`](crate::engine::Engine) and executes its actions.
//! Completed in Task 7.

use crate::engine::GameRule;

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
