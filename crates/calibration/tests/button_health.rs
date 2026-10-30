//! Button health against the fixtures in `fixtures/button_health/` (one JSON
//! per symptom, plus a DualSense whose R1 is going bad) and a few edge cases.

use gamepadlab_calibration::button_health::*;
use serde_json::{json, Value};
use std::path::PathBuf;

fn fixtures() -> Vec<(String, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/button_health");
    let mut out: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .expect("fixtures/button_health exists")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).unwrap();
            (name, serde_json::from_str(&text).unwrap())
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn every_fixture_gets_the_expected_grade_reasons_and_advice() {
    let all = fixtures();
    assert!(
        all.len() >= 6,
        "expected the six symptom fixtures, got {}",
        all.len()
    );
    for (name, fx) in all {
        let session: HealthSession = serde_json::from_value(fx["session"].clone()).unwrap();
        let report = analyze(&session);
        let got = serde_json::to_value(&report).unwrap();
        for want in fx["expect"].as_array().unwrap() {
            let button = &want["button"];
            let b = got["buttons"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| &b["button"] == button)
                .unwrap_or_else(|| panic!("{name}: button {button} missing from the report"));
            assert_eq!(
                b["grade"], want["grade"],
                "{name}: grade of button {button}: {b:#}"
            );
            let codes: Vec<&Value> = b["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| &r["code"])
                .collect();
            let want_codes: Vec<&Value> = want["reasons"].as_array().unwrap().iter().collect();
            assert_eq!(codes, want_codes, "{name}: reasons of button {button}");
            assert_eq!(
                b["advice"], want["advice"],
                "{name}: advice of button {button}"
            );
            if let Some(metrics) = want["metrics"].as_object() {
                for (k, v) in metrics {
                    let actual = &b["metrics"][k];
                    match (v.as_f64(), actual.as_f64()) {
                        (Some(w), Some(a)) => assert!(
                            (w - a).abs() < 1e-4,
                            "{name}: button {button} metric {k}: want {w}, got {a}"
                        ),
                        _ => assert_eq!(actual, v, "{name}: button {button} metric {k}"),
                    }
                }
            }
        }
        let worst = fx["expect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| match w["grade"].as_str().unwrap() {
                "bad" => 2,
                "warning" => 1,
                _ => 0,
            })
            .max()
            .unwrap();
        let overall = ["good", "warning", "bad"][worst];
        assert_eq!(got["overall"], overall, "{name}: overall grade");
    }
}

fn tap(button: u32, start_ms: u64, len_ms: u64) -> [ButtonEvent; 2] {
    [
        ButtonEvent {
            button,
            pressed: true,
            t_us: start_ms * 1000,
        },
        ButtonEvent {
            button,
            pressed: false,
            t_us: (start_ms + len_ms) * 1000,
        },
    ]
}

fn session(events: Vec<ButtonEvent>, plans: Vec<ButtonPlan>) -> HealthSession {
    let end = events.iter().map(|e| e.t_us).max().unwrap_or(0);
    HealthSession {
        precision: Precision::Report { interval_us: 4000 },
        duration_us: end,
        plans,
        events,
    }
}

fn plan(button: u32, requested: u32) -> ButtonPlan {
    ButtonPlan {
        button,
        requested,
        reported_self_releases: 0,
    }
}

#[test]
fn gaps_shorter_than_the_merge_window_are_one_press_and_longer_ones_are_two() {
    let just_inside = (MERGE_GAP_US / 1000) - 1;
    let just_outside = MERGE_GAP_US / 1000;
    for (gap_ms, presses) in [(just_inside, 1), (just_outside, 2)] {
        let mut ev = Vec::new();
        ev.extend(tap(0, 0, 10));
        ev.extend(tap(0, 10 + gap_ms, 100));
        let r = analyze(&session(ev, vec![plan(0, 0)]));
        assert_eq!(r.buttons[0].metrics.presses, presses, "gap of {gap_ms} ms");
        assert_eq!(r.buttons[0].metrics.raw_presses, 2);
    }
}

#[test]
fn a_button_that_never_responds_is_bad_and_points_at_the_cap_and_switch() {
    let r = analyze(&session(Vec::new(), vec![plan(5, 20)]));
    let b = &r.buttons[0];
    assert_eq!(b.grade, Grade::Bad);
    assert_eq!(b.reasons[0].code, ReasonCode::NoPresses);
    assert_eq!(b.metrics.registration_rate, Some(0.0));
    assert_eq!(b.advice, vec![Advice::ReplaceSwitch, Advice::ReplaceCap]);
    assert_eq!(r.overall, Grade::Bad);
}

#[test]
fn extra_taps_and_repeated_edges_do_not_count_as_problems() {
    let mut ev = Vec::new();
    for i in 0..22 {
        ev.extend(tap(3, i * 250, 100));
    }
    // A backend that reports "pressed" twice in a row must not invent a press.
    ev.push(ButtonEvent {
        button: 3,
        pressed: true,
        t_us: 21 * 250_000 + 50_000,
    });
    let r = analyze(&session(ev, vec![plan(3, 20)]));
    let b = &r.buttons[0];
    assert_eq!(b.metrics.taps, 22);
    assert_eq!(b.metrics.missed, 0);
    assert_eq!(b.metrics.registration_rate, Some(1.0));
    assert_eq!(b.grade, Grade::Good);
}

#[test]
fn without_a_plan_every_button_seen_is_reported() {
    let mut ev = Vec::new();
    ev.extend(tap(7, 0, 100));
    ev.extend(tap(2, 300, 100));
    let r = analyze(&session(ev, Vec::new()));
    let buttons: Vec<u32> = r.buttons.iter().map(|b| b.button).collect();
    assert_eq!(buttons, vec![2, 7]);
    assert!(r
        .buttons
        .iter()
        .all(|b| b.metrics.registration_rate.is_none()));
}

#[test]
fn live_status_counts_merged_presses_and_the_current_hold() {
    let mut ev = Vec::new();
    ev.extend(tap(1, 0, 100));
    // Chatter: one physical press, two raw clicks.
    ev.push(ButtonEvent {
        button: 1,
        pressed: true,
        t_us: 400_000,
    });
    ev.push(ButtonEvent {
        button: 1,
        pressed: false,
        t_us: 408_000,
    });
    ev.push(ButtonEvent {
        button: 1,
        pressed: true,
        t_us: 416_000,
    });
    let live = live_status(&ev, 1_416_000);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].presses, 2);
    assert_eq!(live[0].taps, 1);
    assert!(live[0].held);
    assert_eq!(live[0].held_for_us, 1_016_000);
}

#[test]
fn json_contract_uses_the_names_the_ui_reads() {
    let mut ev = Vec::new();
    ev.extend(tap(0, 0, 100));
    let r = analyze(&HealthSession {
        precision: Precision::Event,
        duration_us: 500_000,
        plans: vec![plan(0, 1)],
        events: ev,
    });
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["precision"], json!({ "kind": "event" }));
    assert_eq!(v["overall"], "good");
    let m = &v["buttons"][0]["metrics"];
    for key in [
        "presses",
        "taps",
        "rawPresses",
        "requested",
        "missed",
        "registrationRate",
        "chatter",
        "releaseBounces",
        "selfReleases",
        "slowReleases",
        "stuckPresses",
        "neverReleased",
        "medianTapMs",
        "longestPressMs",
    ] {
        assert!(m.get(key).is_some(), "metrics.{key} missing: {m}");
    }
    let report = serde_json::to_value(Precision::Report { interval_us: 4000 }).unwrap();
    assert_eq!(report, json!({ "kind": "report", "intervalUs": 4000 }));
}
