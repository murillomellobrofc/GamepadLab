//! Platform checks that explain why a controller might be missing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputMonitoringAccess {
    Granted,
    Denied,
    Unknown,
    NotApplicable,
}

/// macOS: whether the app may read HID input (Privacy & Security >
/// Input Monitoring). Gamepads usually work without it, but some devices
/// that also expose keyboard usages are blocked until it is granted.
pub fn input_monitoring_access() -> InputMonitoringAccess {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "IOKit", kind = "framework")]
        extern "C" {
            fn IOHIDCheckAccess(request: u32) -> u32;
        }
        const LISTEN_EVENT: u32 = 1;
        // SAFETY: plain C call with an enum argument, no pointers involved.
        match unsafe { IOHIDCheckAccess(LISTEN_EVENT) } {
            0 => InputMonitoringAccess::Granted,
            1 => InputMonitoringAccess::Denied,
            _ => InputMonitoringAccess::Unknown,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        InputMonitoringAccess::NotApplicable
    }
}

/// Linux: true when a known controller's hidraw node exists but the
/// current user cannot open it (udev rule missing).
pub fn hidraw_needs_udev_rule() -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(entries) = std::fs::read_dir("/sys/class/hidraw") else {
            return false;
        };
        for entry in entries.flatten() {
            let uevent = entry.path().join("device/uevent");
            let Ok(text) = std::fs::read_to_string(uevent) else {
                continue;
            };
            let upper = text.to_uppercase();
            let known = [
                "054C:000005C4",
                "054C:000009CC",
                "054C:00000CE6",
                "054C:00000DF2",
            ]
            .iter()
            .any(|id| upper.contains(id));
            if known {
                let dev = std::path::Path::new("/dev").join(entry.file_name());
                if std::fs::OpenOptions::new().read(true).open(&dev).is_err() {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
