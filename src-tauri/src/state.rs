use crate::history::History;
use gamepadlab_calibration::{CalibrationProfile, Sample};
use gamepadlab_haptics::HapticsEngine;
use gamepadlab_input::InputService;
use gamepadlab_profiles::ProfileStore;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::{Mutex, RwLock};

/// (capture kind, side) → samples, e.g. ("rotation", "left").
pub type CaptureKey = (String, String);

/// Everything the commands share. Lives behind an `Arc` in Tauri state.
pub struct AppState {
    pub input: InputService,
    pub haptics: HapticsEngine,
    pub profiles: ProfileStore,
    pub ui_rate: AtomicU32,
    pub data_dir: PathBuf,
    /// Active calibration per connected controller id (drives the
    /// "calibrated" view).
    pub calibrations: RwLock<HashMap<String, CalibrationProfile>>,
    pub captures: Mutex<HashMap<CaptureKey, Vec<Sample>>>,
    pub pending_capture: Mutex<Option<CaptureKey>>,
    /// `None` when the database could not be opened (reported to the UI).
    pub history: Option<History>,
}
