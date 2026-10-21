pub use crate::model::Axis;

pub(crate) fn length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }
    pub fn intersects(self, other: Self) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && other.w > 0.0
            && other.h > 0.0
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
    pub fn normalized(self) -> Self {
        Self {
            x: if self.x.is_finite() { self.x } else { 0.0 },
            y: if self.y.is_finite() { self.y } else { 0.0 },
            w: length(self.w),
            h: length(self.h),
        }
    }
    pub fn inset(self, inset: Insets) -> Self {
        Self {
            x: self.x + length(inset.left),
            y: self.y + length(inset.top),
            w: length(self.w - length(inset.left) - length(inset.right)),
            h: length(self.h - length(inset.top) - length(inset.bottom)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}
impl Insets {
    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Fixed(f32),
    Flex(f32),
    Fill,
    Text,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}
