//! Step 1: stick released. Centre offset, noise and drift over time.

use crate::geometry::Vec2;
use crate::sample::Sample;
use crate::spikes::detect_spikes;
use crate::stats::{mean, round_to, slope, std_dev};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriftGrade {
    Excellent,
    Acceptable,
    Poor,
    Defective,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestAnalysis {
    pub samples: usize,
    pub duration_ms: f32,
    pub mean: Vec2,
    pub std: Vec2,
    pub std_radial: f32,
    pub max_deviation: f32,
    /// Distance of the resting point from the middle, in % of full travel.
    pub offset_pct: f32,
    pub offset_pct_x: f32,
    pub offset_pct_y: f32,
    /// How fast the resting point wanders (travel units per second).
    pub drift_rate_per_sec: f32,
    pub noise_p2p: f32,
    pub grade: DriftGrade,
    /// Inner deadzone that hides the remaining noise once the centre is
    /// corrected, with a safety margin.
    pub suggested_deadzone: f32,
}

pub const DEADZONE_MARGIN: f32 = 0.02;

pub fn grade_for(offset_pct: f32, std_radial: f32, drift_rate: f32) -> DriftGrade {
    let mut g = if offset_pct < 2.0 {
        DriftGrade::Excellent
    } else if offset_pct < 5.0 {
        DriftGrade::Acceptable
    } else if offset_pct < 10.0 {
        DriftGrade::Poor
    } else {
        DriftGrade::Defective
    };
    // A noisy or wandering centre is never better than "poor".
    if (std_radial > 0.02 || drift_rate > 0.02)
        && matches!(g, DriftGrade::Excellent | DriftGrade::Acceptable)
    {
        g = DriftGrade::Poor;
    }
    g
}

pub fn analyze_rest(samples: &[Sample]) -> Option<RestAnalysis> {
    let finite: Vec<Sample> = samples.iter().copied().filter(Sample::is_finite).collect();
    if finite.len() < 5 {
        return None;
    }
    // One-sample spikes are reported separately; they must not inflate the
    // centre, the noise or the suggested deadzone.
    let spikes: std::collections::HashSet<usize> = detect_spikes(&finite)
        .events
        .iter()
        .map(|e| e.index)
        .collect();
    let s: Vec<&Sample> = finite
        .iter()
        .enumerate()
        .filter(|(i, _)| !spikes.contains(i))
        .map(|(_, p)| p)
        .collect();
    if s.len() < 5 {
        return None;
    }
    let xs: Vec<f32> = s.iter().map(|p| p.x).collect();
    let ys: Vec<f32> = s.iter().map(|p| p.y).collect();
    let ts: Vec<f32> = s.iter().map(|p| p.t as f32 / 1e6).collect();
    let m = Vec2::new(mean(&xs), mean(&ys));
    let dists: Vec<f32> = s.iter().map(|p| (p.pos() - m).length()).collect();
    let std_radial = (dists.iter().map(|d| d * d).sum::<f32>() / dists.len() as f32).sqrt();
    let max_deviation = dists.iter().copied().fold(0.0, f32::max);
    let rate = Vec2::new(slope(&ts, &xs), slope(&ts, &ys)).length();
    let p2p = |v: &[f32]| {
        v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min)
    };
    let offset_pct = m.length() * 100.0;
    let suggested = round_to(
        (max_deviation.max(4.0 * std_radial) + DEADZONE_MARGIN).clamp(0.03, 0.4),
        0.005,
    );
    Some(RestAnalysis {
        samples: s.len(),
        duration_ms: (s.last().unwrap().t.saturating_sub(s[0].t)) as f32 / 1000.0,
        mean: m,
        std: Vec2::new(std_dev(&xs), std_dev(&ys)),
        std_radial,
        max_deviation,
        offset_pct,
        offset_pct_x: m.x.abs() * 100.0,
        offset_pct_y: m.y.abs() * 100.0,
        drift_rate_per_sec: rate,
        noise_p2p: p2p(&xs).max(p2p(&ys)),
        grade: grade_for(offset_pct, std_radial, rate),
        suggested_deadzone: suggested,
    })
}
