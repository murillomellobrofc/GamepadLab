//! Quality score, automatic suggestion and the full per-stick report
//! (before and after calibration).

use crate::circularity::{analyze_rotation, CircularityAnalysis, SectorStatus, DEFAULT_SECTORS};
use crate::directions::{analyze_directions, DirectionsAnalysis};
use crate::geometry::Vec2;
use crate::profile::StickCalibration;
use crate::rest::{analyze_rest, RestAnalysis};
use crate::sample::Sample;
use crate::spikes::{detect_spikes, merge_reports, SpikeReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StickSide {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityScore {
    pub score: f32,
    pub grade: String,
    /// Machine-readable issue codes, translated by the UI.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickReport {
    pub stick: StickSide,
    pub calibrated: bool,
    pub rest: Option<RestAnalysis>,
    pub circularity: Option<CircularityAnalysis>,
    pub directions: Option<DirectionsAnalysis>,
    pub spikes: Option<SpikeReport>,
    pub score: QualityScore,
    pub suggested: StickCalibration,
}

pub fn grade(score: f32) -> &'static str {
    match score {
        s if s >= 90.0 => "A",
        s if s >= 80.0 => "B",
        s if s >= 65.0 => "C",
        s if s >= 50.0 => "D",
        _ => "F",
    }
}

pub fn score_stick(
    rest: Option<&RestAnalysis>,
    circ: Option<&CircularityAnalysis>,
    dirs: Option<&DirectionsAnalysis>,
    spikes: Option<&SpikeReport>,
) -> QualityScore {
    let mut score = 100.0f32;
    let mut issues = Vec::new();
    let mut penalty = |amount: f32, cap: f32, code: &str| {
        let p = amount.clamp(0.0, cap);
        if p > 0.5 {
            score -= p;
            issues.push(code.to_string());
        }
    };
    if let Some(r) = rest {
        penalty((r.offset_pct - 2.0) * 3.0, 30.0, "drift");
        penalty((r.std_radial - 0.006) * 1000.0, 15.0, "noise");
        penalty(
            if r.drift_rate_per_sec > 0.01 {
                10.0
            } else {
                0.0
            },
            10.0,
            "wandering",
        );
    }
    if let Some(c) = circ {
        penalty((95.0 - c.coverage_pct) * 0.5, 20.0, "lowCoverage");
        penalty((0.95 - c.radius_min) * 150.0, 25.0, "incompleteTravel");
        let under = c
            .sectors
            .iter()
            .filter(|s| s.status == SectorStatus::Under)
            .count();
        penalty(under as f32 * 2.0, 15.0, "weakSectors");
    }
    if let Some(d) = dirs {
        let bad = d
            .directions
            .iter()
            .filter(|r| r.samples > 0 && !r.ok)
            .count();
        penalty(bad as f32 * 2.0, 10.0, "aim");
    }
    if let Some(s) = spikes {
        penalty(s.spike_count as f32 * 5.0, 20.0, "spikes");
        penalty(s.jump_count as f32 * 8.0, 20.0, "jumps");
    }
    let score = score.clamp(0.0, 100.0).round();
    QualityScore {
        score,
        grade: grade(score).into(),
        issues,
    }
}

/// Builds the calibration the wizard proposes from what was measured.
pub fn suggest(
    rest: Option<&RestAnalysis>,
    circ: Option<&CircularityAnalysis>,
    dirs: Option<&DirectionsAnalysis>,
) -> StickCalibration {
    let mut c = StickCalibration::default();
    if let Some(r) = rest {
        c.center = r.mean;
        c.inner_deadzone = r.suggested_deadzone;
    }
    let tidy = |v: f32| {
        let v = v.clamp(0.6, 1.2);
        if (v - 1.0).abs() < 0.02 {
            1.0
        } else {
            (v * 1000.0).round() / 1000.0
        }
    };
    if let Some(d) = dirs {
        c.range_pos = Vec2::new(tidy(d.range_pos.x), tidy(d.range_pos.y));
        c.range_neg = Vec2::new(tidy(d.range_neg.x), tidy(d.range_neg.y));
    } else if let Some(ci) = circ {
        if ci.incomplete_travel {
            c.outer_deadzone = ci.suggested_outer_deadzone;
        }
    }
    c
}

/// Analyses the captured steps for one stick. With `calibration`, every
/// sample goes through it first, giving the "after" picture.
pub fn build_report(
    stick: StickSide,
    rest: &[Sample],
    rotation: &[Sample],
    directions: &[Sample],
    calibration: Option<&StickCalibration>,
) -> StickReport {
    let map = |v: &[Sample]| -> Vec<Sample> {
        match calibration {
            Some(c) => v
                .iter()
                .map(|s| {
                    let p = c.apply(s.pos());
                    Sample::new(s.t, p.x, p.y)
                })
                .collect(),
            None => v.to_vec(),
        }
    };
    let (rest, rotation, directions) = (map(rest), map(rotation), map(directions));
    let r = analyze_rest(&rest);
    let center = match (&r, calibration) {
        (_, Some(_)) => Vec2::ZERO,
        (Some(r), None) => r.mean,
        (None, None) => Vec2::ZERO,
    };
    let circ = analyze_rotation(&rotation, center, DEFAULT_SECTORS);
    let dirs = analyze_directions(&directions, center);
    let parts: Vec<SpikeReport> = [&rest, &rotation]
        .into_iter()
        .filter(|v| v.len() >= 5)
        .map(|v| detect_spikes(v))
        .collect();
    let spikes = merge_reports(&parts);
    let score = score_stick(r.as_ref(), circ.as_ref(), dirs.as_ref(), spikes.as_ref());
    let suggested = match calibration {
        Some(c) => c.clone(),
        None => suggest(r.as_ref(), circ.as_ref(), dirs.as_ref()),
    };
    StickReport {
        stick,
        calibrated: calibration.is_some(),
        rest: r,
        circularity: circ,
        directions: dirs,
        spikes,
        score,
        suggested,
    }
}
