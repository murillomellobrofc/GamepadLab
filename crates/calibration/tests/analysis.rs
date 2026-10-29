//! Synthetic-data tests for every analysis: perfect circle, flattened
//! circle, known drift, noise, spikes, gate shapes, edge values and the
//! before/after comparison.

use gamepadlab_calibration::*;
use std::f32::consts::TAU;

/// Deterministic pseudo-random noise (no external crates).
struct Noise(u64);
impl Noise {
    fn gauss(&mut self) -> f32 {
        let mut s = 0.0;
        for _ in 0..6 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            s += (self.0 >> 40) as f32 / (1u64 << 24) as f32;
        }
        s - 3.0
    }
}

fn rotation(n: usize, center: Vec2, reach: impl Fn(f32) -> f32) -> Vec<Sample> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU * 2.0;
            let p = center + Vec2::from_polar(reach(a), a);
            Sample::new(i as u64 * 4000, p.x, p.y)
        })
        .collect()
}

fn resting(n: usize, center: Vec2, sigma: f32, drift_per_s: Vec2, seed: u64) -> Vec<Sample> {
    let mut noise = Noise(seed);
    (0..n)
        .map(|i| {
            let t = i as f32 * 0.004;
            let p = center
                + drift_per_s.scale(t)
                + Vec2::new(noise.gauss() * sigma, noise.gauss() * sigma);
            Sample::new(i as u64 * 4000, p.x, p.y)
        })
        .collect()
}

fn eight_directions(center: Vec2, reach: impl Fn(f32) -> f32, offset_deg: f32) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut t = 0;
    for k in 0..8 {
        let a = (k as f32 * 45.0 + offset_deg).to_radians();
        for _ in 0..60 {
            let p = center + Vec2::from_polar(reach(a), a);
            out.push(Sample::new(t, p.x, p.y));
            t += 4000;
        }
        for _ in 0..10 {
            out.push(Sample::new(t, center.x, center.y));
            t += 4000;
        }
    }
    out
}

// ------------------------------------------------------------- circularity

#[test]
fn perfect_circle() {
    let c = analyze_rotation(&rotation(2000, Vec2::ZERO, |_| 1.0), Vec2::ZERO, 32).unwrap();
    assert!((c.roundness_pct - 100.0).abs() < 0.5, "{}", c.roundness_pct);
    assert!(c.avg_error_pct < 0.5);
    assert_eq!(c.coverage_pct, 100.0);
    assert!(c.failing_sectors.is_empty());
    assert!(!c.incomplete_travel);
    assert_eq!(c.suggested_outer_deadzone, 0.0);
    assert!((c.radius_avg - 1.0).abs() < 1e-3);
}

#[test]
fn flattened_circle_is_detected() {
    // Ellipse: full reach on X, 80 % on Y.
    let samples: Vec<Sample> = (0..2000)
        .map(|i| {
            let a = i as f32 / 2000.0 * TAU * 2.0;
            Sample::new(i as u64 * 4000, a.cos(), 0.8 * a.sin())
        })
        .collect();
    let c = analyze_rotation(&samples, Vec2::ZERO, 32).unwrap();
    assert!((c.radius_min - 0.8).abs() < 0.01, "{}", c.radius_min);
    assert!((c.roundness_pct - 80.0).abs() < 1.0);
    assert!(c.incomplete_travel);
    assert!(!c.failing_sectors.is_empty());
    // Failing sectors sit around 90° and 270°.
    for &i in &c.failing_sectors {
        let a = c.sectors[i].angle_deg;
        let near_vertical = (a - 90.0).abs() < 50.0 || (a - 270.0).abs() < 50.0;
        assert!(near_vertical, "sector at {a}° should not fail");
    }
    let d = analyze_directions(
        &eight_directions(
            Vec2::ZERO,
            |a| {
                let p = Vec2::new(a.cos(), 0.8 * a.sin());
                p.length()
            },
            0.0,
        ),
        Vec2::ZERO,
    )
    .unwrap();
    assert!((d.range_pos.y - 0.8).abs() < 0.01);
    assert!((d.range_pos.x - 1.0).abs() < 0.01);
}

