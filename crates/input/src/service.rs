//! The input service: one dedicated thread that polls every backend,
//! normalises states, tracks min/max, measures polling rate and records
//! calibration captures at full rate. The UI side only reads snapshots.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::demo::{DemoBackend, DemoScenario, ManualInput, DEMO_ID};
use crate::model::*;
use crate::registry::{Change, Registry};
use crate::stats::{PollMeter, PollStats};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

const MAX_CAPTURE_SAMPLES: usize = 200_000;
const MAX_RECENT_ERRORS: usize = 50;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub poll_interval: Duration,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_micros(750),
        }
    }
}

/// Notifications for the UI layer (connection changes and errors).
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceEvent {
    Connected(ControllerInfo),
    Updated(ControllerInfo),
    Disconnected(ControllerInfo),
    Error {
        controller: Option<ControllerId>,
        error: InputError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureTarget {
    LeftStick,
    RightStick,
    LeftTrigger,
    RightTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureSample {
    /// Microseconds since the capture started.
    pub t: u64,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub controller: ControllerId,
    pub target: CaptureTarget,
    pub samples: Vec<CaptureSample>,
    pub duration_ms: f32,
}

#[derive(Debug, Clone)]
pub struct FrameEntryRaw {
    pub snapshot: InputSnapshot,
    pub stats: PollStats,
}

enum Command {
    Send(ControllerId, OutputCommand, Sender<Result<(), InputError>>),
    SetDemo(bool),
    DemoScenario(DemoScenario),
    DemoManual(ManualInput),
    ResetMinMax(ControllerId),
    CaptureStart(ControllerId, CaptureTarget),
    CaptureStop(Sender<Option<Capture>>),
    EnableExtended(ControllerId, Sender<Result<bool, InputError>>),
    Shutdown,
}

struct LiveSlot {
    snapshot: InputSnapshot,
    meter: PollMeter,
}

#[derive(Default)]
struct Shared {
    live: Mutex<HashMap<ControllerId, LiveSlot>>,
    controllers: RwLock<Vec<ControllerInfo>>,
    recent_errors: Mutex<VecDeque<String>>,
    backend_names: RwLock<Vec<String>>,
}

pub struct InputService {
    tx: Sender<Command>,
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl InputService {
    /// Starts the service thread with the given backends.
    pub fn start(
        config: ServiceConfig,
        backends: Vec<Box<dyn GamepadBackend>>,
        events: Sender<ServiceEvent>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        *shared.backend_names.write().unwrap() = backends
            .iter()
            .map(|b| b.name().to_string())
            .chain(["demo".into()])
            .collect();
        let thread_shared = shared.clone();
        let handle = std::thread::Builder::new()
            .name("gamepadlab-input".into())
            .spawn(move || run(config, backends, rx, events, thread_shared))
            .expect("spawn input thread");
        Self {
            tx,
            shared,
            handle: Some(handle),
        }
    }

    /// Starts with every backend compiled in for this platform. Backends that
    /// fail to initialise are reported as errors and skipped.
    pub fn start_default(events: Sender<ServiceEvent>) -> Self {
        let mut backends: Vec<Box<dyn GamepadBackend>> = Vec::new();
        #[cfg(feature = "gilrs")]
        match crate::gilrs_backend::GilrsBackend::new() {
            Ok(b) => backends.push(Box::new(b)),
            Err(error) => {
                log::error!("gilrs backend unavailable: {error}");
                let _ = events.send(ServiceEvent::Error {
                    controller: None,
                    error,
                });
            }
        }
        #[cfg(feature = "hid")]
        match crate::hid::HidBackend::new() {
            Ok(b) => backends.push(Box::new(b)),
            Err(error) => {
                log::warn!("hid backend unavailable: {error}");
                let _ = events.send(ServiceEvent::Error {
                    controller: None,
                    error,
                });
            }
        }
        Self::start(ServiceConfig::default(), backends, events)
    }

    pub fn controllers(&self) -> Vec<ControllerInfo> {
        self.shared.controllers.read().unwrap().clone()
    }

    pub fn backend_names(&self) -> Vec<String> {
        self.shared.backend_names.read().unwrap().clone()
    }

    pub fn recent_errors(&self) -> Vec<String> {
        self.shared
            .recent_errors
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    /// Latest snapshot + polling stats of every visible controller. Drains
    /// the per-frame interval lists, so call it once per UI frame.
    pub fn take_frame(&self) -> Vec<FrameEntryRaw> {
        let order = self.controllers();
        let mut live = self.shared.live.lock().unwrap();
        let mut frame = Vec::with_capacity(order.len());
        for c in &order {
            if let Some(slot) = live.get_mut(&c.id) {
                frame.push(FrameEntryRaw {
                    snapshot: slot.snapshot.clone(),
                    stats: slot.meter.take_stats(),
                });
            }
        }
        frame
    }

    pub fn latest(&self, id: &ControllerId) -> Option<InputSnapshot> {
        self.shared
            .live
            .lock()
            .unwrap()
            .get(id)
            .map(|s| s.snapshot.clone())
    }

    pub fn send_output(&self, id: &ControllerId, cmd: OutputCommand) -> Result<(), InputError> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::Send(id.clone(), cmd, tx))
            .map_err(|_| InputError::Other("input service stopped".into()))?;
        rx.recv_timeout(Duration::from_millis(500))
            .unwrap_or_else(|_| Err(InputError::Other("input service timeout".into())))
    }

    pub fn set_demo(&self, enabled: bool) {
        let _ = self.tx.send(Command::SetDemo(enabled));
    }

    pub fn demo_scenario(&self, scenario: DemoScenario) {
        let _ = self.tx.send(Command::DemoScenario(scenario));
    }

    pub fn demo_manual(&self, input: ManualInput) {
        let _ = self.tx.send(Command::DemoManual(input));
    }

    pub fn reset_minmax(&self, id: &ControllerId) {
        let _ = self.tx.send(Command::ResetMinMax(id.clone()));
    }

    pub fn capture_start(&self, id: &ControllerId, target: CaptureTarget) {
        let _ = self.tx.send(Command::CaptureStart(id.clone(), target));
    }

    pub fn capture_stop(&self) -> Option<Capture> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::CaptureStop(tx)).ok()?;
        rx.recv_timeout(Duration::from_secs(2)).ok().flatten()
    }

    pub fn enable_extended(&self, id: &ControllerId) -> Result<bool, InputError> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::EnableExtended(id.clone(), tx))
            .map_err(|_| InputError::Other("input service stopped".into()))?;
        rx.recv_timeout(Duration::from_secs(3))
            .unwrap_or_else(|_| Err(InputError::Other("input service timeout".into())))
    }
}

