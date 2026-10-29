//! Calibration wizard captures, analyses and profile management commands.

use crate::calibrate;
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use gamepadlab_calibration::{
    analyze_trigger, build_report, CalibrationProfile, Sample, StickCalibration, StickReport,
    StickSide, TriggerAnalysis,
};
use gamepadlab_input::{CaptureTarget, ControllerInfo};
use gamepadlab_profiles::{DeviceProfiles, Profile, ProfileError};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

impl From<ProfileError> for CmdError {
    fn from(e: ProfileError) -> Self {
        CmdError::new(e.code(), e.to_string())
    }
}

const KINDS: [&str; 5] = [
    "rest",
    "rotation",
    "directions",
    "triggerRest",
    "triggerFull",
];

fn side(target: &str) -> CmdResult<&'static str> {
    match target {
        "left" => Ok("left"),
        "right" => Ok("right"),
        other => Err(CmdError::invalid(format!("unknown side {other}"))),
    }
}

fn controller(state: &AppState, id: &str) -> CmdResult<ControllerInfo> {
    state
        .input
        .controllers()
        .into_iter()
        .find(|c| c.id.as_str() == id)
        .ok_or_else(|| CmdError::new("disconnected", format!("controller {id} is not connected")))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    kind: String,
    target: String,
    samples: Vec<Sample>,
    duration_ms: f32,
}

#[tauri::command]
pub fn capture_start(state: St, id: String, kind: String, target: String) -> CmdResult<()> {
    let c = controller(&state, &id)?;
    if !KINDS.contains(&kind.as_str()) {
        return Err(CmdError::invalid(format!("unknown capture kind {kind}")));
    }
    let side = side(&target)?;
    let trigger = kind.starts_with("trigger");
    let t = match (trigger, side) {
        (false, "left") => CaptureTarget::LeftStick,
        (false, _) => CaptureTarget::RightStick,
        (true, "left") => CaptureTarget::LeftTrigger,
        (true, _) => CaptureTarget::RightTrigger,
    };
    *state.pending_capture.lock().unwrap() = Some((kind, side.to_string()));
    state.input.capture_start(&c.id, t);
    Ok(())
}

#[tauri::command]
pub fn capture_stop(state: St) -> CmdResult<CaptureResult> {
    let key = state
        .pending_capture
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| CmdError::invalid("no capture running"))?;
    let cap = state
        .input
        .capture_stop()
        .ok_or_else(|| CmdError::invalid("no capture running"))?;
    state
        .captures
        .lock()
        .unwrap()
        .insert(key.clone(), cap.samples.clone());
    Ok(CaptureResult {
        kind: key.0,
        target: key.1,
        samples: cap.samples,
        duration_ms: cap.duration_ms,
    })
}

#[tauri::command]
pub fn capture_clear(state: St) {
    state.captures.lock().unwrap().clear();
    *state.pending_capture.lock().unwrap() = None;
    let _ = state.input.capture_stop();
}

fn stored(state: &AppState, kind: &str, side: &str) -> Vec<Sample> {
    state
        .captures
        .lock()
        .unwrap()
        .get(&(kind.to_string(), side.to_string()))
        .cloned()
        .unwrap_or_default()
}

#[tauri::command]
pub fn analyze_stick(
    state: St,
    stick: String,
    calibration: Option<StickCalibration>,
) -> CmdResult<StickReport> {
    let side_name = side(&stick)?;
    let s = if side_name == "left" {
        StickSide::Left
    } else {
        StickSide::Right
    };
    let rest = stored(&state, "rest", side_name);
    let rotation = stored(&state, "rotation", side_name);
    let directions = stored(&state, "directions", side_name);
    if rest.is_empty() && rotation.is_empty() && directions.is_empty() {
        return Err(CmdError::invalid("no captured data for this stick yet"));
    }
    Ok(build_report(
        s,
        &rest,
        &rotation,
        &directions,
        calibration.as_ref(),
    ))
}

