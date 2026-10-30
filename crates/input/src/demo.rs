//! Simulated DualSense used by demo mode and by tests.
//!
//! It is deliberately imperfect (left stick drifts, has a weak sector and the
//! odd spike; left trigger never reaches 100 %) so every analysis screen has
//! something to show. Scenarios let the calibration wizard and the guided
//! button test drive it like a person would. In the button-health scenarios
//! its R1 is going bad (loose cap, worn switch): some taps never arrive, some
//! chatter or bounce, and it lets go on its own while held.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::firmware::{
    FirmwareModel, FirmwareOp, FirmwarePhase, FirmwareReply, FirmwareStatus, NvStatus,
};
use crate::mapping::{hid_capabilities, PID_DUALSENSE, VID_SONY};
use crate::model::*;
use gamepadlab_calibration::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

pub const DEMO_ID: &str = "demo:dualsense";
const REPORT_US: u64 = 4_000; // 250 Hz, like a DualSense over Bluetooth

/// The demo button that is going bad in the button-health scenarios.
pub const HEALTH_FAULTY: Button = Button::RightBumper;
const HEALTH_TAP_START: f32 = 0.4;
const HEALTH_TAP_PERIOD: f32 = 0.26;
const HEALTH_TAP_DOWN: f32 = 0.1;
/// Taps of the faulty button that never arrive, chatter, or bounce on release.
const HEALTH_MISSED: [u32; 3] = [3, 9, 15];
const HEALTH_CHATTER: [u32; 4] = [1, 6, 11, 17];
const HEALTH_BOUNCE: [u32; 2] = [4, 13];
const HEALTH_HOLD: (f32, f32) = (0.3, 2.6);
/// Moments (s) the faulty button lets go on its own while held.
const HEALTH_DROPOUTS: [(f32, f32); 2] = [(1.4, 1.412), (2.2, 2.208)];

/// Whether `button` is down `secs` into a health tap round.
fn health_tap_down(button: usize, presses: u32, secs: f32) -> bool {
    let t = secs - HEALTH_TAP_START;
    if t < 0.0 {
        return false;
    }
    let k = (t / HEALTH_TAP_PERIOD) as u32;
    if k >= presses {
        return false;
    }
    let w = t - k as f32 * HEALTH_TAP_PERIOD;
    if button != HEALTH_FAULTY.index() {
        return w < HEALTH_TAP_DOWN;
    }
    if HEALTH_MISSED.contains(&k) {
        false
    } else if HEALTH_CHATTER.contains(&k) {
        w < 0.008 || (0.016..HEALTH_TAP_DOWN).contains(&w)
    } else if HEALTH_BOUNCE.contains(&k) {
        w < HEALTH_TAP_DOWN || (HEALTH_TAP_DOWN + 0.008..HEALTH_TAP_DOWN + 0.016).contains(&w)
    } else {
        w < HEALTH_TAP_DOWN
    }
}

/// Holds `button`, pulling the trigger fully when it is L2/R2.
fn press(pressed: &mut Vec<usize>, lt: &mut f32, rt: &mut f32, button: usize) {
    pressed.push(button);
    if button == Button::LeftTrigger.index() {
        *lt = 1.0;
    }
    if button == Button::RightTrigger.index() {
        *rt = 1.0;
    }
}

