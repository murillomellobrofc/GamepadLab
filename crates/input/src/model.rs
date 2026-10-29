//! Core data model shared by every backend and sent to the UI.

use gamepadlab_calibration::Vec2;
use serde::{Deserialize, Serialize};

/// Stable identifier of a connected controller for the lifetime of the app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControllerId(pub String);

impl ControllerId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ControllerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControllerKind {
    Xbox,
    Ps4,
    Ps5,
    SwitchPro,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionType {
    Usb,
    Bluetooth,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub rumble: bool,
    pub trigger_rumble: bool,
    pub adaptive_triggers: bool,
    pub gyro: bool,
    pub accel: bool,
    pub touchpad: bool,
    pub battery: bool,
    pub lightbar: bool,
    pub player_leds: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatteryState {
    Discharging,
    Charging,
    Full,
    Wired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    /// Charge in percent, when the device reports it.
    pub level: Option<u8>,
    pub state: BatteryState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerInfo {
    pub id: ControllerId,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: Option<String>,
    pub kind: ControllerKind,
    pub connection: ConnectionType,
    pub button_count: u32,
    pub axis_count: u32,
    pub capabilities: Capabilities,
    pub battery: Option<BatteryInfo>,
    /// Which backend produced this device ("gilrs", "hid", "demo", "scripted").
    pub backend: String,
    /// Virtual pads created by other software (Steam Input, DS4Windows…).
    pub is_virtual: bool,
    pub is_demo: bool,
    /// Key used to store calibration profiles: `vvvv:pppp[:serial]`.
    pub profile_key: String,
}

impl ControllerInfo {
    pub fn make_profile_key(vendor_id: u16, product_id: u16, serial: Option<&str>) -> String {
        match serial.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => format!("{vendor_id:04x}:{product_id:04x}:{}", s.to_lowercase()),
            None => format!("{vendor_id:04x}:{product_id:04x}"),
        }
    }
}

/// Standard button order (W3C "standard" gamepad layout plus extras).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(usize)]
pub enum Button {
    South = 0,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Guide,
    Touchpad,
    Misc,
    Paddle1,
    Paddle2,
    Paddle3,
    Paddle4,
}

pub const BUTTON_COUNT: usize = 23;

impl Button {
    pub const ALL: [Button; BUTTON_COUNT] = [
        Button::South,
        Button::East,
        Button::West,
        Button::North,
        Button::LeftBumper,
        Button::RightBumper,
        Button::LeftTrigger,
        Button::RightTrigger,
        Button::Select,
        Button::Start,
        Button::LeftStick,
        Button::RightStick,
        Button::DpadUp,
        Button::DpadDown,
        Button::DpadLeft,
        Button::DpadRight,
        Button::Guide,
        Button::Touchpad,
        Button::Misc,
        Button::Paddle1,
        Button::Paddle2,
        Button::Paddle3,
        Button::Paddle4,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Option<Button> {
        Button::ALL.get(i).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ButtonState {
    pub pressed: bool,
    /// Analog value 0..1 (equals 0/1 for digital buttons).
    pub value: f32,
}

impl ButtonState {
    pub fn digital(pressed: bool) -> Self {
        Self {
            pressed,
            value: if pressed { 1.0 } else { 0.0 },
        }
    }
    pub fn analog(value: f32, threshold: f32) -> Self {
        Self {
            pressed: value >= threshold,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionSample {
    /// Angular velocity in degrees per second (pitch, yaw, roll as reported).
    pub gyro: [f32; 3],
    /// Acceleration in g.
    pub accel: [f32; 3],
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TouchPoint {
    pub id: u8,
    pub active: bool,
    /// Normalised 0..1, origin top-left.
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TouchState {
    pub pressed: bool,
    pub points: Vec<TouchPoint>,
}

/// One raw element exactly as the backend reports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAxisValue {
    pub code: u32,
    pub label: String,
    /// Normalised value as delivered by the OS/driver (-1..1).
    pub value: f32,
    /// Integer value straight from the HID report, when available.
    pub raw: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawButtonValue {
    pub code: u32,
    pub label: String,
    pub pressed: bool,
    pub value: f32,
}

/// Full device state right after one input report (or one group of OS
/// events sharing a timestamp).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    pub timestamp_us: u64,
    pub buttons: Vec<ButtonState>,
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub raw_axes: Vec<RawAxisValue>,
    pub raw_buttons: Vec<RawButtonValue>,
    pub motion: Option<MotionSample>,
    pub touch: Option<TouchState>,
    pub battery: Option<BatteryInfo>,
    /// `true` when every state corresponds to exactly one hardware report,
    /// which makes the measured interval the real polling interval.
    pub report_based: bool,
}

impl DeviceState {
    pub fn neutral(timestamp_us: u64) -> Self {
        Self {
            timestamp_us,
            buttons: vec![ButtonState::default(); BUTTON_COUNT],
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            left_trigger: 0.0,
            right_trigger: 0.0,
            raw_axes: Vec::new(),
            raw_buttons: Vec::new(),
            motion: None,
            touch: None,
            battery: None,
            report_based: false,
        }
    }

    pub fn set_button(&mut self, b: Button, state: ButtonState) {
        self.buttons[b.index()] = state;
    }

    pub fn button(&self, b: Button) -> ButtonState {
        self.buttons.get(b.index()).copied().unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAxis {
    pub code: u32,
    pub label: String,
    pub value: f32,
    pub raw: Option<i32>,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawButton {
    pub code: u32,
    pub label: String,
    pub pressed: bool,
    pub value: f32,
    pub max_value: f32,
    pub press_count: u32,
}

/// What the UI receives for a controller: state plus seen min/max.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSnapshot {
    pub id: ControllerId,
    pub seq: u64,
    pub timestamp_us: u64,
    pub buttons: Vec<ButtonState>,
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub raw_axes: Vec<RawAxis>,
    pub raw_buttons: Vec<RawButton>,
    /// Rising edges per standard button, counted at full report rate so a
    /// press shorter than one UI frame is never lost.
    pub button_presses: Vec<u32>,
    pub motion: Option<MotionSample>,
    pub touch: Option<TouchState>,
    pub battery: Option<BatteryInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_key_includes_serial_when_present() {
        assert_eq!(
            ControllerInfo::make_profile_key(0x054c, 0x0ce6, Some("AA:BB")),
            "054c:0ce6:aa:bb"
        );
        assert_eq!(
            ControllerInfo::make_profile_key(0x045e, 0x0b13, Some("  ")),
            "045e:0b13"
        );
        assert_eq!(
            ControllerInfo::make_profile_key(0x045e, 0x0b13, None),
            "045e:0b13"
        );
    }

    #[test]
    fn button_indices_match_order() {
        for (i, b) in Button::ALL.iter().enumerate() {
            assert_eq!(b.index(), i);
            assert_eq!(Button::from_index(i), Some(*b));
        }
        assert_eq!(Button::ALL.len(), BUTTON_COUNT);
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let s = DeviceState::neutral(5);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"leftStick\""));
        assert!(json.contains("\"timestampUs\":5"));
    }
}
