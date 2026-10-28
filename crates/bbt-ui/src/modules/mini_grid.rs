// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A split-tree thumbnail composed from flat boxes and text. The component owns
//! no renderer or device; its caller supplies the external size and focus key.
use crate::{self as ui, components as c, modules::seat_row::named};
use bbt_ui_model::{Activity, TOKENS, draw::Color as Rgba, layout::Axis};

#[derive(Debug, Clone)]
pub enum Node {
    Cell {
        key: String,
        name: String,
        activity: Activity,
    },
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<Node>,
        b: Box<Node>,
    },
}
impl Node {
    fn class(&self) -> &str {
        if matches!(self, Self::Cell { .. }) {
            "cell"
        } else {
            "mg"
        }
    }
    fn minimum(&self) -> [f32; 2] {
        match self {
            Self::Cell { .. } => [2.; 2],
            // Every nested .mg is an overflow-hidden flex item. Its automatic
            // minimum is zero; the 2px minimum applies only to cell children.
            Self::Split { .. } => [0.; 2],
        }
    }
}
#[derive(Debug, Clone)]
pub struct MiniGrid {
    pub tree: Node,
    pub focused_key: Option<String>,
    pub size: [f32; 2],
    pub big: bool,
}
fn flat(size: [f32; 2], fill: ui::Color, radius: f32) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.width = Some(ui::Length::Px(size[0]));
    s.overrides.height = Some(ui::Length::Px(size[1]));
    s.overrides.padding = Some([ui::Length::Px(0.); 4]);
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.fill = Some(ui::Paint::solid(fill));
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s
}
impl MiniGrid {
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        self.element_named(ctx, "grid")
    }
    pub fn element_named(&self, ctx: ui::Context, path: &str) -> c::Element {
        let content = self.node(
            &self.tree,
            self.size,
            true,
            ctx,
            &if matches!(self.tree, Node::Cell { .. }) {
                format!("{path}/cell")
            } else {
                path.into()
            },
        );
        if matches!(self.tree, Node::Cell { .. }) {
            let mut root = c::Box::new(
                flat(self.size, ui::Color::Line, TOKENS.radius.grid as f32),
                vec![content.into()],
            );
            root.gap = ui::Length::Px(0.);
            root.clip = true;
            // A single cell still has the root .mg wrapper.
            named(path, c::Element::Box(root))
        } else {
            content
        }
    }
    fn node(
        &self,
        node: &Node,
        size: [f32; 2],
        root: bool,
        ctx: ui::Context,
        path: &str,
    ) -> c::Element {
        match node {
            Node::Cell {
                key,
                name,
                activity,
            } => {
                let surface = Rgba::hex(ctx.theme.palette().surface_2);
                // CSS state rules follow .focus and therefore win when both apply.
                let fill = match activity {
                    Activity::Waiting(_) => {
                        ui::Color::Rgba(Rgba::hex(TOKENS.color.state.waiting).mix(surface, 0.25).0)
                    }
                    Activity::Errored => {
                        ui::Color::Rgba(Rgba::hex(TOKENS.color.state.errored).mix(surface, 0.25).0)
                    }
                    _ if self.focused_key.as_deref() == Some(key) => ui::Color::Accent,
                    _ => ui::Color::Surface2,
                };
                let mut children = vec![];
                if self.big {
                    let mut text = c::Text::new(
                        name.split_whitespace().next().unwrap_or(""),
                        ui::Text {
                            step: ui::TypeStep::Caption,
                            line_height: 1.4,
                            ..Default::default()
                        },
                    );
                    text.wrap = false;
                    children = vec![
                        c::Element::Spacer.into(),
                        named(format!("{path}/text"), c::Element::Text(text)).into(),
                        c::Element::Spacer.into(),
                    ];
                }
                let mut cell = c::Box::new(flat(size.map(|v| v.max(2.)), fill, 0.), children);
                cell.gap = ui::Length::Px(0.);
                named(path, c::Element::Box(cell))
            }
            Node::Split { axis, ratio, a, b } => {
                let gap = if root && self.big { 3. } else { 1. };
                let i = usize::from(*axis == Axis::Column);
                let available = (size[i] - gap).max(0.);
                let min_a = a.minimum()[i];
                let min_b = b.minimum()[i];
                let first = if available >= min_a + min_b {
                    (available * ratio).clamp(min_a, available - min_b)
                } else {
                    min_a
                };
                let second = if available >= min_a + min_b {
                    available - first
                } else {
                    min_b
                };
                let mut a_size = size;
                let mut b_size = size;
                a_size[i] = first;
                b_size[i] = second;
                let a_path = format!("{path}/{}", a.class());
                let b_path = format!(
                    "{path}/{}{}",
                    b.class(),
                    if a.class() == b.class() { "1" } else { "" }
                );
                let mut panel = c::Box::new(
                    flat(
                        size,
                        ui::Color::Line,
                        if root { TOKENS.radius.grid as f32 } else { 0. },
                    ),
                    vec![
                        c::Item {
                            element: self.node(a, a_size, false, ctx, &a_path),
                            extent: c::Extent::Fixed(first),
                        },
                        c::Item {
                            element: self.node(b, b_size, false, ctx, &b_path),
                            extent: c::Extent::Fixed(second),
                        },
                    ],
                );
                panel.direction = *axis;
                panel.gap = ui::Length::Px(gap);
                panel.align = c::Alignment::Start;
                panel.cross_overflow = true;
                panel.clip = true;
                named(path, c::Element::Box(panel))
            }
        }
    }
}
