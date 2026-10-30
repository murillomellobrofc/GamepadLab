//! Stick and trigger calibration math for GamepadLab.
//!
//! Pure functions over sample buffers: no I/O, no UI, no platform code, so
//! every analysis can be unit-tested with synthetic data.
//!
//! * [`rest`]: centre offset, noise and drift while released
//! * [`circularity`]: reach per sector during a full rotation
//! * [`directions`]: reach and aim in 8 directions, gate shape
//! * [`spikes`]: spikes, jumps and potentiometer noise
//! * [`trigger`]: rest/full travel of analog triggers
//! * [`report`]: quality score, suggested calibration, before/after
//! * [`pipeline`]: applying a calibration (deadzones, curves, gates…)
//! * [`button_health`]: chatter, missed presses, stuck buttons and release bounce

pub mod button_health;
pub mod circularity;
pub mod curve;
pub mod directions;
pub mod geometry;
pub mod pipeline;
pub mod profile;
pub mod report;
pub mod rest;
pub mod sample;
pub mod spikes;
pub mod stats;
pub mod trigger;

pub use circularity::{analyze_rotation, CircularityAnalysis, Sector, SectorStatus};
pub use curve::{CurveKind, ResponseCurve};
pub use directions::{analyze_directions, DirectionResult, DirectionsAnalysis};
pub use geometry::Vec2;
pub use profile::{
    CalibrationProfile, DeadzoneMode, GateShape, Invalid, StickCalibration, TriggerCalibration,
};
pub use report::{build_report, score_stick, suggest, QualityScore, StickReport, StickSide};
pub use rest::{analyze_rest, DriftGrade, RestAnalysis};
pub use sample::Sample;
pub use spikes::{detect_spikes, NoiseVerdict, SpikeReport};
pub use trigger::{analyze_trigger, TriggerAnalysis};