fn health_hold_down(button: usize, secs: f32) -> bool {
    let dropped = button == HEALTH_FAULTY.index()
        && HEALTH_DROPOUTS.iter().any(|&(a, b)| (a..b).contains(&secs));
    (HEALTH_HOLD.0..HEALTH_HOLD.1).contains(&secs) && !dropped
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DemoScenario {
    Showcase,
    Idle,
    RotateLeft,
    RotateRight,
    /// Both sticks rotating (firmware range calibration).
    RotateBoth,
    DirectionsLeft,
    DirectionsRight,
    TriggerRest,
    TriggerFull,
    Buttons,
    PressButton(usize),
    Chord(Vec<usize>),
    /// Button-health tap round: `presses` taps of one button, one every 260 ms.
    HealthTaps {
        button: usize,
        presses: u32,
    },
    /// Button-health hold step: one button held for 2.3 s.
    HealthHold(usize),
    Manual,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInput {
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub buttons: Vec<bool>,
}

/// Tiny deterministic PRNG (xorshift64*), so demo runs are reproducible.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in 0..1.
    pub fn uniform(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Approximately normal (sum of uniforms), mean 0, std 1.
    pub fn gauss(&mut self) -> f32 {
        (0..6).map(|_| self.uniform()).sum::<f32>() - 3.0
    }
}

/// Physical character of one simulated stick.
#[derive(Debug, Clone, Copy)]
struct StickModel {
    drift: Vec2,
    noise: f32,
    /// Extra reach at the diagonals (square-ish gate), e.g. 0.05 = +5 %.
    diagonal_bulge: f32,
    base_reach: f32,
    /// Weak sector: (centre angle in degrees, half-width, reach there).
    dent: Option<(f32, f32, f32)>,
    spike_every_us: Option<u64>,
}

impl StickModel {
    fn reach(&self, angle: f32) -> f32 {
        let mut r = self.base_reach + self.diagonal_bulge * (1.0 - (4.0 * angle).cos()) * 0.5;
        if let Some((centre, half, depth)) = self.dent {
            let d =
                gamepadlab_calibration::geometry::angle_diff_deg(angle.to_degrees(), centre).abs();
            if d < half {
                let k = 0.5 + 0.5 * (PI * d / half).cos();
                r = r * (1.0 - k) + depth * k;
            }
        }
        r
    }
}

const LEFT: StickModel = StickModel {
    drift: Vec2 {
        x: 0.034,
        y: -0.021,
    },
    noise: 0.0065,
    diagonal_bulge: 0.04,
    base_reach: 0.965,
    dent: Some((200.0, 22.0, 0.80)),
    spike_every_us: Some(2_700_000),
};

const RIGHT: StickModel = StickModel {
    drift: Vec2 { x: 0.004, y: 0.006 },
    noise: 0.0025,
    diagonal_bulge: 0.05,
    base_reach: 1.0,
    dent: None,
    spike_every_us: None,
};

/// Simulated DualSense firmware calibration (temporary until saved).
#[derive(Debug, Clone, Copy)]
struct DemoFirmware {
    phase: FirmwarePhase,
    samples: u32,
    unsaved: bool,
    saved: (StickModel, StickModel),
}

pub struct DemoBackend {
    left: StickModel,
    right: StickModel,
    fw: DemoFirmware,
    reboot_pending: bool,
    active: bool,
    announced: bool,
    scenario: DemoScenario,
    scenario_start_us: u64,
    next_report_us: u64,
    rng: Rng,
    manual: ManualInput,
    rumble: (f32, f32),
    battery: u8,
    battery_tick_us: u64,
}

impl Default for DemoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoBackend {
    pub fn new() -> Self {
        Self {
            left: LEFT,
            right: RIGHT,
            fw: DemoFirmware {
                phase: FirmwarePhase::Idle,
                samples: 0,
                unsaved: false,
                saved: (LEFT, RIGHT),
            },
            reboot_pending: false,
            active: false,
            announced: false,
            scenario: DemoScenario::Showcase,
            scenario_start_us: 0,
            next_report_us: 0,
            rng: Rng::new(0x5EED_CAFE),
            manual: ManualInput::default(),
            rumble: (0.0, 0.0),
            battery: 78,
            battery_tick_us: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn set_scenario(&mut self, scenario: DemoScenario, now_us: u64) {
        self.scenario = scenario;
        self.scenario_start_us = now_us;
    }

    pub fn scenario(&self) -> &DemoScenario {
        &self.scenario
    }

    pub fn set_manual(&mut self, input: ManualInput) {
        self.manual = input;
    }

    pub fn id() -> ControllerId {
        ControllerId::new(DEMO_ID)
    }

    pub fn info(&self) -> ControllerInfo {
        ControllerInfo {
            id: Self::id(),
            name: "DualSense (demo)".into(),
            vendor_id: VID_SONY,
            product_id: PID_DUALSENSE,
            serial: Some("DEMO-0001".into()),
            kind: ControllerKind::Ps5,
            connection: ConnectionType::Usb,
            button_count: 19,
            axis_count: 6,
            capabilities: hid_capabilities(ControllerKind::Ps5),
            battery: Some(self.battery_info()),
            backend: "demo".into(),
            is_virtual: false,
            is_demo: true,
            profile_key: "demo:dualsense".into(),
        }
    }

    fn battery_info(&self) -> BatteryInfo {
        BatteryInfo {
            level: Some(self.battery),
            state: BatteryState::Discharging,
        }
    }

    /// Generates the state of one report at time `t` (µs since service start).
    pub fn state_at(&mut self, t: u64) -> DeviceState {
        let since = t.saturating_sub(self.scenario_start_us);
        let secs = since as f32 / 1e6;
        let abs = t as f32 / 1e6;
        let mut st = DeviceState::neutral(t);
        st.report_based = true;

        let (lm, rm) = (self.left, self.right);
        let mut left = lm.drift;
        let mut right = rm.drift;
        let mut lt = 0.0f32;
        let mut rt = 0.0f32;
        let mut pressed: Vec<usize> = Vec::new();

        match self.scenario.clone() {
            DemoScenario::Showcase => {
                let a = abs * TAU * 0.33;
                let r = if (abs % 6.0) < 3.0 { 1.0 } else { 0.55 };
                left = left + Vec2::from_polar(r * lm.reach(a), a);
                right = right + Vec2::new((abs * 1.7).sin() * 0.8, (abs * 2.3).sin() * 0.6);
                lt = ((abs * 1.3).sin() + 1.0) * 0.5;
                rt = (abs * 0.6).fract();
                let slot = (abs / 0.45) as usize;
                if (abs % 0.45) < 0.26 {
                    pressed.push(SHOWCASE_ORDER[slot % SHOWCASE_ORDER.len()]);
                }
            }
            DemoScenario::Idle | DemoScenario::TriggerRest => {
                if self.scenario == DemoScenario::TriggerRest {
                    lt = 0.018;
                    rt = 0.004;
                }
            }
            DemoScenario::RotateLeft | DemoScenario::RotateRight => {
                let (model, base) = if self.scenario == DemoScenario::RotateLeft {
                    (lm, &mut left)
                } else {
                    (rm, &mut right)
                };
                let a = rotation_angle(secs);
                let push = 1.0 - 0.012 * self.rng.uniform();
                *base = model.drift.scale(0.3) + Vec2::from_polar(model.reach(a) * push, a);
            }
            DemoScenario::RotateBoth => {
                let a = rotation_angle(secs);
                left = lm.drift.scale(0.3) + Vec2::from_polar(lm.reach(a), a);
                let b = -a + 0.7;
                right = rm.drift.scale(0.3) + Vec2::from_polar(rm.reach(b), b);
            }
            DemoScenario::DirectionsLeft | DemoScenario::DirectionsRight => {
                let (model, base) = if self.scenario == DemoScenario::DirectionsLeft {
                    (lm, &mut left)
                } else {
                    (rm, &mut right)
                };
                let slot = (secs / 0.95) as usize % 8;
                let within = secs % 0.95;
                if within > 0.12 && within < 0.75 {
                    let a = DIRECTIONS_DEG[slot].to_radians();
                    *base = model.drift.scale(0.3) + Vec2::from_polar(model.reach(a), a);
                }
            }
            DemoScenario::TriggerFull => {
                let k = (secs / 0.6).min(1.0);
                lt = 0.018 + (0.962 - 0.018) * k;
                rt = 0.004 + (1.0 - 0.004) * k;
            }
            DemoScenario::Buttons => {
                let slot = (secs / 0.35) as usize;
                if (secs % 0.35) < 0.2 {
                    pressed.push(SHOWCASE_ORDER[slot % SHOWCASE_ORDER.len()]);
                }
            }
            DemoScenario::PressButton(i) => {
                if (0.4..0.7).contains(&secs) {
                    pressed.push(i);
                    if i == Button::LeftTrigger.index() {
                        lt = 1.0;
                    }
                    if i == Button::RightTrigger.index() {
                        rt = 1.0;
                    }
                }
            }
            DemoScenario::HealthTaps { button, presses } => {
                if health_tap_down(button, presses, secs) {
                    press(&mut pressed, &mut lt, &mut rt, button);
                }
            }
            DemoScenario::HealthHold(button) => {
                if health_hold_down(button, secs) {
                    press(&mut pressed, &mut lt, &mut rt, button);
                }
            }
            DemoScenario::Chord(list) => {
                if (0.3..1.8).contains(&secs) {
                    // Simulated ghosting: with four or more buttons held, West
                    // never registers.
                    let ghost = list.len() >= 4;
                    for i in list {
                        if ghost && i == Button::West.index() {
                            continue;
                        }
                        pressed.push(i);
                        if i == Button::LeftTrigger.index() {
                            lt = 1.0;
                        }
                        if i == Button::RightTrigger.index() {
                            rt = 1.0;
                        }
                    }
                }
            }
            DemoScenario::Manual => {
                left = self.manual.left_stick;
                right = self.manual.right_stick;
                lt = self.manual.left_trigger;
                rt = self.manual.right_trigger;
                for (i, &b) in self.manual.buttons.iter().enumerate() {
                    if b {
                        pressed.push(i);
                    }
                }
            }
        }

        let manual = self.scenario == DemoScenario::Manual;
        if !manual {
            left = left + Vec2::new(self.rng.gauss() * lm.noise, self.rng.gauss() * lm.noise);
            right = right + Vec2::new(self.rng.gauss() * rm.noise, self.rng.gauss() * rm.noise);
            if let Some(every) = lm.spike_every_us {
                if t % every < REPORT_US {
                    left = left + Vec2::new(0.22, -0.17);
                }
            }
            lt += self.rng.gauss() * 0.002;
            rt += self.rng.gauss() * 0.0015;
        }

        // Quantise like an 8-bit HID report and keep the raw byte.
        let (lx, ly, lraw) = quantize(left);
        let (rx, ry, rraw) = quantize(right);
        let (lt, lt_raw) = quantize_trigger(lt);
        let (rt, rt_raw) = quantize_trigger(rt);
        st.left_stick = Vec2::new(lx, ly);
        st.right_stick = Vec2::new(rx, ry);
        st.left_trigger = lt;
        st.right_trigger = rt;

        for i in pressed {
            if let Some(b) = Button::from_index(i) {
                st.set_button(b, ButtonState::digital(true));
            }
        }
        st.set_button(Button::LeftTrigger, ButtonState::analog(lt, 0.12));
        st.set_button(Button::RightTrigger, ButtonState::analog(rt, 0.12));

        let labels = ["LX", "LY", "RX", "RY", "L2", "R2"];
        let values = [lx, ly, rx, ry, lt * 2.0 - 1.0, rt * 2.0 - 1.0];
        let raws = [lraw.0, lraw.1, rraw.0, rraw.1, lt_raw, rt_raw];
        st.raw_axes = (0..6)
            .map(|i| RawAxisValue {
                code: i as u32,
                label: labels[i].into(),
                value: values[i],
                raw: Some(raws[i]),
            })
            .collect();
        st.raw_buttons = DS_BUTTON_LABELS
            .iter()
            .map(|(b, label)| {
                let s = st.button(*b);
                RawButtonValue {
                    code: b.index() as u32,
                    label: (*label).into(),
                    pressed: s.pressed,
                    value: s.value,
                }
            })
            .collect();

        let shake = (self.rumble.0 * 0.35 + self.rumble.1 * 0.8) * self.rng.gauss() * 0.15;
        let showcase = matches!(self.scenario, DemoScenario::Showcase);
        let motion_k = if showcase { 1.0 } else { 0.08 };
        st.motion = Some(MotionSample {
            gyro: [
                (abs * 1.1).sin() * 45.0 * motion_k + self.rng.gauss() * 0.4,
                (abs * 0.7).cos() * 30.0 * motion_k + self.rng.gauss() * 0.4,
                (abs * 1.9).sin() * 12.0 * motion_k + self.rng.gauss() * 0.4,
            ],
            accel: [
                (abs * 0.5).sin() * 0.18 * motion_k + shake,
                -0.98 + (abs * 0.8).cos() * 0.05 * motion_k + shake,
                (abs * 0.5).cos() * 0.2 * motion_k + shake * 0.5,
            ],
            timestamp_us: t,
        });
        let touching = showcase && (abs % 5.0) < 2.6;
        let ta = abs * TAU * 0.4;
        st.touch = Some(TouchState {
            pressed: showcase && (abs % 5.0) > 2.2 && (abs % 5.0) < 2.5,
            points: vec![
                TouchPoint {
                    id: 1,
                    active: touching,
                    x: 0.5 + 0.3 * ta.cos(),
                    y: 0.5 + 0.3 * ta.sin(),
                },
                TouchPoint {
                    id: 2,
                    active: touching && (abs % 5.0) > 1.2,
                    x: 0.25,
                    y: 0.7,
                },
            ],
        });
        st.battery = Some(self.battery_info());
        st
    }
}

impl DemoBackend {
    fn fw_status(&self) -> FirmwareStatus {
        FirmwareStatus {
            model: FirmwareModel::Ds5,
            simulated: true,
            genuine: Some(true),
            clone_reason: None,
            firmware: Some("Demo 1.0 (simulated)".into()),
            usb: true,
            battery: Some(self.battery),
            nv: NvStatus::Locked,
            phase: self.fw.phase,
            center_samples: self.fw.samples,
            unsaved_changes: self.fw.unsaved,
            blockers: Vec::new(),
        }
    }

    /// Same state machine as the real controllers, applied to the simulated
    /// sticks: centre calibration removes the drift, range calibration
    /// restores full travel (the worn sector stays: that is mechanical).
    pub fn firmware(
        &mut self,
        id: &ControllerId,
        op: &FirmwareOp,
    ) -> Result<FirmwareReply, InputError> {
        if id.as_str() != DEMO_ID || !self.active {
            return Err(InputError::Disconnected(id.to_string()));
        }
        let fail = |m: &str| Err(InputError::FirmwareProtocol(m.into()));
        match op {
            FirmwareOp::Status => {}
            FirmwareOp::CenterBegin | FirmwareOp::RangeBegin => {
                if self.fw.phase != FirmwarePhase::Idle {
                    return fail("another calibration is still running");
                }
                self.fw.phase = if matches!(op, FirmwareOp::CenterBegin) {
                    FirmwarePhase::Center
                } else {
                    FirmwarePhase::Range
                };
                self.fw.samples = 0;
            }
            FirmwareOp::CenterSample => {
                if self.fw.phase != FirmwarePhase::Center {
                    return fail("center calibration is not running");
                }
                self.fw.samples += 1;
            }
            FirmwareOp::CenterEnd => {
                if self.fw.phase != FirmwarePhase::Center || self.fw.samples == 0 {
                    return fail("take at least one center sample first");
                }
                self.fw.phase = FirmwarePhase::Idle;
                self.fw.unsaved = true;
                self.left.drift = Vec2::new(0.003, -0.002);
                self.right.drift = Vec2::new(0.001, 0.001);
            }
            FirmwareOp::RangeEnd => {
                if self.fw.phase != FirmwarePhase::Range {
                    return fail("range calibration is not running");
                }
                self.fw.phase = FirmwarePhase::Idle;
                self.fw.unsaved = true;
                for m in [&mut self.left, &mut self.right] {
                    m.base_reach = 1.0;
                    m.diagonal_bulge = 0.035;
                }
            }
            FirmwareOp::SavePermanent { acknowledge_risk } => {
                if !acknowledge_risk {
                    return Err(InputError::FirmwareBlocked(
                        "the risk was not acknowledged".into(),
                    ));
                }
                if !self.fw.unsaved || self.fw.phase != FirmwarePhase::Idle {
                    return Err(InputError::FirmwareBlocked(
                        "calibrate (temporarily) and check the result first".into(),
                    ));
                }
                self.fw.saved = (self.left, self.right);
                self.fw.unsaved = false;
            }
            FirmwareOp::Reboot => {
                (self.left, self.right) = self.fw.saved;
                self.fw.unsaved = false;
                self.fw.phase = FirmwarePhase::Idle;
                self.reboot_pending = true;
            }
            FirmwareOp::SwitchRead
            | FirmwareOp::SwitchWrite { .. }
            | FirmwareOp::SwitchClear { .. } => {
                return Err(InputError::Unsupported(
                    "Switch operations on the demo DualSense".into(),
                ));
            }
        }
        Ok(FirmwareReply {
            status: self.fw_status(),
            switch: None,
            backup: None,
        })
    }
}

/// Two slow turns each way, then two fast turns each way (16 s loop).
fn rotation_angle(secs: f32) -> f32 {
    let s = secs % 16.0;
    let slow = 0.4 * TAU;
    let fast = 1.3 * TAU;
    if s < 5.0 {
        s * slow
    } else if s < 10.0 {
        5.0 * slow - (s - 5.0) * slow
    } else if s < 13.0 {
        (s - 10.0) * fast
    } else {
        3.0 * fast - (s - 13.0) * fast
    }
}

/// Up, up-right, right, down-right, down, down-left, left, up-left.
pub const DIRECTIONS_DEG: [f32; 8] = [90.0, 45.0, 0.0, 315.0, 270.0, 225.0, 180.0, 135.0];

/// Guide (16) is left out: it toggles controller navigation in the UI.
const SHOWCASE_ORDER: [usize; 16] = [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18];

const DS_BUTTON_LABELS: [(Button, &str); 19] = [
    (Button::South, "Cross"),
    (Button::East, "Circle"),
    (Button::West, "Square"),
    (Button::North, "Triangle"),
    (Button::LeftBumper, "L1"),
    (Button::RightBumper, "R1"),
    (Button::LeftTrigger, "L2"),
    (Button::RightTrigger, "R2"),
    (Button::Select, "Share"),
    (Button::Start, "Options"),
    (Button::LeftStick, "L3"),
    (Button::RightStick, "R3"),
    (Button::DpadUp, "Up"),
    (Button::DpadDown, "Down"),
    (Button::DpadLeft, "Left"),
    (Button::DpadRight, "Right"),
    (Button::Guide, "PS"),
    (Button::Touchpad, "Touchpad"),
    (Button::Misc, "Mute"),
];

/// 8-bit quantisation; raw bytes follow HID convention (y grows downwards).
fn quantize(v: Vec2) -> (f32, f32, (i32, i32)) {
    let to_byte = |f: f32| {
        ((f.clamp(-1.0, 1.0) + 1.0) * 127.5)
            .round()
            .clamp(0.0, 255.0) as i32
    };
    let bx = to_byte(v.x);
    let by = to_byte(-v.y);
    let x = bx as f32 / 127.5 - 1.0;
    let y = -(by as f32 / 127.5 - 1.0);
    (x, y, (bx, by))
}

fn quantize_trigger(v: f32) -> (f32, i32) {
    let b = (v.clamp(0.0, 1.0) * 255.0).round() as i32;
    (b as f32 / 255.0, b)
}

impl GamepadBackend for DemoBackend {
    fn name(&self) -> &'static str {
        "demo"
    }

    fn priority(&self) -> u8 {
        0
    }

    fn poll(&mut self, clock: &Clock, out: &mut Vec<BackendEvent>) {
        let now = clock.now_us();
        if !self.active {
            if self.announced {
                self.announced = false;
                out.push(BackendEvent::Disconnected(Self::id()));
            }
            return;
        }
        if self.reboot_pending && self.announced {
            // Simulated reboot: the pad drops off the bus and comes back.
            self.reboot_pending = false;
            self.announced = false;
            out.push(BackendEvent::Disconnected(Self::id()));
            return;
        }
        if !self.announced {
            self.announced = true;
            self.next_report_us = now;
            self.battery_tick_us = now;
            self.scenario_start_us = now;
            out.push(BackendEvent::Connected(self.info()));
        }
        if now.saturating_sub(self.next_report_us) > 100_000 {
            self.next_report_us = now; // fell behind (app suspended): skip ahead
        }
        while self.next_report_us <= now {
            let t = self.next_report_us;
            let st = self.state_at(t);
            out.push(BackendEvent::State(Self::id(), st));
            let jitter = (self.rng.gauss() * 120.0) as i64;
            self.next_report_us = (t as i64 + REPORT_US as i64 + jitter).max(t as i64 + 500) as u64;
        }
        if now.saturating_sub(self.battery_tick_us) > 60_000_000 {
            self.battery_tick_us = now;
            self.battery = self.battery.saturating_sub(1).max(5);
            out.push(BackendEvent::Updated(self.info()));
        }
    }

    fn send(&mut self, id: &ControllerId, cmd: &OutputCommand) -> Result<(), InputError> {
        if id.as_str() != DEMO_ID || !self.active {
            return Err(InputError::Disconnected(id.to_string()));
        }
        if let OutputCommand::Rumble { weak, strong } = cmd {
            self.rumble = (*weak, *strong);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announces_then_streams_at_about_250hz() {
        let mut d = DemoBackend::new();
        d.set_active(true);
        let clock = Clock::new();
        let mut out = Vec::new();
        d.poll(&clock, &mut out);
        assert!(matches!(out[0], BackendEvent::Connected(_)));
        // Simulate 1 s worth of reports by generating states directly.
        let mut t = 0u64;
        let mut n = 0;
        while t < 1_000_000 {
            let s = d.state_at(t);
            assert_eq!(s.buttons.len(), BUTTON_COUNT);
            t += REPORT_US;
            n += 1;
        }
        assert_eq!(n, 250);
    }

    #[test]
    fn idle_shows_left_drift() {
        let mut d = DemoBackend::new();
        d.set_scenario(DemoScenario::Idle, 0);
        let mut sx = 0.0;
        let n = 400;
        for i in 0..n {
            sx += d.state_at(i * REPORT_US + 1).left_stick.x;
        }
        let mean = sx / n as f32;
        assert!((mean - LEFT.drift.x).abs() < 0.01, "{mean}");
    }

    #[test]
    fn rotation_reaches_the_gate() {
        let mut d = DemoBackend::new();
        d.set_scenario(DemoScenario::RotateRight, 0);
        let max = (0..2000)
            .map(|i| d.state_at(i * REPORT_US).right_stick.length())
            .fold(0.0f32, f32::max);
        assert!(max > 0.95, "{max}");
    }

    #[test]
    fn chord_ghosts_west() {
        let mut d = DemoBackend::new();
        let chord = vec![0, 1, 2, 3];
        d.set_scenario(DemoScenario::Chord(chord), 0);
        let s = d.state_at(1_000_000);
        assert!(s.button(Button::South).pressed);
        assert!(!s.button(Button::West).pressed);
    }

    #[test]
    fn deactivating_disconnects() {
        let mut d = DemoBackend::new();
        let clock = Clock::new();
        let mut out = Vec::new();
        d.set_active(true);
        d.poll(&clock, &mut out);
        out.clear();
        d.set_active(false);
        d.poll(&clock, &mut out);
        assert!(matches!(out.as_slice(), [BackendEvent::Disconnected(_)]));
    }

    fn mean_left_x(d: &mut DemoBackend) -> f32 {
        d.set_scenario(DemoScenario::Idle, 0);
        (0..400)
            .map(|i| d.state_at(i * REPORT_US + 1).left_stick.x)
            .sum::<f32>()
            / 400.0
    }

    #[test]
    fn simulated_firmware_calibration_fixes_drift_until_reboot() {
        let mut d = DemoBackend::new();
        d.set_active(true);
        let id = DemoBackend::id();
        assert!(mean_left_x(&mut d) > 0.025);
        d.firmware(&id, &FirmwareOp::CenterBegin).unwrap();
        assert!(
            d.firmware(&id, &FirmwareOp::CenterEnd).is_err(),
            "needs a sample"
        );
        d.firmware(&id, &FirmwareOp::CenterSample).unwrap();
        let st = d.firmware(&id, &FirmwareOp::CenterEnd).unwrap().status;
        assert!(st.unsaved_changes && st.simulated);
        assert!(mean_left_x(&mut d).abs() < 0.01);
        // Reboot without saving → drift is back.
        d.firmware(&id, &FirmwareOp::Reboot).unwrap();
        assert!(mean_left_x(&mut d) > 0.025);
        // Calibrate again and save: survives a reboot.
        d.firmware(&id, &FirmwareOp::CenterBegin).unwrap();
        d.firmware(&id, &FirmwareOp::CenterSample).unwrap();
        d.firmware(&id, &FirmwareOp::CenterEnd).unwrap();
        assert!(d
            .firmware(
                &id,
                &FirmwareOp::SavePermanent {
                    acknowledge_risk: false
                }
            )
            .is_err());
        d.firmware(
            &id,
            &FirmwareOp::SavePermanent {
                acknowledge_risk: true,
            },
        )
        .unwrap();
        d.firmware(&id, &FirmwareOp::Reboot).unwrap();
        assert!(mean_left_x(&mut d).abs() < 0.01);
    }

    #[test]
    fn simulated_reboot_reconnects() {
        let mut d = DemoBackend::new();
        let clock = Clock::new();
        let mut out = Vec::new();
        d.set_active(true);
        d.poll(&clock, &mut out);
        d.firmware(&DemoBackend::id(), &FirmwareOp::Reboot).unwrap();
        out.clear();
        d.poll(&clock, &mut out);
        assert!(matches!(out.as_slice(), [BackendEvent::Disconnected(_)]));
        out.clear();
        d.poll(&clock, &mut out);
        assert!(matches!(out[0], BackendEvent::Connected(_)));
    }

    #[test]
    fn quantize_roundtrip_is_close() {
        let (x, y, (bx, by)) = quantize(Vec2::new(1.0, 1.0));
        assert_eq!((bx, by), (255, 0));
        assert!((x - 1.0).abs() < 1e-6 && (y - 1.0).abs() < 1e-6);
    }
}
