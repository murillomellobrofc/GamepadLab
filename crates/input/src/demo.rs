//! Simulated DualSense used by demo mode and by tests.
//!
//! It is deliberately imperfect (left stick drifts, has a weak sector and the
//! odd spike; left trigger never reaches 100 %) so every analysis screen has
//! something to show. Scenarios let the calibration wizard and the guided
//! button test drive it like a person would.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::mapping::{hid_capabilities, PID_DUALSENSE, VID_SONY};
use crate::model::*;
use gamepadlab_calibration::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

pub const DEMO_ID: &str = "demo:dualsense";
const REPORT_US: u64 = 4_000; // 250 Hz, like a DualSense over Bluetooth

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DemoScenario {
    Showcase,
    Idle,
    RotateLeft,
    RotateRight,
    DirectionsLeft,
    DirectionsRight,
    TriggerRest,
    TriggerFull,
    Buttons,
    PressButton(usize),
    Chord(Vec<usize>),
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

pub struct DemoBackend {
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

        let mut left = LEFT.drift;
        let mut right = RIGHT.drift;
        let mut lt = 0.0f32;
        let mut rt = 0.0f32;
        let mut pressed: Vec<usize> = Vec::new();

        match self.scenario.clone() {
            DemoScenario::Showcase => {
                let a = abs * TAU * 0.33;
                let r = if (abs % 6.0) < 3.0 { 1.0 } else { 0.55 };
                left = left + Vec2::from_polar(r * LEFT.reach(a), a);
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
                    (LEFT, &mut left)
                } else {
                    (RIGHT, &mut right)
                };
                let a = rotation_angle(secs);
                let push = 1.0 - 0.012 * self.rng.uniform();
                *base = model.drift.scale(0.3) + Vec2::from_polar(model.reach(a) * push, a);
            }
            DemoScenario::DirectionsLeft | DemoScenario::DirectionsRight => {
                let (model, base) = if self.scenario == DemoScenario::DirectionsLeft {
                    (LEFT, &mut left)
                } else {
                    (RIGHT, &mut right)
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
            left = left + Vec2::new(self.rng.gauss() * LEFT.noise, self.rng.gauss() * LEFT.noise);
            right = right
                + Vec2::new(
                    self.rng.gauss() * RIGHT.noise,
                    self.rng.gauss() * RIGHT.noise,
                );
            if let Some(every) = LEFT.spike_every_us {
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

const SHOWCASE_ORDER: [usize; 17] = [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];

const DS_BUTTON_LABELS: [(Button, &str); 19] = [
    (Button::South, "Cross"),
    (Button::East, "Circle"),
    (Button::West, "Square"),
    (Button::North, "Triangle"),
    (Button::LeftBumper, "L1"),
    (Button::RightBumper, "R1"),
    (Button::LeftTrigger, "L2"),
    (Button::RightTrigger, "R2"),
    (Button::Select, "Create"),
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

    #[test]
    fn quantize_roundtrip_is_close() {
        let (x, y, (bx, by)) = quantize(Vec2::new(1.0, 1.0));
        assert_eq!((bx, by), (255, 0));
        assert!((x - 1.0).abs() < 1e-6 && (y - 1.0).abs() < 1e-6);
    }
}
