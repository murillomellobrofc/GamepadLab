//! Button health: chatter, missed presses, stuck buttons and release bounce,
//! worked out from a timestamped list of press/release edges.
//!
//! Pure like the rest of the crate: the input service records the edges, the
//! UI decides how many presses to ask for, and this module only reads numbers.
//!
//! Edges closer than [`MERGE_GAP_US`] belong to one physical press (nobody lifts
//! a finger and presses again that fast). Inside a physical press, each extra
//! release is a glitch, classified by where it happens:
//!
//! * within [`PRESS_SETTLE_US`] of the first contact → press chatter;
//! * within [`RELEASE_SETTLE_US`] of the final release → release bounce;
//! * anywhere else → the button let go on its own while held.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Release→press gaps shorter than this are contact bounce inside one press.
/// Fast human tapping still leaves 40 ms or more between presses.
pub const MERGE_GAP_US: u64 = 25_000;
/// Glitches this close to the first contact count as press chatter.
pub const PRESS_SETTLE_US: u64 = 30_000;
/// Glitches this close to the final release count as release bounce.
pub const RELEASE_SETTLE_US: u64 = 30_000;
/// Presses at least this long are deliberate holds (the "hold" step), not taps.
/// They do not count towards the requested taps.
pub const HOLD_MIN_US: u64 = 1_500_000;
/// A tap that takes this long to come back up, and [`SLOW_RELEASE_FACTOR`]
/// times longer than the typical tap, points at a cap or dome that sticks.
pub const SLOW_RELEASE_US: u64 = 600_000;
/// See [`SLOW_RELEASE_US`].
pub const SLOW_RELEASE_FACTOR: f64 = 4.0;
/// A single press this long, or one never released by the end, is stuck.
pub const STUCK_PRESS_US: u64 = 8_000_000;
/// Share of presses with chatter (or release bounce) that makes the grade "bad".
/// A single occurrence already gives "attention".
pub const GLITCH_BAD_RATIO: f64 = 0.10;
/// Chatter or bounce counts that are "bad" whatever the ratio.
pub const GLITCH_BAD_COUNT: u32 = 3;
/// Share of requested taps that never arrived that makes the grade "bad".
/// A single missed tap already gives "attention".
pub const MISSED_BAD_RATIO: f64 = 0.10;
/// Slow releases that make the grade "bad" (one gives "attention").
pub const SLOW_RELEASE_BAD_COUNT: u32 = 3;

/// One edge of one button. `t_us` counts from the start of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonEvent {
    pub button: u32,
    pub pressed: bool,
    pub t_us: u64,
}

/// What the user was asked to do with one button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonPlan {
    pub button: u32,
    /// Taps requested in the tap round (0 = no registration check).
    pub requested: u32,
    /// Times the user said "I did not let go" when a hold ended early.
    #[serde(default)]
    pub reported_self_releases: u32,
}

/// How finely the source sees button changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Precision {
    /// Every input report is seen (HID): glitches shorter than one report
    /// interval are still invisible.
    Report { interval_us: u32 },
    /// Only state changes reported by the system API, at its polling rate:
    /// fast chatter can be missed entirely.
    Event,
}

