//! Switch Pro stick calibration as stored in the controller's SPI flash
//! (layout documented by the Nintendo_Switch_Reverse_Engineering project):
//! nine bytes per stick packing six 12-bit values; factory data at 0x603D,
//! user data (written by the console's "Calibrate Control Sticks") at
//! 0x8010 with the magic `B2 A1` in front of each stick.

use gamepadlab_calibration::Sample;
use serde::{Deserialize, Serialize};

pub const FACTORY_STICKS_ADDR: u32 = 0x603D;
pub const FACTORY_STICKS_LEN: u8 = 18;
pub const USER_STICKS_ADDR: u32 = 0x8010;
pub const USER_STICKS_LEN: u8 = 22;
pub const USER_MAGIC: [u8; 2] = [0xB2, 0xA1];

/// Smallest deltas the console accepts for a Pro Controller; below these it
/// ignores the calibration and applies its own default.
pub const MIN_DELTA: u16 = 0x542;
pub const MIN_DELTA_X_MAX: u16 = 0x4D5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchStickCal {
    pub center_x: u16,
    pub center_y: u16,
    pub x_max_above: u16,
    pub x_min_below: u16,
    pub y_max_above: u16,
    pub y_min_below: u16,
}

impl Default for SwitchStickCal {
    /// Used before anything is read from the controller.
    fn default() -> Self {
        Self {
            center_x: 2048,
            center_y: 2048,
            x_max_above: 1600,
            x_min_below: 1600,
            y_max_above: 1600,
            y_min_below: 1600,
        }
    }
}

fn unpack(b: &[u8]) -> [u16; 6] {
    let pair = |i: usize| {
        (
            ((b[i + 1] as u16) << 8) & 0xF00 | b[i] as u16,
            ((b[i + 2] as u16) << 4) | (b[i + 1] as u16 >> 4),
        )
    };
    let (a, c) = pair(0);
    let (d, e) = pair(3);
    let (f, g) = pair(6);
    [a, c, d, e, f, g]
}

fn pack(v: [u16; 6]) -> [u8; 9] {
    let mut out = [0u8; 9];
    for k in 0..3 {
        let (a, b) = (v[2 * k] & 0xFFF, v[2 * k + 1] & 0xFFF);
        out[3 * k] = (a & 0xFF) as u8;
        out[3 * k + 1] = ((a >> 8) as u8 & 0x0F) | (((b & 0x0F) as u8) << 4);
        out[3 * k + 2] = (b >> 4) as u8;
    }
    out
}

impl SwitchStickCal {
    /// Left stick order: x max, y max, x center, y center, x min, y min.
    pub fn decode_left(b: &[u8]) -> Self {
        let d = unpack(b);
        Self {
            x_max_above: d[0],
            y_max_above: d[1],
            center_x: d[2],
            center_y: d[3],
            x_min_below: d[4],
            y_min_below: d[5],
        }
    }

    /// Right stick order: x center, y center, x min, y min, x max, y max.
    pub fn decode_right(b: &[u8]) -> Self {
        let d = unpack(b);
        Self {
            center_x: d[0],
            center_y: d[1],
            x_min_below: d[2],
            y_min_below: d[3],
            x_max_above: d[4],
            y_max_above: d[5],
        }
    }

    pub fn encode_left(&self) -> [u8; 9] {
        pack([
            self.x_max_above,
            self.y_max_above,
            self.center_x,
            self.center_y,
            self.x_min_below,
            self.y_min_below,
        ])
    }

    pub fn encode_right(&self) -> [u8; 9] {
        pack([
            self.center_x,
            self.center_y,
            self.x_min_below,
            self.y_min_below,
            self.x_max_above,
            self.y_max_above,
        ])
    }

    /// Raw 12-bit reading → -1..1 (y up positive), unclamped past ±1.
    pub fn normalize(&self, raw_x: u16, raw_y: u16) -> (f32, f32) {
        let axis = |raw: u16, c: u16, pos: u16, neg: u16| {
            let d = raw as f32 - c as f32;
            let span = if d >= 0.0 { pos } else { neg }.max(1) as f32;
            (d / span).clamp(-1.25, 1.25)
        };
        (
            axis(raw_x, self.center_x, self.x_max_above, self.x_min_below),
            axis(raw_y, self.center_y, self.y_max_above, self.y_min_below),
        )
    }

    /// Inverse of [`normalize`]: normalised value back to raw units.
    pub fn denormalize(&self, x: f32, y: f32) -> (f32, f32) {
        let axis = |v: f32, c: u16, pos: u16, neg: u16| {
            c as f32 + v * if v >= 0.0 { pos } else { neg } as f32
        };
        (
            axis(x, self.center_x, self.x_max_above, self.x_min_below),
            axis(y, self.center_y, self.y_max_above, self.y_min_below),
        )
    }

