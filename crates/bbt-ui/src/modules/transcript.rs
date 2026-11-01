// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Generic prepared message bodies. A host can use bbt-doc for Markdown,
//! without a UI -> document dependency or parsing work in the frame loop.
use super::seat_row::named;
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, layout::Axis};

#[derive(Debug, Clone)]
pub struct Message {
    pub key: String,
    pub title: String,
    pub body: c::Element,
}
pub struct Transcript<'a> {
    pub messages: &'a [Message],
    pub scroll: &'a ScrollStick,
}
impl Transcript<'_> {
    pub fn element(&self, size: [f32; 2]) -> c::Element {
        let children = self
            .messages
            .iter()
            .map(|m| {
                let title = c::Element::Text(c::Text::new(
                    &m.title,
                    ui::Text {
                        weight: 600,
                        paint: ui::Paint::solid(ui::Color::Muted),
                        ..Default::default()
                    },
                ));
                named(
                    &m.key,
                    c::Element::Stack(c::Stack {
                        direction: Axis::Column,
                        gap: ui::Length::Px(STYLE.spacing.s3 as f32),
                        align: c::Alignment::Stretch,
                        children: vec![title.into(), m.body.clone().into()],
                    }),
                )
                .into()
            })
            .collect();
        c::Element::ScrollView(c::ScrollView {
            viewport: size,
            offset: [0., self.scroll.layout_offset()],
            content_width: None,
            content: Box::new(c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(STYLE.spacing.s6 as f32),
                align: c::Alignment::Stretch,
                children,
            })),
        })
    }
}
/// Same requested-offset discipline as the app's chat_scroll::Position:
/// layout clamps a projection, never the user's retained intent.
#[derive(Debug, Clone, Copy)]
pub struct ScrollStick {
    requested: f32,
    following: bool,
}
impl Default for ScrollStick {
    fn default() -> Self {
        Self {
            requested: 0.,
            following: true,
        }
    }
}
impl ScrollStick {
    pub fn following(&self) -> bool {
        self.following
    }
    pub fn layout_offset(&self) -> f32 {
        if self.following {
            f32::MAX
        } else {
            self.requested
        }
    }
    pub fn user_scroll(&mut self, offset: f32, maximum: f32) {
        if offset.is_finite() && maximum.is_finite() {
            self.requested = offset.max(0.);
            self.following = offset >= maximum.max(0.);
        }
    }
    pub fn jump_to_latest(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Apply {
    Applied,
    Duplicate,
    Gap { expected: u64 },
    Retired,
}
/// One current view; generation tokens must come from this retained controller,
/// never from a recyclable display name. A gap is explicit and can be retried or
/// repaired by a snapshot; it is never silently applied out of order.
#[derive(Debug)]
pub struct StreamToView<T> {
    generation: u64,
    next: u64,
    value: Option<T>,
}
impl<T> Default for StreamToView<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            next: 0,
            value: None,
        }
    }
}
impl<T> StreamToView<T> {
    pub fn open(&mut self, value: T) -> u64 {
        self.retire();
        self.value = Some(value);
        self.generation
    }
    pub fn retire(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("view generation exhausted");
        self.next = 0;
        self.value = None;
    }
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
    pub fn apply(&mut self, generation: u64, sequence: u64, f: impl FnOnce(&mut T)) -> Apply {
        if generation != self.generation || self.value.is_none() {
            return Apply::Retired;
        }
        if sequence < self.next {
            return Apply::Duplicate;
        }
        if sequence > self.next {
            return Apply::Gap {
                expected: self.next,
            };
        }
        let Some(next) = self.next.checked_add(1) else {
            return Apply::Retired;
        };
        f(self.value.as_mut().expect("current view"));
        self.next = next;
        Apply::Applied
    }
}
