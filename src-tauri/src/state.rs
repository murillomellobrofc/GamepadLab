use gamepadlab_input::InputService;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;

/// Everything the commands share. Lives behind an `Arc` in Tauri state.
pub struct AppState {
    pub input: InputService,
    pub ui_rate: AtomicU32,
    pub data_dir: PathBuf,
}
