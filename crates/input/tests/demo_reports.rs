//! Runs the demo controller through every calibration step and checks the
//! analyses find its deliberate faults. With GAMEPADLAB_WRITE_GOLDEN=1 it
//! also writes the reports the browser mock transport serves.

use gamepadlab_calibration::*;
use gamepadlab_input::{DemoBackend, DemoScenario};

const STEP_US: u64 = 4_000;

fn capture(
    demo: &mut DemoBackend,
    scenario: DemoScenario,
    secs: f32,
    pick: impl Fn(&gamepadlab_input::DeviceState) -> (f32, f32),
) -> Vec<Sample> {
    demo.set_scenario(scenario, 0);
    let n = (secs * 1e6 / STEP_US as f32) as u64;
    (0..n)
        .map(|i| {
            let t = 1_000_000 + i * STEP_US; // skip t=0 so spikes are periodic
            let st = demo.state_at(t);
            let (x, y) = pick(&st);
            Sample::new(i * STEP_US, x, y)
        })
        .collect()
}

fn stick_report(left: bool) -> (StickReport, StickReport) {
    let mut demo = DemoBackend::new();
    let pick = move |s: &gamepadlab_input::DeviceState| {
        let v = if left { s.left_stick } else { s.right_stick };
        (v.x, v.y)
    };
    let rest = capture(&mut demo, DemoScenario::Idle, 2.0, pick);
    let rot = capture(
        &mut demo,
        if left {
            DemoScenario::RotateLeft
        } else {
            DemoScenario::RotateRight
        },
        16.0,
        pick,
    );
    let dirs = capture(
        &mut demo,
        if left {
            DemoScenario::DirectionsLeft
        } else {
            DemoScenario::DirectionsRight
        },
        7.6,
        pick,
    );
    let side = if left {
        StickSide::Left
    } else {
        StickSide::Right
    };
    let before = build_report(side, &rest, &rot, &dirs, None);
    let after = build_report(side, &rest, &rot, &dirs, Some(&before.suggested));
    (before, after)
}

#[test]
fn demo_left_stick_shows_its_faults() {
    let (before, after) = stick_report(true);
    let r = before.rest.as_ref().unwrap();
    assert!(r.offset_pct > 3.0, "drift {}", r.offset_pct);
    let c = before.circularity.as_ref().unwrap();
    assert!(
        c.sectors
            .iter()
            .any(|s| s.status == SectorStatus::Under && (180.0..225.0).contains(&s.angle_deg)),
        "weak sector near 200°"
    );
    assert!(before.spikes.as_ref().unwrap().spike_count > 0);
    assert!(after.rest.as_ref().unwrap().offset_pct < 1.0);
    assert!(after.score.score >= before.score.score);
}

#[test]
fn demo_right_stick_is_healthy() {
    let (before, _) = stick_report(false);
    assert!(before.score.score >= 80.0, "{:?}", before.score);
    assert_eq!(
        before
            .directions
            .as_ref()
            .unwrap()
            .directions
            .iter()
            .filter(|d| d.ok)
            .count(),
        8
    );
}

#[test]
fn demo_triggers() {
    let mut demo = DemoBackend::new();
    let l = |s: &gamepadlab_input::DeviceState| (s.left_trigger, 0.0);
    let rest = capture(&mut demo, DemoScenario::TriggerRest, 1.5, l);
    let full = capture(&mut demo, DemoScenario::TriggerFull, 2.0, l);
    let a = analyze_trigger(&rest, &full).unwrap();
    assert!(!a.full_travel, "the demo left trigger stops at ~96 %");
    assert!(a.suggested_deadzone > 0.02);
    if std::env::var("GAMEPADLAB_WRITE_GOLDEN").is_ok() {
        let r = |s: &gamepadlab_input::DeviceState| (s.right_trigger, 0.0);
        let rest_r = capture(&mut demo, DemoScenario::TriggerRest, 1.5, r);
        let full_r = capture(&mut demo, DemoScenario::TriggerFull, 2.0, r);
        let b = analyze_trigger(&rest_r, &full_r).unwrap();
        write(
            "triggers.json",
            &serde_json::json!({ "left": a, "right": b }),
        );
    }
}

