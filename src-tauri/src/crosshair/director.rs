//! Pure decision of what the overlay shows (completed in Task 3).

use super::model::Crosshair;

/// An enabled game with an assigned crosshair.
#[derive(Clone, Debug, PartialEq)]
pub struct CrosshairRule {
    pub exe_name_lower: String,
    pub crosshair: Crosshair,
}
