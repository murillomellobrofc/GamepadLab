//! Helpers shared by the Sony parsers.

use crate::model::*;
use gamepadlab_calibration::Vec2;

pub fn le_i16(b: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([b[i], b[i + 1]])
}

pub fn le_u32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// 8-bit stick byte → -1..1, y flipped so up is positive.
pub fn stick(x: u8, y: u8) -> Vec2 {
    Vec2::new(x as f32 / 127.5 - 1.0, -(y as f32 / 127.5 - 1.0))
}

/// Sony hat nibble (0 = N, clockwise, 8 = released) → dpad buttons.
pub fn apply_hat(st: &mut DeviceState, hat: u8) {
    let (up, right, down, left) = match hat & 0x0F {
        0 => (true, false, false, false),
        1 => (true, true, false, false),
        2 => (false, true, false, false),
        3 => (false, true, true, false),
        4 => (false, false, true, false),
        5 => (false, false, true, true),
        6 => (false, false, false, true),
        7 => (true, false, false, true),
        _ => (false, false, false, false),
    };
    st.set_button(Button::DpadUp, ButtonState::digital(up));
    st.set_button(Button::DpadRight, ButtonState::digital(right));
    st.set_button(Button::DpadDown, ButtonState::digital(down));
    st.set_button(Button::DpadLeft, ButtonState::digital(left));
}

/// Face buttons and shoulders shared by DS4 and DualSense (and the DS5
/// Bluetooth "simple" report).
pub fn apply_sony_buttons(st: &mut DeviceState, b0: u8, b1: u8, b2: u8) {
    apply_hat(st, b0);
    let bit = |v: u8, n: u8| ButtonState::digital(v & (1 << n) != 0);
    st.set_button(Button::West, bit(b0, 4));
    st.set_button(Button::South, bit(b0, 5));
    st.set_button(Button::East, bit(b0, 6));
    st.set_button(Button::North, bit(b0, 7));
    st.set_button(Button::LeftBumper, bit(b1, 0));
    st.set_button(Button::RightBumper, bit(b1, 1));
    st.set_button(Button::Select, bit(b1, 4));
    st.set_button(Button::Start, bit(b1, 5));
    st.set_button(Button::LeftStick, bit(b1, 6));
    st.set_button(Button::RightStick, bit(b1, 7));
    st.set_button(Button::Guide, bit(b2, 0));
    st.set_button(Button::Touchpad, bit(b2, 1));
}

pub fn set_triggers(st: &mut DeviceState, l2: u8, r2: u8) {
    st.left_trigger = l2 as f32 / 255.0;
    st.right_trigger = r2 as f32 / 255.0;
    st.set_button(
        Button::LeftTrigger,
        ButtonState::analog(st.left_trigger, 0.12),
    );
    st.set_button(
        Button::RightTrigger,
        ButtonState::analog(st.right_trigger, 0.12),
    );
}

/// Raw axis list for the data panel (byte values plus normalised value).
pub fn sony_raw_axes(lx: u8, ly: u8, rx: u8, ry: u8, l2: u8, r2: u8) -> Vec<RawAxisValue> {
    let norm = |v: u8| v as f32 / 127.5 - 1.0;
    [
        ("LX", lx),
        ("LY", ly),
        ("RX", rx),
        ("RY", ry),
        ("L2", l2),
        ("R2", r2),
    ]
    .iter()
    .enumerate()
    .map(|(i, (label, v))| RawAxisValue {
        code: i as u32,
        label: (*label).into(),
        value: norm(*v),
        raw: Some(*v as i32),
    })
    .collect()
}

pub const SONY_BUTTON_LABELS: [(Button, &str); 23] = [
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
    (Button::Paddle1, "Fn L"),
    (Button::Paddle2, "Fn R"),
    (Button::Paddle3, "Back L"),
    (Button::Paddle4, "Back R"),
];

