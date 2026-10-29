//! Time-varying rumble patterns.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternKind {
    Constant,
    /// On for half the period, off for the other half.
    Pulse,
    /// Smooth sine between 0 and the set intensity.
    Wave,
    RampUp,
    RampDown,
    /// Two short beats then a rest, like a heartbeat.
    Heartbeat,
}

/// One rumble segment. Intensities are 0..1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RumbleRequest {
    pub weak: f32,
    pub strong: f32,
    pub duration_ms: u32,
    pub pattern: PatternKind,
    /// Period for pulse/wave/heartbeat (ignored by the others).
    pub period_ms: u32,
}

impl RumbleRequest {
    pub fn constant(weak: f32, strong: f32, duration_ms: u32) -> Self {
        Self {
            weak,
            strong,
            duration_ms,
            pattern: PatternKind::Constant,
            period_ms: 500,
        }
    }

    /// Clamps values into safe ranges (0..1, 10 ms – 30 s).
    pub fn sanitized(self) -> Self {
        Self {
            weak: finite01(self.weak),
            strong: finite01(self.strong),
            duration_ms: self.duration_ms.clamp(10, 30_000),
            period_ms: self.period_ms.clamp(40, 10_000),
            ..self
        }
    }

    /// Envelope 0..1 at `t_ms` from the start of the segment.
    pub fn envelope(&self, t_ms: f32) -> f32 {
        if t_ms < 0.0 || t_ms >= self.duration_ms as f32 {
            return 0.0;
        }
        let period = self.period_ms.max(1) as f32;
        let phase = (t_ms % period) / period;
        match self.pattern {
            PatternKind::Constant => 1.0,
            PatternKind::Pulse => {
                if phase < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            PatternKind::Wave => 0.5 - 0.5 * (phase * TAU).cos(),
            PatternKind::RampUp => t_ms / self.duration_ms as f32,
            PatternKind::RampDown => 1.0 - t_ms / self.duration_ms as f32,
            PatternKind::Heartbeat => {
                if phase < 0.12 || (0.25..0.37).contains(&phase) {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// (weak, strong) at `t_ms`.
    pub fn intensity_at(&self, t_ms: f32) -> (f32, f32) {
        let k = self.envelope(t_ms);
        (self.weak * k, self.strong * k)
    }
}

fn finite01(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Weak motor alone, strong motor alone, then both, with short gaps, so the
/// person can tell the two motors apart (and see it in the UI).
pub fn motor_test_program(intensity: f32) -> Vec<RumbleRequest> {
    let i = finite01(intensity);
    let gap = RumbleRequest::constant(0.0, 0.0, 400);
    vec![
        RumbleRequest::constant(i, 0.0, 1000),
        gap,
        RumbleRequest::constant(0.0, i, 1000),
        gap,
        RumbleRequest::constant(i, i, 1000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(pattern: PatternKind) -> RumbleRequest {
        RumbleRequest {
            weak: 0.5,
            strong: 1.0,
            duration_ms: 1000,
            pattern,
            period_ms: 200,
        }
    }

    #[test]
    fn constant_and_bounds() {
        let r = req(PatternKind::Constant);
        assert_eq!(r.intensity_at(0.0), (0.5, 1.0));
        assert_eq!(r.intensity_at(999.0), (0.5, 1.0));
        assert_eq!(r.intensity_at(1000.0), (0.0, 0.0));
        assert_eq!(r.intensity_at(-1.0), (0.0, 0.0));
    }

    #[test]
    fn pulse_halves() {
        let r = req(PatternKind::Pulse);
        assert_eq!(r.envelope(50.0), 1.0);
        assert_eq!(r.envelope(150.0), 0.0);
        assert_eq!(r.envelope(250.0), 1.0);
    }

    #[test]
    fn wave_is_smooth() {
        let r = req(PatternKind::Wave);
        assert!(r.envelope(0.0) < 1e-6);
        assert!((r.envelope(100.0) - 1.0).abs() < 1e-5);
        assert!((r.envelope(50.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ramps() {
        assert!((req(PatternKind::RampUp).envelope(250.0) - 0.25).abs() < 1e-6);
        assert!((req(PatternKind::RampDown).envelope(250.0) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn heartbeat_has_two_beats() {
        let r = req(PatternKind::Heartbeat);
        let on: Vec<bool> = (0..20).map(|i| r.envelope(i as f32 * 10.0) > 0.5).collect();
        let rising = on.windows(2).filter(|w| !w[0] && w[1]).count() + usize::from(on[0]);
        assert_eq!(rising, 2);
    }

    #[test]
    fn sanitize_clamps() {
        let r = RumbleRequest {
            weak: 3.0,
            strong: f32::NAN,
            duration_ms: 0,
            pattern: PatternKind::Pulse,
            period_ms: 1,
        }
        .sanitized();
        assert_eq!(
            (r.weak, r.strong, r.duration_ms, r.period_ms),
            (1.0, 0.0, 10, 40)
        );
    }

    #[test]
    fn motor_test_separates_motors() {
        let p = motor_test_program(0.8);
        assert_eq!(p.len(), 5);
        assert_eq!((p[0].weak, p[0].strong), (0.8, 0.0));
        assert_eq!((p[2].weak, p[2].strong), (0.0, 0.8));
        assert_eq!((p[4].weak, p[4].strong), (0.8, 0.8));
    }

    /// Shared with the UI preview (apps/ui/src/lib/haptics.test.ts).
    #[test]
    fn golden_values_match_ui_fixture() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/haptics_golden.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let r: RumbleRequest = serde_json::from_value(case["request"].clone()).unwrap();
            for pair in case["samples"].as_array().unwrap() {
                let t = pair[0].as_f64().unwrap() as f32;
                let expect = pair[1].as_f64().unwrap() as f32;
                assert!(
                    (r.envelope(t) - expect).abs() < 1e-4,
                    "{:?} at {t}",
                    r.pattern
                );
            }
        }
    }
}
