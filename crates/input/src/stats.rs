//! Polling-rate and jitter measurement from report timestamps.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Gaps longer than this are idle time (stick released, device asleep), not
/// polling intervals, and are left out of the statistics.
pub const IDLE_GAP_US: u64 = 250_000;
const WINDOW: usize = 2048;
/// Statistics cover roughly the last second of reports.
const STATS_SPAN_US: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntervalSource {
    /// Every sample is one hardware report: the interval is the real polling period.
    Report,
    /// Samples come from OS events that only fire on change: move the stick
    /// continuously to measure.
    Change,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollStats {
    pub hz: f32,
    pub avg_ms: f32,
    pub min_ms: f32,
    pub max_ms: f32,
    pub jitter_ms: f32,
    pub p99_ms: f32,
    pub samples: u32,
    pub report_count: u64,
    pub source: IntervalSource,
    /// Intervals (µs) recorded since the last frame sent to the UI.
    pub recent_intervals_us: Vec<u32>,
}

impl Default for PollStats {
    fn default() -> Self {
        Self {
            hz: 0.0,
            avg_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            jitter_ms: 0.0,
            p99_ms: 0.0,
            samples: 0,
            report_count: 0,
            source: IntervalSource::Change,
            recent_intervals_us: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PollMeter {
    last_ts: Option<u64>,
    intervals: VecDeque<u32>,
    pending: Vec<u32>,
    report_count: u64,
    source: IntervalSource,
}

impl Default for PollMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl PollMeter {
    pub fn new() -> Self {
        Self {
            last_ts: None,
            intervals: VecDeque::with_capacity(WINDOW),
            pending: Vec::new(),
            report_count: 0,
            source: IntervalSource::Change,
        }
    }

    pub fn record(&mut self, timestamp_us: u64, report_based: bool) {
        self.report_count += 1;
        self.source = if report_based {
            IntervalSource::Report
        } else {
            IntervalSource::Change
        };
        if let Some(prev) = self.last_ts {
            let dt = timestamp_us.saturating_sub(prev);
            if dt > 0 && dt <= IDLE_GAP_US {
                let dt = dt.min(u32::MAX as u64) as u32;
                if self.intervals.len() == WINDOW {
                    self.intervals.pop_front();
                }
                self.intervals.push_back(dt);
                if self.pending.len() < 4096 {
                    self.pending.push(dt);
                }
            }
        }
        self.last_ts = Some(timestamp_us);
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Computes statistics over the most recent ~1 s of intervals and drains
    /// the per-frame interval list.
    pub fn take_stats(&mut self) -> PollStats {
        let mut stats = self.peek_stats();
        stats.recent_intervals_us = std::mem::take(&mut self.pending);
        stats
    }

    pub fn peek_stats(&self) -> PollStats {
        let mut recent: Vec<u32> = Vec::new();
        let mut span = 0u64;
        for &dt in self.intervals.iter().rev() {
            if span >= STATS_SPAN_US {
                break;
            }
            span += dt as u64;
            recent.push(dt);
        }
        compute_stats(&recent, self.report_count, self.source)
    }
}

pub fn compute_stats(intervals_us: &[u32], report_count: u64, source: IntervalSource) -> PollStats {
    if intervals_us.is_empty() {
        return PollStats {
            report_count,
            source,
            ..PollStats::default()
        };
    }
    let n = intervals_us.len() as f64;
    let sum: f64 = intervals_us.iter().map(|&v| v as f64).sum();
    let mean = sum / n;
    let var = intervals_us
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    let mut sorted = intervals_us.to_vec();
    sorted.sort_unstable();
    let p99_idx = ((sorted.len() as f64 * 0.99).ceil() as usize).clamp(1, sorted.len()) - 1;
    PollStats {
        hz: (1_000_000.0 / mean) as f32,
        avg_ms: (mean / 1000.0) as f32,
        min_ms: sorted[0] as f32 / 1000.0,
        max_ms: *sorted.last().unwrap() as f32 / 1000.0,
        jitter_ms: (var.sqrt() / 1000.0) as f32,
        p99_ms: sorted[p99_idx] as f32 / 1000.0,
        samples: intervals_us.len() as u32,
        report_count,
        source,
        recent_intervals_us: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_1000hz() {
        let mut m = PollMeter::new();
        for i in 0..2000u64 {
            m.record(i * 1000, true);
        }
        let s = m.take_stats();
        assert!((s.hz - 1000.0).abs() < 0.5, "{}", s.hz);
        assert!(s.jitter_ms < 1e-6);
        assert_eq!(s.min_ms, 1.0);
        assert_eq!(s.max_ms, 1.0);
        assert_eq!(s.source, IntervalSource::Report);
        assert_eq!(s.recent_intervals_us.len(), 1999);
        assert!(m.take_stats().recent_intervals_us.is_empty());
    }

    #[test]
    fn jitter_and_extremes() {
        let mut m = PollMeter::new();
        let mut t = 0u64;
        for i in 0..400 {
            t += if i % 2 == 0 { 3000 } else { 5000 };
            m.record(t, true);
        }
        let s = m.peek_stats();
        assert!((s.avg_ms - 4.0).abs() < 0.01);
        assert!((s.jitter_ms - 1.0).abs() < 0.01);
        assert!((s.hz - 250.0).abs() < 1.0);
        assert_eq!(s.min_ms, 3.0);
        assert_eq!(s.max_ms, 5.0);
    }

    #[test]
    fn idle_gaps_are_ignored() {
        let mut m = PollMeter::new();
        m.record(0, false);
        m.record(8_000, false);
        m.record(2_000_000, false);
        m.record(2_008_000, false);
        let s = m.peek_stats();
        assert_eq!(s.samples, 2);
        assert!((s.avg_ms - 8.0).abs() < 1e-3);
        assert_eq!(s.source, IntervalSource::Change);
    }

    #[test]
    fn empty_stats_are_zero() {
        let s = compute_stats(&[], 0, IntervalSource::Report);
        assert_eq!(s.hz, 0.0);
        assert_eq!(s.samples, 0);
    }
}
