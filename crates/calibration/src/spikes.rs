//! Spikes (one-sample excursions that come right back), jumps (sudden steps)
//! and potentiometer noise.

use crate::sample::Sample;
use crate::stats::median;
use serde::{Deserialize, Serialize};

pub const SPIKE_THRESHOLD: f32 = 0.12;
pub const JUMP_THRESHOLD: f32 = 0.25;
const STILL: f32 = 0.05;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeEvent {
    pub index: usize,
    pub timestamp_us: u64,
    pub magnitude: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoiseVerdict {
    Clean,
    Noisy,
    Faulty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeReport {
    pub spike_count: usize,
    pub jump_count: usize,
    /// RMS distance from a 5-sample moving median (spikes excluded).
    pub noise_rms: f32,
    pub events: Vec<SpikeEvent>,
    pub verdict: NoiseVerdict,
}

pub fn detect_spikes(samples: &[Sample]) -> SpikeReport {
    let s: Vec<Sample> = samples.iter().copied().filter(Sample::is_finite).collect();
    let n = s.len();
    let mut events = Vec::new();
    let mut jumps = 0usize;
    let mut spike_idx = std::collections::HashSet::new();
    let dist = |a: &Sample, b: &Sample| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();

    for i in 2..n.saturating_sub(2) {
        let neighbours = [s[i - 2], s[i - 1], s[i + 1], s[i + 2]];
        let mx = median(&neighbours.map(|p| p.x));
        let my = median(&neighbours.map(|p| p.y));
        let dev = ((s[i].x - mx).powi(2) + (s[i].y - my).powi(2)).sqrt();
        let into = dist(&s[i], &s[i - 1]);
        let out = dist(&s[i + 1], &s[i]);
        if dev > SPIKE_THRESHOLD && into > SPIKE_THRESHOLD * 0.8 && out > SPIKE_THRESHOLD * 0.8 {
            spike_idx.insert(i);
            events.push(SpikeEvent {
                index: i,
                timestamp_us: s[i].t,
                magnitude: dev,
            });
            continue;
        }
        let before = dist(&s[i - 1], &s[i - 2]);
        let step = dist(&s[i], &s[i - 1]);
        let after = dist(&s[i + 1], &s[i]);
        if step > JUMP_THRESHOLD && before < STILL && after < STILL {
            jumps += 1;
        }
    }

    let mut sq = 0.0f64;
    let mut cnt = 0usize;
    for i in 2..n.saturating_sub(2) {
        if (i - 2..=i + 2).any(|j| spike_idx.contains(&j)) {
            continue;
        }
        let win: Vec<Sample> = s[i - 2..=i + 2].to_vec();
        let mx = median(&win.iter().map(|p| p.x).collect::<Vec<_>>());
        let my = median(&win.iter().map(|p| p.y).collect::<Vec<_>>());
        sq += ((s[i].x - mx).powi(2) + (s[i].y - my).powi(2)) as f64;
        cnt += 1;
    }
    let noise_rms = if cnt > 0 {
        (sq / cnt as f64).sqrt() as f32
    } else {
        0.0
    };
    let per_k = if n > 0 {
        events.len() as f32 * 1000.0 / n as f32
    } else {
        0.0
    };
    let verdict = if per_k > 3.0 || jumps > 2 {
        NoiseVerdict::Faulty
    } else if noise_rms > 0.01 || !events.is_empty() {
        NoiseVerdict::Noisy
    } else {
        NoiseVerdict::Clean
    };
    events.truncate(200);
    SpikeReport {
        spike_count: spike_idx.len(),
        jump_count: jumps,
        noise_rms,
        events,
        verdict,
    }
}

/// Combines reports of separate captures (analysing them back to back
/// would turn the gap between captures into a fake jump).
pub fn merge_reports(reports: &[SpikeReport]) -> Option<SpikeReport> {
    let first = reports.first()?;
    let worst = reports
        .iter()
        .map(|r| r.verdict)
        .max_by_key(|v| match v {
            NoiseVerdict::Clean => 0,
            NoiseVerdict::Noisy => 1,
            NoiseVerdict::Faulty => 2,
        })
        .unwrap_or(first.verdict);
    Some(SpikeReport {
        spike_count: reports.iter().map(|r| r.spike_count).sum(),
        jump_count: reports.iter().map(|r| r.jump_count).sum(),
        // The first capture is the resting one: the cleanest noise reading.
        noise_rms: first.noise_rms,
        events: reports
            .iter()
            .flat_map(|r| r.events.iter().cloned())
            .take(200)
            .collect(),
        verdict: worst,
    })
}
