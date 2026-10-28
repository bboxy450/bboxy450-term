// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Placement preview composed with the accepted MiniGrid and L6 elements.
use super::{
    mini_grid::{MiniGrid, Node},
    seat_row::named,
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, layout::Axis};

#[derive(Debug, Clone)]
pub struct Banner {
    pub tree: Node,
    pub focused_key: String,
    pub title: String,
    pub anchor: String,
    pub anchor_color: [f32; 4],
}
impl Banner {
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let mut grid = MiniGrid {
            tree: self.tree.clone(),
            focused_key: Some(self.focused_key.clone()),
            size: [168., 104.],
            big: true,
        }
        .element_named(ctx, "banner/mg");
        fn preview(e: &mut c::Element, focus: bool, accent: [f32; 4]) {
            match e {
                c::Element::Named { name, content } => {
                    let cell = name.rsplit('/').next().unwrap().starts_with("cell");
                    let mut focused = focus;
                    if let c::Element::Box(b) = content.as_mut() {
                        if cell {
                            focused =
                                b.style.overrides.fill == Some(ui::Paint::solid(ui::Color::Accent));
                            if focused {
                                b.style.overrides.opacity = Some(0.55);
                                b.style.overrides.shadow = Some(ui::Shadow::Custom {
                                    blur: 18.,
                                    color: [accent[0], accent[1], accent[2], 0.7],
                                });
                            } else {
                                b.style.overrides.fill = Some(ui::Paint {
                                    color: ui::Color::Foreground,
                                    alpha: ui::Alpha::Value(0.14),
                                });
                            }
                        } else {
                            b.style.overrides.fill = Some(ui::Paint {
                                color: ui::Color::Line,
                                alpha: ui::Alpha::Hairline,
                            });
                            if name == "banner/mg" {
                                b.style.overrides.radii =
                                    Some([ui::Length::Px(STYLE.radius.tile as f32); 4]);
                            }
                        }
                    }
                    preview(content, focused, accent);
                }
                c::Element::Box(b) => {
                    for child in &mut b.children {
                        preview(&mut child.element, focus, accent)
                    }
                }
                c::Element::Text(t) => {
                    for run in &mut t.runs {
                        run.style.paint = ui::Paint::solid(if focus {
                            ui::Color::OnAccent
                        } else {
                            ui::Color::Muted
                        });
                        run.style.weight = if focus { 600 } else { 400 };
                    }
                }
                _ => {}
            }
        }
        preview(
            &mut grid,
            false,
            ui::Paint::solid(ui::Color::Accent)
                .resolve(ctx)
                .expect("palette accent"),
        );
        let mut title = c::Text::new(
            &self.title,
            ui::Text {
                size: Some(26.),
                weight: 600,
                tracking_em: -0.01,
                line_height: 1.1,
                ..Default::default()
            },
        );
        title.wrap = true;
        let mut caption = c::Text::new(
            "of ",
            ui::Text {
                size: Some(STYLE.r#type.subtitle as f32),
                paint: ui::Paint::solid(ui::Color::Muted),
                line_height: 1.4,
                ..Default::default()
            },
        );
        caption.wrap = false;
        caption.runs.push(c::TextRun {
            value: self.anchor.clone(),
            style: ui::Text {
                size: Some(STYLE.r#type.subtitle as f32),
                weight: 600,
                paint: ui::Paint::solid(ui::Color::Rgba(self.anchor_color)),
                line_height: 1.4,
                ..Default::default()
            },
        });
        caption.runs.push(c::TextRun {
            value: " · the new seat lands in the lit cell".into(),
            style: ui::Text {
                size: Some(STYLE.r#type.subtitle as f32),
                paint: ui::Paint::solid(ui::Color::Muted),
                line_height: 1.4,
                ..Default::default()
            },
        });
        let mut line = c::Box::new(
            plain(None, [6., 0., 0., 0.], 0.),
            vec![named("banner/txt/of", c::Element::Text(caption)).into()],
        );
        line.direction = Axis::Column;
        line.align = c::Alignment::Stretch;
        line.gap = ui::Length::Px(0.);
        let words = named(
            "banner/txt",
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Stretch,
                children: vec![
                    named("banner/txt/big", c::Element::Text(title)).into(),
                    c::Element::Box(line).into(),
                ],
            }),
        );
        let mut style = ui::Style::new(ui::Preset::Modal);
        style.overrides.padding = Some([
            ui::Length::Px(16.),
            ui::Length::Px(22.),
            ui::Length::Px(16.),
            ui::Length::Px(22.),
        ]);
        let mut panel = c::Box::new(style, vec![grid.into(), words.into()]);
        panel.gap = ui::Length::Px(22.);
        named("banner", c::Element::Box(panel))
    }
}

/// The same destination preview for a new tab, using the actual swarm's seats.
#[derive(Debug, Clone)]
pub struct TabBanner {
    pub swarm: String,
    pub seats: Vec<(String, [f32; 4])>,
}
impl TabBanner {
    pub fn element(&self, _ctx: ui::Context) -> c::Element {
        use super::strip::text;
        let mut mini_style = ui::Style::new(ui::Preset::Card);
        mini_style.overrides.width = Some(ui::Length::Px(202.));
        mini_style.overrides.padding = Some([10., 4., 10., 4.].map(ui::Length::Px));
        let mut rows = vec![
            text(
                self.swarm.to_uppercase(),
                9.,
                600,
                false,
                ui::Color::Foreground,
                1.,
                1.4,
            )
            .into(),
        ];
        for (name, color) in &self.seats {
            rows.push(text(name, 11., 500, false, ui::Color::Rgba(*color), 1., 1.4).into());
        }
        let mut active = c::Box::new(
            plain(None, [4., 8., 4., 8.], 4.),
            vec![
                text(
                    "+ new seat · here",
                    11.,
                    500,
                    false,
                    ui::Color::OnAccent,
                    1.,
                    1.4,
                )
                .into(),
            ],
        );
        active.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Accent));
        rows.push(c::Element::Box(active).into());
        let mut mini = c::Box::new(mini_style, rows);
        mini.direction = Axis::Column;
        mini.gap = ui::Length::Px(10.);
        mini.align = c::Alignment::Stretch;
        let mut words = c::Box::new(
            plain(None, [0.; 4], 0.),
            vec![
                text("New tab", 26., 600, false, ui::Color::Foreground, 1., 1.1).into(),
                text(
                    format!("in {} · the new seat gets a tab of its own", self.swarm),
                    15.,
                    400,
                    false,
                    ui::Color::Muted,
                    1.,
                    1.4,
                )
                .into(),
            ],
        );
        words.direction = Axis::Column;
        words.gap = ui::Length::Px(6.);
        words.align = c::Alignment::Stretch;
        let mut panel = c::Box::new(
            ui::Style::new(ui::Preset::Modal),
            vec![
                named("banner/sidebar", c::Element::Box(mini)).into(),
                c::Element::Box(words).into(),
            ],
        );
        panel.style.overrides.padding = Some([16., 22., 16., 22.].map(ui::Length::Px));
        panel.gap = ui::Length::Px(22.);
        named("banner", c::Element::Box(panel))
    }
}
