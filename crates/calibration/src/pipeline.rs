//! Applies a calibration to raw stick/trigger values.
//!
//! Stick order: swap → centre → per-direction range → invert → deadzone
//! (inner/outer, rescaled) → curve → sensitivity → anti-deadzone → gate.

use crate::geometry::Vec2;
use crate::profile::{DeadzoneMode, GateShape, StickCalibration, TriggerCalibration};
use std::f32::consts::FRAC_PI_8;

/// Maps a magnitude through an inner/outer deadzone so that the output
/// starts at 0 at the inner edge and reaches 1 at `1 - outer`. Values past
/// the outer edge keep growing (corners of square-gated sticks); the gate
/// and the final clamp decide what survives.
pub fn deadzone_scale(m: f32, inner: f32, outer: f32) -> f32 {
    let top = (1.0 - outer).max(inner + 1e-3);
    if m <= inner {
        0.0
    } else {
        (m - inner) / (top - inner)
    }
}

/// Curve on the 0..1 part, linear pass-through of anything above 1.
fn shape(curve: &crate::curve::ResponseCurve, s: f32) -> f32 {
    curve.eval(s.min(1.0)) + (s - 1.0).max(0.0)
}

/// Lifts any non-zero output to at least `ad` (cancels a game's deadzone).
pub fn anti_deadzone(m: f32, ad: f32) -> f32 {
    if m <= 0.0 || ad <= 0.0 {
        m
    } else {
        ad + (1.0 - ad) * m
    }
}

/// Largest radius allowed at `angle` for a gate shape.
pub fn gate_radius(gate: GateShape, angle: f32) -> f32 {
    match gate {
        GateShape::None | GateShape::Square => f32::INFINITY,
        GateShape::Circle => 1.0,
        GateShape::Octagon => {
            // Regular octagon with vertices on the unit circle at 0°, 45°, …
            let sector = std::f32::consts::FRAC_PI_4;
            let local = angle.rem_euclid(sector) - FRAC_PI_8;
            FRAC_PI_8.cos() / local.cos()
        }
    }
}

impl StickCalibration {
    pub fn apply(&self, raw: Vec2) -> Vec2 {
        if !raw.is_finite() {
            return Vec2::ZERO;
        }
        let mut p = if self.swap_axes {
            Vec2::new(raw.y, raw.x)
        } else {
            raw
        };
        p = p - self.center;
        let range = |v: f32, pos: f32, neg: f32| {
            let r = if v >= 0.0 { pos } else { neg };
            v / r.clamp(0.2, 1.5)
        };
        p = Vec2::new(
            range(p.x, self.range_pos.x, self.range_neg.x),
            range(p.y, self.range_pos.y, self.range_neg.y),
        );
        if self.invert_x {
            p.x = -p.x;
        }
        if self.invert_y {
            p.y = -p.y;
        }
        let inner = self.inner_deadzone.clamp(0.0, 0.9);
        let outer = self.outer_deadzone.clamp(0.0, 0.5);
        p = match self.deadzone_mode {
            DeadzoneMode::Radial => {
                let m = p.length();
                if m <= 1e-6 {
                    Vec2::ZERO
                } else {
                    let scaled = deadzone_scale(m, inner, outer);
                    let shaped = anti_deadzone(shape(&self.curve, scaled), self.anti_deadzone);
                    let out = if scaled > 0.0 { shaped } else { 0.0 };
                    p.scale(out / m)
                }
            }
            DeadzoneMode::Axial => {
                let axis = |v: f32| {
                    let s = deadzone_scale(v.abs(), inner, outer);
                    if s <= 0.0 {
                        0.0
                    } else {
                        v.signum() * anti_deadzone(shape(&self.curve, s), self.anti_deadzone)
                    }
                };
                Vec2::new(axis(p.x), axis(p.y))
            }
        };
        p = Vec2::new(p.x * self.sensitivity.x, p.y * self.sensitivity.y);
        let limit = gate_radius(self.gate, p.angle());
        if limit.is_finite() {
            p = p.clamp_length(limit);
        }
        p.clamp_unit_square()
    }
}