pub fn raw_buttons(st: &DeviceState, count: usize) -> Vec<RawButtonValue> {
    SONY_BUTTON_LABELS
        .iter()
        .take(count)
        .map(|(b, label)| {
            let s = st.button(*b);
            RawButtonValue {
                code: b.index() as u32,
                label: (*label).into(),
                pressed: s.pressed,
                value: s.value,
            }
        })
        .collect()
}

/// One touch contact: 4 bytes, bit 7 of the first byte means "not touching".
pub fn touch_point(b: &[u8], width: f32, height: f32) -> TouchPoint {
    let x = (b[1] as u16 | ((b[2] as u16 & 0x0F) << 8)) as f32;
    let y = ((b[2] as u16 >> 4) | ((b[3] as u16) << 4)) as f32;
    TouchPoint {
        id: b[0] & 0x7F,
        active: b[0] & 0x80 == 0,
        x: (x / width).clamp(0.0, 1.0),
        y: (y / height).clamp(0.0, 1.0),
    }
}

/// Per-axis IMU calibration: value = (raw - bias) * numer / denom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisCal {
    pub bias: i32,
    pub numer: i32,
    pub denom: i32,
}

impl AxisCal {
    pub fn apply(&self, raw: i16) -> f32 {
        if self.denom == 0 {
            return raw as f32;
        }
        (raw as i32 - self.bias) as f32 * self.numer as f32 / self.denom as f32
    }
}

pub const GYRO_RES_PER_DEG_S: i32 = 1024;
pub const ACC_RES_PER_G: i32 = 8192;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuCalibration {
    pub gyro: [AxisCal; 3],
    pub accel: [AxisCal; 3],
}

impl Default for ImuCalibration {
    /// Used when the feature report cannot be read: nominal ±2000 °/s and
    /// 8192 LSB/g.
    fn default() -> Self {
        let g = AxisCal {
            bias: 0,
            numer: GYRO_RES_PER_DEG_S,
            denom: 16,
        };
        let a = AxisCal {
            bias: 0,
            numer: ACC_RES_PER_G,
            denom: ACC_RES_PER_G,
        };
        Self {
            gyro: [g; 3],
            accel: [a; 3],
        }
    }
}

impl ImuCalibration {
    /// Builds the calibration from the values found in the Sony feature
    /// report (same math as Linux `hid-playstation`).
    pub fn from_values(
        gyro_bias: [i16; 3],
        gyro_plus: [i16; 3],
        gyro_minus: [i16; 3],
        speed_plus: i16,
        speed_minus: i16,
        acc_plus: [i16; 3],
        acc_minus: [i16; 3],
    ) -> Option<Self> {
        let speed_2x = speed_plus as i32 + speed_minus as i32;
        let mut cal = ImuCalibration::default();
        for i in 0..3 {
            let denom = gyro_plus[i] as i32 - gyro_minus[i] as i32;
            if denom == 0 || speed_2x == 0 {
                return None;
            }
            cal.gyro[i] = AxisCal {
                bias: gyro_bias[i] as i32,
                numer: speed_2x * GYRO_RES_PER_DEG_S,
                denom,
            };
            let range_2g = acc_plus[i] as i32 - acc_minus[i] as i32;
            if range_2g == 0 {
                return None;
            }
            cal.accel[i] = AxisCal {
                bias: acc_plus[i] as i32 - range_2g / 2,
                numer: 2 * ACC_RES_PER_G,
                denom: range_2g,
            };
        }
        Some(cal)
    }

    pub fn motion(&self, gyro: [i16; 3], accel: [i16; 3], timestamp_us: u64) -> MotionSample {
        MotionSample {
            gyro: [0, 1, 2].map(|i| self.gyro[i].apply(gyro[i]) / GYRO_RES_PER_DEG_S as f32),
            accel: [0, 1, 2].map(|i| self.accel[i].apply(accel[i]) / ACC_RES_PER_G as f32),
            timestamp_us,
        }
    }
}

pub fn battery(level: u8, state: BatteryState) -> BatteryInfo {
    BatteryInfo {
        level: Some(level.min(100)),
        state,
    }
}
