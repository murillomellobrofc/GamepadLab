//! App info, platform warnings and the "copy diagnostics" report.

use crate::state::AppState;
use gamepadlab_input::platform::{
    hidraw_needs_udev_rule, input_monitoring_access, InputMonitoringAccess,
};
use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub backends: Vec<String>,
    pub updater_configured: bool,
    /// Installed from apt/dnf/pacman/Flatpak: the package manager owns updates.
    pub package_managed: bool,
    pub data_dir: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlatformWarning {
    pub code: &'static str,
    pub message: String,
}

/// On Linux the built-in updater can only replace an AppImage. Anything else (.deb, .rpm,
/// Arch package, Flatpak) is updated by the system package manager instead.
pub fn package_managed() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_none()
}

pub fn os_version() -> String {
    System::long_os_version().unwrap_or_default()
}

/// `version` is the bundle version the updater compares against (tauri.conf.json), not the crate's.
pub fn app_info(state: &AppState, version: String, updater_configured: bool) -> AppInfo {
    AppInfo {
        version,
        os: std::env::consts::OS.to_string(),
        os_version: os_version(),
        arch: std::env::consts::ARCH.to_string(),
        backends: state.input.backend_names(),
        updater_configured,
        package_managed: package_managed(),
        data_dir: state.data_dir.display().to_string(),
    }
}

fn steam_running() -> bool {
    let Ok(mut sys) = System::new() else {
        return false;
    };
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_lowercase();
        name == "steam" || name == "steam.exe" || name == "steamwebhelper" || name == "steam_osx"
    })
}

pub fn warnings(state: &AppState) -> Vec<PlatformWarning> {
    let mut out = Vec::new();
    if steam_running() {
        out.push(PlatformWarning {
            code: "steamRunning",
            message: "Steam is running: Steam Input can remap, hide or duplicate controllers."
                .into(),
        });
    }
    if input_monitoring_access() == InputMonitoringAccess::Denied {
        out.push(PlatformWarning {
            code: "inputMonitoring",
            message: "macOS Input Monitoring permission is denied.".into(),
        });
    }
    if hidraw_needs_udev_rule() {
        out.push(PlatformWarning {
            code: "hidrawPermission",
            message: "No permission for hidraw: install the GamepadLab udev rule.".into(),
        });
    }
    if state.input.controllers().iter().any(|c| c.is_virtual) {
        out.push(PlatformWarning {
            code: "virtualDevice",
            message: "A virtual controller (Steam Input, ViGEm…) is connected.".into(),
        });
    }
    out
}

pub fn diagnostics(state: &AppState, version: String, ui_level: &str) -> String {
    let mut s = String::new();
    let info = app_info(state, version, false);
    s.push_str("GamepadLab diagnostics\n======================\n");
    s.push_str(&format!("version: {}\n", info.version));
    s.push_str(&format!(
        "os: {} {} ({})\n",
        info.os, info.os_version, info.arch
    ));
    s.push_str(&format!("backends: {}\n", info.backends.join(", ")));
    s.push_str(&format!("log level: {ui_level}\n"));
    if let Some(p) = crate::logging::log_path() {
        s.push_str(&format!("log file: {}\n", p.display()));
    }
    s.push_str(&format!(
        "input monitoring (macOS): {:?}\n",
        input_monitoring_access()
    ));
    s.push_str("\ncontrollers:\n");
    let controllers = state.input.controllers();
    if controllers.is_empty() {
        s.push_str("  (none)\n");
    }
    for c in controllers {
        s.push_str(&format!(
            "  - {} [{:04x}:{:04x}] kind={:?} via={} link={:?} buttons={} axes={} virtual={} demo={}\n",
            c.name,
            c.vendor_id,
            c.product_id,
            c.kind,
            c.backend,
            c.connection,
            c.button_count,
            c.axis_count,
            c.is_virtual,
            c.is_demo
        ));
        if let Some(snap) = state.input.latest(&c.id) {
            s.push_str(&format!(
                "    last report: seq={} L=({:+.3},{:+.3}) R=({:+.3},{:+.3}) LT={:.3} RT={:.3}\n",
                snap.seq,
                snap.left_stick.x,
                snap.left_stick.y,
                snap.right_stick.x,
                snap.right_stick.y,
                snap.left_trigger,
                snap.right_trigger
            ));
        }
    }
    s.push_str("\nwarnings:\n");
    let w = warnings(state);
    if w.is_empty() {
        s.push_str("  (none)\n");
    }
    for warning in w {
        s.push_str(&format!("  - {}: {}\n", warning.code, warning.message));
    }
    s.push_str("\nrecent errors:\n");
    let mut errors = state.input.recent_errors();
    errors.extend(crate::logging::recent());
    if errors.is_empty() {
        s.push_str("  (none)\n");
    }
    for e in errors.iter().rev().take(40) {
        s.push_str(&format!("  {e}\n"));
    }
    s.push_str("\nNo telemetry: this report is only copied to your clipboard.\n");
    s
}
