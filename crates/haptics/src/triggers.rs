//! DualSense adaptive trigger effects, encoded to the 11-byte blocks of the
//! output report (zone-based modes 0x21/0x25/0x26, as documented by the
//! community "TriggerEffectGenerator" research).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TriggerEffect {
    Off,
    /// Constant resistance from `position` (0–9) to the end, `strength` 1–8.
    Feedback {
        position: u8,
        strength: u8,
    },
    /// Resistance between `start` (2–7) and `end` (start+1–8) that "breaks"
    /// like a gun trigger, `strength` 1–8.
    Weapon {
        start: u8,
        end: u8,
        strength: u8,
    },
    /// Vibration from `position` (0–9), `amplitude` 1–8, `frequency` 1–255 Hz.
    Vibration {
        position: u8,
        amplitude: u8,
        frequency: u8,
    },
}

impl TriggerEffect {
    pub fn encode(&self) -> [u8; 11] {
        let mut out = [0u8; 11];
        match *self {
            TriggerEffect::Off => out[0] = 0x05,
            TriggerEffect::Feedback { position, strength } => {
                let position = position.min(9);
                let strength = strength.clamp(1, 8);
                let force = ((strength - 1) & 0x07) as u32;
                let mut active: u16 = 0;
                let mut forces: u32 = 0;
                for i in position..10 {
                    forces |= force << (3 * i as u32);
                    active |= 1 << i;
                }
                out[0] = 0x21;
                out[1..3].copy_from_slice(&active.to_le_bytes());
                out[3..7].copy_from_slice(&forces.to_le_bytes());
            }
            TriggerEffect::Weapon {
                start,
                end,
                strength,
            } => {
                let start = start.clamp(2, 7);
                let end = end.clamp(start + 1, 8);
                let strength = strength.clamp(1, 8);
                let zones: u16 = (1 << start) | (1 << end);
                out[0] = 0x25;
                out[1..3].copy_from_slice(&zones.to_le_bytes());
                out[3] = strength - 1;
            }
            TriggerEffect::Vibration {
                position,
                amplitude,
                frequency,
            } => {
                let position = position.min(9);
                let amplitude = amplitude.clamp(1, 8);
                let strength = ((amplitude - 1) & 0x07) as u32;
                let mut active: u16 = 0;
                let mut amps: u32 = 0;
                for i in position..10 {
                    amps |= strength << (3 * i as u32);
                    active |= 1 << i;
                }
                out[0] = 0x26;
                out[1..3].copy_from_slice(&active.to_le_bytes());
                out[3..7].copy_from_slice(&amps.to_le_bytes());
                out[9] = frequency.max(1);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_is_reset_mode() {
        assert_eq!(
            TriggerEffect::Off.encode(),
            [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn feedback_from_zone_zero_full_strength() {
        let b = TriggerEffect::Feedback {
            position: 0,
            strength: 8,
        }
        .encode();
        assert_eq!(b[0], 0x21);
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), 0x03FF);
        // ten zones × 3 bits of 0b111
        assert_eq!(u32::from_le_bytes([b[3], b[4], b[5], b[6]]), 0x3FFF_FFFF);
    }

    #[test]
    fn feedback_later_position() {
        let b = TriggerEffect::Feedback {
            position: 5,
            strength: 1,
        }
        .encode();
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), 0b11_1110_0000);
        assert_eq!(u32::from_le_bytes([b[3], b[4], b[5], b[6]]), 0);
    }

    #[test]
    fn weapon_zones() {
        let b = TriggerEffect::Weapon {
            start: 2,
            end: 5,
            strength: 8,
        }
        .encode();
        assert_eq!(b[0], 0x25);
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), (1 << 2) | (1 << 5));
        assert_eq!(b[3], 7);
    }

    #[test]
    fn weapon_clamps_invalid_ranges() {
        let b = TriggerEffect::Weapon {
            start: 9,
            end: 1,
            strength: 0,
        }
        .encode();
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), (1 << 7) | (1 << 8));
        assert_eq!(b[3], 0);
    }

    #[test]
    fn vibration_frequency() {
        let b = TriggerEffect::Vibration {
            position: 0,
            amplitude: 4,
            frequency: 40,
        }
        .encode();
        assert_eq!((b[0], b[9]), (0x26, 40));
        assert_eq!(u32::from_le_bytes([b[3], b[4], b[5], b[6]]) & 0x7, 3);
    }

    #[test]
    fn serde_tagged() {
        let e: TriggerEffect =
            serde_json::from_str(r#"{"type":"weapon","start":2,"end":6,"strength":5}"#).unwrap();
        assert_eq!(
            e,
            TriggerEffect::Weapon {
                start: 2,
                end: 6,
                strength: 5
            }
        );
    }
}
