//! Button health commands: a read-only recording of every button edge of one
//! controller, analysed by `gamepadlab_calibration::button_health`.
//! Nothing here sends anything to the controller.

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use gamepadlab_calibration::button_health::{analyze, ButtonPlan, HealthReport, HealthSession};
use gamepadlab_input::HealthProgress;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

fn no_session() -> CmdError {
    CmdError::invalid("no button health session running")
}

/// Starts recording `id` (replacing any running session).
#[tauri::command]
pub fn button_health_start(state: St, id: String) -> CmdResult<()> {
    let c = state
        .input
        .controllers()
        .into_iter()
        .find(|c| c.id.as_str() == id)
        .ok_or_else(|| {
            CmdError::new("disconnected", format!("controller {id} is not connected"))
        })?;
    log::info!("button health: recording {} via {}", c.name, c.backend);
    state.input.health_start(&c.id);
    Ok(())
}

/// Live press counts and the current hold, for the round on screen.
#[tauri::command]
pub fn button_health_progress(state: St) -> CmdResult<HealthProgress> {
    state.input.health_progress().ok_or_else(no_session)
}

/// Ends the session and analyses the planned buttons.
#[tauri::command]
pub fn button_health_stop(state: St, plans: Vec<ButtonPlan>) -> CmdResult<HealthReport> {
    if plans.is_empty() {
        let _ = state.input.health_stop();
        return Err(CmdError::invalid("no buttons were tested"));
    }
    let rec = state.input.health_stop().ok_or_else(no_session)?;
    let report = analyze(&HealthSession {
        precision: rec.precision,
        duration_us: rec.duration_us,
        plans,
        events: rec.events,
    });
    log::info!(
        "button health: {} buttons, overall {:?}, precision {:?}",
        report.buttons.len(),
        report.overall,
        report.precision
    );
    Ok(report)
}

/// Drops a running session without analysing it.
#[tauri::command]
pub fn button_health_cancel(state: St) {
    let _ = state.input.health_stop();
}
