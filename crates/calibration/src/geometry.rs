use serde::{Deserialize, Serialize};

/// 2-D point/vector. Stick convention: x right = +1, y up = +1.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const ONE: Vec2 = Vec2 { x: 1.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_polar(radius: f32, angle_rad: f32) -> Self {
        Self::new(radius * angle_rad.cos(), radius * angle_rad.sin())
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Angle in radians, `0` = right, counter-clockwise, range `-π..=π`.
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// Angle in degrees normalised to `0..360`.
    pub fn angle_deg(self) -> f32 {
        let d = self.angle().to_degrees();
        if d < 0.0 {
            d + 360.0
        } else {
            d
        }
    }

    pub fn scale(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k)
    }

    pub fn clamp_length(self, max: f32) -> Self {
        let len = self.length();
        if len > max && len > 0.0 {
            self.scale(max / len)
        } else {
            self
        }
    }

    pub fn clamp_unit_square(self) -> Self {
        Self::new(self.x.clamp(-1.0, 1.0), self.y.clamp(-1.0, 1.0))
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

/// Smallest signed difference between two angles in degrees (`-180..180`).
pub fn angle_diff_deg(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polar_roundtrip() {
        let v = Vec2::from_polar(0.8, 1.0);
        assert!((v.length() - 0.8).abs() < 1e-6);
        assert!((v.angle() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn angle_deg_is_positive() {
        assert!((Vec2::new(0.0, -1.0).angle_deg() - 270.0).abs() < 1e-4);
        assert!((Vec2::new(1.0, 0.0).angle_deg()).abs() < 1e-4);
    }

    #[test]
    fn angle_diff_wraps() {
        assert!((angle_diff_deg(350.0, 10.0) + 20.0).abs() < 1e-4);
        assert!((angle_diff_deg(10.0, 350.0) - 20.0).abs() < 1e-4);
    }

    #[test]
    fn clamp_length_keeps_direction() {
        let v = Vec2::new(3.0, 4.0).clamp_length(1.0);
        assert!((v.length() - 1.0).abs() < 1e-6);
        assert!((v.x - 0.6).abs() < 1e-6);
    }
}
