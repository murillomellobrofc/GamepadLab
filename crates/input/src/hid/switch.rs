//! Nintendo Switch Pro Controller, full report mode (0x30). Experimental:
//! only enabled on request, because switching modes can stop the OS driver
//! from reading the controller until it is reconnected.

use super::Driver;
use crate::backend::{InputError, OutputCommand};
use crate::firmware::switch_cal::{self, SwitchStickCal};
use crate::firmware::{switch_spi_read, switch_subcommand, FeatureIo};
use crate::model::*;
use gamepadlab_calibration::Vec2;

const ACCEL_G_PER_LSB: f32 = 0.000_244;
const GYRO_DPS_PER_LSB: f32 = 0.061_03;
const NEUTRAL_RUMBLE: [u8; 8] = [0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40];

pub struct SwitchPro {
    pub usb: bool,
    counter: u8,
    /// Active stick calibration (user block if present, else factory).
    pub cal_left: SwitchStickCal,
    pub cal_right: SwitchStickCal,
}

impl SwitchPro {
    pub fn new(usb: bool) -> Self {
        Self {
            usb,
            counter: 0,
            cal_left: SwitchStickCal::default(),
            cal_right: SwitchStickCal::default(),
        }
    }

    /// Reads factory and user stick calibration from SPI flash.
    pub fn load_calibration(&mut self, io: &dyn FeatureIo) {
        let factory = switch_spi_read(
            io,
            &mut self.counter,
            switch_cal::FACTORY_STICKS_ADDR,
            switch_cal::FACTORY_STICKS_LEN,
        );
        let user = switch_spi_read(
            io,
            &mut self.counter,
            switch_cal::USER_STICKS_ADDR,
            switch_cal::USER_STICKS_LEN,
        );
        match factory {
            Ok(f) => {
                self.cal_left = SwitchStickCal::decode_left(&f[0..9]);
                self.cal_right = SwitchStickCal::decode_right(&f[9..18]);
            }
            Err(e) => log::warn!("Switch Pro: factory calibration unreadable: {e}"),
        }
        if let Ok(u) = user {
            let (l, r) = switch_cal::parse_user_block(&u);
            if let Some(l) = l {
                self.cal_left = l;
            }
            if let Some(r) = r {
                self.cal_right = r;
            }
        }
    }

    pub fn subcommand(&mut self, id: u8, args: &[u8]) -> Vec<u8> {
        let mut r = vec![0u8; 49];
        r[0] = 0x01;
        r[1] = self.counter & 0x0F;
        self.counter = self.counter.wrapping_add(1);
        r[2..10].copy_from_slice(&NEUTRAL_RUMBLE);
        r[10] = id;
        r[11..11 + args.len()].copy_from_slice(args);
        r
    }

    /// Reports to send after opening: USB handshake, full report mode, IMU
    /// on, player 1 light.
    pub fn init_sequence(&mut self) -> Vec<Vec<u8>> {
        let mut seq = Vec::new();
        if self.usb {
            for cmd in [0x02u8, 0x03, 0x02, 0x04] {
                seq.push(vec![0x80, cmd]);
            }
        }
        seq.push(self.subcommand(0x03, &[0x30]));
        seq.push(self.subcommand(0x40, &[0x01]));
        seq.push(self.subcommand(0x30, &[0x01]));
        seq
    }

