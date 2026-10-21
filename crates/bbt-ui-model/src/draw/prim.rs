use crate::layout::Rect;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color(pub [f32; 4]);
impl Color {
    pub const TRANSPARENT: Self = Self([0.0; 4]);
    pub const WHITE: Self = Self([1.0; 4]);
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);
    pub fn hex(value: &str) -> Self {
        let s = value.trim_start_matches('#');
        let n = u32::from_str_radix(s, 16).expect("design colour must be hexadecimal");
        match s.len() {
            6 => Self([
                ((n >> 16) & 255) as f32 / 255.0,
                ((n >> 8) & 255) as f32 / 255.0,
                (n & 255) as f32 / 255.0,
                1.0,
            ]),
            8 => Self([
                ((n >> 24) & 255) as f32 / 255.0,
                ((n >> 16) & 255) as f32 / 255.0,
                ((n >> 8) & 255) as f32 / 255.0,
                (n & 255) as f32 / 255.0,
            ]),
            _ => panic!("design colour requires six or eight hex digits"),
        }
    }
    pub fn alpha(self, alpha: f32) -> Self {
        Self([
            self.0[0],
            self.0[1],
            self.0[2],
            self.0[3] * alpha.clamp(0.0, 1.0),
        ])
    }
    pub fn mix(self, other: Self, amount: f32) -> Self {
        let t = amount.clamp(0.0, 1.0);
        Self(std::array::from_fn(|i| {
            self.0[i] * (1.0 - t) + other.0[i] * t
        }))
    }
    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let hp = h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
        let rgb = match hp as u32 {
            0 => [c, x, 0.0],
            1 => [x, c, 0.0],
            2 => [0.0, c, x],
            3 => [0.0, x, c],
            4 => [x, 0.0, c],
            _ => [c, 0.0, x],
        };
        Self([
            rgb[0] + l - c / 2.0,
            rgb[1] + l - c / 2.0,
            rgb[2] + l - c / 2.0,
            1.0,
        ])
    }
    pub fn to_hsl(self) -> [f32; 3] {
        let [r, g, b, _] = self.0;
        let hi = r.max(g).max(b);
        let lo = r.min(g).min(b);
        let d = hi - lo;
        let l = (hi + lo) * 0.5;
        if d <= f32::EPSILON {
            return [0.0, 0.0, l];
        }
        let h = if hi == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if hi == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        } * 60.0;
        [h, d / (1.0 - (2.0 * l - 1.0).abs()).max(f32::EPSILON), l]
    }
}

pub type Pt = [f32; 2];
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeScale {
    Input,
    Title,
    Subtitle,
    Kind,
    Body,
    Row,
    Section,
    Badge,
    Caption,
    Band,
    Custom(f32),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub scale: TypeScale,
    pub weight: u16,
    pub mono: bool,
    pub tracking_em: f32,
    pub uppercase: bool,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            scale: TypeScale::Body,
            weight: 400,
            mono: false,
            tracking_em: 0.0,
            uppercase: false,
        }
    }
}
impl TextStyle {
    pub fn pixels(self) -> f32 {
        let t = &crate::STYLE.r#type;
        match self.scale {
            TypeScale::Input => t.input as f32,
            TypeScale::Title => t.title as f32,
            TypeScale::Subtitle => t.subtitle as f32,
            TypeScale::Kind => t.kind as f32,
            TypeScale::Body => t.body as f32,
            TypeScale::Row => t.row as f32,
            TypeScale::Section => t.section as f32,
            TypeScale::Badge => t.badge as f32,
            TypeScale::Caption => t.caption as f32,
            TypeScale::Band => 10.5,
            TypeScale::Custom(n) => n,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Rect {
        rect: Rect,
        radii: [f32; 4],
        fill: Color,
        stroke: Option<(f32, Color)>,
        inner_highlight: Option<Color>,
        shadow: Option<(f32, f32, Color)>,
    },
    Hairline {
        from: Pt,
        to: Pt,
        width: f32,
        color: Color,
    },
    Gloss {
        rect: Rect,
        radii: [f32; 4],
        color: Color,
    },
    /// Origin is the top-left of the line box; the renderer owns baseline and ellipsis.
    Text {
        origin: Pt,
        text: Arc<str>,
        style: TextStyle,
        max_width: Option<f32>,
        color: Color,
    },
    Icon {
        rect: Rect,
        icon: Arc<str>,
        color: Color,
    },
    Image {
        rect: Rect,
        image: Arc<str>,
        radius: f32,
    },
    Clip {
        rect: Rect,
        radii: [f32; 4],
    },
    Unclip,
}
