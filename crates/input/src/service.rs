//! The input service: one dedicated thread that polls every backend,
//! normalises states, tracks min/max, measures polling rate and records
//! calibration captures and button-health sessions at full rate. The UI side
//! only reads snapshots.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::demo::{DemoBackend, DemoScenario, ManualInput, DEMO_ID};
use crate::firmware::{FirmwareOp, FirmwareReply};
use crate::model::*;
use crate::registry::{Change, Registry};
use crate::stats::{PollMeter, PollStats};
use gamepadlab_calibration::button_health::{live_status, ButtonEvent, LiveButton, Precision};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

const MAX_CAPTURE_SAMPLES: usize = 200_000;
const MAX_HEALTH_EVENTS: usize = 200_000;
/// Report intervals kept to estimate the report rate of a health session.
const HEALTH_INTERVALS: usize = 512;
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

/// Captured reading (`t` = µs since the capture started); shared with the
/// calibration crate so captures feed the analyses directly.
pub use gamepadlab_calibration::Sample as CaptureSample;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub controller: ControllerId,
    pub target: CaptureTarget,
    pub samples: Vec<CaptureSample>,
    pub duration_ms: f32,
}

/// Button edges recorded for a button-health session (`t_us` from its start).
#[derive(Debug, Clone)]
pub struct HealthRecording {
    pub controller: ControllerId,
    pub precision: Precision,
    pub duration_us: u64,
    pub events: Vec<ButtonEvent>,
}

/// Live view of a running button-health session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthProgress {
    pub controller: ControllerId,
    pub precision: Precision,
    pub elapsed_ms: f32,
    pub buttons: Vec<LiveButton>,
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
    HealthStart(ControllerId),
    HealthProgress(Sender<Option<HealthProgress>>),
    HealthStop(Sender<Option<HealthRecording>>),
    EnableExtended(ControllerId, Sender<Result<bool, InputError>>),
    Firmware(
        ControllerId,
        FirmwareOp,
        Sender<Result<FirmwareReply, InputError>>,
    ),
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

    /// Runs a firmware calibration step on the controller (blocking, ≤ 25 s).
    /// Starts recording every button edge of `id` at full rate (replaces any
    /// running session). Read-only: nothing is sent to the controller.
    pub fn health_start(&self, id: &ControllerId) {
        let _ = self.tx.send(Command::HealthStart(id.clone()));
    }

    pub fn health_progress(&self) -> Option<HealthProgress> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::HealthProgress(tx)).ok()?;
        rx.recv_timeout(Duration::from_secs(2)).ok().flatten()
    }

    /// Ends the session and hands over what was recorded.
    pub fn health_stop(&self) -> Option<HealthRecording> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::HealthStop(tx)).ok()?;
        rx.recv_timeout(Duration::from_secs(2)).ok().flatten()
    }

    pub fn firmware(&self, id: &ControllerId, op: FirmwareOp) -> Result<FirmwareReply, InputError> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Command::Firmware(id.clone(), op, tx))
            .map_err(|_| InputError::Other("input service stopped".into()))?;
        rx.recv_timeout(Duration::from_secs(25))
            .unwrap_or_else(|_| Err(InputError::Other("input service timeout".into())))
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

/// A running button-health session: edges of every button of one controller.
struct HealthState {
    id: ControllerId,
    started_us: Option<u64>,
    last_us: u64,
    prev: Vec<bool>,
    events: Vec<ButtonEvent>,
    /// False as soon as one state came from an event-based source (gilrs).
    report_based: bool,
    intervals: VecDeque<u64>,
}

impl HealthState {
    fn new(id: ControllerId) -> Self {
        Self {
            id,
            started_us: None,
            last_us: 0,
            prev: Vec::new(),
            events: Vec::new(),
            report_based: true,
            intervals: VecDeque::with_capacity(HEALTH_INTERVALS),
        }
    }

    fn push(&mut self, st: &DeviceState) {
        let pressed: Vec<bool> = st.buttons.iter().map(|b| b.pressed).collect();
        self.report_based &= st.report_based;
        let Some(start) = self.started_us else {
            // The first state is the baseline: buttons already held are not edges.
            self.started_us = Some(st.timestamp_us);
            self.last_us = st.timestamp_us;
            self.prev = pressed;
            return;
        };
        if self.intervals.len() == HEALTH_INTERVALS {
            self.intervals.pop_front();
        }
        self.intervals
            .push_back(st.timestamp_us.saturating_sub(self.last_us));
        self.last_us = st.timestamp_us;
        if self.prev.len() < pressed.len() {
            self.prev.resize(pressed.len(), false);
        }
        for (i, &now) in pressed.iter().enumerate() {
            if now != self.prev[i] && self.events.len() < MAX_HEALTH_EVENTS {
                self.events.push(ButtonEvent {
                    button: i as u32,
                    pressed: now,
                    t_us: st.timestamp_us.saturating_sub(start),
                });
            }
            self.prev[i] = now;
        }
    }

    fn elapsed_us(&self) -> u64 {
        self.started_us
            .map_or(0, |s| self.last_us.saturating_sub(s))
    }

    fn precision(&self) -> Precision {
        if !self.report_based {
            return Precision::Event;
        }
        let mut v: Vec<u64> = self.intervals.iter().copied().filter(|&d| d > 0).collect();
        v.sort_unstable();
        let interval_us = v
            .get(v.len() / 2)
            .copied()
            .unwrap_or(0)
            .min(u32::MAX as u64) as u32;
        Precision::Report { interval_us }
    }

    fn progress(&self) -> HealthProgress {
        HealthProgress {
            controller: self.id.clone(),
            precision: self.precision(),
            elapsed_ms: self.elapsed_us() as f32 / 1000.0,
            buttons: live_status(&self.events, self.elapsed_us()),
        }
    }

    fn finish(self) -> HealthRecording {
        HealthRecording {
            controller: self.id.clone(),
            precision: self.precision(),
            duration_us: self.elapsed_us(),
            events: self.events,
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
    let mut health: Option<HealthState> = None;
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
                Command::Firmware(id, op, reply) => {
                    let result = if id.as_str() == DEMO_ID {
                        demo.firmware(&id, &op)
                    } else {
                        match owners.get(&id).and_then(|&i| backends.get_mut(i)) {
                            Some(b) => b.firmware(&id, &op),
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
                Command::HealthStart(id) => health = Some(HealthState::new(id)),
                Command::HealthProgress(reply) => {
                    let _ = reply.send(health.as_ref().map(HealthState::progress));
                }
                Command::HealthStop(reply) => {
                    let _ = reply.send(health.take().map(HealthState::finish));
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
                    let recording = capture.as_ref().is_some_and(|c| c.id == id)
                        || health.as_ref().is_some_and(|h| h.id == id);
                    if recording {
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
                    if let Some(h) = health.as_mut().filter(|h| h.id == id) {
                        h.push(&st);
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