    pub fn parse_full(&self, d: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        if d.len() < 25 || d[0] != 0x30 {
            return None;
        }
        let mut st = DeviceState::neutral(timestamp_us);
        st.report_based = true;
        let bit = |v: u8, n: u8| ButtonState::digital(v & (1 << n) != 0);
        let (r, s, l) = (d[3], d[4], d[5]);
        // Positional mapping: B is the bottom face button on Nintendo pads.
        st.set_button(Button::West, bit(r, 0)); // Y
        st.set_button(Button::North, bit(r, 1)); // X
        st.set_button(Button::South, bit(r, 2)); // B
        st.set_button(Button::East, bit(r, 3)); // A
        st.set_button(Button::RightBumper, bit(r, 6));
        st.set_button(Button::RightTrigger, bit(r, 7));
        st.set_button(Button::Select, bit(s, 0));
        st.set_button(Button::Start, bit(s, 1));
        st.set_button(Button::RightStick, bit(s, 2));
        st.set_button(Button::LeftStick, bit(s, 3));
        st.set_button(Button::Guide, bit(s, 4));
        st.set_button(Button::Misc, bit(s, 5)); // Capture
        st.set_button(Button::DpadDown, bit(l, 0));
        st.set_button(Button::DpadUp, bit(l, 1));
        st.set_button(Button::DpadRight, bit(l, 2));
        st.set_button(Button::DpadLeft, bit(l, 3));
        st.set_button(Button::LeftBumper, bit(l, 6));
        st.set_button(Button::LeftTrigger, bit(l, 7));
        st.left_trigger = st.button(Button::LeftTrigger).value;
        st.right_trigger = st.button(Button::RightTrigger).value;

        let (lx, ly) = stick12(&d[6..9]);
        let (rx, ry) = stick12(&d[9..12]);
        let (nlx, nly) = self.cal_left.normalize(lx, ly);
        let (nrx, nry) = self.cal_right.normalize(rx, ry);
        st.left_stick = Vec2::new(nlx, nly);
        st.right_stick = Vec2::new(nrx, nry);

        let i = |o: usize| i16::from_le_bytes([d[13 + o], d[14 + o]]);
        st.motion = Some(MotionSample {
            accel: [i(0), i(2), i(4)].map(|v| v as f32 * ACCEL_G_PER_LSB),
            gyro: [i(6), i(8), i(10)].map(|v| v as f32 * GYRO_DPS_PER_LSB),
            timestamp_us,
        });
        let nib = d[2] >> 4;
        st.battery = Some(BatteryInfo {
            level: Some(((nib >> 1) as u16 * 25).min(100) as u8),
            state: if nib & 1 != 0 {
                BatteryState::Charging
            } else {
                BatteryState::Discharging
            },
        });
        st.raw_axes = [
            ("LX", lx, nlx),
            ("LY", ly, nly),
            ("RX", rx, nrx),
            ("RY", ry, nry),
        ]
        .iter()
        .enumerate()
        .map(|(k, (label, raw, value))| RawAxisValue {
            code: k as u32,
            label: (*label).into(),
            value: *value,
            raw: Some(*raw as i32),
        })
        .collect();
        st.raw_buttons = Button::ALL[..19]
            .iter()
            .map(|b| {
                let s = st.button(*b);
                RawButtonValue {
                    code: b.index() as u32,
                    label: format!("{b:?}"),
                    pressed: s.pressed,
                    value: s.value,
                }
            })
            .collect();
        Some(st)
    }
}

fn stick12(b: &[u8]) -> (u16, u16) {
    let x = b[0] as u16 | ((b[1] as u16 & 0x0F) << 8);
    let y = (b[1] as u16 >> 4) | ((b[2] as u16) << 4);
    (x, y)
}

impl Driver for SwitchPro {
    fn init(&mut self, io: &dyn FeatureIo) {
        if self.usb {
            for cmd in [0x02u8, 0x03, 0x02, 0x04] {
                let _ = io.write(&[0x80, cmd]);
                io.sleep_ms(20);
            }
        }
        self.load_calibration(io);
        for (id, args) in [(0x03u8, [0x30u8]), (0x40, [0x01]), (0x30, [0x01])] {
            if let Err(e) = switch_subcommand(io, &mut self.counter, id, &args) {
                log::warn!("Switch Pro: init subcommand 0x{id:02X} failed: {e}");
            }
        }
    }

    fn after_firmware(&mut self, io: &dyn FeatureIo) {
        self.load_calibration(io);
    }

    fn parse(&mut self, d: &[u8], timestamp_us: u64) -> Option<DeviceState> {
        self.parse_full(d, timestamp_us)
    }

    fn output(&mut self, _cmd: &OutputCommand) -> Result<Vec<u8>, InputError> {
        Err(InputError::Unsupported(
            "HD rumble on Switch Pro (not implemented)".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_report() {
        let mut d = vec![0u8; 49];
        d[0] = 0x30;
        d[2] = 0x81; // level 8 (full) not charging? high nibble 8
        d[3] = 0x08; // A
        d[5] = 0x80; // ZL
                     // left stick x = 3648 (full right), y = 2048
        let (x, y) = (3648u16, 2048u16);
        d[6] = (x & 0xFF) as u8;
        d[7] = ((x >> 8) as u8 & 0x0F) | (((y & 0x0F) as u8) << 4);
        d[8] = (y >> 4) as u8;
        d[9..12].copy_from_slice(&[0x00, 0x08, 0x80]); // right ≈ centre
        d[13..15].copy_from_slice(&4096i16.to_le_bytes());
        let sw = SwitchPro::new(false);
        let st = sw.parse_full(&d, 0).unwrap();
        assert!(st.button(Button::East).pressed);
        assert!(st.button(Button::LeftTrigger).pressed);
        assert!((st.left_stick.x - 1.0).abs() < 1e-3);
        assert!(st.left_stick.y.abs() < 1e-3);
        assert_eq!(st.battery.unwrap().level, Some(100));
        assert!((st.motion.unwrap().accel[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn subcommand_layout_and_counter() {
        let mut sw = SwitchPro::new(true);
        let seq = sw.init_sequence();
        assert_eq!(seq.len(), 7);
        assert_eq!(seq[0], vec![0x80, 0x02]);
        assert_eq!(seq[4][0], 0x01);
        assert_eq!(seq[4][10], 0x03);
        assert_eq!(seq[4][11], 0x30);
        assert_eq!(seq[5][1], 1, "packet counter increments");
    }
}
