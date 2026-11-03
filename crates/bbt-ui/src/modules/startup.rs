// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Startup observations rendered as a checklist; the host owns all transitions.
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color, layout::Axis};
use serde::{Deserialize, Serialize};

pub const STEPS: [&str; 4] = [
    "Connect to supervisor",
    "Start session runtime",
    "Verify signed history",
    "Attach cockpit",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub stream: String,
    pub verified: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Startup {
    pub title: String,
    pub description: String,
    /// Time in the current phase, not the total startup duration.
    pub elapsed_secs: u64,
    pub active_step: usize,
    pub history: Option<History>,
    pub details: Vec<[String; 2]>,
    pub error: Option<String>,
    pub logs: Vec<[String; 2]>,
}

fn text(value: impl Into<String>, step: ui::TypeStep, color: ui::Color, weight: u16) -> c::Element {
    c::Element::Text(c::Text::new(
        value,
        ui::Text {
            step,
            size: (step == ui::TypeStep::Title).then_some(STYLE.r#type.heading as f32),
            weight,
            paint: ui::Paint::solid(color),
            ..Default::default()
        },
    ))
}
fn stack(axis: Axis, gap: f32, children: Vec<c::Element>) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: axis,
        gap: ui::Length::Px(gap),
        align: if axis == Axis::Row {
            c::Alignment::Center
        } else {
            c::Alignment::Stretch
        },
        children: children.into_iter().map(Into::into).collect(),
    })
}
fn surface(fill: ui::Paint, padding: f32, radius: f32, children: Vec<c::Element>) -> c::Element {
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.fill = Some(fill);
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    style.overrides.padding = Some([ui::Length::Px(padding); 4]);
    style.overrides.radii = Some([ui::Length::Px(radius); 4]);
    let mut panel = c::Box::new(style, children.into_iter().map(Into::into).collect());
    panel.direction = Axis::Column;
    panel.align = c::Alignment::Stretch;
    panel.gap = ui::Length::Px(0.);
    c::Element::Box(panel)
}

