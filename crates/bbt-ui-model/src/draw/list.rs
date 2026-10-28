// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{Color, Prim, TextStyle};
use crate::layout::Rect;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct DrawList {
    pub primitives: Vec<Prim>,
    pub revision: u64,
}

impl DrawList {
    pub fn push(&mut self, prim: Prim) {
        self.primitives.push(prim);
    }
    pub fn rect(&mut self, rect: Rect, radius: f32, fill: Color) {
        self.push(Prim::Rect {
            rect,
            radii: [radius; 4],
            fill,
            stroke: None,
            inner_highlight: None,
            shadow: None,
        });
    }
    pub fn text(&mut self, rect: Rect, text: impl Into<Arc<str>>, style: TextStyle, color: Color) {
        self.push(Prim::Text {
            origin: [rect.x, rect.y],
            baseline: None,
            text: text.into(),
            style,
            max_width: Some(rect.w),
            color,
        });
    }
    pub fn line(&mut self, from: [f32; 2], to: [f32; 2], width: f32, color: Color) {
        self.push(Prim::Hairline {
            from,
            to,
            width,
            color,
        });
    }
    pub fn clip(&mut self, rect: Rect, radii: [f32; 4]) {
        self.push(Prim::Clip { rect, radii });
    }
    pub fn unclip(&mut self) {
        self.push(Prim::Unclip);
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        Self::validate_primitives(&self.primitives, 0)
    }
    fn validate_primitives(primitives: &[Prim], groups: usize) -> Result<(), &'static str> {
        if groups > 64 {
            return Err("draw-list group nesting exceeded");
        }
        let mut depth = 0usize;
        for p in primitives {
            match p {
                Prim::Path {
                    origin,
                    scale,
                    points,
                    stroke,
                    color,
                    fill,
                } => {
                    stroke.validate(points)?;
                    if origin
                        .iter()
                        .chain(color.0.iter())
                        .chain(fill.iter().flat_map(|c| c.0.iter()))
                        .any(|n| !n.is_finite())
                        || !scale.is_finite()
                        || *scale <= 0.
                    {
                        return Err("invalid path transform or colour");
                    }
                }
                Prim::PathShadow {
                    origin,
                    scale,
                    points,
                    stroke,
                    shadow,
                    ..
                } => {
                    stroke.validate(points)?;
                    shadow.validate()?;
                    if origin.iter().any(|v| !v.is_finite()) || !scale.is_finite() || *scale <= 0. {
                        return Err("invalid path shadow transform");
                    }
                }
                Prim::Group {
                    primitives,
                    opacity,
                    saturation,
                    mask,
                } => {
                    if !opacity.is_finite()
                        || !(0.0..=1.0).contains(opacity)
                        || !saturation.is_finite()
                        || !(0.0..=4.0).contains(saturation)
                    {
                        return Err("invalid draw-list group filter");
                    }
                    if let Some(mask) = mask {
                        mask.validate()?;
                    }
                    Self::validate_primitives(primitives, groups + 1)?;
                }
                Prim::Clip { .. } => depth += 1,
                Prim::Text { baseline, .. } | Prim::TextShadow { baseline, .. }
                    if baseline.is_some_and(|y| !y.is_finite()) =>
                {
                    return Err("invalid text baseline");
                }
                Prim::Unclip if depth == 0 => return Err("unbalanced draw-list clip"),
                Prim::Unclip => depth -= 1,
                _ => {}
            }
        }
        if depth == 0 {
            Ok(())
        } else {
            Err("unclosed draw-list clip")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec renderer §1 — valid recursive draw-list and clip scopes
    #[test]
    fn group_filters_and_clip_scopes_validate_recursively() {
        let group = |primitives, opacity, saturation| DrawList {
            primitives: vec![Prim::Group {
                primitives,
                opacity,
                saturation,
                mask: None,
            }],
            ..Default::default()
        };
        assert!(group(vec![], 0.5, 0.1).validate().is_ok());
        assert!(group(vec![], f32::NAN, 1.).validate().is_err());
        assert!(group(vec![], 1., -1.).validate().is_err());
        assert!(group(vec![Prim::Unclip], 1., 1.).validate().is_err());
        let mut outer = group(vec![Prim::Unclip], 1., 1.);
        outer.primitives.insert(
            0,
            Prim::Clip {
                rect: Rect::new(0., 0., 1., 1.),
                radii: [0.; 4],
            },
        );
        // A child's pop cannot consume the outer clip.
        assert!(outer.validate().is_err());
        let mut nested = DrawList::default();
        for _ in 0..66 {
            nested = group(nested.primitives, 1., 1.);
        }
        assert!(nested.validate().is_err());
    }
}
