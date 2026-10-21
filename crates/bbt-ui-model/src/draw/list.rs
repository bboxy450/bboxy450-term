use super::{Color, Prim, TextStyle};
use crate::layout::Rect;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct DrawList { pub primitives: Vec<Prim>, pub revision: u64 }
impl DrawList {
    pub fn push(&mut self, prim: Prim) { self.primitives.push(prim); }
    pub fn rect(&mut self, rect: Rect, radius: f32, fill: Color) { self.push(Prim::Rect { rect, radii: [radius;4], fill, stroke: None, inner_highlight: None, shadow: None }); }
    pub fn text(&mut self, rect: Rect, text: impl Into<Arc<str>>, style: TextStyle, color: Color) { self.push(Prim::Text { origin: [rect.x, rect.y], text: text.into(), style, max_width: Some(rect.w), color }); }
    pub fn line(&mut self, from: [f32;2], to: [f32;2], width: f32, color: Color) { self.push(Prim::Hairline { from, to, width, color }); }
    pub fn clip(&mut self, rect: Rect, radii: [f32;4]) { self.push(Prim::Clip { rect, radii }); }
    pub fn unclip(&mut self) { self.push(Prim::Unclip); }
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut depth = 0usize;
        for p in &self.primitives { match p { Prim::Clip {..} => depth += 1, Prim::Unclip if depth == 0 => return Err("unbalanced draw-list clip"), Prim::Unclip => depth -= 1, _ => {} } }
        if depth == 0 { Ok(()) } else { Err("unclosed draw-list clip") }
    }
}
