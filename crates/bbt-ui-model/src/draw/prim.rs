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
        // Cancellation near full chroma can make unit saturation exceed one
        // by an ULP, which then escapes the RGB gamut on the return trip.
        [
            h,
            (d / (1.0 - (2.0 * l - 1.0).abs()).max(f32::EPSILON)).clamp(0., 1.),
            l,
        ]
    }
}

pub type Pt = [f32; 2];
/// Clamped linear alpha ramp. Draw-list coordinates are logical pixels;
/// retained Boxes translate their group-local endpoints at paint time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearAlphaMask {
    pub start: Pt,
    pub end: Pt,
    pub alpha0: f32,
    pub alpha1: f32,
}
impl LinearAlphaMask {
    pub fn validate(self) -> Result<(), &'static str> {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        let length2 = dx * dx + dy * dy;
        if self.start.iter().chain(&self.end).any(|v| !v.is_finite())
            || !length2.is_finite()
            || length2 <= 0.
            || !self.alpha0.is_finite()
            || !self.alpha1.is_finite()
            || !(0.0..=1.0).contains(&self.alpha0)
            || !(0.0..=1.0).contains(&self.alpha1)
        {
            return Err("invalid group linear alpha mask");
        }
        Ok(())
    }
    pub fn translated(self, offset: Pt) -> Self {
        Self {
            start: std::array::from_fn(|i| self.start[i] + offset[i]),
            end: std::array::from_fn(|i| self.end[i] + offset[i]),
            ..self
        }
    }
}
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
    pub italic: bool,
    pub tracking_em: f32,
    pub uppercase: bool,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            scale: TypeScale::Body,
            weight: 400,
            mono: false,
            italic: false,
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

/// One CSS text shadow. Blur is a logical-pixel radius; it never affects layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextShadow {
    pub color: Color,
    pub blur_radius: f32,
    pub offset: Pt,
}
impl TextShadow {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.blur_radius.is_finite()
            || self.blur_radius < 0.
            || self
                .offset
                .iter()
                .chain(self.color.0.iter())
                .any(|v| !v.is_finite())
        {
            return Err("invalid text shadow");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    /// Vector path; fill precedes stroke, with retained tessellation for each.
    Path {
        origin: Pt,
        scale: f32,
        points: Arc<[super::PathCommand]>,
        stroke: super::PathStroke,
        color: Color,
        fill: Option<Color>,
    },
    /// Blurred union coverage below a vector foreground, using the shared R8
    /// mask cache. The shadow's radius/offset are logical pixels, not path units.
    PathShadow {
        origin: Pt,
        scale: f32,
        points: Arc<[super::PathCommand]>,
        stroke: super::PathStroke,
        filled: bool,
        shadow: TextShadow,
    },
    /// Isolated children: render at full strength, then filter/composite once.
    /// Child clips are balanced locally; an outer clip still bounds the group.
    Group {
        primitives: Vec<Prim>,
        opacity: f32,
        saturation: f32,
        mask: Option<LinearAlphaMask>,
    },
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
    /// Origin is the line-box top-left. A resolved baseline is an absolute logical
    /// y coordinate; None retains the legacy vertically centred mask placement.
    Text {
        origin: Pt,
        baseline: Option<f32>,
        text: Arc<str>,
        style: TextStyle,
        max_width: Option<f32>,
        color: Color,
    },
    /// Glyph coverage beneath its foreground text, with the same shaping inputs.
    TextShadow {
        origin: Pt,
        baseline: Option<f32>,
        text: Arc<str>,
        style: TextStyle,
        max_width: Option<f32>,
        shadow: TextShadow,
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

#[cfg(test)]
mod color_tests {
    use super::Color;
    #[test]
    fn text_shadow_accepts_negative_offsets_but_not_invalid_blur_or_coordinates() {
        let shadow = super::TextShadow {
            color: Color::WHITE,
            blur_radius: 0.,
            offset: [-2., 1.],
        };
        assert!(shadow.validate().is_ok());
        for blur_radius in [-1., f32::NAN, f32::INFINITY] {
            assert!(
                super::TextShadow {
                    blur_radius,
                    ..shadow
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            super::TextShadow {
                offset: [f32::NAN, 0.],
                ..shadow
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn valid_rgb_boundary_colours_round_trip_inside_the_unit_gamut() {
        for value in [
            "FFC53D", "ff0000", "00ff00", "0000ff", "ffff00", "00ffff", "ff00ff", "ffffff",
            "000000", "010101", "fefefe", "0044ff", "ff4400", "44ff00",
        ] {
            let before = Color::hex(value);
            let [h, s, l] = before.to_hsl();
            assert!((0.0..=1.).contains(&s), "{value}: saturation {s}");
            let after = Color::hsl(h, s, l);
            assert!(
                after.0.into_iter().all(|v| (0.0..=1.).contains(&v)),
                "{value}: {:?}",
                after.0
            );
            assert!(
                before
                    .0
                    .iter()
                    .zip(after.0)
                    .all(|(a, b)| (*a - b).abs() < 0.000001),
                "{value}: {:?}",
                after.0
            );
        }
    }
}
