//! Controller detection, hotplug, normalisation and polling-rate measurement.
//!
//! Backends implement [`GamepadBackend`]; the [`InputService`] runs them on a
//! dedicated thread, hides duplicates through the [`registry`], and exposes
//! the latest [`InputSnapshot`] of each controller plus [`PollStats`].

pub mod backend;
pub mod demo;
pub mod firmware;
pub mod mapping;
pub mod model;
pub mod platform;
pub mod registry;
pub mod scripted;
pub mod service;
pub mod stats;
pub mod virtual_device;

#[cfg(feature = "gilrs")]
pub mod gilrs_backend;
#[cfg(feature = "hid")]
pub mod hid;

pub use backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
pub use demo::{DemoBackend, DemoScenario, ManualInput};
pub use firmware::{FirmwareOp, FirmwareReply, FirmwareStatus};
pub use model::*;
pub use service::{
    Capture, CaptureSample, CaptureTarget, FrameEntryRaw, HealthProgress, HealthRecording,
    InputService, ServiceConfig, ServiceEvent,
};
pub use stats::{IntervalSource, PollStats};
pub use virtual_device::{platform_virtual_device, VirtualDevice, VirtualLayout, VirtualState};
