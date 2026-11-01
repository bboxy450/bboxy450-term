// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Tool invocation disclosure. Execution and permissions belong to the host.
use super::{accordion::Accordion, seat_row::named};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color, layout::Axis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
    pub fn color(self) -> [f32; 4] {
        Color::hex(match self {
            Self::Pending => TOKENS.color.state.waiting,
            Self::Running => TOKENS.color.role.executor,
            Self::Succeeded => TOKENS.color.ui.ok,
            Self::Failed => TOKENS.color.state.errored,
            Self::Cancelled => TOKENS.color.state.waiting,
        })
        .0
    }
}
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub key: String,
    pub title: String,
    pub status: Status,
    pub arguments: String,
    pub output: String,
    pub arguments_open: bool,
    pub output_open: bool,
}
impl ToolCall {
    pub fn toggle(&mut self, key: &str) -> bool {
        if key == format!("{}/arguments/toggle", self.key) {
            self.arguments_open = !self.arguments_open;
            true
        } else if key == format!("{}/output/toggle", self.key) {
            self.output_open = !self.output_open;
            true
        } else {
            false
        }
    }
    pub fn element(&self, ctx: ui::Context, width: f32, m: &dyn c::Measure) -> c::Element {
        let mut chip = c::Box::new(
            ui::Style::new(ui::Preset::Card),
            vec![
                c::Element::Text(c::Text::new(
                    self.status.label(),
                    ui::Text {
                        step: ui::TypeStep::Caption,
                        weight: 600,
                        paint: ui::Paint::solid(ui::Color::Rgba(self.status.color())),
                        ..Default::default()
                    },
                ))
                .into(),
            ],
        );
        chip.style.overrides.padding = Some([ui::Length::Px(STYLE.spacing.s3 as f32); 4]);
        chip.style.overrides.material = Some(ui::Material::Flat);
        chip.style.overrides.shadow = Some(ui::Shadow::None);
        chip.style.overrides.radii = Some([ui::Length::Px(TOKENS.radius.chip as f32); 4]);
        chip.style.overrides.fill = Some(ui::Paint {
            color: ui::Color::Rgba(self.status.color()),
            alpha: ui::Alpha::Badge,
        });
        let status = c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(0.),
            align: c::Alignment::Start,
            children: vec![named(format!("{}/status", self.key), c::Element::Box(chip)).into()],
        });
        let mut children = vec![
            status.into(),
            c::Element::Text(c::Text::new(
                &self.title,
                ui::Text {
                    weight: 600,
                    ..Default::default()
                },
            ))
            .into(),
        ];
        for (part, title, value, open) in [
            (
                "arguments",
                "Arguments",
                &self.arguments,
                self.arguments_open,
            ),
            ("output", "Output", &self.output, self.output_open),
        ] {
            let key = format!("{}/{part}", self.key);
            let mut t = c::Text::new(
                value,
                ui::Text {
                    mono: true,
                    ..Default::default()
                },
            );
            t.preserve_whitespace = true;
            let body = c::Element::Text(t);
            let h = if open {
                body.natural(Accordion::body_size(ctx, [width, 0.], true)[0], ctx, m)
                    .expect("tool body style")
                    .0[1]
            } else {
                0.
            };
            let mut e = Accordion {
                key: &key,
                title,
                color: self.status.color(),
                count: 0,
                open,
            }
            .element(
                ctx,
                [width, Accordion::chrome_height(ctx, open) + h],
                Some(body),
            );
            // Stable action regions independent of Accordion's internal header names.
            if let c::Element::Named { content, .. } = &mut e
                && let c::Element::Box(frame) = content.as_mut()
                && let c::Element::Box(inner) = &mut frame.children[0].element
            {
                inner.children[0].element =
                    named(format!("{key}/toggle"), inner.children[0].element.clone());
            }
            children.push(e.into());
        }
        named(
            &self.key,
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(STYLE.spacing.s3 as f32),
                align: c::Alignment::Stretch,
                children,
            }),
        )
    }
}
