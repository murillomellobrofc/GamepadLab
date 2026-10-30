//! Drives the real command handlers through Tauri's IPC layer (mock
//! runtime), with exactly the argument names the UI sends. Uses only the
//! demo backend, so it runs in CI without hardware or a display.

use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindowBuilder;

struct Ipc<W: AsRef<tauri::Webview<tauri::test::MockRuntime>>>(W);

impl<W: AsRef<tauri::Webview<tauri::test::MockRuntime>>> Ipc<W> {
    fn call(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        let url = if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        };
        get_ipc_response(
            &self.0,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: url.parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<Value>().unwrap())
    }

    fn ok(&self, cmd: &str, args: Value) -> Value {
        self.call(cmd, args)
            .unwrap_or_else(|e| panic!("{cmd} failed: {e}"))
    }
}

fn wait_for(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("condition not met in time");
}

fn capture(
    ipc: &Ipc<impl AsRef<tauri::Webview<tauri::test::MockRuntime>>>,
    id: &str,
    kind: &str,
    target: &str,
    scenario: Value,
    ms: u64,
) -> Value {
    ipc.ok("demo_scenario", json!({ "scenario": scenario }));
    std::thread::sleep(Duration::from_millis(30));
    ipc.ok(
        "capture_start",
        json!({ "id": id, "kind": kind, "target": target }),
    );
    std::thread::sleep(Duration::from_millis(ms));
    ipc.ok("capture_stop", json!({}))
}

