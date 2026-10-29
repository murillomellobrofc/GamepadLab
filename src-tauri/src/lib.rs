//! GamepadLab desktop shell: wires the input, calibration, profiles and
//! haptics crates to the web UI through Tauri commands and events.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    os: String,
    os_version: String,
    arch: String,
    backends: Vec<String>,
    updater_configured: bool,
    data_dir: String,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        os_version: String::new(),
        arch: std::env::consts::ARCH.to_string(),
        backends: vec![],
        updater_configured: false,
        data_dir: String::new(),
    }
}

#[tauri::command]
fn list_controllers() -> Vec<serde_json::Value> {
    Vec::new()
}

#[tauri::command]
fn platform_warnings() -> Vec<serde_json::Value> {
    Vec::new()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_controllers,
            platform_warnings
        ])
        .run(tauri::generate_context!())
        .expect("error while running GamepadLab");
}
