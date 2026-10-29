//! Stick and trigger calibration math for GamepadLab.
//!
//! Pure functions over sample buffers: no I/O, no UI, no platform code, so
//! every analysis can be unit-tested with synthetic data.

pub mod geometry;

pub use geometry::Vec2;
