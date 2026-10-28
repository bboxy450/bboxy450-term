// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! L6 floating surfaces. The host owns visibility, dismissal, focus and input.
use crate::{Context, Length, Preset, Style, Text, TypeStep, components as c};
use bbt_ui_model::layout::Axis;

#[derive(Debug, Clone)]
pub struct Modal {
    pub title: String,
    pub body: c::Element,
    pub actions: Vec<c::Element>,
    pub style: Style,
}
impl Modal {
    pub fn new(title: impl Into<String>, body: c::Element, actions: Vec<c::Element>) -> Self {
        let mut style = Style::new(Preset::Modal);
        style.overrides.width = Some(Length::Px(380.));
        Self {
            title: title.into(),
            body,
            actions,
            style,
        }
    }
    pub fn element(self, _ctx: Context) -> c::Element {
        let mut actions = vec![c::Element::Spacer.into()];
        actions.extend(self.actions.into_iter().map(Into::into));
        let column = c::Element::Stack(c::Stack {
            direction: Axis::Column,
            gap: Length::Spacing(5),
            align: c::Alignment::Stretch,
            children: vec![
                c::Element::Text(c::Text::new(
                    self.title,
                    Text {
                        step: TypeStep::Subtitle,
                        weight: 650,
                        ..Default::default()
                    },
                ))
                .into(),
                self.body.into(),
                c::Element::Stack(c::Stack {
                    direction: Axis::Row,
                    gap: Length::Spacing(3),
                    align: c::Alignment::Center,
                    children: actions,
                })
                .into(),
            ],
        });
        let mut panel = c::Box::new(self.style, vec![column.into()]);
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Stretch;
        c::Element::Box(panel)
    }
}

pub mod context_menu;
pub mod dialog;

/// Pure interaction results. Hosts own effects, validation and overlay lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Local,
    Changed,
    Invoke(String),
    Dismiss,
}

pub(crate) fn contains(rect: bbt_ui_model::layout::Rect, at: [f32; 2]) -> bool {
    at[0] >= rect.x && at[0] < rect.right() && at[1] >= rect.y && at[1] < rect.bottom()
}

/// Retain only visible named geometry, respecting scroll and box clipping.
pub(crate) fn named_regions(layout: &c::Layout) -> Vec<(String, bbt_ui_model::layout::Rect)> {
    use bbt_ui_model::layout::Rect;
    fn intersection(a: Rect, b: Rect) -> Rect {
        let x = a.x.max(b.x);
        let y = a.y.max(b.y);
        Rect::new(
            x,
            y,
            (a.right().min(b.right()) - x).max(0.),
            (a.bottom().min(b.bottom()) - y).max(0.),
        )
    }
    fn visit(node: &c::Layout, clip: Rect, out: &mut Vec<(String, Rect)>) {
        let visible = intersection(node.rect, clip);
        if visible.w <= 0. || visible.h <= 0. {
            return;
        }
        if let c::LayoutKind::Named(name) = &node.kind {
            out.push((name.clone(), visible));
        }
        let clip = match &node.kind {
            c::LayoutKind::Box { clip: true, .. } | c::LayoutKind::ScrollView { .. } => visible,
            _ => clip,
        };
        for child in &node.children {
            visit(child, clip, out);
        }
    }
    let mut result = vec![];
    visit(layout, layout.rect, &mut result);
    result
}