    /// Problems the console would reject (or that would break the stick).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, v, min) in [
            ("xMaxAbove", self.x_max_above, MIN_DELTA_X_MAX),
            ("xMinBelow", self.x_min_below, MIN_DELTA),
            ("yMaxAbove", self.y_max_above, MIN_DELTA),
            ("yMinBelow", self.y_min_below, MIN_DELTA),
        ] {
            if v < min {
                out.push(format!(
                    "{name} below the console minimum (0x{v:03X} < 0x{min:03X})"
                ));
            }
        }
        let fits = |c: u16, pos: u16, neg: u16| c >= neg && c as u32 + pos as u32 <= 0xFFF;
        if !fits(self.center_x, self.x_max_above, self.x_min_below)
            || !fits(self.center_y, self.y_max_above, self.y_min_below)
        {
            out.push("range does not fit in 12 bits".into());
        }
        out
    }
}

/// Builds the 22-byte user calibration block (magic + left + magic + right).
pub fn user_block(left: &SwitchStickCal, right: &SwitchStickCal) -> Vec<u8> {
    let mut v = Vec::with_capacity(22);
    v.extend_from_slice(&USER_MAGIC);
    v.extend_from_slice(&left.encode_left());
    v.extend_from_slice(&USER_MAGIC);
    v.extend_from_slice(&right.encode_right());
    v
}