/// A recorded session, as handed over by the input service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSession {
    pub precision: Precision,
    /// Length of the recording; presses still held at the end last until here.
    pub duration_us: u64,
    pub plans: Vec<ButtonPlan>,
    pub events: Vec<ButtonEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Grade {
    Good,
    Warning,
    Bad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReasonCode {
    /// More than one click at the start of a single press.
    Chatter,
    /// More than one click when letting go.
    ReleaseBounce,
    /// Fewer taps arrived than were requested.
    MissedPresses,
    /// Taps were requested and none arrived.
    NoPresses,
    /// Taps that take much longer than the others to come back up.
    SlowRelease,
    /// A press of [`STUCK_PRESS_US`] or more.
    StuckPress,
    /// Still pressed at the end of the session.
    NeverReleased,
    /// Let go on its own while held (detected or reported by the user).
    SelfRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReason {
    pub code: ReasonCode,
    pub grade: Grade,
    pub count: u32,
}

/// What to do about it, mildest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Advice {
    /// Dirty or oxidised contact: clean it (isopropyl alcohol) and test again.
    Clean,
    /// Cap or dome sticking, loose or not reaching the switch: replace the cap.
    ReplaceCap,
    /// Worn switch: replace it.
    ReplaceSwitch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonMetrics {
    /// Physical presses (taps and holds) after merging bounce.
    pub presses: u32,
    /// Taps counted for the registration check (presses shorter than [`HOLD_MIN_US`]).
    pub taps: u32,
    /// Raw press edges, bounce included.
    pub raw_presses: u32,
    pub requested: u32,
    pub missed: u32,
    /// Taps that arrived out of those requested (0–1); `None` without a request.
    pub registration_rate: Option<f32>,
    /// Presses with chatter at the start.
    pub chatter: u32,
    /// Presses that bounced when let go.
    pub release_bounces: u32,
    /// Times the button let go on its own while held (detected + reported).
    pub self_releases: u32,
    pub slow_releases: u32,
    pub stuck_presses: u32,
    pub never_released: bool,
    pub median_tap_ms: Option<f32>,
    pub longest_press_ms: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonHealth {
    pub button: u32,
    pub grade: Grade,
    pub metrics: ButtonMetrics,
    pub reasons: Vec<HealthReason>,
    pub advice: Vec<Advice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub precision: Precision,
    pub duration_ms: f32,
    pub overall: Grade,
    pub buttons: Vec<ButtonHealth>,
}

/// Live view for the UI while a round runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveButton {
    pub button: u32,
    pub presses: u32,
    pub taps: u32,
    pub held: bool,
    /// How long the current press has lasted (0 when released).
    pub held_for_us: u64,
}

/// One physical press: contiguous pulses with bounce gaps between them.
#[derive(Debug, Clone, PartialEq)]
struct Press {
    start: u64,
    /// Final release; `None` while still held.
    end: Option<u64>,
    /// Release→press gaps inside the press.
    gaps: Vec<(u64, u64)>,
}

impl Press {
    fn duration(&self, now: u64) -> u64 {
        self.end.unwrap_or(now).saturating_sub(self.start)
    }
}

/// Splits one button's edges into physical presses. Repeated edges (press
/// while already pressed) are ignored.
fn presses(edges: &[(u64, bool)]) -> Vec<Press> {
    let mut out: Vec<Press> = Vec::new();
    let mut down = false;
    for &(t, pressed) in edges {
        if pressed == down {
            continue;
        }
        down = pressed;
        if pressed {
            match out.last_mut() {
                Some(p)
                    if p.end
                        .is_some_and(|end| t.saturating_sub(end) < MERGE_GAP_US) =>
                {
                    let end = p.end.take().unwrap_or(t);
                    p.gaps.push((end, t));
                }
                _ => out.push(Press {
                    start: t,
                    end: None,
                    gaps: Vec::new(),
                }),
            }
        } else if let Some(p) = out.last_mut() {
            p.end = Some(t);
        }
    }
    out
}

fn edges_by_button(events: &[ButtonEvent]) -> BTreeMap<u32, Vec<(u64, bool)>> {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| e.t_us);
    let mut map: BTreeMap<u32, Vec<(u64, bool)>> = BTreeMap::new();
    for e in sorted {
        map.entry(e.button).or_default().push((e.t_us, e.pressed));
    }
    map
}

fn median(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2
    })
}

fn ms(us: u64) -> f32 {
    (us as f64 / 1000.0) as f32
}

/// Grade of a chatter-like symptom: one occurrence is "attention", a share of
/// [`GLITCH_BAD_RATIO`] or [`GLITCH_BAD_COUNT`] occurrences is "bad".
fn glitch_grade(count: u32, presses: u32) -> Grade {
    let ratio = count as f64 / presses.max(1) as f64;
    if count >= GLITCH_BAD_COUNT || ratio >= GLITCH_BAD_RATIO {
        Grade::Bad
    } else {
        Grade::Warning
    }
}

