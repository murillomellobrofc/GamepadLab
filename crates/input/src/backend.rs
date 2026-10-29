//! The `GamepadBackend` trait every input source implements.

use crate::model::{ControllerId, ControllerInfo, DeviceState};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

/// Something a backend observed since the previous `poll`.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    Connected(ControllerInfo),
    /// Updated metadata (battery, capabilities…) for an existing device.
    Updated(ControllerInfo),
    Disconnected(ControllerId),
    State(ControllerId, DeviceState),
    Error(InputError),
}

/// Output sent to a device. Higher-level effects (patterns, adaptive
/// trigger presets) live in the haptics crate and are encoded down to these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum OutputCommand {
    /// Motor intensities 0..1 (weak = high-frequency/right, strong = low-frequency/left).
    Rumble {
        weak: f32,
        strong: f32,
    },
    /// Impulse-trigger motors (Xbox One and later through WGI), 0..1.
    TriggerRumble {
        left: f32,
        right: f32,
    },
    /// DualSense adaptive trigger effect blocks (11 bytes each, see haptics crate).
    TriggerEffects {
        left: Vec<u8>,
        right: Vec<u8>,
    },
    Lightbar {
        r: u8,
        g: u8,
        b: u8,
    },
    /// Bit mask of the five DualSense player LEDs.
    PlayerLeds {
        mask: u8,
    },
}

/// Errors carry a machine-readable code so the UI can show a human message
/// that says what to do next.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum InputError {
    #[error("controller {0} is not connected")]
    Disconnected(String),
    #[error("permission denied while opening {0}")]
    PermissionDenied(String),
    #[error("driver missing or backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("this controller does not support {0}")]
    Unsupported(String),
    #[error("HID access failed: {0}")]
    Hid(String),
    /// A firmware calibration step was refused by a safety check.
    #[error("firmware calibration blocked: {0}")]
    FirmwareBlocked(String),
    /// The controller answered a firmware command unexpectedly.
    #[error("firmware calibration failed: {0}")]
    FirmwareProtocol(String),
    #[error("{0}")]
    Other(String),
}

impl InputError {
    pub fn code(&self) -> &'static str {
        match self {
            InputError::Disconnected(_) => "disconnected",
            InputError::PermissionDenied(_) => "permissionDenied",
            InputError::BackendUnavailable(_) => "backendUnavailable",
            InputError::Unsupported(_) => "rumbleUnsupported",
            InputError::Hid(_) => "hidAccess",
            InputError::FirmwareBlocked(_) => "firmwareBlocked",
            InputError::FirmwareProtocol(_) => "firmwareProtocol",
            InputError::Other(_) => "internal",
        }
    }
}

/// A source of controllers: gilrs (OS gamepad APIs), raw HID, the demo
/// simulator or a scripted mock used in tests.
pub trait GamepadBackend: Send {
    fn name(&self) -> &'static str;

    /// Higher wins when two backends report the same physical device.
    fn priority(&self) -> u8 {
        1
    }

    /// Non-blocking: append everything that happened since the last call.
    fn poll(&mut self, clock: &Clock, out: &mut Vec<BackendEvent>);

    fn send(&mut self, id: &ControllerId, cmd: &OutputCommand) -> Result<(), InputError>;

    /// Opt-in extended mode for a device owned by another backend (e.g.
    /// switching a Switch Pro controller to full report mode to read the IMU).
    /// Returns `Ok(true)` when this backend will take the device over.
    fn enable_extended(&mut self, _info: &ControllerInfo) -> Result<bool, InputError> {
        Ok(false)
    }

    /// Calibration stored in the controller itself (Sony, Switch Pro).
    fn firmware(
        &mut self,
        _id: &ControllerId,
        _op: &crate::firmware::FirmwareOp,
    ) -> Result<crate::firmware::FirmwareReply, InputError> {
        Err(InputError::Unsupported("firmware calibration".into()))
    }
}

/// Monotonic microsecond clock shared by all backends of one service.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    start: Instant,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn now_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    pub fn at_us(&self, instant: Instant) -> u64 {
        instant.saturating_duration_since(self.start).as_micros() as u64
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}
