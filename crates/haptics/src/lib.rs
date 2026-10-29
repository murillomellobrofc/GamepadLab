//! Rumble patterns, motor tests and DualSense adaptive-trigger effects.
//!
//! Everything here is pure (pattern math, byte encoding) except the
//! [`engine::HapticsEngine`], which plays programs on a timer thread through
//! a [`engine::RumbleSink`] supplied by the app.

pub mod engine;
pub mod pattern;
pub mod triggers;

pub use engine::{HapticsEngine, RumbleObserver, RumbleSink, RumbleState};
pub use pattern::{motor_test_program, PatternKind, RumbleRequest};
pub use triggers::TriggerEffect;