#[tauri::command]
pub fn analyze_trigger_cmd(state: St, side_name: String) -> CmdResult<TriggerAnalysis> {
    let s = side(&side_name)?;
    analyze_trigger(
        &stored(&state, "triggerRest", s),
        &stored(&state, "triggerFull", s),
    )
    .ok_or_else(|| CmdError::invalid("capture the released and pressed trigger first"))
}

#[tauri::command]
pub fn set_active_calibration(
    state: St,
    id: String,
    profile: Option<CalibrationProfile>,
) -> CmdResult<()> {
    match profile {
        Some(p) => {
            let errors = p.validate();
            if !errors.is_empty() {
                return Err(CmdError::invalid(
                    errors
                        .iter()
                        .map(|e| format!("{}: {}", e.field, e.reason))
                        .collect::<Vec<_>>()
                        .join("; "),
                ));
            }
            state.calibrations.write().unwrap().insert(id, p);
        }
        None => {
            state.calibrations.write().unwrap().remove(&id);
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ profiles

#[tauri::command]
pub fn profiles_load(state: St, id: String) -> CmdResult<DeviceProfiles> {
    let c = controller(&state, &id)?;
    let doc = state.profiles.load(&c.profile_key, &c.name)?;
    // Persist the default document the first time it is opened.
    let _ = state.profiles.save(&doc);
    calibrate::load_active(&state, &c);
    Ok(doc)
}

#[tauri::command]
pub fn profiles_create(
    state: St,
    device_key: String,
    name: String,
    calibration: Option<CalibrationProfile>,
) -> CmdResult<Profile> {
    let p = state.profiles.create(&device_key, &name, calibration)?;
    calibrate::refresh_key(&state, &device_key);
    Ok(p)
}

#[tauri::command]
pub fn profiles_update(state: St, device_key: String, profile: Profile) -> CmdResult<Profile> {
    let p = state.profiles.update(&device_key, profile)?;
    calibrate::refresh_key(&state, &device_key);
    Ok(p)
}

#[tauri::command]
pub fn profiles_rename(
    state: St,
    device_key: String,
    profile_id: String,
    name: String,
) -> CmdResult<()> {
    Ok(state.profiles.rename(&device_key, &profile_id, &name)?)
}

#[tauri::command]
pub fn profiles_duplicate(state: St, device_key: String, profile_id: String) -> CmdResult<Profile> {
    Ok(state.profiles.duplicate(&device_key, &profile_id)?)
}

#[tauri::command]
pub fn profiles_delete(state: St, device_key: String, profile_id: String) -> CmdResult<()> {
    state.profiles.delete(&device_key, &profile_id)?;
    calibrate::refresh_key(&state, &device_key);
    Ok(())
}

#[tauri::command]
pub fn profiles_set_active(state: St, device_key: String, profile_id: String) -> CmdResult<()> {
    state.profiles.set_active(&device_key, &profile_id)?;
    calibrate::refresh_key(&state, &device_key);
    Ok(())
}

#[tauri::command]
pub fn profiles_reset(state: St, device_key: String, profile_id: String) -> CmdResult<Profile> {
    let p = state.profiles.reset(&device_key, &profile_id)?;
    calibrate::refresh_key(&state, &device_key);
    Ok(p)
}

#[tauri::command]
pub fn profiles_export(state: St, device_key: String, profile_id: String) -> CmdResult<String> {
    Ok(state
        .profiles
        .export(&device_key, &profile_id, env!("CARGO_PKG_VERSION"))?)
}

#[tauri::command]
pub fn profiles_import(state: St, device_key: String, json: String) -> CmdResult<Profile> {
    let p = state.profiles.import(&device_key, &json)?;
    Ok(p)
}

#[tauri::command]
pub fn controller_profile_key(state: St, id: String) -> CmdResult<String> {
    Ok(controller(&state, &id)?.profile_key)
}
