// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A keyboard-navigable choice prompt; it never executes the chosen action.
use super::seat_row::named;
use crate::{
    self as ui, components as c,
    widgets::{Button, ButtonKind, ButtonState},
};
use bbt_ui_model::{STYLE, layout::Axis};
#[derive(Debug, Clone)]
pub struct OptionItem {
    pub key: String,
    pub label: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy)]
pub enum Key {
    Up,
    Down,
    Home,
    End,
    Enter,
}
#[derive(Debug, Clone)]
pub struct Approval {
    pub key: String,
    pub title: String,
    pub options: Vec<OptionItem>,
    selected: Option<usize>,
}
impl Approval {
    pub fn new(key: impl Into<String>, title: impl Into<String>, options: Vec<OptionItem>) -> Self {
        let selected = options.iter().position(|o| o.enabled);
        Self {
            key: key.into(),
            title: title.into(),
            options,
            selected,
        }
    }
    pub fn selected(&self) -> Option<usize> {
        self.selected
            .filter(|i| self.options.get(*i).is_some_and(|o| o.enabled))
    }
    /// Returns a host action only for Enter on an enabled option.
    pub fn handle(&mut self, key: Key) -> Option<String> {
        let enabled: Vec<_> = self
            .options
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.enabled.then_some(i))
            .collect();
        if enabled.is_empty() {
            self.selected = None;
            return None;
        }
        let at = enabled
            .iter()
            .position(|i| Some(*i) == self.selected)
            .unwrap_or(0);
        match key {
            Key::Enter => return self.selected().map(|i| self.options[i].key.clone()),
            Key::Home => self.selected = enabled.first().copied(),
            Key::End => self.selected = enabled.last().copied(),
            Key::Up => self.selected = Some(enabled[(at + enabled.len() - 1) % enabled.len()]),
            Key::Down => self.selected = Some(enabled[(at + 1) % enabled.len()]),
        }
        None
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let mut children = vec![
            c::Element::Text(c::Text::new(
                &self.title,
                ui::Text {
                    weight: 600,
                    ..Default::default()
                },
            ))
            .into(),
        ];
        for (i, o) in self.options.iter().enumerate() {
            children.push(
                Button {
                    label: o.label.clone(),
                    kind: ButtonKind::Ghost,
                    state: if !o.enabled {
                        ButtonState::Disabled
                    } else if self.selected() == Some(i) {
                        ButtonState::Focus
                    } else {
                        ButtonState::Idle
                    },
                    key: Some(format!("{}/{}", self.key, o.key)),
                    icon: None,
                }
                .element(ctx)
                .expect("approval button style")
                .into(),
            );
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
