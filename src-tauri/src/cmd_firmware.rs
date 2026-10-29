//! Firmware calibration commands: run a step on the controller, keep a
//! backup on disk before any Switch flash write, and log permanent changes
//! in the history.

use crate::error::{CmdError, CmdResult};
use crate::history::NewSession;
use crate::state::AppState;
use gamepadlab_input::firmware::switch_cal::{self, SwitchProposal};
use gamepadlab_input::firmware::{FirmwareOp, FirmwareReply, SwitchCalibrationData};
use gamepadlab_input::{ControllerId, ControllerInfo};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

fn controller(state: &AppState, id: &str) -> CmdResult<ControllerInfo> {
    state
        .input
        .controllers()
        .into_iter()
        .find(|c| c.id.as_str() == id)
        .ok_or_else(|| CmdError::new("disconnected", format!("controller {id} is not connected")))
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn save_backup(
    state: &AppState,
    c: &ControllerInfo,
    op: &str,
    reply: &FirmwareReply,
) -> Option<String> {
    let backup = reply.backup.as_ref()?;
    let dir = state.data_dir.join("firmware-backups");
    std::fs::create_dir_all(&dir).ok()?;
    let safe: String = c
        .profile_key
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let stamp = gamepadlab_profiles::now_rfc3339().replace([':', '-'], "");
    let path = dir.join(format!("{safe}-{stamp}.json"));
    let body = json!({
        "controller": c.name,
        "profileKey": c.profile_key,
        "operation": op,
        "address": format!("0x{:04X}", switch_cal::USER_STICKS_ADDR),
        "userBlockBefore": hex(backup),
        "after": reply.switch,
    });
    match std::fs::write(&path, serde_json::to_vec_pretty(&body).unwrap_or_default()) {
        Ok(()) => Some(path.display().to_string()),
        Err(e) => {
            log::error!("could not write firmware backup: {e}");
            None
        }
    }
}

fn record(state: &AppState, c: &ControllerInfo, summary: String, payload: serde_json::Value) {
    if let Some(h) = state.history.as_ref() {
        if let Err(e) = h.add(&NewSession {
            kind: "firmware".into(),
            controller_name: c.name.clone(),
            controller_key: c.profile_key.clone(),
            score: None,
            summary,
            payload,
        }) {
            log::error!("could not record firmware change in history: {e}");
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareResult {
    #[serde(flatten)]
    reply: FirmwareReply,
    backup_path: Option<String>,
}

#[tauri::command]
pub fn firmware(state: St, id: String, op: FirmwareOp) -> CmdResult<FirmwareResult> {
    let c = controller(&state, &id)?;
    let reply = state.input.firmware(&ControllerId(id), op.clone())?;
    let mut backup_path = None;
    match &op {
        FirmwareOp::SavePermanent { .. } => {
            log::warn!(
                "{}: calibration saved permanently to the controller",
                c.name
            );
            record(
                &state,
                &c,
                "Calibração gravada permanentemente no controle".into(),
                json!({ "op": op, "status": reply.status }),
            );
        }
        FirmwareOp::SwitchWrite { left, right, .. } => {
            backup_path = save_backup(&state, &c, "switchWrite", &reply);
            log::warn!(
                "{}: user stick calibration written (backup {:?})",
                c.name,
                backup_path
            );
            record(
                &state,
                &c,
                "Calibração do usuário gravada na memória do Switch Pro".into(),
                json!({ "op": "switchWrite", "left": left, "right": right, "backupPath": backup_path }),
            );
        }
        FirmwareOp::SwitchClear { .. } => {
            backup_path = save_backup(&state, &c, "switchClear", &reply);
            record(
                &state,
                &c,
                "Calibração do usuário apagada (volta à de fábrica)".into(),
                json!({ "op": "switchClear", "backupPath": backup_path }),
            );
        }
        _ => {}
    }
    Ok(FirmwareResult { reply, backup_path })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchProposals {
    current: SwitchCalibrationData,
    left: Option<SwitchProposal>,
    right: Option<SwitchProposal>,
}

/// Proposes a Switch Pro user calibration from the wizard captures
/// (rest + rotation per stick). Nothing is written here.
#[tauri::command]
pub fn firmware_switch_propose(state: St, id: String) -> CmdResult<SwitchProposals> {
    let reply = state
        .input
        .firmware(&ControllerId(id), FirmwareOp::SwitchRead)?;
    let current = reply
        .switch
        .ok_or_else(|| CmdError::internal("no calibration data in reply"))?;
    let caps = state.captures.lock().unwrap();
    let get = |k: &str, s: &str| {
        caps.get(&(k.to_string(), s.to_string()))
            .cloned()
            .unwrap_or_default()
    };
    let active_left = current.user_left.unwrap_or(current.factory_left);
    let active_right = current.user_right.unwrap_or(current.factory_right);
    let left = switch_cal::propose(&get("rest", "left"), &get("rotation", "left"), &active_left);
    let right = switch_cal::propose(
        &get("rest", "right"),
        &get("rotation", "right"),
        &active_right,
    );
    if left.is_none() && right.is_none() {
        return Err(CmdError::invalid(
            "measure rest and rotation for at least one stick first",
        ));
    }
    Ok(SwitchProposals {
        current,
        left,
        right,
    })
}
