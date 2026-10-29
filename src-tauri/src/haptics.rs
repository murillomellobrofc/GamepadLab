//! Connects the haptics engine to the input service and mirrors the current
//! motor intensity to the UI (visual feedback for every vibration).

use crate::state::AppState;
use gamepadlab_haptics::{HapticsEngine, RumbleObserver, RumbleSink, RumbleState};
use gamepadlab_input::{ControllerId, InputService, OutputCommand};
use serde::Serialize;
use std::sync::{Arc, Weak};
use tauri::{AppHandle, Emitter, Runtime};

pub const EV_RUMBLE: &str = "gp://rumble";

/// Late-bound sink: the engine is created before `AppState` exists.
pub struct ServiceSink(pub std::sync::OnceLock<Weak<AppState>>);

impl RumbleSink for ServiceSink {
    fn set(&self, controller: &str, weak: f32, strong: f32) -> Result<(), String> {
        let state = self
            .0
            .get()
            .and_then(Weak::upgrade)
            .ok_or_else(|| "app not ready".to_string())?;
        send(&state.input, controller, weak, strong)
    }
}

fn send(input: &InputService, controller: &str, weak: f32, strong: f32) -> Result<(), String> {
    input
        .send_output(
            &ControllerId::new(controller),
            OutputCommand::Rumble { weak, strong },
        )
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RumblePayload {
    controller_id: String,
    weak: f32,
    strong: f32,
    active: bool,
}

pub fn start<R: Runtime>(app: AppHandle<R>, sink: Arc<ServiceSink>) -> HapticsEngine {
    let observer: RumbleObserver = Arc::new(move |id: &str, st: RumbleState| {
        let _ = app.emit(
            EV_RUMBLE,
            RumblePayload {
                controller_id: id.to_string(),
                weak: st.weak,
                strong: st.strong,
                active: st.active,
            },
        );
    });
    HapticsEngine::start(sink, observer)
}