impl Startup {
    pub fn element(&self, ctx: ui::Context) -> Result<c::Element, &'static str> {
        let active = self.active_step.min(STEPS.len() - 1);
        let error_color = ui::Color::Rgba(Color::hex(TOKENS.color.state.errored).0);
        let ink = if self.error.is_some() {
            error_color
        } else {
            ui::Color::Accent
        };
        let mut rows = Vec::new();
        for (index, label) in STEPS.iter().enumerate() {
            let current = index == active;
            let done = index < active;
            let color = if current {
                ink
            } else if done {
                ui::Color::Accent
            } else {
                ui::Color::Muted
            };
            let mut mark = surface(
                ui::Paint {
                    color,
                    alpha: ui::Alpha::Value(0.12),
                },
                0.,
                8.,
                vec![text(
                    if done {
                        "✓".into()
                    } else if current && self.error.is_some() {
                        "!".into()
                    } else {
                        (index + 1).to_string()
                    },
                    ui::TypeStep::Body,
                    color,
                    600,
                )],
            );
            if let c::Element::Box(b) = &mut mark {
                b.style.overrides.width = Some(ui::Length::Px(28.));
                b.style.overrides.height = Some(ui::Length::Px(28.));
                b.align = c::Alignment::Center;
            }
            let status = if done {
                "Done"
            } else if current && self.error.is_some() {
                "Failed"
            } else if current {
                "In progress"
            } else {
                "Pending"
            };
            let header = stack(
                Axis::Row,
                12.,
                vec![
                    mark,
                    text(
                        *label,
                        ui::TypeStep::Body,
                        if current || done {
                            ui::Color::Foreground
                        } else {
                            ui::Color::Muted
                        },
                        if current { 600 } else { 400 },
                    ),
                    c::Element::Spacer,
                    text(
                        status,
                        ui::TypeStep::Caption,
                        color,
                        if current { 600 } else { 400 },
                    ),
                ],
            );
            let mut contents = vec![header];
            if index == 2
                && let Some(history) = &self.history
            {
                let mut progress = vec![
                    text(
                        format!("{} of {} records", history.verified, history.total),
                        ui::TypeStep::Title,
                        ui::Color::Foreground,
                        600,
                    ),
                    text(
                        format!("Stream · {}", history.stream),
                        ui::TypeStep::Caption,
                        ui::Color::Muted,
                        400,
                    ),
                ];
                if history.total > 0 {
                    let fraction =
                        (history.verified as f64 / history.total as f64).clamp(0., 1.) as f32;
                    let mut track = surface(ui::Paint::solid(ui::Color::Surface2), 0., 2., vec![]);
                    if let c::Element::Box(b) = &mut track {
                        b.direction = Axis::Row;
                        b.style.overrides.height = Some(ui::Length::Px(4.));
                        for (share, color) in
                            [(fraction, ink), (1. - fraction, ui::Color::Transparent)]
                        {
                            if share > 0. {
                                b.children.push(c::Item {
                                    element: surface(ui::Paint::solid(color), 0., 2., vec![]),
                                    extent: c::Extent::Flex(share),
                                });
                            }
                        }
                    }
                    progress.push(track);
                }
                contents.push(stack(Axis::Column, 8., progress));
            }
            rows.push(surface(
                ui::Paint {
                    color: if current { ink } else { ui::Color::Transparent },
                    alpha: ui::Alpha::Value(0.06),
                },
                8.,
                12.,
                vec![stack(Axis::Column, 12., contents)],
            ));
        }
        let header = stack(
            Axis::Column,
            8.,
            vec![
                text(&self.title, ui::TypeStep::Title, ui::Color::Foreground, 600),
                text(&self.description, ui::TypeStep::Body, ui::Color::Muted, 400),
            ],
        );
        let mut body = vec![header, stack(Axis::Column, 4., rows)];
        for [label, value] in &self.details {
            body.push(stack(
                Axis::Column,
                4.,
                vec![
                    text(label, ui::TypeStep::Caption, ui::Color::Muted, 400),
                    text(value, ui::TypeStep::Body, ui::Color::Foreground, 400),
                ],
            ));
        }
        if let Some(error) = &self.error {
            body.push(surface(
                ui::Paint {
                    color: error_color,
                    alpha: ui::Alpha::Value(0.06),
                },
                12.,
                8.,
                vec![text(error, ui::TypeStep::Body, ui::Color::Foreground, 400)],
            ));
        }
        if !self.logs.is_empty() {
            let mut logs = vec![];
            for [label, path] in &self.logs {
                logs.push(stack(
                    Axis::Column,
                    4.,
                    vec![
                        text(label, ui::TypeStep::Caption, ui::Color::Muted, 600),
                        text(path, ui::TypeStep::Caption, ui::Color::Muted, 400),
                    ],
                ));
            }
            body.push(stack(Axis::Column, 8., logs));
        }
        let button = |label: &str, kind, name| -> Result<c::Element, &'static str> {
            Ok(super::seat_row::named(
                name,
                ui::widgets::Button {
                    label: label.into(),
                    kind,
                    state: ui::widgets::ButtonState::Idle,
                    icon: None,
                    key: None,
                }
                .element(ctx)?,
            ))
        };
        let mut footer = vec![
            text(
                format!("{} s on this step", self.elapsed_secs),
                ui::TypeStep::Caption,
                ui::Color::Muted,
                400,
            ),
            c::Element::Spacer,
            button("Quit", ui::widgets::ButtonKind::Ghost, "startup/quit")?,
        ];
        if self.error.is_some() {
            footer.push(button(
                "Retry",
                ui::widgets::ButtonKind::Primary,
                "startup/retry",
            )?);
        }
        body.push(stack(Axis::Row, 8., footer));
        let mut panel = c::Box::new(
            ui::Style::new(ui::Preset::Modal),
            vec![
                stack(
                    Axis::Column,
                    if self.error.is_some() { 8. } else { 12. },
                    body,
                )
                .into(),
            ],
        );
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Stretch;
        panel.style.overrides.padding = Some([ui::Length::Px(16.); 4]);
        panel.style.overrides.radii = Some([ui::Length::Px(STYLE.radius.panel as f32); 4]);
        Ok(c::Element::Box(panel))
    }
}