impl Drop for InputService {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Per-controller min/max and press bookkeeping.
#[derive(Default)]
struct Tracker {
    seq: u64,
    prev_pressed: Vec<bool>,
    presses: Vec<u32>,
    axis_range: HashMap<u32, (f32, f32)>,
    button_max: HashMap<u32, f32>,
    button_presses: HashMap<u32, (bool, u32)>,
}

impl Tracker {
    fn snapshot(&mut self, id: &ControllerId, st: &DeviceState) -> InputSnapshot {
        self.seq += 1;
        if self.presses.len() < st.buttons.len() {
            self.presses.resize(st.buttons.len(), 0);
            self.prev_pressed.resize(st.buttons.len(), false);
        }
        for (i, b) in st.buttons.iter().enumerate() {
            if b.pressed && !self.prev_pressed[i] {
                self.presses[i] += 1;
            }
            self.prev_pressed[i] = b.pressed;
        }
        let raw_axes = st
            .raw_axes
            .iter()
            .map(|a| {
                let e = self.axis_range.entry(a.code).or_insert((a.value, a.value));
                e.0 = e.0.min(a.value);
                e.1 = e.1.max(a.value);
                RawAxis {
                    code: a.code,
                    label: a.label.clone(),
                    value: a.value,
                    raw: a.raw,
                    min: e.0,
                    max: e.1,
                }
            })
            .collect();
        let raw_buttons = st
            .raw_buttons
            .iter()
            .map(|b| {
                let max = self.button_max.entry(b.code).or_insert(0.0);
                *max = max.max(b.value);
                let presses = self.button_presses.entry(b.code).or_insert((false, 0));
                if b.pressed && !presses.0 {
                    presses.1 += 1;
                }
                presses.0 = b.pressed;
                RawButton {
                    code: b.code,
                    label: b.label.clone(),
                    pressed: b.pressed,
                    value: b.value,
                    max_value: *max,
                    press_count: presses.1,
                }
            })
            .collect();
        InputSnapshot {
            id: id.clone(),
            seq: self.seq,
            timestamp_us: st.timestamp_us,
            buttons: st.buttons.clone(),
            left_stick: st.left_stick,
            right_stick: st.right_stick,
            left_trigger: st.left_trigger,
            right_trigger: st.right_trigger,
            raw_axes,
            raw_buttons,
            button_presses: self.presses.clone(),
            motion: st.motion,
            touch: st.touch.clone(),
            battery: st.battery,
        }
    }
}

struct CaptureState {
    id: ControllerId,
    target: CaptureTarget,
    started_us: Option<u64>,
    samples: Vec<CaptureSample>,
}

impl CaptureState {
    fn push(&mut self, st: &DeviceState) {
        let start = *self.started_us.get_or_insert(st.timestamp_us);
        if self.samples.len() >= MAX_CAPTURE_SAMPLES {
            return;
        }
        let (x, y) = match self.target {
            CaptureTarget::LeftStick => (st.left_stick.x, st.left_stick.y),
            CaptureTarget::RightStick => (st.right_stick.x, st.right_stick.y),
            CaptureTarget::LeftTrigger => (st.left_trigger, 0.0),
            CaptureTarget::RightTrigger => (st.right_trigger, 0.0),
        };
        self.samples.push(CaptureSample {
            t: st.timestamp_us.saturating_sub(start),
            x,
            y,
        });
    }