#[test]
fn dented_sector_is_flagged_under() {
    let samples = rotation(3000, Vec2::ZERO, |a| {
        let deg = a.to_degrees().rem_euclid(360.0);
        if (190.0..215.0).contains(&deg) {
            0.82
        } else {
            1.0
        }
    });
    let c = analyze_rotation(&samples, Vec2::ZERO, 32).unwrap();
    let under: Vec<f32> = c
        .sectors
        .iter()
        .filter(|s| s.status == SectorStatus::Under)
        .map(|s| s.angle_deg)
        .collect();
    assert!(!under.is_empty());
    assert!(
        under.iter().all(|a| (185.0..220.0).contains(a)),
        "{under:?}"
    );
}

#[test]
fn partial_rotation_reports_missing_sectors() {
    let half: Vec<Sample> = rotation(2000, Vec2::ZERO, |_| 1.0)
        .into_iter()
        .filter(|s| s.y >= 0.0)
        .collect();
    let c = analyze_rotation(&half, Vec2::ZERO, 32).unwrap();
    assert!((c.coverage_pct - 50.0).abs() < 7.0, "{}", c.coverage_pct);
    assert!(c.sectors.iter().any(|s| s.status == SectorStatus::Missing));
}

#[test]
fn square_gate_and_octagon() {
    let square = |a: f32| 1.0 / a.cos().abs().max(a.sin().abs());
    let d = analyze_directions(&eight_directions(Vec2::ZERO, square, 0.0), Vec2::ZERO).unwrap();
    assert_eq!(d.gate_shape, GateShape::Square);
    assert!((d.diagonal_ratio - 2f32.sqrt()).abs() < 0.02);
    let round =
        analyze_directions(&eight_directions(Vec2::ZERO, |_| 1.0, 0.0), Vec2::ZERO).unwrap();
    assert_eq!(round.gate_shape, GateShape::Circle);
    assert!(round.directions.iter().all(|d| d.ok));
}

#[test]
fn aim_error_per_direction() {
    let d = analyze_directions(&eight_directions(Vec2::ZERO, |_| 1.0, 12.0), Vec2::ZERO).unwrap();
    for r in &d.directions {
        let err = r.angle_error_deg.unwrap();
        assert!((err - 12.0).abs() < 0.5, "{} {err}", r.name);
        assert!(!r.ok, "12° off is not ok");
    }
}

// -------------------------------------------------------------------- rest

#[test]
fn known_drift_is_measured() {
    let center = Vec2::new(0.07, -0.02);
    let r = analyze_rest(&resting(500, center, 0.004, Vec2::ZERO, 7)).unwrap();
    assert!((r.mean - center).length() < 0.002, "{:?}", r.mean);
    assert!((r.offset_pct - center.length() * 100.0).abs() < 0.3);
    assert!((r.offset_pct_x - 7.0).abs() < 0.3);
    assert_eq!(r.grade, DriftGrade::Poor);
    assert!(r.suggested_deadzone >= r.max_deviation + 0.015);
    assert!(r.suggested_deadzone < 0.08, "{}", r.suggested_deadzone);
    assert!(r.drift_rate_per_sec < 0.01);
}

#[test]
fn drift_grades() {
    let g = |c: Vec2| {
        analyze_rest(&resting(300, c, 0.002, Vec2::ZERO, 3))
            .unwrap()
            .grade
    };
    assert_eq!(g(Vec2::new(0.005, 0.0)), DriftGrade::Excellent);
    assert_eq!(g(Vec2::new(0.035, 0.0)), DriftGrade::Acceptable);
    assert_eq!(g(Vec2::new(0.08, 0.0)), DriftGrade::Poor);
    assert_eq!(g(Vec2::new(0.15, 0.05)), DriftGrade::Defective);
}

#[test]
fn wandering_centre_is_caught() {
    let r = analyze_rest(&resting(500, Vec2::ZERO, 0.002, Vec2::new(0.03, 0.0), 11)).unwrap();
    assert!(
        (r.drift_rate_per_sec - 0.03).abs() < 0.005,
        "{}",
        r.drift_rate_per_sec
    );
    assert!(matches!(r.grade, DriftGrade::Poor | DriftGrade::Defective));
}