impl TriggerCalibration {
    pub fn apply(&self, raw: f32) -> f32 {
        if !raw.is_finite() {
            return 0.0;
        }
        let span = (self.full_value - self.rest_value).max(0.05);
        let v = ((raw - self.rest_value) / span).clamp(0.0, 1.0);
        let dz = self.deadzone.clamp(0.0, 0.5);
        let max = self.max_limit.clamp(dz + 0.05, 1.0);
        let v = ((v - dz) / (max - dz)).clamp(0.0, 1.0);
        if self.digital {
            if v >= self.activation_point {
                1.0
            } else {
                0.0
            }
        } else {
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{CurveKind, ResponseCurve};

    fn identity() -> StickCalibration {
        StickCalibration {
            inner_deadzone: 0.0,
            ..StickCalibration::default()
        }
    }

    #[test]
    fn identity_passes_through() {
        let c = identity();
        for v in [
            Vec2::new(0.3, -0.2),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.5, 0.5),
        ] {
            let o = c.apply(v);
            assert!((o - v).length() < 1e-5, "{v:?} → {o:?}");
        }
    }

    #[test]
    fn radial_deadzone_has_no_jump() {
        let c = StickCalibration {
            inner_deadzone: 0.1,
            ..StickCalibration::default()
        };
        assert_eq!(c.apply(Vec2::new(0.09, 0.0)), Vec2::ZERO);
        let just_outside = c.apply(Vec2::new(0.11, 0.0)).x;
        assert!(just_outside > 0.0 && just_outside < 0.02, "{just_outside}");
        assert!((c.apply(Vec2::new(1.0, 0.0)).x - 1.0).abs() < 1e-5);
        // Direction preserved.
        let o = c.apply(Vec2::new(0.4, 0.4));
        assert!((o.x - o.y).abs() < 1e-6);
    }

    #[test]
    fn outer_deadzone_saturates_early() {
        let c = StickCalibration {
            inner_deadzone: 0.0,
            outer_deadzone: 0.1,
            ..StickCalibration::default()
        };
        assert!((c.apply(Vec2::new(0.9, 0.0)).x - 1.0).abs() < 1e-5);
        assert!((c.apply(Vec2::new(0.45, 0.0)).x - 0.5).abs() < 1e-5);
    }

    #[test]
    fn axial_deadzone_kills_each_axis_independently() {
        let c = StickCalibration {
            inner_deadzone: 0.2,
            deadzone_mode: DeadzoneMode::Axial,
            ..StickCalibration::default()
        };
        let o = c.apply(Vec2::new(0.9, 0.15));
        assert_eq!(o.y, 0.0);
        assert!(o.x > 0.8);
    }

    #[test]
    fn centering_removes_drift() {
        let c = StickCalibration {
            center: Vec2::new(0.05, -0.03),
            inner_deadzone: 0.02,
            ..StickCalibration::default()
        };
        assert_eq!(c.apply(Vec2::new(0.05, -0.03)), Vec2::ZERO);
        assert_eq!(c.apply(Vec2::new(0.06, -0.025)), Vec2::ZERO);
    }

    #[test]
    fn range_rescale_reaches_full() {
        let c = StickCalibration {
            inner_deadzone: 0.0,
            range_pos: Vec2::new(0.8, 1.0),
            range_neg: Vec2::new(1.0, 0.9),
            ..StickCalibration::default()
        };
        assert!((c.apply(Vec2::new(0.8, 0.0)).x - 1.0).abs() < 1e-5);
        assert!((c.apply(Vec2::new(0.0, -0.9)).y + 1.0).abs() < 1e-5);
    }

    #[test]
    fn invert_swap_and_sensitivity() {
        let c = StickCalibration {
            inner_deadzone: 0.0,
            swap_axes: true,
            invert_x: true,
            sensitivity: Vec2::new(1.0, 0.5),
            ..StickCalibration::default()
        };
        let o = c.apply(Vec2::new(0.2, 0.6));
        assert!((o.x + 0.6).abs() < 1e-5);
        assert!((o.y - 0.1).abs() < 1e-5);
    }

    #[test]
    fn curve_and_anti_deadzone() {
        let c = StickCalibration {
            inner_deadzone: 0.0,
            curve: ResponseCurve::preset(CurveKind::Smooth),
            anti_deadzone: 0.2,
            ..StickCalibration::default()
        };
        let o = c.apply(Vec2::new(0.5, 0.0)).x;
        assert!((o - (0.2 + 0.8 * 0.25)).abs() < 1e-5);
        assert_eq!(
            c.apply(Vec2::ZERO),
            Vec2::ZERO,
            "anti-deadzone never moves a resting stick"
        );
    }

    #[test]
    fn gates() {
        let circle = StickCalibration {
            inner_deadzone: 0.0,
            gate: GateShape::Circle,
            ..StickCalibration::default()
        };
        assert!((circle.apply(Vec2::new(1.0, 1.0)).length() - 1.0).abs() < 1e-5);
        let oct = StickCalibration {
            gate: GateShape::Octagon,
            ..circle.clone()
        };
        // At a vertex (45°) the octagon reaches the unit circle; mid-edge it is shorter.
        assert!((oct.apply(Vec2::new(1.0, 1.0)).length() - 1.0).abs() < 1e-4);
        let mid = oct
            .apply(Vec2::from_polar(1.0, 22.5f32.to_radians()))
            .length();
        assert!((mid - FRAC_PI_8.cos()).abs() < 1e-4, "{mid}");
        let square = StickCalibration {
            gate: GateShape::Square,
            ..circle
        };
        assert_eq!(square.apply(Vec2::new(1.0, 1.0)), Vec2::new(1.0, 1.0));
    }

    #[test]
    fn limits_and_garbage() {
        let c = StickCalibration::default();
        assert_eq!(c.apply(Vec2::new(f32::NAN, 0.0)), Vec2::ZERO);
        let o = c.apply(Vec2::new(5.0, -5.0));
        assert!(o.x <= 1.0 && o.y >= -1.0);
    }

    #[test]
    fn trigger_mapping() {
        let t = TriggerCalibration {
            rest_value: 0.02,
            full_value: 0.96,
            deadzone: 0.0,
            max_limit: 1.0,
            ..TriggerCalibration::default()
        };
        assert_eq!(t.apply(0.02), 0.0);
        assert!((t.apply(0.96) - 1.0).abs() < 1e-6);
        assert!((t.apply(0.49) - 0.5).abs() < 1e-3);
        let d = TriggerCalibration {
            digital: true,
            activation_point: 0.6,
            ..TriggerCalibration::default()
        };
        assert_eq!(d.apply(0.5), 0.0);
        assert_eq!(d.apply(0.7), 1.0);
        let dz = TriggerCalibration {
            deadzone: 0.1,
            max_limit: 0.9,
            ..TriggerCalibration::default()
        };
        assert_eq!(dz.apply(0.05), 0.0);
        assert_eq!(dz.apply(0.95), 1.0);
        assert!((dz.apply(0.5) - 0.5).abs() < 1e-5);
        assert_eq!(dz.apply(f32::INFINITY), 0.0);
    }
}
