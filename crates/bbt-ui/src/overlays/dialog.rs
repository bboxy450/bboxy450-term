// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Body-agnostic focus and actions for Modal. Hosts retain editors, validation,
//! effects and the previous focus to restore after dismissal.
pub use super::Outcome;
use crate::{components as c, cursor::Cursor};
use bbt_ui_model::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Field,
    Action,
}
/// IDs must be unique and name the corresponding retained Element::Named node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    pub id: String,
    pub kind: ControlKind,
    pub enabled: bool,
}
impl Control {
    pub fn new(id: impl Into<String>, kind: ControlKind) -> Self {
        Self {
            id: id.into(),
            kind,
            enabled: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Focus(String),
    Next { backwards: bool },
    Activate(String),
    Submit { composing: bool, repeat: bool },
    Cancel { composing: bool },
}
#[derive(Debug, Clone)]
pub struct Controller {
    pub controls: Vec<Control>,
    pub default_action: Option<String>,
    focused: Option<usize>,
    open: bool,
}
impl Controller {
    pub fn new(controls: Vec<Control>, default_action: Option<String>) -> Self {
        let focused = controls.iter().position(|control| control.enabled);
        Self {
            controls,
            default_action,
            focused,
            open: true,
        }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn focused(&self) -> Option<&str> {
        self.focused
            .and_then(|i| self.controls.get(i))
            .filter(|c| c.enabled)
            .map(|c| c.id.as_str())
    }
    pub fn handle(&mut self, command: Command) -> Outcome {
        if !self.open {
            return Outcome::Local;
        }
        match command {
            Command::Cancel { composing: false } => {
                self.open = false;
                Outcome::Dismiss
            }
            Command::Cancel { composing: true }
            | Command::Submit {
                composing: true, ..
            }
            | Command::Submit { repeat: true, .. } => Outcome::Local,
            Command::Focus(id) => self
                .controls
                .iter()
                .position(|c| c.enabled && c.id == id)
                .map_or(Outcome::Local, |index| self.focus(index)),
            Command::Activate(id) => {
                if let Some(index) = self.controls.iter().position(|c| c.enabled && c.id == id) {
                    if self.controls[index].kind == ControlKind::Action {
                        Outcome::Invoke(id)
                    } else {
                        self.focus(index)
                    }
                } else {
                    Outcome::Local
                }
            }
            Command::Submit {
                composing: false,
                repeat: false,
            } => {
                let focused = self
                    .focused
                    .and_then(|i| self.controls.get(i))
                    .filter(|c| c.enabled && c.kind == ControlKind::Action);
                let action = focused.or_else(|| {
                    self.default_action.as_ref().and_then(|id| {
                        self.controls
                            .iter()
                            .find(|c| c.enabled && c.kind == ControlKind::Action && &c.id == id)
                    })
                });
                action.map_or(Outcome::Local, |c| Outcome::Invoke(c.id.clone()))
            }
            Command::Next { backwards } => {
                let enabled: Vec<_> = self
                    .controls
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.enabled.then_some(i))
                    .collect();
                let current = enabled.iter().position(|i| Some(*i) == self.focused);
                if enabled.is_empty() {
                    self.focused = None;
                    Outcome::Local
                } else {
                    let next = match current {
                        Some(i) if backwards => (i + enabled.len() - 1) % enabled.len(),
                        Some(i) => (i + 1) % enabled.len(),
                        None if backwards => enabled.len() - 1,
                        None => 0,
                    };
                    self.focus(enabled[next])
                }
            }
        }
    }
    fn focus(&mut self, index: usize) -> Outcome {
        if self.focused == Some(index) {
            Outcome::Local
        } else {
            self.focused = Some(index);
            Outcome::Changed
        }
    }
    /// Inert dialog body consumes the click. Outside cancels; no click reaches
    /// underlying application content. Hosts cancel field preedit on focus loss.
    pub fn click(&mut self, at: [f32; 2], regions: &Regions) -> Outcome {
        if !super::contains(regions.bounds, at) {
            self.handle(Command::Cancel { composing: false })
        } else {
            regions.hit(at).map_or(Outcome::Local, |id| {
                self.handle(Command::Activate(id.into()))
            })
        }
    }
}
#[derive(Debug, Clone)]
pub struct Regions {
    pub bounds: Rect,
    controls: Vec<(Control, Rect)>,
}
impl Regions {
    /// Pass the panel subtree, not the window-sized backdrop. Arbitrary nested
    /// components remain intact; only configured named nodes become controls.
    pub fn retained(panel: &c::Layout, controls: &[Control]) -> Self {
        Self {
            bounds: panel.rect,
            controls: super::named_regions(panel)
                .into_iter()
                .filter_map(|(name, rect)| {
                    controls
                        .iter()
                        .find(|control| control.id == name)
                        .map(|control| (control.clone(), rect))
                })
                .collect(),
        }
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<&str> {
        self.controls
            .iter()
            .rev()
            .find(|(control, rect)| control.enabled && super::contains(*rect, at))
            .map(|(control, _)| control.id.as_str())
    }
    pub fn cursor(&self, at: [f32; 2]) -> Cursor {
        self.controls
            .iter()
            .rev()
            .find(|(control, rect)| control.enabled && super::contains(*rect, at))
            .map_or(Cursor::Default, |(control, _)| match control.kind {
                ControlKind::Field => Cursor::Text,
                ControlKind::Action => Cursor::Pointer,
            })
    }
}
