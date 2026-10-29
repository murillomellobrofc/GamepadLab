//! Golden values shared with the UI's TypeScript preview
//! (apps/ui/src/lib/calibration.test.ts). Regenerate with
//! `GAMEPADLAB_WRITE_GOLDEN=1 cargo test -p gamepadlab-calibration --test golden`.

use gamepadlab_calibration::*;
use serde_json::{json, Value};

fn cases() -> Vec<(StickCalibration, &'static str)> {
    let d = StickCalibration::default;
    vec![
        (d(), "default"),
        (
            StickCalibration {
                inner_deadzone: 0.15,
                outer_deadzone: 0.1,
                curve: ResponseCurve::preset(CurveKind::Smooth),
                ..d()
            },
            "radial-smooth",
        ),
        (
            StickCalibration {
                inner_deadzone: 0.2,
                deadzone_mode: DeadzoneMode::Axial,
                curve: ResponseCurve::preset(CurveKind::Aggressive),
                anti_deadzone: 0.2,
                ..d()
            },
            "axial-aggressive-antidz",
        ),
        (
            StickCalibration {
                center: Vec2::new(0.05, -0.03),
                range_pos: Vec2::new(0.9, 0.85),
                range_neg: Vec2::new(1.0, 0.95),
                curve: ResponseCurve::preset(CurveKind::SCurve),
                gate: GateShape::Octagon,
                ..d()
            },
            "centered-ranges-scurve-octagon",
        ),
        (
            StickCalibration {
                swap_axes: true,
                invert_x: true,
                sensitivity: Vec2::new(1.2, 0.8),
                gate: GateShape::Circle,
                inner_deadzone: 0.08,
                ..d()
            },
            "swap-invert-sens-circle",
        ),
        (
            StickCalibration {
                curve: ResponseCurve::custom(vec![Vec2::new(0.25, 0.1), Vec2::new(0.6, 0.7)]),
                gate: GateShape::Square,
                inner_deadzone: 0.0,
                ..d()
            },
            "custom-square",
        ),
    ]
}

const INPUTS: [(f32, f32); 12] = [
    (0.0, 0.0),
    (0.05, 0.0),
    (0.1, 0.05),
    (0.3, -0.2),
    (-0.5, 0.5),
    (0.7, 0.7),
    (1.0, 0.0),
    (0.0, -1.0),
    (1.0, 1.0),
    (-0.9, -0.3),
    (0.2, 0.95),
    (1.2, -1.2),
];

fn build() -> Value {
    let sticks: Vec<Value> = cases()
        .into_iter()
        .map(|(cal, name)| {
            let outputs: Vec<Value> = INPUTS
                .iter()
                .map(|&(x, y)| {
                    let o = cal.apply(Vec2::new(x, y));
                    json!([[x, y], [o.x, o.y]])
                })
                .collect();
            json!({ "name": name, "calibration": cal, "samples": outputs })
        })
        .collect();
    let trig = TriggerCalibration {
        rest_value: 0.02,
        full_value: 0.96,
        deadzone: 0.05,
        max_limit: 0.9,
        ..TriggerCalibration::default()
    };
    let digital = TriggerCalibration {
        digital: true,
        activation_point: 0.4,
        ..TriggerCalibration::default()
    };
    let tin = [0.0f32, 0.02, 0.1, 0.5, 0.9, 0.96, 1.0];
    let triggers: Vec<Value> = [(trig, "analog"), (digital, "digital")]
        .into_iter()
        .map(|(c, name)| {
            let s: Vec<Value> = tin.iter().map(|&v| json!([v, c.apply(v)])).collect();
            json!({ "name": name, "calibration": c, "samples": s })
        })
        .collect();
    let curves: Vec<Value> = [
        ResponseCurve::preset(CurveKind::Linear),
        ResponseCurve::preset(CurveKind::Smooth),
        ResponseCurve::preset(CurveKind::Aggressive),
        ResponseCurve::preset(CurveKind::SCurve),
        ResponseCurve::custom(vec![Vec2::new(0.25, 0.1), Vec2::new(0.6, 0.7)]),
        ResponseCurve::custom(vec![
            Vec2::new(0.5, 0.05),
            Vec2::new(0.8, 0.3),
            Vec2::new(0.9, 0.95),
        ]),
    ]
    .into_iter()
    .map(|c| {
        let s: Vec<Value> = (0..=20)
            .map(|i| i as f32 / 20.0)
            .map(|x| json!([x, c.eval(x)]))
            .collect();
        json!({ "curve": c, "samples": s })
    })
    .collect();
    json!({
        "description": "Calibration pipeline golden values shared by crates/calibration and apps/ui",
        "sticks": sticks,
        "triggers": triggers,
        "curves": curves,
    })
}

fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-5
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn golden_fixture_is_current() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/calibration_golden.json"
    );
    let fresh = build();
    if std::env::var("GAMEPADLAB_WRITE_GOLDEN").is_ok() {
        std::fs::write(path, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
        return;
    }
    let stored: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    assert!(
        close(&stored, &fresh),
        "fixture out of date: regenerate it (see file header)"
    );
}
