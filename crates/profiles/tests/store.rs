use gamepadlab_calibration::{CalibrationProfile, CurveKind, StickCalibration, Vec2};
use gamepadlab_profiles::*;

fn store() -> (tempfile::TempDir, ProfileStore) {
    let dir = tempfile::tempdir().unwrap();
    let s = ProfileStore::new(dir.path().join("profiles"));
    (dir, s)
}

const KEY: &str = "054c:0ce6:aa:bb:cc";

#[test]
fn first_load_creates_a_default_profile() {
    let (_d, s) = store();
    let doc = s.load(KEY, "DualSense").unwrap();
    assert_eq!(doc.schema_version, CURRENT_SCHEMA);
    assert_eq!(doc.profiles.len(), 1);
    assert_eq!(doc.active().unwrap().name, "Padrão");
    assert_eq!(doc.device_name, "DualSense");
}

#[test]
fn create_rename_duplicate_switch_delete() {
    let (_d, s) = store();
    let mut cal = CalibrationProfile::default();
    cal.left_stick.inner_deadzone = 0.12;
    let fps = s.create(KEY, "FPS", Some(cal.clone())).unwrap();
    let doc = s.load(KEY, "").unwrap();
    assert_eq!(doc.profiles.len(), 2);
    assert_eq!(doc.active_profile_id.as_deref(), Some(fps.id.as_str()));

    // Names stay unique.
    let again = s.create(KEY, "FPS", None).unwrap();
    assert_eq!(again.name, "FPS (2)");

    s.rename(KEY, &fps.id, "  Tiro  ").unwrap();
    let copy = s.duplicate(KEY, &fps.id).unwrap();
    assert_eq!(copy.name, "Tiro (cópia)");
    assert_eq!(copy.calibration.left_stick.inner_deadzone, 0.12);
    assert_ne!(copy.id, fps.id);

    let first = s.load(KEY, "").unwrap().profiles[0].id.clone();
    s.set_active(KEY, &first).unwrap();
    assert_eq!(
        s.load(KEY, "").unwrap().active_profile_id.as_deref(),
        Some(first.as_str())
    );

    s.delete(KEY, &first).unwrap();
    let doc = s.load(KEY, "").unwrap();
    assert!(
        doc.active().is_some(),
        "deleting the active profile picks another"
    );
    assert!(matches!(
        s.delete(KEY, "nope"),
        Err(ProfileError::NotFound(_))
    ));
}

#[test]
fn cannot_delete_last_profile() {
    let (_d, s) = store();
    let doc = s.load(KEY, "").unwrap();
    s.save(&doc).unwrap();
    assert!(matches!(
        s.delete(KEY, &doc.profiles[0].id),
        Err(ProfileError::LastProfile)
    ));
}

#[test]
fn update_validates_and_reset_restores_factory() {
    let (_d, s) = store();
    let p = s.create(KEY, "x", None).unwrap();
    let mut bad = p.clone();
    bad.calibration.left_stick.inner_deadzone = 2.0;
    let err = s.update(KEY, bad).unwrap_err();
    assert_eq!(err.code(), "invalidInput");
    assert!(err.to_string().contains("leftStick.innerDeadzone"));

    let mut good = p.clone();
    good.calibration.right_stick.curve.kind = CurveKind::SCurve;
    s.update(KEY, good).unwrap();
    let reset = s.reset(KEY, &p.id).unwrap();
    assert_eq!(reset.calibration, CalibrationProfile::default());
}

#[test]
fn export_import_roundtrip() {
    let (_d, s) = store();
    let cal = CalibrationProfile {
        left_stick: StickCalibration {
            center: Vec2::new(0.03, -0.02),
            inner_deadzone: 0.08,
            ..StickCalibration::default()
        },
        ..CalibrationProfile::default()
    };
    let p = s.create(KEY, "Meu perfil", Some(cal.clone())).unwrap();
    let json = s.export(KEY, &p.id, "0.1.0").unwrap();
    assert!(json.contains("\"format\": \"gamepadlab-profile\""));
    assert!(json.contains("\"schemaVersion\": 2"));

    // Import into another controller.
    let other = "045e:0b13";
    let imported = s.import(other, &json).unwrap();
    assert_eq!(imported.calibration, cal);
    assert_eq!(imported.name, "Meu perfil");
    assert_ne!(imported.id, p.id);
    // Importing twice keeps names unique.
    assert_eq!(s.import(other, &json).unwrap().name, "Meu perfil (2)");
}

