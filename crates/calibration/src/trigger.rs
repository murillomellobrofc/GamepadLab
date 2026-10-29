//! Step 4: trigger released, then fully pressed.

use crate::sample::Sample;
use crate::stats::{percentile, round_to, std_dev};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerAnalysis {
    pub rest_min: f32,
    pub rest_max: f32,
    pub rest_noise: f32,
    pub max_reached: f32,
    pub full_travel: bool,
    pub suggested_deadzone: f32,
    pub suggested_max: f32,
}

pub fn analyze_trigger(rest: &[Sample], full: &[Sample]) -> Option<TriggerAnalysis> {
    let r: Vec<f32> = rest.iter().map(|s| s.x).filter(|v| v.is_finite()).collect();
    let f: Vec<f32> = full.iter().map(|s| s.x).filter(|v| v.is_finite()).collect();
    if r.len() < 3 || f.len() < 3 {
        return None;
    }
    let rest_min = r.iter().copied().fold(f32::MAX, f32::min);
    let rest_max = r.iter().copied().fold(f32::MIN, f32::max);
    // 99th percentile ignores a single glitchy sample at the top.
    let max_reached = percentile(&f, 0.99);
    let full_travel = max_reached >= 0.97;
    Some(TriggerAnalysis {
        rest_min,
        rest_max,
        rest_noise: std_dev(&r),
        max_reached,
        full_travel,
        suggested_deadzone: round_to((rest_max + 0.02).clamp(0.0, 0.3), 0.005),
        suggested_max: if full_travel {
            1.0
        } else {
            round_to((max_reached - 0.01).clamp(0.5, 1.0), 0.005)
        },
    })
}
