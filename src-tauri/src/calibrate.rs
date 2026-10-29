//! Applies the active calibration profile to snapshots for the "calibrated"
//! view, and loads each controller's active profile when it connects.

use crate::emitter::CalibratedView;
use crate::state::AppState;
use gamepadlab_input::{Button, ButtonState, ControllerInfo, InputSnapshot};

pub fn view(state: &AppState, snapshot: &InputSnapshot) -> Option<CalibratedView> {
    let cals = state.calibrations.read().unwrap();
    let cal = cals.get(snapshot.id.as_str())?;
    let lt = cal.left_trigger.apply(snapshot.left_trigger);
    let rt = cal.right_trigger.apply(snapshot.right_trigger);
    let mut buttons = snapshot.buttons.clone();
    if let Some(b) = buttons.get_mut(Button::LeftTrigger.index()) {
        *b = ButtonState::analog(lt, 0.12);
    }
    if let Some(b) = buttons.get_mut(Button::RightTrigger.index()) {
        *b = ButtonState::analog(rt, 0.12);
    }
    Some(CalibratedView {
        left_stick: cal.left_stick.apply(snapshot.left_stick),
        right_stick: cal.right_stick.apply(snapshot.right_stick),
        left_trigger: lt,
        right_trigger: rt,
        buttons,
    })
}

/// Loads the controller's active profile into the live calibration map.
pub fn load_active(state: &AppState, info: &ControllerInfo) {
    match state.profiles.load(&info.profile_key, &info.name) {
        Ok(doc) => {
            if let Some(p) = doc.active() {
                state
                    .calibrations
                    .write()
                    .unwrap()
                    .insert(info.id.0.clone(), p.calibration.clone());
                log::debug!("active profile \"{}\" loaded for {}", p.name, info.name);
            }
        }
        Err(e) => log::warn!("could not load profiles for {}: {e}", info.name),
    }
}

/// Refreshes every connected controller that shares `profile_key`.
pub fn refresh_key(state: &AppState, profile_key: &str) {
    for c in state.input.controllers() {
        if c.profile_key == profile_key {
            load_active(state, &c);
        }
    }
}
