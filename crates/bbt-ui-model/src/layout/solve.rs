use crate::tokens::Density;
use super::{length, Align, Axis, Insets, Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle { pub size: f32, pub weight: u16, pub mono: bool, pub tracking_em: f32 }
impl TextStyle {
    pub fn for_density(density: Density) -> Self {
        Self { size: density.tokens().font as f32, weight: 400, mono: false, tracking_em: 0.0 }
    }
}
pub trait TextMeasure { fn width(&self, text: &str, style: &TextStyle) -> f32; }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Child { pub text: Option<(String, TextStyle)>, pub cross: Option<f32> }
#[derive(Debug, Clone, PartialEq)]
pub struct Box_ { pub axis: Axis, pub pad: Insets, pub gap: f32, pub children: Vec<(Size, Child)>, pub align: Align }
impl Box_ {
    pub fn new(axis: Axis, density: Density, children: Vec<(Size, Child)>) -> Self {
        Self { axis, pad: Insets::default(), gap: density.tokens().gap as f32, children, align: Align::Center }
    }
}

/// Fixed lengths retain their size (and may overflow the clip). Shrink text
/// proportionally before assigning remaining space to flex/fill children.
pub fn solve_box(box_: &Box_, bounds: Rect, measure: &dyn TextMeasure) -> Vec<Rect> {
    let inner = bounds.normalized().inset(box_.pad);
    let (main, cross) = match box_.axis { Axis::Row => (inner.w, inner.h), Axis::Column => (inner.h, inner.w) };
    let gap = length(box_.gap);
    let available = length(main - gap * box_.children.len().saturating_sub(1) as f32);
    let mut fixed = 0.0;
    let mut text = 0.0;
    let mut weights = 0.0;
    let sizes: Vec<_> = box_.children.iter().map(|(size, child)| match size {
        Size::Fixed(value) => { let value = length(*value); fixed += value; value },
        Size::Text => {
            let value = child.text.as_ref().map_or(0.0, |(s, style)| length(measure.width(s, style)));
            text += value; value
        }
        Size::Fill => { weights += 1.0; 0.0 },
        Size::Flex(weight) => { weights += length(*weight); 0.0 },
    }).collect();
    let text_room = length(available - fixed);
    let shrink = if text > text_room { text_room / text } else { 1.0 };
    let remainder = length(available - fixed - text * shrink);
    let mut cursor = 0.0;
    box_.children.iter().zip(sizes).map(|((size, child), natural)| {
        let extent = match size {
            Size::Text => natural * shrink,
            Size::Fill => if weights > 0.0 { remainder / weights } else { 0.0 },
            Size::Flex(weight) => if weights > 0.0 { remainder * length(*weight) / weights } else { 0.0 },
            Size::Fixed(_) => natural,
        };
        let child_cross = if box_.align == Align::Stretch { cross } else { child.cross.map_or(cross, |v| length(v).min(cross)) };
        let offset = match box_.align { Align::Center => (cross - child_cross) / 2.0, Align::End => cross - child_cross, _ => 0.0 };
        let rect = match box_.axis {
            Axis::Row => Rect::new(inner.x + cursor, inner.y + offset, extent, child_cross),
            Axis::Column => Rect::new(inner.x + offset, inner.y + cursor, child_cross, extent),
        };
        cursor += extent + gap; rect
    }).collect()
}
