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

/// Short, reversible disclosure motion on the caller's existing frame clock.
/// No timers, background workers, or autonomous redraws belong to the component.
/// Origin: bboxy450-term — https://github.com/bboxy450/bboxy450-term
/// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#[derive(Debug, Clone, Copy)]
pub struct Reveal {
    from: f32,
    pub target: bool,
    started: std::time::Instant,
}
impl Reveal {
    pub fn new(from: f32, target: bool, now: std::time::Instant) -> Self {
        Self {
            from: if from.is_finite() {
                from.clamp(0., 1.)
            } else {
                u8::from(target) as f32
            },
            target,
            started: now,
        }
    }
    pub fn value(self, now: std::time::Instant) -> f32 {
        let t = (now.saturating_duration_since(self.started).as_secs_f32() * 1000.
            / bbt_ui_model::TOKENS.motion.hover_ms as f32)
            .clamp(0., 1.);
        let eased = 1. - (1. - t).powi(3);
        self.from + (u8::from(self.target) as f32 - self.from) * eased
    }
    pub fn active(self, now: std::time::Instant) -> bool {
        self.value(now) != u8::from(self.target) as f32
    }
}
/// Resize the solved card's frame while keeping its body at natural row sizes.
/// The inner clip and moving bumper share the displayed bounds used for hits.
pub(crate) fn reveal_frame(element: &mut c::Element, height: f32) {
    fn clip_inner(element: &mut c::Element) {
        match element {
            c::Element::FixedHeight { content, .. } | c::Element::Named { content, .. } => {
                clip_inner(content)
            }
            c::Element::Box(frame) => frame.clip = true,
            _ => {}
        }
    }
    match element {
        c::Element::FixedHeight {
            height: h, content, ..
        } => {
            *h = height;
            reveal_frame(content, height);
        }
        c::Element::Named { content, .. } => reveal_frame(content, height),
        c::Element::Box(frame) => {
            frame.style.overrides.height = Some(ui::Length::Px(height));
            if let Some(inner) = frame.children.first_mut() {
                inner.extent = c::Extent::Flex(1.);
                clip_inner(&mut inner.element);
            }
        }
        _ => {}
    }
}
#[cfg(test)]
mod reveal_tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn accordion_reveal_reverses_without_a_jump_and_finishes_without_more_frames() {
        let now = Instant::now();
        let opening = Reveal::new(0., true, now);
        let half = now + Duration::from_millis(60);
        let at = opening.value(half);
        assert!(at > 0. && at < 1.);
        let closing = Reveal::new(at, false, half);
        assert_eq!(closing.value(half), at);
        let mut previous = at;
        for ms in 0..=120 {
            let time = half + Duration::from_millis(ms);
            let value = closing.value(time);
            assert!(value >= 0. && value <= previous);
            previous = value;
        }
        assert_eq!(previous, 0.);
        assert!(!closing.active(half + Duration::from_millis(120)));
        assert_eq!(opening.value(now - Duration::from_millis(1)), 0.);
        assert!(!Reveal::new(1., true, now).active(now));
    }
}
