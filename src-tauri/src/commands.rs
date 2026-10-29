//! Tauri commands. Each one is a thin, typed wrapper over the crates.

use crate::error::{CmdError, CmdResult};
use crate::platform::{self, AppInfo, PlatformWarning};
use crate::state::AppState;
use gamepadlab_input::{ControllerId, ControllerInfo, DemoScenario, ManualInput};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

type St<'a> = State<'a, Arc<AppState>>;

pub fn updater_configured(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.is_empty() && !k.starts_with("REPLACE_"))
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: St) -> AppInfo {
    platform::app_info(&state, updater_configured(&app))
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
pub fn diagnostics(state: St) -> String {
    platform::diagnostics(&state, &log::max_level().to_string())
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

pub fn data_dir(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("gamepadlab"))
}
