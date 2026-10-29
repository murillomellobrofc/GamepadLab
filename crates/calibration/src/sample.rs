use serde::{Deserialize, Serialize};

/// One captured reading. For sticks `x`/`y` are the axes; for triggers `x`
/// holds the value and `y` is 0. `t` is microseconds since capture start.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Sample {
    pub t: u64,
    pub x: f32,
    pub y: f32,
}

impl Sample {
    pub fn new(t: u64, x: f32, y: f32) -> Self {
        Self { t, x, y }
    }
    pub fn pos(&self) -> crate::Vec2 {
        crate::Vec2::new(self.x, self.y)
    }
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
