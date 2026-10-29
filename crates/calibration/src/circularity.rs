//! Step 2: full rotation along the edge. Reach per sector, roundness and
//! the circularity error used by other testers (RMS of 1 − r).

use crate::geometry::Vec2;
use crate::sample::Sample;
use crate::stats::{median, percentile, round_to};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SECTORS: usize = 32;
/// Only samples this far from the centre count as "at the edge".
pub const EDGE_THRESHOLD: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SectorStatus {
    Ok,
    /// Clearly shorter than the rest: worn spot or obstruction.
    Under,
    /// Clearly longer than the rest: spikes or a loose module.
    Over,
    /// No samples: the rotation skipped this direction.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sector {
    pub index: usize,
    /// Centre angle, degrees counter-clockwise from right.
    pub angle_deg: f32,
    pub max_radius: f32,
    pub samples: usize,
    pub status: SectorStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CircularityAnalysis {
    pub samples: usize,
    pub sectors: Vec<Sector>,
    pub radius_min: f32,
    pub radius_avg: f32,
    pub radius_max: f32,
    /// Smallest over largest sector reach, %. 100 = perfectly round.
    pub roundness_pct: f32,
    /// RMS of (1 − r) over sectors, %. Same idea as hardwaretester.com.
    pub avg_error_pct: f32,
    pub coverage_pct: f32,
    pub failing_sectors: Vec<usize>,
    /// The stick does not reach the edge in some directions.
    pub incomplete_travel: bool,
    pub suggested_outer_deadzone: f32,
    pub suggested_scale: f32,
}

pub fn analyze_rotation(samples: &[Sample], center: Vec2, n: usize) -> Option<CircularityAnalysis> {
    let n = n.clamp(8, 180);
    let width = 360.0 / n as f32;
    let mut max_r = vec![0.0f32; n];
    let mut count = vec![0usize; n];
    let mut used = 0;
    for s in samples.iter().filter(|s| s.is_finite()) {
        let p = s.pos() - center;
        let r = p.length();
        if r < EDGE_THRESHOLD {
            continue;
        }
        let idx = ((p.angle_deg() / width) as usize).min(n - 1);
        max_r[idx] = max_r[idx].max(r);
        count[idx] += 1;
        used += 1;
    }
    if used < n {
        return None;
    }
    let covered: Vec<f32> = (0..n).filter(|&i| count[i] > 0).map(|i| max_r[i]).collect();
    if covered.is_empty() {
        return None;
    }
    let med = median(&covered);
    let mut sectors = Vec::with_capacity(n);
    let mut failing = Vec::new();
    for i in 0..n {
        let status = if count[i] == 0 {
            SectorStatus::Missing
        } else if max_r[i] < med * 0.92 || max_r[i] < 0.8 {
            SectorStatus::Under
        } else if max_r[i] > med * 1.12 || max_r[i] > 1.3 {
            SectorStatus::Over
        } else {
            SectorStatus::Ok
        };
        if status != SectorStatus::Ok {
            failing.push(i);
        }
        sectors.push(Sector {
            index: i,
            angle_deg: (i as f32 + 0.5) * width,
            max_radius: max_r[i],
            samples: count[i],
            status,
        });
    }
    let rmin = covered.iter().copied().fold(f32::MAX, f32::min);
    let rmax = covered.iter().copied().fold(0.0, f32::max);
    let ravg = covered.iter().sum::<f32>() / covered.len() as f32;
    let err =
        (covered.iter().map(|r| (1.0 - r).powi(2)).sum::<f32>() / covered.len() as f32).sqrt();
    // Robust low reach: ignore a single dented sector when sizing the rescale.
    let p10 = percentile(&covered, 0.1);
    let incomplete = ravg < 0.95 || p10 < 0.93;
    let outer = if incomplete {
        round_to((1.0 - p10).clamp(0.0, 0.3), 0.005)
    } else {
        0.0
    };
    Some(CircularityAnalysis {
        samples: used,
        sectors,
        radius_min: rmin,
        radius_avg: ravg,
        radius_max: rmax,
        roundness_pct: if rmax > 0.0 { rmin / rmax * 100.0 } else { 0.0 },
        avg_error_pct: err * 100.0,
        coverage_pct: covered.len() as f32 / n as f32 * 100.0,
        failing_sectors: failing,
        incomplete_travel: incomplete,
        suggested_outer_deadzone: outer,
        suggested_scale: if p10 > 0.0 { 1.0 / p10 } else { 1.0 },
    })
}