/// Parses a 22-byte user block; a stick without its magic has no user calibration.
pub fn parse_user_block(b: &[u8]) -> (Option<SwitchStickCal>, Option<SwitchStickCal>) {
    if b.len() < 22 {
        return (None, None);
    }
    let left = (b[0..2] == USER_MAGIC).then(|| SwitchStickCal::decode_left(&b[2..11]));
    let right = (b[11..13] == USER_MAGIC).then(|| SwitchStickCal::decode_right(&b[13..22]));
    (left, right)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchProposal {
    pub calibration: SwitchStickCal,
    /// Some deltas were raised to the console minimum: the stick travels
    /// less than the console expects.
    pub raised_to_minimum: bool,
    pub problems: Vec<String>,
}

/// Proposes a new user calibration from a resting capture and a full
/// rotation, both in normalised units produced with `current`.
pub fn propose(
    rest: &[Sample],
    rotation: &[Sample],
    current: &SwitchStickCal,
) -> Option<SwitchProposal> {
    let raw = |s: &Sample| current.denormalize(s.x, s.y);
    let rest: Vec<(f32, f32)> = rest.iter().filter(|s| s.is_finite()).map(raw).collect();
    let rot: Vec<(f32, f32)> = rotation.iter().filter(|s| s.is_finite()).map(raw).collect();
    if rest.len() < 5 || rot.len() < 32 {
        return None;
    }
    let cx = rest.iter().map(|p| p.0).sum::<f32>() / rest.len() as f32;
    let cy = rest.iter().map(|p| p.1).sum::<f32>() / rest.len() as f32;
    let reach = |f: &dyn Fn(&(f32, f32)) -> f32| rot.iter().map(f).fold(0.0f32, f32::max);
    // 3 % short of the measured edge so full deflection always reads 100 %.
    let shrink = |v: f32| (v * 0.97).round().clamp(0.0, 2047.0) as u16;
    let mut cal = SwitchStickCal {
        center_x: cx.round().clamp(0.0, 4095.0) as u16,
        center_y: cy.round().clamp(0.0, 4095.0) as u16,
        x_max_above: shrink(reach(&|p| p.0 - cx)),
        x_min_below: shrink(reach(&|p| cx - p.0)),
        y_max_above: shrink(reach(&|p| p.1 - cy)),
        y_min_below: shrink(reach(&|p| cy - p.1)),
    };
    let mut raised = false;
    for (v, min) in [
        (&mut cal.x_max_above, MIN_DELTA_X_MAX),
        (&mut cal.x_min_below, MIN_DELTA),
        (&mut cal.y_max_above, MIN_DELTA),
        (&mut cal.y_min_below, MIN_DELTA),
    ] {
        if *v < min {
            *v = min;
            raised = true;
        }
    }
    Some(SwitchProposal {
        problems: cal.problems(),
        calibration: cal,
        raised_to_minimum: raised,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_documented_sample() {
        // Left-stick sample values from the reverse-engineering notes.
        let cal = SwitchStickCal {
            x_max_above: 0x4F7,
            y_max_above: 0x424,
            center_x: 0x79F,
            center_y: 0x8A0,
            x_min_below: 0x510,
            y_min_below: 0x479,
        };
        let bytes = cal.encode_left();
        assert_eq!(SwitchStickCal::decode_left(&bytes), cal);
        let r = cal.encode_right();
        assert_eq!(SwitchStickCal::decode_right(&r), cal);
        assert_ne!(bytes, r, "left and right use different orders");
    }

    #[test]
    fn packing_matches_reference_formula() {
        let b = [0xC6, 0xF8, 0x71, 0x68, 0x84, 0x42, 0xA3, 0x54, 0x42];
        let d = unpack(&b);
        assert_eq!(d[0], ((0xF8u16 << 8) & 0xF00) | 0xC6);
        assert_eq!(d[1], (0x71u16 << 4) | (0xF8 >> 4));
        assert_eq!(pack(d), b);
    }

    #[test]
    fn user_block_roundtrip_and_magic() {
        let l = SwitchStickCal::default();
        let r = SwitchStickCal {
            center_x: 2000,
            ..SwitchStickCal::default()
        };
        let block = user_block(&l, &r);
        assert_eq!(block.len(), 22);
        assert_eq!(&block[0..2], &USER_MAGIC);
        assert_eq!(&block[11..13], &USER_MAGIC);
        assert_eq!(parse_user_block(&block), (Some(l), Some(r)));
        assert_eq!(
            parse_user_block(&[0xFF; 22]),
            (None, None),
            "erased flash = no user calibration"
        );
    }

    #[test]
    fn normalize_denormalize_inverse() {
        let c = SwitchStickCal {
            center_x: 1900,
            center_y: 2100,
            x_max_above: 1400,
            x_min_below: 1500,
            y_max_above: 1450,
            y_min_below: 1380,
        };
        for (rx, ry) in [(1900, 2100), (3300, 2100), (400, 720), (2600, 3550)] {
            let (x, y) = c.normalize(rx, ry);
            let (bx, by) = c.denormalize(x, y);
            assert!((bx - rx as f32).abs() < 0.5 && (by - ry as f32).abs() < 0.5);
        }
        assert_eq!(c.normalize(1900 + 1400, 2100).0, 1.0);
    }

    #[test]
    fn proposal_from_captures() {
        let current = SwitchStickCal::default();
        // Rest at raw (2000, 2060); rotation with raw deltas of 1450.
        let to_norm = |raw: f32| (raw - 2048.0) / 1600.0;
        let rest: Vec<Sample> = (0..50)
            .map(|i| Sample::new(i, to_norm(2000.0), to_norm(2060.0)))
            .collect();
        let rot: Vec<Sample> = (0..400)
            .map(|i| {
                let a = i as f32 / 400.0 * std::f32::consts::TAU;
                let rx = 2000.0 + 1450.0 * a.cos();
                let ry = 2060.0 + 1450.0 * a.sin();
                Sample::new(i, (rx - 2048.0) / 1600.0, (ry - 2048.0) / 1600.0)
            })
            .collect();
        let p = propose(&rest, &rot, &current).unwrap();
        assert_eq!(
            (p.calibration.center_x, p.calibration.center_y),
            (2000, 2060)
        );
        assert!(
            (p.calibration.x_max_above as i32 - 1406).abs() <= 2,
            "{:?}",
            p.calibration
        );
        assert!(!p.raised_to_minimum);
        assert!(p.problems.is_empty());
    }

    #[test]
    fn short_travel_is_raised_to_console_minimum() {
        let current = SwitchStickCal::default();
        let rest = vec![Sample::new(0, 0.0, 0.0); 10];
        let rot: Vec<Sample> = (0..100)
            .map(|i| {
                let a = i as f32 / 100.0 * std::f32::consts::TAU;
                Sample::new(i, 0.5 * a.cos(), 0.5 * a.sin())
            })
            .collect();
        let p = propose(&rest, &rot, &current).unwrap();
        assert!(p.raised_to_minimum);
        assert_eq!(p.calibration.x_min_below, MIN_DELTA);
        assert!(p.calibration.problems().is_empty());
    }

    #[test]
    fn problems_flag_bad_values() {
        let bad = SwitchStickCal {
            center_x: 100,
            x_min_below: 0x600,
            ..SwitchStickCal::default()
        };
        assert!(!bad.problems().is_empty());
        let tiny = SwitchStickCal {
            y_max_above: 0x100,
            ..SwitchStickCal::default()
        };
        assert!(tiny.problems()[0].contains("yMaxAbove"));
    }
}
