//! DualSense / DualSense Edge HID protocol (USB report 0x01, Bluetooth 0x31).

use super::common::*;
use super::crc::seal_bt_report;
use super::Driver;
use crate::backend::{InputError, OutputCommand};
use crate::model::*;

pub const FEATURE_CALIBRATION: u8 = 0x05;
pub const FEATURE_CALIBRATION_LEN: usize = 41;
const USB_OUTPUT_LEN: usize = 48;
const BT_OUTPUT_LEN: usize = 78;
const TOUCH_W: f32 = 1920.0;
const TOUCH_H: f32 = 1080.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Ds5Output {
    pub weak: u8,
    pub strong: u8,
    pub lightbar: (u8, u8, u8),
    pub player_leds: u8,
    pub left_trigger: [u8; 11],
    pub right_trigger: [u8; 11],
}

impl Default for Ds5Output {
    fn default() -> Self {
        let off = [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        Self {
            weak: 0,
            strong: 0,
            lightbar: (0x10, 0x40, 0xF0),
            player_leds: 0b00100,
            left_trigger: off,
            right_trigger: off,
        }
    }
}

pub struct DualSense {
    pub bluetooth: bool,
    pub edge: bool,
    pub calibration: ImuCalibration,
    pub output: Ds5Output,
    seq: u8,
    lightbar_released: bool,
}

impl DualSense {
    pub fn new(bluetooth: bool, edge: bool) -> Self {
        Self {
            bluetooth,
            edge,
            calibration: ImuCalibration::default(),
            output: Ds5Output::default(),
            seq: 0,
            lightbar_released: false,
        }
    }

    pub fn parse_calibration(buf: &[u8]) -> Option<ImuCalibration> {
        if buf.len() < 35 || buf[0] != FEATURE_CALIBRATION {
            return None;
        }
        let v = |i| le_i16(buf, i);
        ImuCalibration::from_values(
            [v(1), v(3), v(5)],
            [v(7), v(11), v(15)],
            [v(9), v(13), v(17)],
            v(19),
            v(21),
            [v(23), v(27), v(31)],
            [v(25), v(29), v(33)],
        )
    }

    /// Parses the full-format payload that starts at the left stick X byte.
    pub fn parse_full(&self, p: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        if p.len() < 53 {
            return None;
        }
        let mut st = DeviceState::neutral(timestamp_us);
        st.report_based = true;
        st.left_stick = stick(p[0], p[1]);
        st.right_stick = stick(p[2], p[3]);
        set_triggers(&mut st, p[4], p[5]);
        let (b0, b1, b2) = (p[7], p[8], p[9]);
        apply_sony_buttons(&mut st, b0, b1, b2);
        st.set_button(Button::Misc, ButtonState::digital(b2 & 0x04 != 0));
        if self.edge {
            st.set_button(Button::Paddle1, ButtonState::digital(b2 & 0x10 != 0));
            st.set_button(Button::Paddle2, ButtonState::digital(b2 & 0x20 != 0));
            st.set_button(Button::Paddle3, ButtonState::digital(b2 & 0x40 != 0));
            st.set_button(Button::Paddle4, ButtonState::digital(b2 & 0x80 != 0));
        }
        let gyro = [le_i16(p, 15), le_i16(p, 17), le_i16(p, 19)];
        let accel = [le_i16(p, 21), le_i16(p, 23), le_i16(p, 25)];
        let sensor_ts = le_u32(p, 27) as u64 / 3; // 0.33 µs units
        st.motion = Some(self.calibration.motion(gyro, accel, sensor_ts));
        st.touch = Some(TouchState {
            pressed: b2 & 0x02 != 0,
            points: vec![
                touch_point(&p[32..36], TOUCH_W, TOUCH_H),
                touch_point(&p[36..40], TOUCH_W, TOUCH_H),
            ],
        });
        st.battery = Some(parse_battery(p[52]));
        st.raw_axes = sony_raw_axes(p[0], p[1], p[2], p[3], p[4], p[5]);
        st.raw_buttons = raw_buttons(&st, if self.edge { 23 } else { 19 });
        Some(st)
    }

    /// Bluetooth "simple" report sent before the calibration report is read.
    pub fn parse_simple(&self, d: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        if d.len() < 10 {
            return None;
        }
        let mut st = DeviceState::neutral(timestamp_us);
        st.report_based = true;
        st.left_stick = stick(d[1], d[2]);
        st.right_stick = stick(d[3], d[4]);
        apply_sony_buttons(&mut st, d[5], d[6], d[7]);
        set_triggers(&mut st, d[8], d[9]);
        st.raw_axes = sony_raw_axes(d[1], d[2], d[3], d[4], d[8], d[9]);
        st.raw_buttons = raw_buttons(&st, 18);
        Some(st)
    }

