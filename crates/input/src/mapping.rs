//! Vendor/product tables: controller kind, display names, capabilities.

use crate::model::{Capabilities, ControllerKind};

pub const VID_SONY: u16 = 0x054c;
pub const VID_MICROSOFT: u16 = 0x045e;
pub const VID_NINTENDO: u16 = 0x057e;
pub const VID_VALVE: u16 = 0x28de;

pub const PID_DS4_V1: u16 = 0x05c4;
pub const PID_DS4_V2: u16 = 0x09cc;
pub const PID_DS4_DONGLE: u16 = 0x0ba0;
pub const PID_DUALSENSE: u16 = 0x0ce6;
pub const PID_DUALSENSE_EDGE: u16 = 0x0df2;
pub const PID_SWITCH_PRO: u16 = 0x2009;
pub const PID_STEAM_VIRTUAL: u16 = 0x11ff;

pub fn is_dualshock4(vid: u16, pid: u16) -> bool {
    vid == VID_SONY && matches!(pid, PID_DS4_V1 | PID_DS4_V2 | PID_DS4_DONGLE)
}

pub fn is_dualsense(vid: u16, pid: u16) -> bool {
    vid == VID_SONY && matches!(pid, PID_DUALSENSE | PID_DUALSENSE_EDGE)
}

pub fn is_switch_pro(vid: u16, pid: u16) -> bool {
    vid == VID_NINTENDO && pid == PID_SWITCH_PRO
}

/// Detects the controller family from VID/PID, falling back to the name.
pub fn detect_kind(vid: u16, pid: u16, name: &str) -> ControllerKind {
    if is_dualsense(vid, pid) {
        return ControllerKind::Ps5;
    }
    if is_dualshock4(vid, pid) {
        return ControllerKind::Ps4;
    }
    if is_switch_pro(vid, pid) {
        return ControllerKind::SwitchPro;
    }
    if vid == VID_MICROSOFT {
        return ControllerKind::Xbox;
    }
    let n = name.to_lowercase();
    if n.contains("dualsense") {
        ControllerKind::Ps5
    } else if n.contains("dualshock") || (vid == VID_SONY && n.contains("wireless controller")) {
        ControllerKind::Ps4
    } else if n.contains("pro controller") {
        ControllerKind::SwitchPro
    } else if n.contains("xbox") || n.contains("xinput") {
        ControllerKind::Xbox
    } else {
        ControllerKind::Generic
    }
}

/// Virtual devices created by remappers. They mirror a real pad, so the UI
/// flags them instead of hiding them.
pub fn is_virtual(vid: u16, pid: u16, name: &str) -> bool {
    (vid == VID_VALVE && pid == PID_STEAM_VIRTUAL)
        || name.to_lowercase().contains("steam virtual")
        || name.to_lowercase().contains("vigem")
}

pub fn friendly_name(vid: u16, pid: u16, os_name: &str) -> String {
    let known = match (vid, pid) {
        (VID_SONY, PID_DUALSENSE) => Some("DualSense"),
        (VID_SONY, PID_DUALSENSE_EDGE) => Some("DualSense Edge"),
        (VID_SONY, PID_DS4_V1) => Some("DualShock 4 (v1)"),
        (VID_SONY, PID_DS4_V2) => Some("DualShock 4 (v2)"),
        (VID_SONY, PID_DS4_DONGLE) => Some("DualShock 4 (adaptador USB)"),
        (VID_NINTENDO, PID_SWITCH_PRO) => Some("Switch Pro Controller"),
        _ => None,
    };
    match known {
        Some(k) => k.to_string(),
        None if os_name.trim().is_empty() => format!("Controle {vid:04x}:{pid:04x}"),
        None => os_name.trim().to_string(),
    }
}

/// Capabilities available when the device is driven through raw HID.
pub fn hid_capabilities(kind: ControllerKind) -> Capabilities {
    match kind {
        ControllerKind::Ps5 => Capabilities {
            rumble: true,
            trigger_rumble: false,
            adaptive_triggers: true,
            gyro: true,
            accel: true,
            touchpad: true,
            battery: true,
            lightbar: true,
            player_leds: true,
        },
        ControllerKind::Ps4 => Capabilities {
            rumble: true,
            gyro: true,
            accel: true,
            touchpad: true,
            battery: true,
            lightbar: true,
            ..Capabilities::default()
        },
        ControllerKind::SwitchPro => Capabilities {
            gyro: true,
            accel: true,
            battery: true,
            ..Capabilities::default()
        },
        _ => Capabilities::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_ids() {
        assert_eq!(detect_kind(0x054c, 0x0ce6, ""), ControllerKind::Ps5);
        assert_eq!(detect_kind(0x054c, 0x0df2, ""), ControllerKind::Ps5);
        assert_eq!(detect_kind(0x054c, 0x09cc, ""), ControllerKind::Ps4);
        assert_eq!(detect_kind(0x057e, 0x2009, ""), ControllerKind::SwitchPro);
        assert_eq!(detect_kind(0x045e, 0x0b13, ""), ControllerKind::Xbox);
    }

    #[test]
    fn detects_by_name() {
        assert_eq!(
            detect_kind(0, 0, "Xbox Wireless Controller"),
            ControllerKind::Xbox
        );
        assert_eq!(
            detect_kind(0, 0, "DualSense Wireless Controller"),
            ControllerKind::Ps5
        );
        assert_eq!(
            detect_kind(0x0810, 0x0001, "Twin USB Joystick"),
            ControllerKind::Generic
        );
    }

    #[test]
    fn flags_steam_virtual_pad() {
        assert!(is_virtual(0x28de, 0x11ff, "Steam Virtual Gamepad"));
        assert!(!is_virtual(0x045e, 0x0b13, "Xbox Wireless Controller"));
    }

    #[test]
    fn friendly_names() {
        assert_eq!(
            friendly_name(0x054c, 0x0ce6, "Wireless Controller"),
            "DualSense"
        );
        assert_eq!(friendly_name(0x1234, 0x5678, "  "), "Controle 1234:5678");
        assert_eq!(
            friendly_name(0x045e, 0x0b13, "Xbox Wireless Controller"),
            "Xbox Wireless Controller"
        );
    }
}
