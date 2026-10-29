//! DualShock 4 HID protocol (USB report 0x01, Bluetooth 0x11).

use super::common::*;
use super::crc::seal_bt_report;
use super::Driver;
use crate::backend::{InputError, OutputCommand};
use crate::model::*;

const USB_OUTPUT_LEN: usize = 32;
const BT_OUTPUT_LEN: usize = 78;
const TOUCH_W: f32 = 1920.0;
const TOUCH_H: f32 = 943.0;

pub struct DualShock4 {
    pub bluetooth: bool,
    pub calibration: ImuCalibration,
    weak: u8,
    strong: u8,
    lightbar: (u8, u8, u8),
}

impl DualShock4 {
    pub fn new(bluetooth: bool) -> Self {
        Self {
            bluetooth,
            calibration: ImuCalibration::default(),
            weak: 0,
            strong: 0,
            lightbar: (0x00, 0x20, 0x40),
        }
    }

    pub fn parse_calibration(buf: &[u8], bluetooth: bool) -> Option<ImuCalibration> {
        if buf.len() < 35 {
            return None;
        }
        let v = |i| le_i16(buf, i);
        let (plus, minus) = if bluetooth {
            ([v(7), v(11), v(15)], [v(9), v(13), v(17)])
        } else {
            ([v(7), v(9), v(11)], [v(13), v(15), v(17)])
        };
        ImuCalibration::from_values(
            [v(1), v(3), v(5)],
            plus,
            minus,
            v(19),
            v(21),
            [v(23), v(27), v(31)],
            [v(25), v(29), v(33)],
        )
    }

    /// Payload starts at the left stick X byte (common report part).
    pub fn parse_full(&self, p: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        if p.len() < 42 {
            return None;
        }
        let mut st = DeviceState::neutral(timestamp_us);
        st.report_based = true;
        st.left_stick = stick(p[0], p[1]);
        st.right_stick = stick(p[2], p[3]);
        apply_sony_buttons(&mut st, p[4], p[5], p[6]);
        set_triggers(&mut st, p[7], p[8]);
        let ts = u16::from_le_bytes([p[9], p[10]]) as u64 * 16 / 3; // 5.33 µs units
        let gyro = [le_i16(p, 12), le_i16(p, 14), le_i16(p, 16)];
        let accel = [le_i16(p, 18), le_i16(p, 20), le_i16(p, 22)];
        st.motion = Some(self.calibration.motion(gyro, accel, ts));
        st.battery = Some(parse_battery(p[29]));
        st.touch = Some(TouchState {
            pressed: p[6] & 0x02 != 0,
            points: vec![
                touch_point(&p[34..38], TOUCH_W, TOUCH_H),
                touch_point(&p[38..42], TOUCH_W, TOUCH_H),
            ],
        });
        st.raw_axes = sony_raw_axes(p[0], p[1], p[2], p[3], p[7], p[8]);
        st.raw_buttons = raw_buttons(&st, 18);
        Some(st)
    }

    pub fn build_output(&self) -> Vec<u8> {
        let (mut r, base) = if self.bluetooth {
            let mut r = vec![0u8; BT_OUTPUT_LEN];
            r[0] = 0x11;
            r[1] = 0xC0; // HID + CRC32
            (r, 3)
        } else {
            let mut r = vec![0u8; USB_OUTPUT_LEN];
            r[0] = 0x05;
            (r, 1)
        };
        r[base] = 0x01 | 0x02; // motor + lightbar
        r[base + 3] = self.weak;
        r[base + 4] = self.strong;
        r[base + 5] = self.lightbar.0;
        r[base + 6] = self.lightbar.1;
        r[base + 7] = self.lightbar.2;
        if self.bluetooth {
            seal_bt_report(0xA2, &mut r);
        }
        r
    }
}

pub fn parse_battery(status: u8) -> BatteryInfo {
    let capacity = status & 0x0F;
    let cable = status & 0x10 != 0;
    if cable {
        if capacity >= 10 {
            battery(100, BatteryState::Full)
        } else {
            battery(capacity * 10 + 5, BatteryState::Charging)
        }
    } else {
        battery((capacity * 10 + 5).min(100), BatteryState::Discharging)
    }
}

