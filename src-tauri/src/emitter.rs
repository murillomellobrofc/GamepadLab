//! Bridges the input service to the webview: a frame emitter limited to the
//! UI rate (60/120 Hz) and a forwarder for connection and error events.

use crate::state::AppState;
use gamepadlab_calibration::Vec2;
use gamepadlab_input::{ButtonState, ControllerInfo, InputSnapshot, PollStats, ServiceEvent};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const EV_FRAME: &str = "gp://frame";
pub const EV_CONNECTION: &str = "gp://connection";
pub const EV_UPDATED: &str = "gp://controller-updated";
pub const EV_ERROR: &str = "gp://error";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalibratedView {
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub buttons: Vec<ButtonState>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FrameEntry {
    pub snapshot: InputSnapshot,
    pub calibrated: Option<CalibratedView>,
    pub stats: PollStats,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UiFrame {
    pub emitted_at_us: u64,
    pub entries: Vec<FrameEntry>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ConnectionPayload {
    controller: ControllerInfo,
    connected: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub controller_id: Option<String>,
}

pub fn spawn_frame_emitter(app: AppHandle, state: Arc<AppState>) {
    std::thread::Builder::new()
        .name("gamepadlab-frames".into())
        .spawn(move || {
            let start = Instant::now();
            let mut next = Instant::now();
            loop {
                let hz = state.ui_rate.load(Ordering::Relaxed).clamp(15, 240);
                let period = Duration::from_micros(1_000_000 / hz as u64);
                next += period;
                let now = Instant::now();
                if next > now {
                    std::thread::sleep(next - now);
                } else {
                    next = now; // fell behind; do not burst
                }
                let raw = state.input.take_frame();
                if raw.is_empty() {
                    continue;
                }
                let entries = raw
                    .into_iter()
                    .map(|e| FrameEntry {
                        calibrated: crate::calibrate::view(&state, &e.snapshot),
                        snapshot: e.snapshot,
                        stats: e.stats,
                    })
                    .collect();
                let frame = UiFrame {
                    emitted_at_us: start.elapsed().as_micros() as u64,
                    entries,
                };
                if let Err(e) = app.emit(EV_FRAME, &frame) {
                    log::debug!("frame emit failed: {e}");
                }
            }
        })
        .expect("spawn frame emitter");
}

pub fn spawn_event_forwarder(app: AppHandle, rx: Receiver<ServiceEvent>) {
    std::thread::Builder::new()
        .name("gamepadlab-events".into())
        .spawn(move || {
            for ev in rx {
                let result = match ev {
                    ServiceEvent::Connected(controller) => app.emit(
                        EV_CONNECTION,
                        ConnectionPayload {
                            controller,
                            connected: true,
                        },
                    ),
                    ServiceEvent::Disconnected(controller) => app.emit(
                        EV_CONNECTION,
                        ConnectionPayload {
                            controller,
                            connected: false,
                        },
                    ),
                    ServiceEvent::Updated(controller) => app.emit(EV_UPDATED, controller),
                    ServiceEvent::Error { controller, error } => app.emit(
                        EV_ERROR,
                        ErrorPayload {
                            code: error.code().to_string(),
                            message: error.to_string(),
                            controller_id: controller.map(|c| c.0),
                        },
                    ),
                };
                if let Err(e) = result {
                    log::warn!("event emit failed: {e}");
                }
            }
        })
        .expect("spawn event forwarder");
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamepadlab_input::{DemoBackend, DemoScenario, PollStats};

    /// The UI's TypeScript types (apps/ui/src/api/types.ts) rely on these
    /// exact camelCase field names.
    #[test]
    fn frame_json_matches_ui_contract() {
        let mut demo = DemoBackend::new();
        demo.set_scenario(DemoScenario::Showcase, 0);
        let st = demo.state_at(1_000);
        let snapshot = InputSnapshot {
            id: DemoBackend::id(),
            seq: 1,
            timestamp_us: st.timestamp_us,
            buttons: st.buttons.clone(),
            left_stick: st.left_stick,
            right_stick: st.right_stick,
            left_trigger: st.left_trigger,
            right_trigger: st.right_trigger,
            raw_axes: vec![],
            raw_buttons: vec![],
            motion: st.motion,
            touch: st.touch.clone(),
            battery: st.battery,
        };
        let frame = UiFrame {
            emitted_at_us: 5,
            entries: vec![FrameEntry {
                snapshot,
                calibrated: None,
                stats: PollStats::default(),
            }],
        };
        let v = serde_json::to_value(&frame).unwrap();
        let e = &v["entries"][0];
        for key in [
            "id",
            "seq",
            "timestampUs",
            "buttons",
            "leftStick",
            "rightStick",
            "leftTrigger",
            "rightTrigger",
            "rawAxes",
            "rawButtons",
            "motion",
            "touch",
            "battery",
        ] {
            assert!(e["snapshot"].get(key).is_some(), "snapshot.{key}");
        }
        for key in [
            "hz",
            "avgMs",
            "minMs",
            "maxMs",
            "jitterMs",
            "p99Ms",
            "recentIntervalsUs",
            "source",
        ] {
            assert!(e["stats"].get(key).is_some(), "stats.{key}");
        }
        assert_eq!(e["stats"]["source"], "change");
        assert!(e["snapshot"]["motion"]["timestampUs"].is_number());
        assert_eq!(e["snapshot"]["battery"]["state"], "discharging");
        assert!(v.get("emittedAtUs").is_some());
        let info = serde_json::to_value(demo.info()).unwrap();
        for key in [
            "vendorId",
            "productId",
            "profileKey",
            "isDemo",
            "isVirtual",
            "buttonCount",
        ] {
            assert!(info.get(key).is_some(), "info.{key}");
        }
        assert_eq!(info["kind"], "ps5");
        assert_eq!(info["capabilities"]["adaptiveTriggers"], true);
        let sc: DemoScenario = serde_json::from_str(r#"{"pressButton":3}"#).unwrap();
        assert_eq!(sc, DemoScenario::PressButton(3));
        let sc: DemoScenario = serde_json::from_str(r#""rotateLeft""#).unwrap();
        assert_eq!(sc, DemoScenario::RotateLeft);
    }
}
