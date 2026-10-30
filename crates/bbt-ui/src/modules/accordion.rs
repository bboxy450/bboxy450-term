// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The swarm card's band, radius, inset and bumper for non-seat sidebar sections.
use crate::{
    self as ui, components as c,
    modules::{
        seat_row::named,
        swarm_card::{SwarmCard, card_style, plain},
    },
};
use bbt_ui_model::layout::Axis;
pub struct Accordion<'a> {
    pub key: &'a str,
    pub title: &'a str,
    pub color: [f32; 4],
    pub count: usize,
    pub open: bool,
}
impl Accordion<'_> {
    fn border(ctx: ui::Context) -> [f32; 4] {
        let s = card_style(ui::Color::Surface)
            .resolve(ctx)
            .expect("card style");
        let w = s.border.filter(|b| !b.outside).map_or(0., |b| b.width);
        s.border_sides.map(|side| if side { w } else { 0. })
    }
    pub fn chrome_height(ctx: ui::Context, open: bool) -> f32 {
        let card = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .expect("card style");
        let band = ui::Style::new(ui::Preset::Band)
            .resolve(ctx)
            .expect("band style");
        let border = Self::border(ctx);
        border[0]
            + border[2]
            + card.padding[0]
            + card.padding[2]
            + band.height.expect("band height")
            + card.bumper[usize::from(!open)]
            + if open { 3. } else { 0. }
    }
    pub fn body_size(ctx: ui::Context, size: [f32; 2], open: bool) -> [f32; 2] {
        let card = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .expect("card style");
        let border = Self::border(ctx);
        [
            (size[0] - card.padding[1] - card.padding[3] - border[1] - border[3]).max(1.),
            (size[1] - Self::chrome_height(ctx, open)).max(0.),
        ]
    }
    pub fn element(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        body: Option<c::Element>,
    ) -> c::Element {
        let chrome = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .expect("card style");
        let card = SwarmCard {
            key: self.key.into(),
            name: self.title.into(),
            color: self.color,
            seats: vec![],
            expanded: self.open,
            closed: !self.open,
            unfocused: false,
            tabs: vec![],
            body: false,
            new_seat: false,
            extra: vec![],
            shadows: vec![],
        };
        let mut head = card.header_element_with_count(ctx, self.count);
        c::namespace(&mut head, &format!("{}/", self.key));
        let mut children = vec![head.into()];
        if let Some(body) = body.filter(|_| self.open) {
            children.push(
                c::Element::Box(c::Box::new(
                    plain(Some(3.), [0.; 4], ui::Color::Transparent, 0.),
                    vec![],
                ))
                .into(),
            );
            children.push(c::Item {
                element: body,
                extent: c::Extent::Flex(1.),
            });
        }
        let mut inner = c::Box::new(
            plain(None, chrome.padding, ui::Color::Transparent, 0.),
            children,
        );
        inner.direction = Axis::Column;
        inner.align = c::Alignment::Stretch;
        inner.gap = ui::Length::Px(0.);
        let bumper = c::Element::Box(c::Box::new(
            plain(
                Some(chrome.bumper[usize::from(!self.open)]),
                [0.; 4],
                ui::Color::Rgba(self.color),
                0.,
            ),
            vec![],
        ));
        let mut style = card_style(ui::Color::Surface);
        style.overrides.height = Some(ui::Length::Px(size[1]));
        let mut frame = c::Box::new(
            style,
            vec![
                c::Item {
                    element: c::Element::Box(inner),
                    extent: c::Extent::Flex(1.),
                },
                bumper.into(),
            ],
        );
        frame.direction = Axis::Column;
        frame.align = c::Alignment::Stretch;
        frame.gap = ui::Length::Px(0.);
        frame.clip = true;
        named(self.key, c::Element::Box(frame))
    }
}