#[test]
fn noise_raises_deadzone_and_lowers_grade() {
    let quiet = analyze_rest(&resting(500, Vec2::ZERO, 0.002, Vec2::ZERO, 5)).unwrap();
    let noisy = analyze_rest(&resting(500, Vec2::ZERO, 0.03, Vec2::ZERO, 5)).unwrap();
    assert!(noisy.std_radial > quiet.std_radial * 5.0);
    assert!(noisy.suggested_deadzone > quiet.suggested_deadzone);
    assert_eq!(quiet.grade, DriftGrade::Excellent);
    assert_eq!(noisy.grade, DriftGrade::Poor);
    let s = detect_spikes(&resting(500, Vec2::ZERO, 0.03, Vec2::ZERO, 5));
    assert!(s.noise_rms > 0.01);
    assert_eq!(s.verdict, NoiseVerdict::Noisy);
}

// ------------------------------------------------------------------ spikes

#[test]
fn spikes_do_not_inflate_rest_deadzone() {
    let mut s = resting(500, Vec2::new(0.03, 0.0), 0.003, Vec2::ZERO, 12);
    s[250].x += 0.25;
    s[251].x += 0.0; // single-sample spike
    let r = analyze_rest(&s).unwrap();
    assert!(r.suggested_deadzone < 0.06, "{}", r.suggested_deadzone);
    assert!(r.max_deviation < 0.03);
}

#[test]
fn spikes_are_counted() {
    let mut s = resting(1000, Vec2::ZERO, 0.002, Vec2::ZERO, 9);
    for &i in &[100usize, 400, 700] {
        s[i].x += 0.4;
        s[i].y -= 0.3;
    }
    let r = detect_spikes(&s);
    assert_eq!(r.spike_count, 3);
    assert_eq!(
        r.events.iter().map(|e| e.index).collect::<Vec<_>>(),
        vec![100, 400, 700]
    );
    assert!(
        r.noise_rms < 0.005,
        "spikes are excluded from noise: {}",
        r.noise_rms
    );
    assert_eq!(r.verdict, NoiseVerdict::Noisy);
}

#[test]
fn many_spikes_mean_faulty_and_jumps_are_separate() {
    let mut s = resting(1000, Vec2::ZERO, 0.001, Vec2::ZERO, 2);
    for i in (50..1000).step_by(100) {
        s[i].x += 0.5;
    }
    assert_eq!(detect_spikes(&s).verdict, NoiseVerdict::Faulty);
    // A permanent step (stick moved and stayed) is a jump, not a spike.
    let mut step = resting(400, Vec2::ZERO, 0.0005, Vec2::ZERO, 4);
    for p in step.iter_mut().skip(200) {
        p.x += 0.5;
    }
    let r = detect_spikes(&step);
    assert_eq!(r.spike_count, 0);
    assert_eq!(r.jump_count, 1);
}

#[test]
fn spike_in_rotation_marks_sector_over() {
    let mut s = rotation(3000, Vec2::ZERO, |_| 0.98);
    s[1500].x *= 1.4;
    s[1500].y *= 1.4;
    let c = analyze_rotation(&s, Vec2::ZERO, 32).unwrap();
    assert!(c.sectors.iter().any(|x| x.status == SectorStatus::Over));
}

// ---------------------------------------------------------------- triggers

#[test]
fn trigger_full_and_incomplete() {
    let rest: Vec<Sample> = (0..100)
        .map(|i| Sample::new(i, 0.018 + (i % 3) as f32 * 0.002, 0.0))
        .collect();
    let full: Vec<Sample> = (0..100).map(|i| Sample::new(i, 0.995, 0.0)).collect();
    let a = analyze_trigger(&rest, &full).unwrap();
    assert!(a.full_travel);
    assert_eq!(a.suggested_max, 1.0);
    assert!(
        (a.suggested_deadzone - 0.04).abs() < 0.006,
        "{}",
        a.suggested_deadzone
    );
    let short: Vec<Sample> = (0..100).map(|i| Sample::new(i, 0.9, 0.0)).collect();
    let b = analyze_trigger(&rest, &short).unwrap();
    assert!(!b.full_travel);
    assert!((b.suggested_max - 0.89).abs() < 0.006);
    let cal = TriggerCalibration {
        rest_value: 0.0,
        full_value: 1.0,
        deadzone: b.suggested_deadzone,
        max_limit: b.suggested_max,
        ..TriggerCalibration::default()
    };
    assert_eq!(
        cal.apply(0.9),
        1.0,
        "suggested max makes the short trigger reach 100 %"
    );
}