fn write(name: &str, value: &serde_json::Value) {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/ui/src/api/mock/fixtures"
    );
    std::fs::write(
        format!("{dir}/{name}"),
        serde_json::to_string(value).unwrap() + "\n",
    )
    .unwrap();
}

#[test]
fn write_mock_fixtures() {
    if std::env::var("GAMEPADLAB_WRITE_GOLDEN").is_err() {
        return;
    }
    for left in [true, false] {
        let (before, after) = stick_report(left);
        let side = if left { "left" } else { "right" };
        write(
            &format!("report-{side}.json"),
            &serde_json::json!({ "before": before, "after": after }),
        );
    }
}

/// Records a full button-health session (tap round + hold) for one demo
/// button, sampling every report like the input service does.
fn health_session(demo: &mut DemoBackend, button: usize) -> button_health::HealthSession {
    use button_health::{ButtonEvent, ButtonPlan, HealthSession, Precision};
    let mut prev = [false; 19];
    let mut events = Vec::new();
    let mut offset = 0;
    let steps = [
        (
            DemoScenario::HealthTaps {
                button,
                presses: 20,
            },
            6.0,
        ),
        (DemoScenario::HealthHold(button), 3.0),
    ];
    for (scenario, secs) in steps {
        demo.set_scenario(scenario, 0);
        let n = (secs * 1e6 / STEP_US as f32) as u64;
        for i in 0..n {
            let st = demo.state_at(i * STEP_US);
            for (b, s) in st.buttons.iter().enumerate().take(prev.len()) {
                if s.pressed != prev[b] {
                    prev[b] = s.pressed;
                    events.push(ButtonEvent {
                        button: b as u32,
                        pressed: s.pressed,
                        t_us: offset + i * STEP_US,
                    });
                }
            }
        }
        offset += n * STEP_US;
    }
    HealthSession {
        precision: Precision::Report {
            interval_us: STEP_US as u32,
        },
        duration_us: offset,
        plans: vec![ButtonPlan {
            button: button as u32,
            requested: 20,
            reported_self_releases: 0,
        }],
        events,
    }
}

#[test]
fn demo_button_health_flags_only_the_failing_r1() {
    use button_health::{analyze, Advice, Grade, ReasonCode};
    let mut demo = DemoBackend::new();
    let mut buttons = Vec::new();
    for button in 0..19 {
        let report = analyze(&health_session(&mut demo, button));
        let b = report.buttons[0].clone();
        if button == gamepadlab_input::demo::HEALTH_FAULTY.index() {
            assert_eq!(b.grade, Grade::Bad, "R1 should fail: {b:#?}");
            let codes: Vec<ReasonCode> = b.reasons.iter().map(|r| r.code).collect();
            for code in [
                ReasonCode::MissedPresses,
                ReasonCode::Chatter,
                ReasonCode::ReleaseBounce,
                ReasonCode::SelfRelease,
            ] {
                assert!(codes.contains(&code), "R1 missing {code:?}: {codes:?}");
            }
            assert_eq!(b.metrics.missed, 3);
            assert_eq!(b.advice.first(), Some(&Advice::ReplaceSwitch));
        } else {
            assert_eq!(b.grade, Grade::Good, "button {button}: {b:#?}");
            assert_eq!(b.metrics.taps, 20, "button {button}");
        }
        buttons.push(b);
    }
    if std::env::var("GAMEPADLAB_WRITE_GOLDEN").is_ok() {
        write(
            "button-health.json",
            &serde_json::json!({
                "precision": { "kind": "report", "intervalUs": STEP_US },
                "durationMs": 9000.0,
                "buttons": buttons,
            }),
        );
    }
}
