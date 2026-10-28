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