    pub fn build_output(&mut self) -> Vec<u8> {
        let (mut r, base) = if self.bluetooth {
            let mut r = vec![0u8; BT_OUTPUT_LEN];
            r[0] = 0x31;
            r[1] = (self.seq & 0x0F) << 4;
            r[2] = 0x10;
            self.seq = self.seq.wrapping_add(1);
            (r, 3)
        } else {
            let mut r = vec![0u8; USB_OUTPUT_LEN];
            r[0] = 0x02;
            (r, 1)
        };
        let c = &mut r[base..base + 47];
        // valid_flag0: compatible vibration, haptics select, right + left trigger.
        c[0] = 0x01 | 0x02 | 0x04 | 0x08;
        // valid_flag1: lightbar + player indicator control.
        c[1] = 0x04 | 0x10;
        c[2] = self.output.weak;
        c[3] = self.output.strong;
        c[10..21].copy_from_slice(&self.output.right_trigger);
        c[21..32].copy_from_slice(&self.output.left_trigger);
        // valid_flag2: lightbar setup + compatible vibration v2 (newer firmware).
        c[38] = 0x04;
        if !self.lightbar_released {
            c[38] |= 0x02;
            c[41] = 0x02; // fade the boot-time blue pulse out so our colour shows
            self.lightbar_released = true;
        }
        c[42] = 0x00;
        c[43] = self.output.player_leds & 0x1F;
        c[44] = self.output.lightbar.0;
        c[45] = self.output.lightbar.1;
        c[46] = self.output.lightbar.2;
        if self.bluetooth {
            seal_bt_report(0xA2, &mut r);
        }
        r
    }
}

pub fn parse_battery(status: u8) -> BatteryInfo {
    let capacity = status & 0x0F;
    let level = (capacity as u16 * 10 + 5).min(100) as u8;
    match status >> 4 {
        0x0 => battery(level, BatteryState::Discharging),
        0x1 => battery(level, BatteryState::Charging),
        0x2 => battery(100, BatteryState::Full),
        _ => BatteryInfo {
            level: None,
            state: BatteryState::Unknown,
        },
    }
}

impl Driver for DualSense {
    fn feature_report(&self) -> Option<(u8, usize)> {
        Some((FEATURE_CALIBRATION, FEATURE_CALIBRATION_LEN))
    }

    fn on_feature_report(&mut self, buf: &[u8]) {
        match Self::parse_calibration(buf) {
            Some(c) => self.calibration = c,
            None => log::warn!("DualSense: invalid IMU calibration, using defaults"),
        }
    }

    fn parse(&mut self, d: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        match d.first()? {
            0x01 if d.len() >= 64 => self.parse_full(&d[1..], timestamp_us),
            0x01 => self.parse_simple(d, timestamp_us),
            0x31 if d.len() >= 55 => self.parse_full(&d[2..], timestamp_us),
            _ => None,
        }
    }

