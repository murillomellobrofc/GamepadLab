//! Schema migrations.
//!
//! * v1 (pre-release prototype): one `deadzone` per stick, `curve` as a
//!   string, `center` as `[x, y]`, `invertY` flag; triggers only had
//!   `deadzone`.
//! * v2 (current): full `StickCalibration` / `TriggerCalibration` objects.

use crate::{DeviceProfiles, Profile, ProfileError, Result, EXPORT_FORMAT};
use gamepadlab_calibration::{
    CalibrationProfile, CurveKind, ResponseCurve, StickCalibration, TriggerCalibration, Vec2,
};
use serde_json::{json, Value};

pub const CURRENT_SCHEMA: u32 = 2;

fn num(v: &Value, key: &str) -> Option<f32> {
    v.get(key).and_then(Value::as_f64).map(|x| x as f32)
}

fn v1_stick(v: &Value) -> StickCalibration {
    let mut s = StickCalibration::default();
    if let Some(dz) = num(v, "deadzone") {
        s.inner_deadzone = dz.clamp(0.0, 0.9);
    }
    if let Some(c) = v.get("center").and_then(Value::as_array) {
        let x = c.first().and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let y = c.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        s.center = Vec2::new(x, y);
    }
    s.curve = ResponseCurve::preset(match v.get("curve").and_then(Value::as_str) {
        Some("smooth") | Some("exponential") => CurveKind::Smooth,
        Some("aggressive") => CurveKind::Aggressive,
        Some("s") | Some("scurve") | Some("s-curve") => CurveKind::SCurve,
        _ => CurveKind::Linear,
    });
    s.invert_y = v.get("invertY").and_then(Value::as_bool).unwrap_or(false);
    s
}

fn v1_trigger(v: &Value) -> TriggerCalibration {
    TriggerCalibration {
        deadzone: num(v, "deadzone").unwrap_or(0.02).clamp(0.0, 0.5),
        ..TriggerCalibration::default()
    }
}

/// Upgrades one profile object (as found in a device file or an export).
pub fn migrate_profile_value(mut v: Value, from: u32) -> Result<Profile> {
    if from > CURRENT_SCHEMA {
        return Err(ProfileError::Invalid(format!(
            "made by a newer GamepadLab (schema {from}); update the app"
        )));
    }
    if from < 2 {
        let empty = json!({});
        let cal = v.get("calibration").cloned().unwrap_or(json!({}));
        let pick = |k: &str| cal.get(k).unwrap_or(&empty).clone();
        let upgraded = CalibrationProfile {
            left_stick: v1_stick(&pick("left")),
            right_stick: v1_stick(&pick("right")),
            left_trigger: v1_trigger(&pick("l2")),
            right_trigger: v1_trigger(&pick("r2")),
            measured_at: None,
        };
        v["calibration"] = serde_json::to_value(upgraded)?;
        if v.get("createdAt").is_none() {
            v["createdAt"] = v.get("created").cloned().unwrap_or(json!(""));
        }
        if v.get("updatedAt").is_none() {
            v["updatedAt"] = v["createdAt"].clone();
        }
        if v.get("id").is_none() {
            v["id"] = json!(crate::new_id());
        }
    }
    serde_json::from_value(v).map_err(|e| ProfileError::Invalid(e.to_string()))
}

fn schema_of(v: &Value) -> u32 {
    v.get("schemaVersion")
        .or_else(|| v.get("version"))
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32
}

pub fn migrate_device_value(v: Value) -> Result<DeviceProfiles> {
    let from = schema_of(&v);
    if from > CURRENT_SCHEMA {
        return Err(ProfileError::Invalid(format!(
            "profile file schema {from} is newer than this app"
        )));
    }
    let profiles = v
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|p| migrate_profile_value(p, from))
        .collect::<Result<Vec<_>>>()?;
    Ok(DeviceProfiles {
        schema_version: CURRENT_SCHEMA,
        device_key: v
            .get("deviceKey")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        device_name: v
            .get("deviceName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        active_profile_id: v
            .get("activeProfileId")
            .and_then(Value::as_str)
            .map(String::from),
        profiles,
    })
}

/// Accepts an export envelope (any supported version) or a bare profile.
pub fn migrate_import_value(v: Value) -> Result<Profile> {
    if !v.is_object() {
        return Err(ProfileError::Invalid("not a JSON object".into()));
    }
    if let Some(format) = v.get("format").and_then(Value::as_str) {
        if format != EXPORT_FORMAT {
            return Err(ProfileError::Invalid(format!(
                "unknown format \"{format}\""
            )));
        }
        let from = schema_of(&v);
        let profile = v
            .get("profile")
            .cloned()
            .ok_or_else(|| ProfileError::Invalid("missing \"profile\"".into()))?;
        return migrate_profile_value(profile, from);
    }
    if v.get("calibration").is_some() && v.get("name").is_some() {
        let from = schema_of(&v);
        return migrate_profile_value(v, from);
    }
    Err(ProfileError::Invalid(
        "this is not a GamepadLab profile".into(),
    ))
}
