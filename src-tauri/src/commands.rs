//! Tauri commands. Each one is a thin, typed wrapper over the crates.

use crate::error::{CmdError, CmdResult};
use crate::platform::{self, AppInfo, PlatformWarning};
use crate::state::AppState;
use gamepadlab_haptics::{motor_test_program, RumbleRequest, TriggerEffect};
use gamepadlab_input::{ControllerId, ControllerInfo, DemoScenario, ManualInput, OutputCommand};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, State};

type St<'a> = State<'a, Arc<AppState>>;

pub fn updater_configured<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.is_empty() && !k.starts_with("REPLACE_"))
        && !platform::package_managed()
}

#[tauri::command]
pub fn app_info<R: Runtime>(app: AppHandle<R>, state: St) -> AppInfo {
    let version = app.package_info().version.to_string();
    platform::app_info(&state, version, updater_configured(&app))
}

#[tauri::command]
pub fn list_controllers(state: St) -> Vec<ControllerInfo> {
    state.input.controllers()
}

#[tauri::command]
pub fn platform_warnings(state: St) -> Vec<PlatformWarning> {
    platform::warnings(&state)
}

#[tauri::command]
pub fn set_ui_rate(state: St, hz: u32) -> CmdResult<()> {
    if !(15..=240).contains(&hz) {
        return Err(CmdError::invalid(format!(
            "UI rate {hz} Hz out of range 15–240"
        )));
    }
    state.ui_rate.store(hz, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn reset_minmax(state: St, id: String) {
    state.input.reset_minmax(&ControllerId(id));
}

#[tauri::command]
pub fn set_demo(state: St, enabled: bool) {
    state.input.set_demo(enabled);
}

#[tauri::command]
pub fn demo_scenario(state: St, scenario: DemoScenario) {
    state.input.demo_scenario(scenario);
}

#[tauri::command]
pub fn demo_manual(state: St, input: ManualInput) {
    state.input.demo_manual(input);
}

#[tauri::command]
pub fn enable_sensors(state: St, id: String) -> CmdResult<bool> {
    Ok(state.input.enable_extended(&ControllerId(id))?)
}

#[tauri::command]
pub fn diagnostics<R: Runtime>(app: AppHandle<R>, state: St) -> String {
    let version = app.package_info().version.to_string();
    platform::diagnostics(&state, version, &log::max_level().to_string())
}

#[tauri::command]
pub fn set_log_level(level: String) {
    let f = crate::logging::set_level(&level);
    log::info!("log level set to {f}");
}

#[tauri::command]
pub fn log_frontend(level: String, message: String) {
    let message: String = message.chars().take(2000).collect();
    match level.as_str() {
        "error" => log::error!(target: "ui", "{message}"),
        "warn" => log::warn!(target: "ui", "{message}"),
        "debug" => log::debug!(target: "ui", "{message}"),
        _ => log::info!(target: "ui", "{message}"),
    }
}

/// Opens System Settings > Privacy & Security > Input Monitoring (macOS).
#[tauri::command]
pub fn open_privacy_settings() -> CmdResult<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn()
            .map_err(CmdError::io)?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(CmdError::new("invalidInput", "only available on macOS"))
    }
}

/// App data folder; `GAMEPADLAB_DATA_DIR` overrides it (tests, portable use).
pub fn data_dir<R: Runtime>(app: &AppHandle<R>) -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("GAMEPADLAB_DATA_DIR") {
        return dir.into();
    }
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("gamepadlab"))
}

fn require_controller(state: &AppState, id: &str) -> CmdResult<ControllerInfo> {
    state
        .input
        .controllers()
        .into_iter()
        .find(|c| c.id.as_str() == id)
        .ok_or_else(|| CmdError::new("disconnected", format!("controller {id} is not connected")))
}

fn require_rumble(state: &AppState, id: &str) -> CmdResult<()> {
    let c = require_controller(state, id)?;
    if !c.capabilities.rumble {
        return Err(CmdError::new(
            "rumbleUnsupported",
            format!("{} has no rumble motors", c.name),
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn rumble_play(state: St, id: String, request: RumbleRequest) -> CmdResult<()> {
    require_rumble(&state, &id)?;
    state.haptics.play(&id, vec![request]);
    Ok(())
}

#[tauri::command]
pub fn rumble_sequence(state: St, id: String, requests: Vec<RumbleRequest>) -> CmdResult<()> {
    require_rumble(&state, &id)?;
    if requests.is_empty() || requests.len() > 64 {
        return Err(CmdError::invalid(
            "a rumble sequence needs 1 to 64 segments",
        ));
    }
    state.haptics.play(&id, requests);
    Ok(())
}

#[tauri::command]
pub fn rumble_motor_test(state: St, id: String, intensity: f32) -> CmdResult<()> {
    require_rumble(&state, &id)?;
    state.haptics.play(&id, motor_test_program(intensity));
    Ok(())
}

#[tauri::command]
pub fn rumble_stop(state: St, id: String) {
    state.haptics.stop(&id);
}

#[tauri::command]
pub fn set_trigger_effects(
    state: St,
    id: String,
    left: TriggerEffect,
    right: TriggerEffect,
) -> CmdResult<()> {
    let c = require_controller(&state, &id)?;
    if !c.capabilities.adaptive_triggers {
        return Err(CmdError::new(
            "rumbleUnsupported",
            format!("{} has no adaptive triggers", c.name),
        ));
    }
    state.input.send_output(
        &c.id,
        OutputCommand::TriggerEffects {
            left: left.encode().to_vec(),
            right: right.encode().to_vec(),
        },
    )?;
    Ok(())
}

#[tauri::command]
pub fn set_lightbar(state: St, id: String, r: u8, g: u8, b: u8) -> CmdResult<()> {
    let c = require_controller(&state, &id)?;
    if !c.capabilities.lightbar {
        return Err(CmdError::new(
            "rumbleUnsupported",
            format!("{} has no lightbar", c.name),
        ));
    }
    state
        .input
        .send_output(&c.id, OutputCommand::Lightbar { r, g, b })?;
    Ok(())
}

#[tauri::command]
pub fn set_player_leds(state: St, id: String, mask: u8) -> CmdResult<()> {
    let c = require_controller(&state, &id)?;
    if !c.capabilities.player_leds {
        return Err(CmdError::new(
            "rumbleUnsupported",
            format!("{} has no player LEDs", c.name),
        ));
    }
    state
        .input
        .send_output(&c.id, OutputCommand::PlayerLeds { mask: mask & 0x1F })?;
    Ok(())
}
