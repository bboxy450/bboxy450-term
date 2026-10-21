use super::Color;
use crate::{Density, Theme, TOKENS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dim { pub saturate: f32, pub opacity: f32, pub wash: f32 }
impl Default for Dim {
    fn default() -> Self { Self { saturate: TOKENS.dim.saturate as f32, opacity: TOKENS.dim.opacity as f32, wash: TOKENS.dim.wash as f32 } }
}
#[derive(Debug, Clone, Copy)]
pub struct DrawCtx { pub theme: Theme, pub density: Density, pub dim: Option<Dim> }
impl DrawCtx {
    pub fn new(theme: Theme, density: Density) -> Self { Self { theme, density, dim: None } }
    pub fn dimmed(self) -> Self { Self { dim: Some(Dim::default()), ..self } }
    pub fn color(self, color: Color) -> Color {
        match self.dim {
            None => color,
            Some(dim) => { let [h,s,l] = color.to_hsl(); let mut result = Color::hsl(h, s * dim.saturate, l); result.0[3] = color.0[3] * dim.opacity; result }
        }
    }
    /// Identity strokes and bumpers deliberately bypass dimming.
    pub fn identity(self, color: Color) -> Color { color }
    pub fn ground(self, color: Color) -> Color { self.dim.map_or(color, |d| color.mix(Color::hex(self.theme.palette().fg_dim), d.wash)) }
    pub fn fg(self) -> Color { self.color(Color::hex(self.theme.palette().fg)) }
    pub fn muted(self) -> Color { self.color(Color::hex(self.theme.palette().fg_dim)) }
    pub fn hairline(self) -> Color { self.color(Color::hex(self.theme.palette().line).alpha(crate::STYLE.material.hairline_alpha as f32)) }
}