impl Driver for DualShock4 {
    fn feature_report(&self) -> Option<(u8, usize)> {
        Some(if self.bluetooth {
            (0x05, 41)
        } else {
            (0x02, 37)
        })
    }

    fn on_feature_report(&mut self, buf: &[u8]) {
        match Self::parse_calibration(buf, self.bluetooth) {
            Some(c) => self.calibration = c,
            None => log::warn!("DualShock 4: invalid IMU calibration, using defaults"),
        }
    }

    fn parse(&mut self, d: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        match d.first()? {
            0x01 if d.len() >= 43 => self.parse_full(&d[1..], timestamp_us),
            0x11 if d.len() >= 45 => self.parse_full(&d[3..], timestamp_us),
            _ => None,
        }
    }

    fn output(&mut self, cmd: &OutputCommand) -> Result<Vec<u8>, InputError> {
        match cmd {
            OutputCommand::Rumble { weak, strong } => {
                self.weak = (weak.clamp(0.0, 1.0) * 255.0) as u8;
                self.strong = (strong.clamp(0.0, 1.0) * 255.0) as u8;
            }
            OutputCommand::Lightbar { r, g, b } => self.lightbar = (*r, *g, *b),
            other => {
                return Err(InputError::Unsupported(format!(
                    "{} on DualShock 4",
                    match other {
                        OutputCommand::TriggerEffects { .. } => "adaptive triggers",
                        OutputCommand::PlayerLeds { .. } => "player LEDs",
                        _ => "this output",
                    }
                )))
            }
        }
        Ok(self.build_output())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::crc::crc32;

    #[test]
    fn parses_usb_report() {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1] = 128;
        r[2] = 128;
        r[3] = 0; // RX left
        r[4] = 255; // RY down
        r[5] = 0x80 | 0x08; // triangle, hat released
        r[6] = 0x02 | 0x20; // R1, options
        r[7] = 0x01; // PS
        r[9] = 200;
        r[30] = 0x10 | 0x0B; // cable, full
        r[35] = 0x80;
        r[39] = 0x80;
        let mut ds = DualShock4::new(false);
        let st = ds.parse(&r, 0).unwrap();
        assert!((st.right_stick.x + 1.0).abs() < 1e-3);
        assert!((st.right_stick.y + 1.0).abs() < 1e-2, "down is negative");
        assert!(st.button(Button::North).pressed);
        assert!(st.button(Button::RightBumper).pressed);
        assert!(st.button(Button::Start).pressed);
        assert!(st.button(Button::Guide).pressed);
        assert!(!st.button(Button::DpadUp).pressed);
        assert!((st.right_trigger - 200.0 / 255.0).abs() < 1e-6);
        assert_eq!(st.battery.unwrap().state, BatteryState::Full);
        assert!(!st.touch.unwrap().points[0].active);
    }

    #[test]
    fn battery_states() {
        assert_eq!(parse_battery(0x05).level, Some(55));
        assert_eq!(parse_battery(0x15).state, BatteryState::Charging);
        assert_eq!(parse_battery(0x1B).state, BatteryState::Full);
    }

    #[test]
    fn outputs() {
        let mut ds = DualShock4::new(false);
        let r = ds
            .output(&OutputCommand::Rumble {
                weak: 1.0,
                strong: 0.0,
            })
            .unwrap();
        assert_eq!((r.len(), r[0], r[1], r[4], r[5]), (32, 0x05, 0x03, 255, 0));
        let mut bt = DualShock4::new(true);
        let r = bt
            .output(&OutputCommand::Lightbar { r: 9, g: 8, b: 7 })
            .unwrap();
        assert_eq!((r[0], r[1]), (0x11, 0xC0));
        assert_eq!(&r[8..11], &[9, 8, 7]);
        assert_eq!(&r[74..], &crc32(&[&[0xA2], &r[..74]]).to_le_bytes());
        assert!(bt.output(&OutputCommand::PlayerLeds { mask: 1 }).is_err());
    }
}