#[test]
fn import_rejects_garbage_and_invalid_values() {
    let (_d, s) = store();
    assert!(s.import(KEY, "not json").is_err());
    assert!(s.import(KEY, "[]").is_err());
    assert!(s.import(KEY, r#"{"format":"other","profile":{}}"#).is_err());
    assert!(s.import(KEY, r#"{"hello":1}"#).is_err());
    let invalid = r#"{"format":"gamepadlab-profile","schemaVersion":2,"profile":{
        "id":"x","name":"bad","createdAt":"","updatedAt":"",
        "calibration":{"leftStick":{"outerDeadzone":0.9}}}}"#;
    let err = s.import(KEY, invalid).unwrap_err();
    assert!(err.to_string().contains("outerDeadzone"), "{err}");
    let future = r#"{"format":"gamepadlab-profile","schemaVersion":99,"profile":{"name":"x","calibration":{}}}"#;
    assert!(s
        .import(KEY, future)
        .unwrap_err()
        .to_string()
        .contains("newer"));
}

#[test]
fn migrates_v1_export() {
    let (_d, s) = store();
    let v1 = r#"{
        "format": "gamepadlab-profile",
        "version": 1,
        "profile": {
            "name": "Antigo",
            "created": "2026-01-02T03:04:05Z",
            "calibration": {
                "left":  { "deadzone": 0.1, "curve": "smooth", "center": [0.04, -0.01], "invertY": true },
                "right": { "deadzone": 0.06, "curve": "s-curve" },
                "l2": { "deadzone": 0.05 }
            }
        }
    }"#;
    let p = s.import(KEY, v1).unwrap();
    assert_eq!(p.name, "Antigo");
    assert_eq!(p.created_at, "2026-01-02T03:04:05Z");
    let l = &p.calibration.left_stick;
    assert_eq!(l.inner_deadzone, 0.1);
    assert_eq!(l.curve.kind, CurveKind::Smooth);
    assert!(l.invert_y);
    assert!((l.center.x - 0.04).abs() < 1e-6);
    assert_eq!(p.calibration.right_stick.curve.kind, CurveKind::SCurve);
    assert_eq!(p.calibration.left_trigger.deadzone, 0.05);
    assert_eq!(p.calibration.right_trigger.deadzone, 0.02);
}

#[test]
fn migrates_v1_device_file_on_load() {
    let (_d, s) = store();
    std::fs::create_dir_all(s.dir()).unwrap();
    let v1 = r#"{"version":1,"deviceKey":"045e:0b13","deviceName":"Xbox",
        "activeProfileId":"a","profiles":[{"id":"a","name":"Velho",
        "calibration":{"left":{"deadzone":0.2}}}]}"#;
    std::fs::write(s.dir().join("045e_0b13.json"), v1).unwrap();
    let doc = s.load("045e:0b13", "").unwrap();
    assert_eq!(doc.schema_version, 2);
    assert_eq!(doc.active().unwrap().name, "Velho");
    assert_eq!(doc.profiles[0].calibration.left_stick.inner_deadzone, 0.2);
}

#[test]
fn corrupt_file_is_backed_up_not_fatal() {
    let (_d, s) = store();
    std::fs::create_dir_all(s.dir()).unwrap();
    std::fs::write(s.dir().join("045e_0b13.json"), "{ broken").unwrap();
    let doc = s.load("045e:0b13", "Xbox").unwrap();
    assert_eq!(doc.profiles.len(), 1);
    assert!(s.dir().join("045e_0b13.broken.json").exists());
}

#[test]
fn files_are_per_device_and_keys_are_sanitised() {
    let (_d, s) = store();
    s.create("054c:0ce6:../../etc", "x", None).unwrap();
    let names: Vec<String> = std::fs::read_dir(s.dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1);
    assert!(
        !names[0].contains('/') && !names[0].contains(".."),
        "{names:?}"
    );
}