#[test]
fn full_flow_over_ipc() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("GAMEPADLAB_DATA_DIR", dir.path());
    std::env::set_var("GAMEPADLAB_NO_HARDWARE", "1");
    let mut app = gamepadlab_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app");
    // The setup hook runs on the first event-loop iteration.
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let ipc = Ipc(webview);

    let info = ipc.ok("app_info", json!({}));
    // The bundle version (what the updater compares), not the crate's.
    assert_eq!(info["version"], app.package_info().version.to_string());
    assert_eq!(info["updaterConfigured"], false);
    assert_eq!(info["packageManaged"], cfg!(target_os = "linux"));
    assert_eq!(ipc.ok("list_controllers", json!({})), json!([]));

    // App icon swap (dark/light symbol).
    ipc.ok("set_app_icon", json!({ "variant": "light" }));
    ipc.ok("set_app_icon", json!({ "variant": "dark" }));
    let err = ipc
        .call("set_app_icon", json!({ "variant": "purple" }))
        .unwrap_err();
    assert_eq!(err["code"], "invalidInput");

    // Validation errors come back as { code, message }.
    let err = ipc.call("set_ui_rate", json!({ "hz": 5 })).unwrap_err();
    assert_eq!(err["code"], "invalidInput");
    ipc.ok("set_ui_rate", json!({ "hz": 120 }));
    ipc.ok("set_log_level", json!({ "level": "debug" }));
    ipc.ok(
        "log_frontend",
        json!({ "level": "info", "message": "hello from the test" }),
    );

    ipc.ok("set_demo", json!({ "enabled": true }));
    wait_for(|| {
        ipc.ok("list_controllers", json!({}))
            .as_array()
            .is_some_and(|a| a.len() == 1)
    });
    let pad = ipc.ok("list_controllers", json!({}))[0].clone();
    let id = pad["id"].as_str().unwrap().to_string();
    let key = pad["profileKey"].as_str().unwrap().to_string();
    assert_eq!(pad["isDemo"], true);

    // Wizard captures and analyses.
    let rest = capture(&ipc, &id, "rest", "left", json!("idle"), 400);
    assert_eq!(rest["kind"], "rest");
    assert!(
        rest["samples"].as_array().unwrap().len() > 40,
        "{}",
        rest["samples"]
    );
    capture(&ipc, &id, "rotation", "left", json!("rotateLeft"), 2600);
    capture(
        &ipc,
        &id,
        "directions",
        "left",
        json!("directionsLeft"),
        1200,
    );
    let report = ipc.ok(
        "analyze_stick",
        json!({ "stick": "left", "calibration": null }),
    );
    assert_eq!(report["stick"], "left");
    assert!(
        report["rest"]["offsetPct"].as_f64().unwrap() > 2.0,
        "demo drift"
    );
    assert!(report["circularity"]["sectors"].as_array().unwrap().len() == 32);
    let after = ipc.ok(
        "analyze_stick",
        json!({ "stick": "left", "calibration": report["suggested"] }),
    );
    assert_eq!(after["calibrated"], true);
    assert!(ipc
        .call(
            "analyze_stick",
            json!({ "stick": "middle", "calibration": null })
        )
        .is_err());
    assert!(ipc
        .call(
            "capture_start",
            json!({ "id": id, "kind": "bogus", "target": "left" })
        )
        .is_err());

    capture(&ipc, &id, "triggerRest", "left", json!("triggerRest"), 300);
    capture(&ipc, &id, "triggerFull", "left", json!("triggerFull"), 900);
    let trig = ipc.ok("analyze_trigger_cmd", json!({ "sideName": "left" }));
    assert_eq!(trig["fullTravel"], false);

    // Profiles.
    let doc = ipc.ok("profiles_load", json!({ "id": id }));
    assert_eq!(doc["deviceKey"], key);
    let created = ipc.ok(
        "profiles_create",
        json!({ "deviceKey": key, "name": "IPC", "calibration": null }),
    );
    let pid = created["id"].as_str().unwrap().to_string();
    let mut updated = created.clone();
    updated["calibration"]["leftStick"] = report["suggested"].clone();
    ipc.ok(
        "profiles_update",
        json!({ "deviceKey": key, "profile": updated }),
    );
    ipc.ok(
        "profiles_rename",
        json!({ "deviceKey": key, "profileId": pid, "name": "Renomeado" }),
    );
    let dup = ipc.ok(
        "profiles_duplicate",
        json!({ "deviceKey": key, "profileId": pid }),
    );
    ipc.ok(
        "profiles_set_active",
        json!({ "deviceKey": key, "profileId": pid }),
    );
    let exported = ipc.ok(
        "profiles_export",
        json!({ "deviceKey": key, "profileId": pid }),
    );
    let imported = ipc.ok(
        "profiles_import",
        json!({ "deviceKey": key, "json": exported }),
    );
    assert_eq!(imported["name"], "Renomeado (2)");
    ipc.ok(
        "profiles_reset",
        json!({ "deviceKey": key, "profileId": pid }),
    );
    ipc.ok(
        "profiles_delete",
        json!({ "deviceKey": key, "profileId": dup["id"] }),
    );
    assert_eq!(
        ipc.ok("profiles_load", json!({ "id": id }))["profiles"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    ipc.ok(
        "set_active_calibration",
        json!({ "id": id, "profile": updated["calibration"] }),
    );
    let bad = json!({ "leftStick": { "innerDeadzone": 3.0 } });
    assert_eq!(
        ipc.call(
            "set_active_calibration",
            json!({ "id": id, "profile": bad })
        )
        .unwrap_err()["code"],
        "invalidInput"
    );

    // Outputs.
    ipc.ok(
        "rumble_play",
        json!({ "id": id, "request": { "weak": 0.5, "strong": 0.7, "durationMs": 200, "pattern": "pulse", "periodMs": 100 } }),
    );
    ipc.ok("rumble_motor_test", json!({ "id": id, "intensity": 0.5 }));
    ipc.ok("rumble_stop", json!({ "id": id }));
    ipc.ok(
        "set_trigger_effects",
        json!({ "id": id, "left": { "type": "feedback", "position": 2, "strength": 5 }, "right": { "type": "off" } }),
    );
    ipc.ok(
        "set_lightbar",
        json!({ "id": id, "r": 10, "g": 200, "b": 120 }),
    );
    ipc.ok("set_player_leds", json!({ "id": id, "mask": 4 }));
    ipc.ok("demo_scenario", json!({ "scenario": { "pressButton": 3 } }));
    ipc.ok(
        "demo_manual",
        json!({ "input": { "leftStick": { "x": 0.5, "y": 0.0 }, "rightStick": { "x": 0.0, "y": 0.0 }, "leftTrigger": 0.0, "rightTrigger": 0.0, "buttons": [true] } }),
    );
    ipc.ok("reset_minmax", json!({ "id": id }));

    // Button health: read-only recording of the demo pad, whose R1 is going bad.
    ipc.ok("button_health_start", json!({ "id": id }));
    ipc.ok(
        "demo_scenario",
        json!({ "scenario": { "healthTaps": { "button": 5, "presses": 4 } } }),
    );
    std::thread::sleep(Duration::from_millis(1_700));
    let live = ipc.ok("button_health_progress", json!({}));
    assert!(live["buttons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["button"] == 5));
    let health = ipc.ok(
        "button_health_stop",
        json!({ "plans": [{ "button": 5, "requested": 4 }] }),
    );
    assert_eq!(health["overall"], "bad");
    assert_eq!(health["buttons"][0]["metrics"]["missed"], 1);
    assert_eq!(health["precision"]["kind"], "report");
    let err = ipc.call("button_health_progress", json!({})).unwrap_err();
    assert_eq!(err["code"], "invalidInput");
    ipc.ok("button_health_start", json!({ "id": id }));
    ipc.ok("button_health_cancel", json!({}));
    let err = ipc
        .call(
            "button_health_stop",
            json!({ "plans": [{ "button": 5, "requested": 4 }] }),
        )
        .unwrap_err();
    assert_eq!(err["code"], "invalidInput");

    // History and files.
    let sid = ipc.ok(
        "history_add",
        json!({ "session": { "kind": "buttons", "controllerName": "DualSense (demo)", "controllerKey": key, "score": 100, "summary": "ok", "payload": { "results": [] } } }),
    );
    assert!(sid.as_i64().unwrap() > 0);
    assert_eq!(
        ipc.ok("history_list", json!({ "limit": 10 }))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let hid = ipc.ok(
        "history_add",
        json!({ "session": { "kind": "buttonHealth", "controllerName": "DualSense (demo)", "controllerKey": key, "score": 20, "summary": "R1 ruim", "payload": { "report": health } } }),
    );
    assert!(hid.as_i64().unwrap() > sid.as_i64().unwrap());
    ipc.ok("history_delete", json!({ "id": sid }));
    ipc.ok("history_clear", json!({}));
    let file = dir.path().join("out.txt");
    ipc.ok("save_text_file", json!({ "path": file, "contents": "olá" }));
    assert_eq!(ipc.ok("read_text_file", json!({ "path": file })), "olá");
    ipc.ok(
        "save_binary_file",
        json!({ "path": dir.path().join("b.bin"), "base64": "AAEC" }),
    );
    assert_eq!(
        std::fs::read(dir.path().join("b.bin")).unwrap(),
        vec![0, 1, 2]
    );
    assert!(ipc
        .call(
            "save_text_file",
            json!({ "path": "relative.txt", "contents": "x" })
        )
        .is_err());

    // Firmware calibration (simulated on the demo DualSense).
    let st = ipc.ok("firmware", json!({ "id": id, "op": { "op": "status" } }));
    assert_eq!(st["status"]["model"], "ds5");
    assert_eq!(st["status"]["simulated"], true);
    assert_eq!(st["status"]["blockers"], json!([]));
    ipc.ok(
        "firmware",
        json!({ "id": id, "op": { "op": "centerBegin" } }),
    );
    ipc.ok(
        "firmware",
        json!({ "id": id, "op": { "op": "centerSample" } }),
    );
    let st = ipc.ok("firmware", json!({ "id": id, "op": { "op": "centerEnd" } }));
    assert_eq!(st["status"]["unsavedChanges"], true);
    let err = ipc
        .call(
            "firmware",
            json!({ "id": id, "op": { "op": "savePermanent", "acknowledgeRisk": false } }),
        )
        .unwrap_err();
    assert_eq!(err["code"], "firmwareBlocked");
    let saved = ipc.ok(
        "firmware",
        json!({ "id": id, "op": { "op": "savePermanent", "acknowledgeRisk": true } }),
    );
    assert_eq!(saved["status"]["unsavedChanges"], false);
    let hist = ipc.ok("history_list", json!({ "limit": 10 }));
    assert_eq!(hist[0]["kind"], "firmware");
    let err = ipc
        .call("firmware_switch_propose", json!({ "id": id }))
        .unwrap_err();
    assert_eq!(err["code"], "rumbleUnsupported");

    let diag = ipc.ok("diagnostics", json!({}));
    assert!(diag.as_str().unwrap().contains("DualSense (demo)"));
    assert!(ipc.ok("platform_warnings", json!({})).is_array());
    assert_eq!(ipc.ok("overlay_is_open", json!({})), false);

    ipc.ok("set_demo", json!({ "enabled": false }));
    wait_for(|| ipc.ok("list_controllers", json!({})) == json!([]));
}
