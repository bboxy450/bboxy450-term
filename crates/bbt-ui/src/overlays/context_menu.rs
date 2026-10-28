// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained context menus. Bodies are existing components; actions are host-owned IDs.
pub use super::Outcome;
use crate::{self as ui, components as c, cursor::Cursor};
use bbt_ui_model::{
    STYLE,
    layout::{Axis, Rect},
};
const PREFIX: &str = "context-menu/item/";

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub body: c::Element,
    pub enabled: bool,
}
impl Item {
    pub fn new(id: impl Into<String>, body: c::Element) -> Self {
        Self {
            id: id.into(),
            body,
            enabled: true,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Menu {
    pub items: Vec<Item>,
    pub selected: Option<usize>,
    pub width: f32,
}
impl Menu {
    pub fn new(items: Vec<Item>) -> Self {
        let selected = items.iter().position(|item| item.enabled);
        Self {
            items,
            selected,
            width: 240.,
        }
    }
    fn rows(&self) -> c::Element {
        let mut items = vec![];
        for (index, item) in self.items.iter().enumerate() {
            let mut style = transparent();
            style.overrides.padding = Some([ui::Length::Spacing(3); 4]);
            style.overrides.radii = Some([ui::Length::Px(STYLE.radius.row as f32); 4]);
            if item.enabled && self.selected == Some(index) {
                style.overrides.fill = Some(ui::Paint {
                    color: ui::Color::Accent,
                    alpha: ui::Alpha::Value(STYLE.tint.selection_alpha as f32),
                });
            }
            if !item.enabled {
                style.overrides.opacity = Some(0.45);
            }
            let mut row = c::Box::new(style, vec![item.body.clone().into()]);
            row.direction = Axis::Column;
            row.align = c::Alignment::Stretch;
            row.gap = ui::Length::Px(0.);
            items.push(
                c::Element::Named {
                    name: format!("{PREFIX}{index}"),
                    content: Box::new(c::Element::Box(row)),
                }
                .into(),
            );
        }
        let mut rows = c::Box::new(transparent(), items);
        rows.direction = Axis::Column;
        rows.align = c::Alignment::Stretch;
        rows.gap = ui::Length::Px(0.);
        c::Element::Box(rows)
    }
    /// Clamp the panel inside the viewport and reveal keyboard selection in tall menus.
    /// Coordinates stay absolute; retained regions exclude clipped children.
    pub fn layout(
        &self,
        anchor: [f32; 2],
        viewport: Rect,
        ctx: ui::Context,
        measure: &dyn c::Measure,
    ) -> ui::style::Result<c::Layout> {
        if ![
            anchor[0], anchor[1], viewport.x, viewport.y, viewport.w, viewport.h, self.width,
        ]
        .iter()
        .all(|n| n.is_finite())
            || viewport.w <= 0.
            || viewport.h <= 0.
            || self.width <= 0.
        {
            return Err("invalid context menu bounds");
        }
        let width = self.width.min(viewport.w);
        let padding = (STYLE.spacing.s2 as f32)
            .min(width / 2.)
            .min(viewport.h / 2.);
        let inner = (width - 2. * padding).max(0.);
        let rows = self.rows();
        let solved = rows.layout(Rect::new(0., 0., inner, 0.), ctx, measure)?;
        let height = (solved.rect.h + 2. * padding).min(viewport.h);
        let content_height = (height - 2. * padding).max(0.);
        let offset = self
            .selected
            .and_then(|selected| {
                super::named_regions(&solved)
                    .into_iter()
                    .find(|(key, _)| key == &format!("{PREFIX}{selected}"))
            })
            .map_or(0., |(_, rect)| {
                (rect.bottom() - content_height)
                    .max(0.)
                    .min((solved.rect.h - content_height).max(0.))
            });
        let mut style = ui::Style::new(ui::Preset::Modal);
        style.overrides.padding = Some([ui::Length::Px(padding); 4]);
        style.overrides.width = Some(ui::Length::Px(width));
        style.overrides.height = Some(ui::Length::Px(height));
        let scroll = c::Element::ScrollView(c::ScrollView {
            viewport: [inner, content_height],
            offset: [0., offset],
            content_width: Some(inner),
            content: Box::new(rows),
        });
        let mut panel = c::Box::new(style, vec![scroll.into()]);
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Stretch;
        panel.gap = ui::Length::Px(0.);
        panel.clip = true;
        c::Element::Box(panel).layout(
            Rect::new(
                anchor[0].clamp(viewport.x, (viewport.right() - width).max(viewport.x)),
                anchor[1].clamp(viewport.y, (viewport.bottom() - height).max(viewport.y)),
                width,
                0.,
            ),
            ctx,
            measure,
        )
    }
}
fn transparent() -> ui::Style {
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.padding = Some([ui::Length::Px(0.); 4]);
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    style
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Next { backwards: bool },
    First,
    Last,
    Hover(Option<usize>),
    Activate(usize),
    Submit { repeat: bool },
    Cancel,
}
#[derive(Debug, Clone)]
pub struct Controller {
    pub menu: Menu,
    open: bool,
}
impl Controller {
    pub fn new(menu: Menu) -> Self {
        Self { menu, open: true }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn handle(&mut self, command: Command) -> Outcome {
        if !self.open {
            return Outcome::Local;
        }
        match command {
            Command::Cancel => {
                self.open = false;
                Outcome::Dismiss
            }
            Command::Activate(index) => self.invoke(index),
            Command::Submit { repeat: false } => self
                .menu
                .selected
                .map_or(Outcome::Local, |index| self.invoke(index)),
            Command::Submit { repeat: true } => Outcome::Local,
            Command::Hover(index) => self
                .select(index.filter(|i| self.menu.items.get(*i).is_some_and(|item| item.enabled))),
            Command::First => self.select(self.menu.items.iter().position(|item| item.enabled)),
            Command::Last => self.select(self.menu.items.iter().rposition(|item| item.enabled)),
            Command::Next { backwards } => {
                let enabled: Vec<_> = self
                    .menu
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| item.enabled.then_some(i))
                    .collect();
                let current = enabled.iter().position(|i| Some(*i) == self.menu.selected);
                let next = if enabled.is_empty() {
                    None
                } else {
                    Some(
                        enabled[match current {
                            Some(i) if backwards => (i + enabled.len() - 1) % enabled.len(),
                            Some(i) => (i + 1) % enabled.len(),
                            None if backwards => enabled.len() - 1,
                            None => 0,
                        }],
                    )
                };
                self.select(next)
            }
        }
    }
    fn select(&mut self, index: Option<usize>) -> Outcome {
        if self.menu.selected == index {
            Outcome::Local
        } else {
            self.menu.selected = index;
            Outcome::Changed
        }
    }
    fn invoke(&mut self, index: usize) -> Outcome {
        if let Some(item) = self.menu.items.get(index).filter(|item| item.enabled) {
            let id = item.id.clone();
            self.open = false;
            Outcome::Invoke(id)
        } else {
            Outcome::Local
        }
    }
    pub fn click(&mut self, at: [f32; 2], regions: &Regions) -> Outcome {
        if !super::contains(regions.bounds, at) {
            self.handle(Command::Cancel)
        } else if let Some(index) = regions.hit(at) {
            self.handle(Command::Activate(index))
        } else {
            Outcome::Local
        }
    }
    pub fn hover(&mut self, at: [f32; 2], regions: &Regions) -> Outcome {
        self.handle(Command::Hover(regions.hit(at)))
    }
}
#[derive(Debug, Clone)]
pub struct Regions {
    pub bounds: Rect,
    rows: Vec<(usize, Rect, bool)>,
}
impl Regions {
    pub fn retained(layout: &c::Layout, menu: &Menu) -> Self {
        Self {
            bounds: layout.rect,
            rows: super::named_regions(layout)
                .into_iter()
                .filter_map(|(key, rect)| {
                    let index = key.strip_prefix(PREFIX)?.parse::<usize>().ok()?;
                    Some((index, rect, menu.items.get(index)?.enabled))
                })
                .collect(),
        }
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<usize> {
        self.rows
            .iter()
            .rev()
            .find(|(_, rect, enabled)| *enabled && super::contains(*rect, at))
            .map(|(index, _, _)| *index)
    }
    pub fn cursor(&self, at: [f32; 2]) -> Cursor {
        if self.hit(at).is_some() {
            Cursor::Pointer
        } else {
            Cursor::Default
        }
    }
}
