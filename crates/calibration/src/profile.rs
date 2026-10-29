//! Calibration settings stored in a profile and applied to live input.

use crate::curve::ResponseCurve;
use crate::geometry::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeadzoneMode {
    /// Circular deadzone on the stick magnitude, rescaled so output starts at
    /// zero right at the edge of the deadzone (no jump). The default.
    #[default]
    Radial,
    /// Independent deadzone per axis (good for D-pad-like stick use).
    Axial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GateShape {
    /// Only clamp to the -1..1 square the OS reports.
    #[default]
    None,
    Circle,
    Octagon,
    Square,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StickCalibration {
    /// Measured resting position, subtracted first.
    pub center: Vec2,
    /// Measured reach towards +x / +y after centring (1 = full).
    pub range_pos: Vec2,
    /// Measured reach towards -x / -y after centring, as positive numbers.
    pub range_neg: Vec2,
    pub inner_deadzone: f32,
    /// Fraction of the travel at the edge that already counts as 100 %.
    pub outer_deadzone: f32,
    pub deadzone_mode: DeadzoneMode,
    pub curve: ResponseCurve,
    pub sensitivity: Vec2,
    pub invert_x: bool,
    pub invert_y: bool,
    pub swap_axes: bool,
    /// Minimum output once outside the deadzone, to cancel a game's own deadzone.
    pub anti_deadzone: f32,
    pub gate: GateShape,
}

impl Default for StickCalibration {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            range_pos: Vec2::ONE,
            range_neg: Vec2::ONE,
            inner_deadzone: 0.05,
            outer_deadzone: 0.0,
            deadzone_mode: DeadzoneMode::Radial,
            curve: ResponseCurve::default(),
            sensitivity: Vec2::ONE,
            invert_x: false,
            invert_y: false,
            swap_axes: false,
            anti_deadzone: 0.0,
            gate: GateShape::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TriggerCalibration {
    /// Ignore everything below this (after normalising rest→full).
    pub deadzone: f32,
    /// Normalised value that already counts as fully pressed.
    pub max_limit: f32,
    /// Threshold for digital mode.
    pub activation_point: f32,
    pub digital: bool,
    /// Measured raw value when released.
    pub rest_value: f32,
    /// Measured raw value when fully pressed.
    pub full_value: f32,
}

impl Default for TriggerCalibration {
    fn default() -> Self {
        Self {
            deadzone: 0.02,
            max_limit: 1.0,
            activation_point: 0.5,
            digital: false,
            rest_value: 0.0,
            full_value: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CalibrationProfile {
    pub left_stick: StickCalibration,
    pub right_stick: StickCalibration,
    pub left_trigger: TriggerCalibration,
    pub right_trigger: TriggerCalibration,
    /// RFC 3339 time of the last wizard measurement, if any.
    pub measured_at: Option<String>,
}

/// A validation problem with a field path, for the import screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invalid {
    pub field: String,
    pub reason: String,
}

fn check(out: &mut Vec<Invalid>, field: &str, ok: bool, reason: &str) {
    if !ok {
        out.push(Invalid {
            field: field.into(),
            reason: reason.into(),
        });
    }
}

fn finite(v: f32) -> bool {
    v.is_finite()
}

impl StickCalibration {
    pub fn validate(&self, prefix: &str, out: &mut Vec<Invalid>) {
        let f = |name: &str| format!("{prefix}.{name}");
        check(
            out,
            &f("center"),
            self.center.is_finite() && self.center.length() <= 0.6,
            "must be within 0.6 of the middle",
        );
        for (name, v) in [("rangePos", self.range_pos), ("rangeNeg", self.range_neg)] {
            check(
                out,
                &f(name),
                finite(v.x)
                    && finite(v.y)
                    && (0.2..=1.5).contains(&v.x)
                    && (0.2..=1.5).contains(&v.y),
                "must be between 0.2 and 1.5",
            );
        }
        check(
            out,
            &f("innerDeadzone"),
            finite(self.inner_deadzone) && (0.0..=0.9).contains(&self.inner_deadzone),
            "must be between 0 and 0.9",
        );
        check(
            out,
            &f("outerDeadzone"),
            finite(self.outer_deadzone) && (0.0..=0.5).contains(&self.outer_deadzone),
            "must be between 0 and 0.5",
        );
        check(
            out,
            &f("deadzones"),
            self.inner_deadzone + self.outer_deadzone < 0.95,
            "inner + outer must leave room to move",
        );
        check(
            out,
            &f("sensitivity"),
            finite(self.sensitivity.x)
                && finite(self.sensitivity.y)
                && (0.1..=3.0).contains(&self.sensitivity.x)
                && (0.1..=3.0).contains(&self.sensitivity.y),
            "must be between 0.1 and 3",
        );
        check(
            out,
            &f("antiDeadzone"),
            finite(self.anti_deadzone) && (0.0..=0.5).contains(&self.anti_deadzone),
            "must be between 0 and 0.5",
        );
        if let Err(reason) = self.curve.validate() {
            check(out, &f("curve"), false, &reason);
        }
    }
}

impl TriggerCalibration {
    pub fn validate(&self, prefix: &str, out: &mut Vec<Invalid>) {
        let f = |name: &str| format!("{prefix}.{name}");
        check(
            out,
            &f("deadzone"),
            finite(self.deadzone) && (0.0..=0.5).contains(&self.deadzone),
            "must be between 0 and 0.5",
        );
        check(
            out,
            &f("maxLimit"),
            finite(self.max_limit)
                && (0.5..=1.0).contains(&self.max_limit)
                && self.max_limit > self.deadzone + 0.1,
            "must be between 0.5 and 1 and above the deadzone",
        );
        check(
            out,
            &f("activationPoint"),
            finite(self.activation_point) && (0.05..=0.95).contains(&self.activation_point),
            "must be between 0.05 and 0.95",
        );
        check(
            out,
            &f("restValue"),
            finite(self.rest_value) && (0.0..=0.5).contains(&self.rest_value),
            "must be between 0 and 0.5",
        );
        check(
            out,
            &f("fullValue"),
            finite(self.full_value)
                && (0.5..=1.0).contains(&self.full_value)
                && self.full_value > self.rest_value + 0.2,
            "must be between 0.5 and 1",
        );
    }
}

impl CalibrationProfile {
    pub fn validate(&self) -> Vec<Invalid> {
        let mut out = Vec::new();
        self.left_stick.validate("leftStick", &mut out);
        self.right_stick.validate("rightStick", &mut out);
        self.left_trigger.validate("leftTrigger", &mut out);
        self.right_trigger.validate("rightTrigger", &mut out);
        out
    }
}
