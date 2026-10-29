//! Applies the active calibration profile to snapshots for the "calibrated"
//! view. Filled in by the calibration phase.

use crate::emitter::CalibratedView;
use crate::state::AppState;
use gamepadlab_input::InputSnapshot;

pub fn view(_state: &AppState, _snapshot: &InputSnapshot) -> Option<CalibratedView> {
    None
}
