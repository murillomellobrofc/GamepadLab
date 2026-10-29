//! Stream overlay: a small frameless, transparent, always-on-top window
//! (overlay.html) that mirrors one controller live.

use crate::error::{CmdError, CmdResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";
pub const EV_SETTINGS: &str = "overlay://settings";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayConfig {
    pub settings: serde_json::Value,
    pub controller_id: Option<String>,
}

#[derive(Default)]
pub struct OverlayState(pub Mutex<Option<OverlayConfig>>);

fn always_on_top(settings: &serde_json::Value) -> bool {
    settings
        .get("alwaysOnTop")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

#[tauri::command]
pub async fn overlay_open<R: Runtime>(
    app: AppHandle<R>,
    settings: serde_json::Value,
    controller_id: Option<String>,
) -> CmdResult<()> {
    let config = OverlayConfig {
        settings,
        controller_id,
    };
    let on_top = always_on_top(&config.settings);
    *app.state::<OverlayState>().0.lock().unwrap() = Some(config.clone());
    if let Some(w) = app.get_webview_window(LABEL) {
        w.set_always_on_top(on_top).map_err(CmdError::internal)?;
        w.show().map_err(CmdError::internal)?;
        let _ = app.emit_to(LABEL, EV_SETTINGS, &config);
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("GamepadLab Overlay")
        .inner_size(480.0, 340.0)
        .min_inner_size(220.0, 160.0)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(on_top)
        .resizable(true)
        .build()
        .map_err(CmdError::internal)?;
    log::info!("overlay window opened");
    Ok(())
}

#[tauri::command]
pub fn overlay_update<R: Runtime>(
    app: AppHandle<R>,
    settings: serde_json::Value,
    controller_id: Option<String>,
) -> CmdResult<()> {
    let config = OverlayConfig {
        settings,
        controller_id,
    };
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_always_on_top(always_on_top(&config.settings));
    }
    *app.state::<OverlayState>().0.lock().unwrap() = Some(config.clone());
    let _ = app.emit_to(LABEL, EV_SETTINGS, &config);
    Ok(())
}

#[tauri::command]
pub fn overlay_close<R: Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    if let Some(w) = app.get_webview_window(LABEL) {
        w.close().map_err(CmdError::internal)?;
    }
    Ok(())
}

/// Lets the overlay window read its configuration when it starts.
#[tauri::command]
pub fn overlay_state<R: Runtime>(app: AppHandle<R>) -> Option<OverlayConfig> {
    app.state::<OverlayState>().0.lock().unwrap().clone()
}

#[tauri::command]
pub fn overlay_is_open<R: Runtime>(app: AppHandle<R>) -> bool {
    app.get_webview_window(LABEL).is_some()
}