    fn finish(self) -> Capture {
        let duration_ms = self
            .samples
            .last()
            .map(|s| s.t as f32 / 1000.0)
            .unwrap_or(0.0);
        Capture {
            controller: self.id,
            target: self.target,
            samples: self.samples,
            duration_ms,
        }
    }
}

fn run(
    config: ServiceConfig,
    mut backends: Vec<Box<dyn GamepadBackend>>,
    rx: Receiver<Command>,
    events: Sender<ServiceEvent>,
    shared: Arc<Shared>,
) {
    let clock = Clock::new();
    let mut demo = DemoBackend::new();
    let mut registry = Registry::new();
    let mut owners: HashMap<ControllerId, usize> = HashMap::new();
    let mut trackers: HashMap<ControllerId, Tracker> = HashMap::new();
    let mut capture: Option<CaptureState> = None;
    let mut buf: Vec<BackendEvent> = Vec::with_capacity(256);
    const DEMO_OWNER: usize = usize::MAX;

    let record_error = |shared: &Shared, msg: String| {
        let mut q = shared.recent_errors.lock().unwrap();
        if q.len() == MAX_RECENT_ERRORS {
            q.pop_front();
        }
        q.push_back(msg);
    };

    log::info!(
        "input service started with backends: {:?}",
        shared.backend_names.read().unwrap()
    );

    loop {
        // 1. Commands from the UI side.
        loop {
            let cmd = match rx.try_recv() {
                Ok(c) => c,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            };
            match cmd {
                Command::Shutdown => {
                    log::info!("input service stopping");
                    return;
                }
                Command::Send(id, out, reply) => {
                    let result = if id.as_str() == DEMO_ID {
                        demo.send(&id, &out)
                    } else {
                        match owners.get(&id).and_then(|&i| backends.get_mut(i)) {
                            Some(b) => b.send(&id, &out),
                            None => Err(InputError::Disconnected(id.to_string())),
                        }
                    };
                    let _ = reply.send(result);
                }
                Command::SetDemo(on) => demo.set_active(on),
                Command::DemoScenario(s) => demo.set_scenario(s, clock.now_us()),
                Command::DemoManual(m) => demo.set_manual(m),
                Command::ResetMinMax(id) => {
                    trackers.remove(&id);
                }
                Command::CaptureStart(id, target) => {
                    capture = Some(CaptureState {
                        id,
                        target,
                        started_us: None,
                        samples: Vec::with_capacity(4096),
                    });
                }
                Command::CaptureStop(reply) => {
                    let _ = reply.send(capture.take().map(CaptureState::finish));
                }
                Command::EnableExtended(id, reply) => {
                    let result = match registry.get(&id).cloned() {
                        None => Err(InputError::Disconnected(id.to_string())),
                        Some(info) => {
                            let mut taken = Ok(false);
                            for b in backends.iter_mut() {
                                match b.enable_extended(&info) {
                                    Ok(true) => {
                                        taken = Ok(true);
                                        break;
                                    }
                                    Ok(false) => {}
                                    Err(e) => taken = Err(e),
                                }
                            }
                            taken
                        }
                    };
                    let _ = reply.send(result);
                }
            }
        }

        // 2. Poll every backend.
        let mut tagged: Vec<(usize, u8, BackendEvent)> = Vec::new();
        for (i, b) in backends.iter_mut().enumerate() {
            buf.clear();
            b.poll(&clock, &mut buf);
            let p = b.priority();
            tagged.extend(buf.drain(..).map(|e| (i, p, e)));
        }
        buf.clear();
        demo.poll(&clock, &mut buf);
        tagged.extend(buf.drain(..).map(|e| (DEMO_OWNER, 0, e)));

        // 3. Apply events.
        let mut list_changed = false;
        for (owner, priority, ev) in tagged {
            match ev {
                BackendEvent::Connected(info) => {
                    log::info!(
                        "connected: {} [{:04x}:{:04x}] via {}",
                        info.name,
                        info.vendor_id,
                        info.product_id,
                        info.backend
                    );
                    owners.insert(info.id.clone(), owner);
                    let changes = registry.add(info, priority);
                    list_changed |= !changes.is_empty();
                    apply_changes(changes, &events, &shared, &mut trackers);
                }
                BackendEvent::Updated(info) => {
                    if let Some(visible) = registry.update(info) {
                        list_changed = true;
                        let _ = events.send(ServiceEvent::Updated(visible));
                    }
                }
                BackendEvent::Disconnected(id) => {
                    log::info!("disconnected: {id}");
                    owners.remove(&id);
                    if capture.as_ref().is_some_and(|c| c.id == id) {
                        let _ = events.send(ServiceEvent::Error {
                            controller: Some(id.clone()),
                            error: InputError::Disconnected(id.to_string()),
                        });
                    }
                    let changes = registry.remove(&id);
                    list_changed |= !changes.is_empty();
                    apply_changes(changes, &events, &shared, &mut trackers);
                    trackers.remove(&id);
                }
                BackendEvent::State(id, st) => {
                    if !registry.is_visible(&id) {
                        continue;
                    }
                    if let Some(c) = capture.as_mut().filter(|c| c.id == id) {
                        c.push(&st);
                    }
                    let snap = trackers.entry(id.clone()).or_default().snapshot(&id, &st);
                    let mut live = shared.live.lock().unwrap();
                    let slot = live.entry(id).or_insert_with(|| LiveSlot {
                        snapshot: snap.clone(),
                        meter: PollMeter::new(),
                    });
                    slot.meter.record(st.timestamp_us, st.report_based);
                    slot.snapshot = snap;
                }
                BackendEvent::Error(error) => {
                    log::warn!("backend error: {error}");
                    record_error(&shared, error.to_string());
                    let _ = events.send(ServiceEvent::Error {
                        controller: None,
                        error,
                    });
                }
            }
        }
        if list_changed {
            *shared.controllers.write().unwrap() = registry.visible();
        }

        std::thread::sleep(config.poll_interval);
    }
}

fn apply_changes(
    changes: Vec<Change>,
    events: &Sender<ServiceEvent>,
    shared: &Shared,
    trackers: &mut HashMap<ControllerId, Tracker>,
) {
    for change in changes {
        match change {
            Change::Shown(info) => {
                let _ = events.send(ServiceEvent::Connected(info));
            }
            Change::Hidden(info) => {
                shared.live.lock().unwrap().remove(&info.id);
                trackers.remove(&info.id);
                let _ = events.send(ServiceEvent::Disconnected(info));
            }
        }
    }
}
