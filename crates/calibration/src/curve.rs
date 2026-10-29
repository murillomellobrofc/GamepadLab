//! Response curves applied to stick magnitude (0..1 → 0..1).

use crate::geometry::Vec2;
use serde::{Deserialize, Serialize};

pub const MAX_CURVE_POINTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurveKind {
    #[default]
    Linear,
    /// x²: finer control near the centre.
    Smooth,
    /// x^0.6: reacts early, good for quick turns.
    Aggressive,
    /// Smoothstep: slow at both ends, fast in the middle.
    SCurve,
    /// User points, interpolated with a monotone cubic (no overshoot).
    Custom,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ResponseCurve {
    pub kind: CurveKind,
    /// Interior control points for `Custom`; (0,0) and (1,1) are implied.
    pub points: Vec<Vec2>,
}

impl ResponseCurve {
    pub fn preset(kind: CurveKind) -> Self {
        Self {
            kind,
            points: Vec::new(),
        }
    }

    pub fn custom(points: Vec<Vec2>) -> Self {
        Self {
            kind: CurveKind::Custom,
            points,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.points.len() > MAX_CURVE_POINTS {
            return Err(format!("at most {MAX_CURVE_POINTS} curve points"));
        }
        for p in &self.points {
            if !p.is_finite() || !(0.0..=1.0).contains(&p.x) || !(0.0..=1.0).contains(&p.y) {
                return Err("curve points must be inside 0..1".into());
            }
        }
        Ok(())
    }

    /// Full knot list for the custom curve: endpoints added, sorted by x,
    /// duplicates dropped and y forced non-decreasing.
    pub fn knots(&self) -> Vec<Vec2> {
        let mut pts: Vec<Vec2> = self
            .points
            .iter()
            .filter(|p| p.is_finite())
            .map(|p| Vec2::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)))
            .filter(|p| p.x > 0.0 && p.x < 1.0)
            .collect();
        pts.push(Vec2::ZERO);
        pts.push(Vec2::ONE);
        pts.sort_by(|a, b| a.x.total_cmp(&b.x));
        pts.dedup_by(|a, b| (a.x - b.x).abs() < 1e-4);
        let mut max_y = 0.0f32;
        for p in pts.iter_mut() {
            max_y = max_y.max(p.y);
            p.y = max_y;
        }
        if let Some(last) = pts.last_mut() {
            last.y = 1.0;
        }
        pts
    }

    pub fn eval(&self, x: f32) -> f32 {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self.kind {
            CurveKind::Linear => x,
            CurveKind::Smooth => x * x,
            CurveKind::Aggressive => x.powf(0.6),
            CurveKind::SCurve => x * x * (3.0 - 2.0 * x),
            CurveKind::Custom => monotone_cubic(&self.knots(), x),
        }
    }
}

/// Fritsch–Carlson monotone cubic Hermite interpolation.
pub fn monotone_cubic(k: &[Vec2], x: f32) -> f32 {
    let n = k.len();
    if n < 2 {
        return x;
    }
    let mut d = vec![0.0f32; n - 1];
    for i in 0..n - 1 {
        let dx = k[i + 1].x - k[i].x;
        d[i] = if dx > 0.0 {
            (k[i + 1].y - k[i].y) / dx
        } else {
            0.0
        };
    }
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 {
            0.0
        } else {
            (d[i - 1] + d[i]) / 2.0
        };
    }
    for i in 0..n - 1 {
        if d[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let a = m[i] / d[i];
        let b = m[i + 1] / d[i];
        let s = a * a + b * b;
        if s > 9.0 {
            let t = 3.0 / s.sqrt();
            m[i] = t * a * d[i];
            m[i + 1] = t * b * d[i];
        }
    }
    let i = match k.iter().position(|p| p.x >= x) {
        Some(0) => 0,
        Some(i) => i - 1,
        None => n - 2,
    };
    let h = k[i + 1].x - k[i].x;
    if h <= 0.0 {
        return k[i].y;
    }
    let t = (x - k[i].x) / h;
    let t2 = t * t;
    let t3 = t2 * t;
    let y = (2.0 * t3 - 3.0 * t2 + 1.0) * k[i].y
        + (t3 - 2.0 * t2 + t) * h * m[i]
        + (-2.0 * t3 + 3.0 * t2) * k[i + 1].y
        + (t3 - t2) * h * m[i + 1];
    y.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_hit_endpoints_and_order() {
        for kind in [
            CurveKind::Linear,
            CurveKind::Smooth,
            CurveKind::Aggressive,
            CurveKind::SCurve,
        ] {
            let c = ResponseCurve::preset(kind);
            assert_eq!(c.eval(0.0), 0.0, "{kind:?}");
            assert!((c.eval(1.0) - 1.0).abs() < 1e-6, "{kind:?}");
            let mut prev = -1.0;
            for i in 0..=100 {
                let y = c.eval(i as f32 / 100.0);
                assert!(y >= prev - 1e-6, "{kind:?} must be monotone");
                prev = y;
            }
        }
        assert!(ResponseCurve::preset(CurveKind::Smooth).eval(0.5) < 0.5);
        assert!(ResponseCurve::preset(CurveKind::Aggressive).eval(0.5) > 0.5);
        assert!((ResponseCurve::preset(CurveKind::SCurve).eval(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn custom_passes_through_points_without_overshoot() {
        let c = ResponseCurve::custom(vec![Vec2::new(0.3, 0.1), Vec2::new(0.7, 0.9)]);
        assert!((c.eval(0.3) - 0.1).abs() < 1e-5);
        assert!((c.eval(0.7) - 0.9).abs() < 1e-5);
        let mut prev = 0.0;
        for i in 0..=200 {
            let y = c.eval(i as f32 / 200.0);
            assert!((0.0..=1.0).contains(&y));
            assert!(y >= prev - 1e-6, "monotone at {i}");
            prev = y;
        }
    }

    #[test]
    fn custom_repairs_bad_points() {
        let c = ResponseCurve::custom(vec![
            Vec2::new(0.8, 0.2), // y lower than an earlier point → raised
            Vec2::new(0.4, 0.6),
            Vec2::new(0.4, 0.7), // duplicate x dropped
            Vec2::new(f32::NAN, 0.5),
        ]);
        let k = c.knots();
        assert_eq!(k.len(), 4);
        assert!(k.windows(2).all(|w| w[0].x < w[1].x && w[0].y <= w[1].y));
        assert!(c.eval(0.9) >= c.eval(0.5));
    }

    #[test]
    fn validation() {
        assert!(ResponseCurve::custom(vec![Vec2::new(1.5, 0.2)])
            .validate()
            .is_err());
        assert!(ResponseCurve::custom(vec![Vec2::new(0.5, 0.2); 17])
            .validate()
            .is_err());
        assert!(ResponseCurve::custom(vec![Vec2::new(0.5, 0.2)])
            .validate()
            .is_ok());
    }

    #[test]
    fn handles_non_finite_input() {
        assert_eq!(ResponseCurve::default().eval(f32::NAN), 0.0);
        assert_eq!(ResponseCurve::default().eval(4.0), 1.0);
    }
}