    fn output(&mut self, cmd: &OutputCommand) -> Result<Vec<u8>, InputError> {
        match cmd {
            OutputCommand::Rumble { weak, strong } => {
                self.output.weak = (weak.clamp(0.0, 1.0) * 255.0) as u8;
                self.output.strong = (strong.clamp(0.0, 1.0) * 255.0) as u8;
            }
            OutputCommand::Lightbar { r, g, b } => self.output.lightbar = (*r, *g, *b),
            OutputCommand::PlayerLeds { mask } => self.output.player_leds = *mask,
            OutputCommand::TriggerEffects { left, right } => {
                self.output.left_trigger = effect_block(left)?;
                self.output.right_trigger = effect_block(right)?;
            }
            OutputCommand::TriggerRumble { .. } => {
                return Err(InputError::Unsupported("impulse triggers".into()))
            }
        }
        Ok(self.build_output())
    }
}

fn effect_block(bytes: &[u8]) -> Result<[u8; 11], InputError> {
    bytes
        .try_into()
        .map_err(|_| InputError::Other("trigger effect must be 11 bytes".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::crc::crc32;

    fn usb_report() -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1] = 255; // LX right
        r[2] = 0; // LY up
        r[3] = 128;
        r[4] = 128;
        r[5] = 255; // L2 full
        r[6] = 0;
        r[8] = 0x20; // cross, hat N (0)
        r[9] = 0x01; // L1
        r[10] = 0x04; // mute
                      // gyro x = 1024 raw, accel y = 8192 raw
        r[16..18].copy_from_slice(&1024i16.to_le_bytes());
        r[24..26].copy_from_slice(&8192i16.to_le_bytes());
        // touch point 1 active at (960, 540)
        r[33] = 0x01;
        r[34] = (960 & 0xFF) as u8;
        r[35] = ((960 >> 8) as u8 & 0x0F) | (((540 & 0x0F) as u8) << 4);
        r[36] = (540 >> 4) as u8;
        r[37] = 0x80; // point 2 inactive
        r[53] = 0x15; // charging, capacity 5
        r
    }

    #[test]
    fn parses_usb_report() {
        let mut ds = DualSense::new(false, false);
        let st = ds.parse(&usb_report(), 42).expect("state");
        assert!((st.left_stick.x - 1.0).abs() < 1e-3);
        assert!((st.left_stick.y - 1.0).abs() < 1e-3, "up is positive");
        assert!(st.right_stick.length() < 0.01);
        assert!((st.left_trigger - 1.0).abs() < 1e-6);
        assert!(st.button(Button::South).pressed);
        assert!(st.button(Button::DpadUp).pressed);
        assert!(st.button(Button::LeftBumper).pressed);
        assert!(st.button(Button::Misc).pressed);
        assert!(st.button(Button::LeftTrigger).pressed);
        let touch = st.touch.unwrap();
        assert!(touch.points[0].active && !touch.points[1].active);
        assert!((touch.points[0].x - 0.5).abs() < 0.01);
        assert!((touch.points[0].y - 0.5).abs() < 0.01);
        let bat = st.battery.unwrap();
        assert_eq!(bat.state, BatteryState::Charging);
        assert_eq!(bat.level, Some(55));
        let m = st.motion.unwrap();
        assert!((m.gyro[0] - 64.0).abs() < 0.01, "{}", m.gyro[0]); // 1024 / 16 °/s
        assert!((m.accel[1] - 1.0).abs() < 1e-3);
        assert_eq!(st.raw_axes[0].raw, Some(255));
    }

    #[test]
    fn parses_bluetooth_report_with_offset() {
        let usb = usb_report();
        let mut bt = vec![0u8; 78];
        bt[0] = 0x31;
        bt[1] = 0x00;
        bt[2..2 + 63].copy_from_slice(&usb[1..64]);
        let mut ds = DualSense::new(true, false);
        let st = ds.parse(&bt, 0).unwrap();
        assert!(st.button(Button::South).pressed);
        assert!((st.left_stick.x - 1.0).abs() < 1e-3);
    }

    #[test]
    fn edge_paddles() {
        let mut r = usb_report();
        r[10] = 0x10 | 0x80;
        let mut ds = DualSense::new(false, true);
        let st = ds.parse(&r, 0).unwrap();
        assert!(st.button(Button::Paddle1).pressed);
        assert!(st.button(Button::Paddle4).pressed);
        assert!(!st.button(Button::Paddle2).pressed);
        assert_eq!(st.raw_buttons.len(), 23);
    }

    #[test]
    fn calibration_math_matches_kernel() {
        let mut buf = [0u8; 41];
        buf[0] = FEATURE_CALIBRATION;
        let put = |b: &mut [u8], i: usize, v: i16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
        for (i, v) in [(1, 10), (3, -5), (5, 0)] {
            put(&mut buf, i, v);
        }
        // plus/minus for pitch, yaw, roll = ±8192, speed ±540
        for i in [7, 11, 15] {
            put(&mut buf, i, 8192);
        }
        for i in [9, 13, 17] {
            put(&mut buf, i, -8192);
        }
        put(&mut buf, 19, 540);
        put(&mut buf, 21, 540);
        for i in [23, 27, 31] {
            put(&mut buf, i, 8192);
        }
        for i in [25, 29, 33] {
            put(&mut buf, i, -8192);
        }
        let cal = DualSense::parse_calibration(&buf).unwrap();
        let m = cal.motion([10 + 16384, -5, 0], [8192, 0, -8192], 0);
        // 16384 raw over a ±8192 span for ±540 °/s → 1080 °/s
        assert!((m.gyro[0] - 1080.0).abs() < 0.1, "{}", m.gyro[0]);
        assert!(m.gyro[1].abs() < 1e-3);
        assert!((m.accel[0] - 1.0).abs() < 1e-3);
        assert!((m.accel[2] + 1.0).abs() < 1e-3);
    }

    #[test]
    fn usb_output_layout() {
        let mut ds = DualSense::new(false, false);
        let r = ds
            .output(&OutputCommand::Rumble {
                weak: 1.0,
                strong: 0.5,
            })
            .unwrap();
        assert_eq!(r.len(), 48);
        assert_eq!(r[0], 0x02);
        assert_eq!(r[3], 255);
        assert_eq!(r[4], 127);
        assert_eq!(r[11], 0x05, "right trigger effect off");
        assert_eq!(r[22], 0x05, "left trigger effect off");
        let r = ds
            .output(&OutputCommand::Lightbar { r: 1, g: 2, b: 3 })
            .unwrap();
        assert_eq!(&r[45..48], &[1, 2, 3]);
    }

    #[test]
    fn bluetooth_output_has_valid_crc() {
        let mut ds = DualSense::new(true, false);
        let r = ds
            .output(&OutputCommand::PlayerLeds { mask: 0b11111 })
            .unwrap();
        assert_eq!(r.len(), 78);
        assert_eq!(r[0], 0x31);
        assert_eq!(r[2], 0x10);
        assert_eq!(r[3 + 43], 0x1F);
        let crc = crc32(&[&[0xA2], &r[..74]]);
        assert_eq!(&r[74..], &crc.to_le_bytes());
        let r2 = ds.build_output();
        assert_eq!(r2[1], 0x10, "sequence counter increments");
    }

    #[test]
    fn rejects_short_trigger_effect() {
        let mut ds = DualSense::new(false, false);
        let err = ds.output(&OutputCommand::TriggerEffects {
            left: vec![1, 2],
            right: vec![0; 11],
        });
        assert!(err.is_err());
    }
}
