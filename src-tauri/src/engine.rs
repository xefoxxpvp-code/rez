//! Pure foreground/display state machine (no Win32). Completed in Task 5.

/// An enabled game the engine watches for.
#[derive(Clone, Debug, PartialEq)]
pub struct GameRule {
    pub id: String,
    pub exe_name_lower: String,
    pub width: u32,
    pub height: u32,
    pub keep_stretched: bool,
}
