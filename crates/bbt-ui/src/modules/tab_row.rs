// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Tab header and optional seat body. Single-seat tabs reduce to a SeatRow.
use crate::{
    self as ui, components as c,
    modules::{
        mini_grid::{MiniGrid, Node},
        seat_row::{SeatRow, named},
    },
};
use bbt_ui_model::{Activity, TOKENS, layout::Axis};
#[derive(Debug, Clone)]
pub struct TabSeat {
    pub row: SeatRow,
    pub children: Vec<c::Element>,
}
#[derive(Debug, Clone)]
pub struct TabRow {
    pub name: String,
    pub shortcut: Option<String>,
    pub tree: Node,
    pub seats: Vec<TabSeat>,
    pub expanded: bool,
    pub focused_key: Option<String>,
}
fn style(h: Option<f32>, padding: [f32; 4]) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.height = h.map(ui::Length::Px);
    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s.overrides.radii = Some([ui::Length::Px(0.); 4]);
    s.overrides.padding = Some(padding.map(ui::Length::Px));
    s
}
fn label(value: &str, size: f32, mono: bool, color: ui::Color) -> c::Element {
    let mut t = c::Text::new(
        value,
        ui::Text {
            size: Some(size),
            mono,
            line_height: 1.4,
            paint: ui::Paint::solid(color),
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    c::Element::Text(t)
}
fn box_row(s: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(s, children);
    b.gap = ui::Length::Px(gap);
    c::Element::Box(b)
}
fn inset(element: c::Element, margin: f32) -> c::Element {
    if margin == 0. {
        element
    } else {
        let mut b = c::Box::new(style(None, [0., margin, 0., margin]), vec![element.into()]);
        b.direction = Axis::Column;
        b.gap = ui::Length::Px(0.);
        b.align = c::Alignment::Stretch;
        c::Element::Box(b)
    }
}
impl TabRow {
    fn seat_body(
        &self,
        index: usize,
        shortcut: bool,
        ctx: ui::Context,
        margin: f32,
    ) -> Vec<c::Item> {
        let source = &self.seats[index];
        let mut row = source.row.clone();
        row.focused = self.focused_key.as_deref() == Some(&row.key);
        row.shortcut = if shortcut {
            self.shortcut.clone()
        } else {
            None
        };
        let prefix = format!("seat{index}/");
        let mut head = if margin > 0. {
            row.element_in_swarm(ctx)
        } else {
            row.element(ctx)
        };
        c::namespace(&mut head, &prefix);
        let mut out = vec![inset(head, margin).into()];
        for (i, child) in source.children.iter().enumerate() {
            let mut child = child.clone();
            c::namespace(&mut child, &format!("seat{index}/child{i}/"));
            out.push(child.into());
        }
        out
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        self.element_with_margin(ctx, 0.)
    }
    pub fn element_in_swarm(&self, ctx: ui::Context) -> c::Element {
        self.element_with_margin(
            ctx,
            ui::Style::new(ui::Preset::CardRow)
                .resolve(ctx)
                .expect("card row Style")
                .margin[3],
        )
    }
    fn element_with_margin(&self, ctx: ui::Context, margin: f32) -> c::Element {
        let d = ctx.density.tokens();
        if self.seats.len() == 1 {
            return named(
                "tab",
                c::Element::Stack(c::Stack {
                    direction: Axis::Column,
                    gap: ui::Length::Px(0.),
                    align: c::Alignment::Stretch,
                    children: self.seat_body(0, true, ctx, margin),
                }),
            );
        }
        let mut children = vec![c::Item {
            element: named(
                "header/tname",
                label(
                    &self.name,
                    d.font_small as f32,
                    false,
                    ui::Color::Foreground,
                ),
            ),
            extent: c::Extent::Content {
                grow: 0.,
                shrink: 1.,
                min: 0.,
                max: None,
                max_fraction: None,
            },
        }];
        if !self.expanded {
            let mut seats = self.seats.iter().collect::<Vec<_>>();
            seats.sort_by_key(|s| match s.row.activity {
                Activity::Errored => 0,
                Activity::Waiting(_) => 1,
                Activity::Active => 2,
                Activity::Idle => 3,
            });
            let count = TOKENS.sidebar.collapse_glyphs as usize;
            let mut glyphs = vec![];
            for (i, s) in seats.iter().take(count).enumerate() {
                glyphs.push(
                    named(
                        format!(
                            "header/roll/glyph{}",
                            if i == 0 { String::new() } else { i.to_string() }
                        ),
                        c::Element::Glyph {
                            kind_colors: Default::default(),
                            shadow: None,
                            activity: s.row.activity,
                            grade: s.row.grade,
                            color: ui::Paint::solid(s.row.color(ctx)),
                            size: d.glyph as f32,
                            scale: 1.,
                        },
                    )
                    .into(),
                );
            }
            if self.seats.len() > count {
                glyphs.push(
                    named(
                        "header/roll/ellip",
                        c::Element::Text({
                            let mut text = c::Text::new(
                                "…",
                                ui::Text {
                                    size: Some(d.glyph as f32),
                                    line_height: 1.,
                                    paint: ui::Paint::solid(ui::Color::Muted),
                                    ..Default::default()
                                },
                            );
                            text.wrap = false;
                            text
                        }),
                    )
                    .into(),
                );
            }
            let roll = named("header/roll", box_row(style(None, [0.; 4]), 2., glyphs));
            children.push(box_row(style(None, [0., 0., 0., 4.]), 0., vec![roll.into()]).into());
        }
        let mut right = vec![];
        if let Some(shortcut) = &self.shortcut {
            right.push(
                named(
                    "header/right/kbd",
                    super::seat_row::keyboard_badge(shortcut, false, false, 400),
                )
                .into(),
            );
        }
        right.push(
            MiniGrid {
                tree: self.tree.clone(),
                focused_key: self.focused_key.clone(),
                size: [
                    TOKENS.sidebar.minigrid_w as f32,
                    TOKENS.sidebar.minigrid_h as f32,
                ],
                big: false,
            }
            .element_named(ctx, "header/right/mg")
            .into(),
        );
        let right = named(
            "header/right",
            box_row(style(None, [0.; 4]), d.gap as f32, right),
        );
        let pushed = c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(0.),
            align: c::Alignment::Center,
            children: vec![c::Element::Spacer.into(), right.into()],
        });
        children.push(c::Item {
            element: pushed,
            extent: c::Extent::Content {
                grow: 1.,
                shrink: 0.,
                min: 0.,
                max: None,
                max_fraction: None,
            },
        });
        let mut header_style = if margin > 0. {
            let mut style = ui::Style::new(ui::Preset::CardRow);
            style.overrides.height = Some(ui::Length::Px(d.tab_h as f32));
            style
        } else {
            let mut s = style(
                Some(d.tab_h as f32),
                [0., d.pad_x as f32, 0., d.pad_x as f32],
            );
            // The row radius, as a seat row's (--r-row): invisible on a transparent row until
            // the rim follows its corners.
            s.overrides.radii = Some([ui::Length::Px(TOKENS.radius.chip as f32); 4]);
            s
        };
        // The tab's alive rim (rim-glow §8) reads its worst seat, state and colour, and swells
        // when one of its seats does; edge to edge like a row, bare while idle.
        if let Some(worst) = self.seats.iter().min_by_key(|s| match s.row.activity {
            Activity::Errored => 0,
            Activity::Waiting(_) => 1,
            Activity::Active => 2,
            Activity::Idle => 3,
        }) {
            header_style.overrides.rim = Some(ui::Rim {
                swell: self.seats.iter().map(|s| s.row.swell).fold(0., f32::max),
                attention: self.seats.iter().any(|s| s.row.attention),
                ..ui::Rim::block(
                    worst.row.activity.into(),
                    false,
                    match worst.row.color(ctx) {
                        ui::Color::Rgba(c) => Some(c),
                        _ => None,
                    },
                    ui::rim_phase(&self.name),
                    true,
                    0.,
                    0.,
                )
            });
        }
        let header = named("header", box_row(header_style, d.gap as f32, children));
        let mut all = vec![inset(header, margin).into()];
        if self.expanded {
            let mut seats = vec![];
            for i in 0..self.seats.len() {
                seats.extend(self.seat_body(i, false, ctx, margin));
            }
            let mut body = c::Box::new(style(None, [0., 0., 0., d.pad_x as f32 + 2.]), seats);
            body.direction = Axis::Column;
            body.gap = ui::Length::Px(0.);
            body.align = c::Alignment::Stretch;
            all.push(named("body", c::Element::Box(body)).into());
        }
        named(
            "tab",
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Stretch,
                children: all,
            }),
        )
    }
}