fn analyze_button(plan: ButtonPlan, edges: &[(u64, bool)], end_us: u64) -> ButtonHealth {
    let list = presses(edges);
    let raw_presses = list.iter().map(|p| 1 + p.gaps.len() as u32).sum::<u32>();

    let mut chatter = 0;
    let mut release_bounces = 0;
    let mut detected_self_releases = 0;
    for p in &list {
        let (mut at_start, mut at_end) = (false, false);
        for &(gap_start, gap_end) in &p.gaps {
            if gap_start.saturating_sub(p.start) <= PRESS_SETTLE_US {
                at_start = true;
            } else if p
                .end
                .is_some_and(|end| end.saturating_sub(gap_end) <= RELEASE_SETTLE_US)
            {
                at_end = true;
            } else {
                detected_self_releases += 1;
            }
        }
        chatter += at_start as u32;
        release_bounces += at_end as u32;
    }

    let durations: Vec<u64> = list.iter().map(|p| p.duration(end_us)).collect();
    let mut taps: Vec<u64> = list
        .iter()
        .zip(&durations)
        .filter(|(p, &d)| p.end.is_some() && d < HOLD_MIN_US)
        .map(|(_, &d)| d)
        .collect();
    let tap_count = taps.len() as u32;
    let median_tap = median(&mut taps);
    let slow_releases = match median_tap {
        Some(m) => taps
            .iter()
            .filter(|&&d| d >= SLOW_RELEASE_US && d as f64 >= m as f64 * SLOW_RELEASE_FACTOR)
            .count() as u32,
        None => 0,
    };
    let stuck_presses = durations.iter().filter(|&&d| d >= STUCK_PRESS_US).count() as u32;
    let never_released = list
        .last()
        .is_some_and(|p| p.end.is_none() && p.duration(end_us) >= HOLD_MIN_US);
    let self_releases = detected_self_releases + plan.reported_self_releases;

    let requested = plan.requested;
    let missed = requested.saturating_sub(tap_count);
    let registration_rate =
        (requested > 0).then(|| (tap_count.min(requested) as f64 / requested as f64) as f32);

    let presses_n = list.len() as u32;
    let mut reasons = Vec::new();
    let mut push = |code, grade, count| {
        reasons.push(HealthReason { code, grade, count });
    };
    if requested > 0 && presses_n == 0 {
        push(ReasonCode::NoPresses, Grade::Bad, requested);
    } else {
        if missed > 0 {
            let bad = missed as f64 / requested as f64 >= MISSED_BAD_RATIO;
            let grade = if bad { Grade::Bad } else { Grade::Warning };
            push(ReasonCode::MissedPresses, grade, missed);
        }
        if chatter > 0 {
            push(
                ReasonCode::Chatter,
                glitch_grade(chatter, presses_n),
                chatter,
            );
        }
        if release_bounces > 0 {
            let grade = glitch_grade(release_bounces, presses_n);
            push(ReasonCode::ReleaseBounce, grade, release_bounces);
        }
        if self_releases > 0 {
            push(ReasonCode::SelfRelease, Grade::Bad, self_releases);
        }
        if slow_releases > 0 {
            let bad = slow_releases >= SLOW_RELEASE_BAD_COUNT;
            let grade = if bad { Grade::Bad } else { Grade::Warning };
            push(ReasonCode::SlowRelease, grade, slow_releases);
        }
        if never_released {
            push(ReasonCode::NeverReleased, Grade::Bad, 1);
        } else if stuck_presses > 0 {
            push(ReasonCode::StuckPress, Grade::Bad, stuck_presses);
        }
    }

    let grade = reasons.iter().map(|r| r.grade).max().unwrap_or(Grade::Good);
    let mut advice: Vec<Advice> = reasons
        .iter()
        .flat_map(|r| advice_for(r.code, r.grade))
        .collect();
    advice.sort_unstable();
    advice.dedup();
    advice.reverse(); // most decisive first

    ButtonHealth {
        button: plan.button,
        grade,
        metrics: ButtonMetrics {
            presses: presses_n,
            taps: tap_count,
            raw_presses,
            requested,
            missed,
            registration_rate,
            chatter,
            release_bounces,
            self_releases,
            slow_releases,
            stuck_presses,
            never_released,
            median_tap_ms: median_tap.map(ms),
            longest_press_ms: durations.iter().max().map(|&d| ms(d)),
        },
        reasons,
        advice,
    }
}

/// What to suggest for each symptom. Mild contact problems usually go away
/// with cleaning; repeated ones mean a worn switch; sticking or missing taps
/// often come from the cap (loose, bent or not reaching the switch).
fn advice_for(code: ReasonCode, grade: Grade) -> Vec<Advice> {
    use Advice::*;
    match (code, grade) {
        (ReasonCode::Chatter | ReasonCode::ReleaseBounce, Grade::Warning) => vec![Clean],
        (ReasonCode::Chatter | ReasonCode::ReleaseBounce, _) => vec![ReplaceSwitch],
        (ReasonCode::MissedPresses, Grade::Warning) => vec![Clean, ReplaceCap],
        (ReasonCode::MissedPresses | ReasonCode::NoPresses, _) => vec![ReplaceCap, ReplaceSwitch],
        (ReasonCode::SelfRelease, _) => vec![ReplaceSwitch],
        (ReasonCode::SlowRelease | ReasonCode::StuckPress | ReasonCode::NeverReleased, _) => {
            vec![Clean, ReplaceCap]
        }
    }
}

/// Analyses every planned button (or every button seen, when there is no plan).
pub fn analyze(session: &HealthSession) -> HealthReport {
    let edges = edges_by_button(&session.events);
    let last_event = session.events.iter().map(|e| e.t_us).max().unwrap_or(0);
    let end_us = session.duration_us.max(last_event);
    let plans: Vec<ButtonPlan> = if session.plans.is_empty() {
        edges
            .keys()
            .map(|&button| ButtonPlan {
                button,
                requested: 0,
                reported_self_releases: 0,
            })
            .collect()
    } else {
        session.plans.clone()
    };
    let buttons: Vec<ButtonHealth> = plans
        .iter()
        .map(|&plan| {
            let e = edges.get(&plan.button).map(Vec::as_slice).unwrap_or(&[]);
            analyze_button(plan, e, end_us)
        })
        .collect();
    let overall = buttons.iter().map(|b| b.grade).max().unwrap_or(Grade::Good);
    HealthReport {
        precision: session.precision,
        duration_ms: ms(end_us),
        overall,
        buttons,
    }
}

/// Press counts and the current hold, per button, at `now_us`.
pub fn live_status(events: &[ButtonEvent], now_us: u64) -> Vec<LiveButton> {
    edges_by_button(events)
        .into_iter()
        .map(|(button, edges)| {
            let list = presses(&edges);
            let held = edges.last().is_some_and(|e| e.1);
            let current = list.last().filter(|p| p.end.is_none());
            let taps = list
                .iter()
                .filter(|p| p.end.is_some() && p.duration(now_us) < HOLD_MIN_US)
                .count() as u32;
            LiveButton {
                button,
                presses: list.len() as u32,
                taps,
                held,
                held_for_us: current.map_or(0, |p| p.duration(now_us)),
            }
        })
        .collect()
}