// ------------------------------------------------------------------ limits

#[test]
fn edge_cases_never_panic() {
    assert!(analyze_rest(&[]).is_none());
    assert!(analyze_rotation(&[], Vec2::ZERO, 32).is_none());
    assert!(analyze_directions(&[], Vec2::ZERO).is_none());
    assert!(analyze_trigger(&[], &[]).is_none());
    let nan = vec![Sample::new(0, f32::NAN, 1.0); 100];
    assert!(analyze_rest(&nan).is_none());
    assert!(analyze_rotation(&nan, Vec2::ZERO, 32).is_none());
    assert_eq!(detect_spikes(&nan).spike_count, 0);
    // Values past ±1 (some drivers overshoot) still analyse.
    let big = rotation(2000, Vec2::ZERO, |_| 1.4);
    let c = analyze_rotation(&big, Vec2::ZERO, 32).unwrap();
    assert!(c.radius_max > 1.3);
    // Only the centre: nothing at the edge.
    let centre = resting(200, Vec2::ZERO, 0.001, Vec2::ZERO, 1);
    assert!(analyze_rotation(&centre, Vec2::ZERO, 32).is_none());
    // Sector count is clamped to a sane range.
    assert_eq!(
        analyze_rotation(&rotation(2000, Vec2::ZERO, |_| 1.0), Vec2::ZERO, 2)
            .unwrap()
            .sectors
            .len(),
        8
    );
    // Everything at exactly ±1 on the axes.
    for v in [
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
    ] {
        let o = StickCalibration::default().apply(v);
        assert!((o.length() - 1.0).abs() < 1e-4);
    }
}

// ------------------------------------------------------- report and after

#[test]
fn full_report_suggests_a_calibration_that_fixes_it() {
    let center = Vec2::new(0.06, -0.03);
    let rest = resting(500, center, 0.004, Vec2::ZERO, 21);
    let reach = |a: f32| Vec2::new(0.9 * a.cos(), 0.85 * a.sin()).length();
    let rot = rotation(3000, center, reach);
    let dirs = eight_directions(center, reach, 0.0);
    let before = build_report(StickSide::Left, &rest, &rot, &dirs, None);
    assert!(!before.calibrated);
    assert!(before.score.issues.contains(&"drift".to_string()));
    assert!(before
        .score
        .issues
        .contains(&"incompleteTravel".to_string()));
    let s = &before.suggested;
    assert!((s.center - center).length() < 0.003);
    assert!((s.range_pos.x - 0.9).abs() < 0.01 && (s.range_pos.y - 0.85).abs() < 0.01);

    let after = build_report(StickSide::Left, &rest, &rot, &dirs, Some(s));
    assert!(after.calibrated);
    let r = after.rest.as_ref().unwrap();
    assert!(r.offset_pct < 0.5, "centre fixed: {}", r.offset_pct);
    let c = after.circularity.as_ref().unwrap();
    assert!(c.radius_min > 0.97, "travel restored: {}", c.radius_min);
    assert!(after.score.score > before.score.score);
    assert!(!after.score.issues.contains(&"drift".to_string()));
}

#[test]
fn healthy_stick_scores_a() {
    let rest = resting(500, Vec2::new(0.004, 0.0), 0.002, Vec2::ZERO, 33);
    let rot = rotation(3000, Vec2::ZERO, |a| {
        1.0 + 0.04 * (1.0 - (4.0 * a).cos()) * 0.5
    });
    let dirs = eight_directions(Vec2::ZERO, |_| 1.0, 0.0);
    let rep = build_report(StickSide::Right, &rest, &rot, &dirs, None);
    assert_eq!(rep.score.grade, "A", "{:?}", rep.score);
    assert!(rep.score.issues.is_empty(), "{:?}", rep.score.issues);
    assert_eq!(rep.suggested.range_pos, Vec2::ONE, "no needless rescale");
}

#[test]
fn quality_grades_cover_the_range() {
    let bad_rest = analyze_rest(&resting(
        300,
        Vec2::new(0.2, 0.0),
        0.05,
        Vec2::new(0.05, 0.0),
        8,
    ))
    .unwrap();
    let s = score_stick(Some(&bad_rest), None, None, None);
    assert!(s.score < 65.0, "{s:?}");
    assert!(matches!(s.grade.as_str(), "D" | "F"));
}
