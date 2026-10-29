//! Virtual controller with the calibration applied (future phase).
//!
//! The idea: GamepadLab reads the physical pad, applies the active profile
//! and republishes the result as a new system controller, so every game sees
//! the calibrated values. This module fixes the interface now; the
//! platform drivers are not implemented yet (see README, "Controle virtual").
//!
//! | Platform | Mechanism | What is missing |
//! |---|---|---|
//! | Windows | ViGEmBus (Xbox 360 / DS4 emulation) | ViGEmBus driver install + `vigem-client` bindings; hiding the physical pad (HidHide) to avoid double input |
//! | Linux | `/dev/uinput` | uinput device creation (e.g. `evdev` crate), the udev rule shipped in `packaging/linux`, EVIOCGRAB on the real device |
//! | macOS | DriverKit (HIDDriverKit virtual device) | a signed system extension with the `com.apple.developer.driverkit.family.hid.virtual.device` entitlement from Apple |

use crate::backend::InputError;
use crate::model::{ButtonState, BUTTON_COUNT};
use gamepadlab_calibration::Vec2;
use serde::{Deserialize, Serialize};

/// Which kind of pad the virtual device presents to games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VirtualLayout {
    Xbox360,
    DualShock4,
}

/// Calibrated state pushed to the virtual device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualState {
    pub buttons: Vec<ButtonState>,
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

impl Default for VirtualState {
    fn default() -> Self {
        Self {
            buttons: vec![ButtonState::default(); BUTTON_COUNT],
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            left_trigger: 0.0,
            right_trigger: 0.0,
        }
    }
}

/// A system-visible virtual controller.
pub trait VirtualDevice: Send {
    /// Human-readable backend name ("ViGEm", "uinput", "DriverKit"…).
    fn backend(&self) -> &'static str;
    /// Whether the platform pieces (driver, permissions) are present.
    fn available(&self) -> Result<(), InputError>;
    fn create(&mut self, layout: VirtualLayout) -> Result<(), InputError>;
    fn update(&mut self, state: &VirtualState) -> Result<(), InputError>;
    /// Rumble requested by the game, forwarded to the physical pad.
    fn take_rumble(&mut self) -> Option<(f32, f32)> {
        None
    }
    fn destroy(&mut self);
}

/// Placeholder until the platform drivers exist: always "not available",
/// with a message that says what is missing.
pub struct UnsupportedVirtualDevice {
    reason: &'static str,
}

impl VirtualDevice for UnsupportedVirtualDevice {
    fn backend(&self) -> &'static str {
        "none"
    }
    fn available(&self) -> Result<(), InputError> {
        Err(InputError::Unsupported(self.reason.into()))
    }
    fn create(&mut self, _layout: VirtualLayout) -> Result<(), InputError> {
        self.available()
    }
    fn update(&mut self, _state: &VirtualState) -> Result<(), InputError> {
        self.available()
    }
    fn destroy(&mut self) {}
}

/// The virtual device implementation for this platform.
pub fn platform_virtual_device() -> Box<dyn VirtualDevice> {
    let reason = if cfg!(target_os = "windows") {
        "virtual controller via ViGEmBus (planned)"
    } else if cfg!(target_os = "linux") {
        "virtual controller via uinput (planned)"
    } else if cfg!(target_os = "macos") {
        "virtual controller via DriverKit (planned, needs an Apple entitlement)"
    } else {
        "virtual controller on this platform"
    };
    Box::new(UnsupportedVirtualDevice { reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_reports_what_is_missing() {
        let mut d = platform_virtual_device();
        let err = d.create(VirtualLayout::Xbox360).unwrap_err();
        assert_eq!(err.code(), "rumbleUnsupported");
        assert!(err.to_string().contains("planned"));
        assert!(d.update(&VirtualState::default()).is_err());
        assert_eq!(d.take_rumble(), None);
        d.destroy();
    }
}
