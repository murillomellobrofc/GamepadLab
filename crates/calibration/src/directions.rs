//! Step 3: push and hold in 8 directions. Reach and aim per direction,
//! gate shape and per-axis range used to rescale.

use crate::geometry::{angle_diff_deg, Vec2};
use crate::profile::GateShape;
use crate::sample::Sample;
use crate::stats::percentile;
use serde::{Deserialize, Serialize};

pub const NAMES: [&str; 8] = [
    "right",
    "upRight",
    "up",
    "upLeft",
    "left",
    "downLeft",
    "down",
    "downRight",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionResult {
    pub name: String,
    pub target_deg: f32,
    pub measured_deg: Option<f32>,
    pub angle_error_deg: Option<f32>,
    pub reach: f32,
    pub samples: usize,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionsAnalysis {
    pub directions: Vec<DirectionResult>,
    pub gate_shape: GateShape,
    /// Average diagonal reach over average cardinal reach.
    pub diagonal_ratio: f32,
    pub range_pos: Vec2,
    pub range_neg: Vec2,
}

pub fn analyze_directions(samples: &[Sample], center: Vec2) -> Option<DirectionsAnalysis> {
    let mut buckets: Vec<Vec<Vec2>> = vec![Vec::new(); 8];
    for s in samples.iter().filter(|s| s.is_finite()) {
        let p = s.pos() - center;
        if p.length() < 0.6 {
            continue;
        }
        let idx = (((p.angle_deg() + 22.5) / 45.0) as usize) % 8;
        buckets[idx].push(p);
    }
    if buckets.iter().all(Vec::is_empty) {
        return None;
    }
    let directions: Vec<DirectionResult> = buckets
        .iter()
        .enumerate()
        .map(|(i, pts)| {
            let target = i as f32 * 45.0;
            if pts.is_empty() {
                return DirectionResult {
                    name: NAMES[i].into(),
                    target_deg: target,
                    measured_deg: None,
                    angle_error_deg: None,
                    reach: 0.0,
                    samples: 0,
                    ok: false,
                };
            }
            let radii: Vec<f32> = pts.iter().map(|p| p.length()).collect();
            // 95th percentile: robust to a stray spike.
            let reach = percentile(&radii, 0.95);
            // Circular mean of the held (near-maximum) samples.
            let held: Vec<&Vec2> = pts.iter().filter(|p| p.length() >= reach * 0.85).collect();
            let (sx, sy) = held.iter().fold((0.0f32, 0.0f32), |(a, b), p| {
                (a + p.x / p.length(), b + p.y / p.length())
            });
            let measured = Vec2::new(sx, sy).angle_deg();
            let err = angle_diff_deg(measured, target);
            DirectionResult {
                name: NAMES[i].into(),
                target_deg: target,
                measured_deg: Some(measured),
                angle_error_deg: Some(err),
                reach,
                samples: pts.len(),
                ok: reach > 0.85 && err.abs() < 8.0,
            }
        })
        .collect();
    let reach = |i: usize| directions[i].reach;
    let avg = |idx: &[usize]| {
        let v: Vec<f32> = idx.iter().map(|&i| reach(i)).filter(|r| *r > 0.0).collect();
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f32>() / v.len() as f32
        }
    };
    let cardinal = avg(&[0, 2, 4, 6]);
    let diagonal = avg(&[1, 3, 5, 7]);
    let ratio = if cardinal > 0.0 && diagonal > 0.0 {
        diagonal / cardinal
    } else {
        1.0
    };
    let gate = if ratio > 1.2 {
        GateShape::Square
    } else if ratio > 1.06 {
        GateShape::Octagon
    } else {
        GateShape::Circle
    };
    let or_one = |r: f32| if r > 0.2 { r.min(1.5) } else { 1.0 };
    Some(DirectionsAnalysis {
        gate_shape: gate,
        diagonal_ratio: ratio,
        range_pos: Vec2::new(or_one(reach(0)), or_one(reach(2))),
        range_neg: Vec2::new(or_one(reach(4)), or_one(reach(6))),
        directions,
    })
}
