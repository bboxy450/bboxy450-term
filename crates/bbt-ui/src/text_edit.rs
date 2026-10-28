// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure UTF-16 text editing shared by retained text fields. Hosts own focus,
//! key translation, IME candidate placement and all effects.
use crate::components::{Edit, Preedit, utf16_byte_index};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Insert(String),
    Commit(String),
    Delete { backwards: bool },
    Move { motion: Motion, extend: bool },
    SelectAll,
    SetCaret { at: usize, extend: bool },
    Preedit { text: String, caret: Option<usize> },
    CancelPreedit,
}

#[derive(Debug, Clone)]
struct Composition {
    range: [usize; 2],
    text: String,
    caret: Option<usize>,
}

/// Indices are UTF-16 offsets at Unicode scalar boundaries, as in the lab.
/// The value stays in the caller's view record; preedit never enters it.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    caret: usize,
    anchor: usize,
    composition: Option<Composition>,
}

impl Editor {
    pub fn new(value: &str) -> Self {
        let mut result = Self::default();
        result.reset(value);
        result
    }
    pub fn reset(&mut self, value: &str) {
        self.caret = value.encode_utf16().count();
        self.anchor = self.caret;
        self.composition = None;
    }
    pub fn composing(&self) -> bool {
        self.composition.is_some()
    }
    pub fn selection(&self) -> [usize; 2] {
        [self.anchor.min(self.caret), self.anchor.max(self.caret)]
    }
    pub fn edit(&self) -> Edit {
        let range = self.selection();
        if let Some(p) = &self.composition {
            Edit {
                caret: Some(p.range[0]),
                preedit: Some(Preedit {
                    text: p.text.clone(),
                    caret: p.caret,
                }),
                replacement: Some(p.range),
                hide_caret: p.caret.is_none(),
                ..Default::default()
            }
        } else {
            Edit {
                caret: Some(self.caret),
                select: (range[0] != range[1]).then_some(range),
                ..Default::default()
            }
        }
    }
    /// Returns whether presentation changed. During composition, only an IME
    /// update/commit, cancellation or an explicit pointer caret may change it.
    pub fn handle(&mut self, value: &mut String, command: Command) -> bool {
        if self.composing()
            && matches!(
                command,
                Command::Insert(_)
                    | Command::Delete { .. }
                    | Command::Move { .. }
                    | Command::SelectAll
            )
        {
            return false;
        }
        match command {
            Command::Insert(text) | Command::Commit(text) => self.insert(value, &text),
            Command::Delete { backwards } => {
                if self.anchor == self.caret
                    && !self.move_caret(
                        value,
                        if backwards {
                            Motion::Left
                        } else {
                            Motion::Right
                        },
                        true,
                    )
                {
                    return false;
                }
                self.insert(value, "")
            }
            Command::Move { motion, extend } => self.move_caret(value, motion, extend),
            Command::SelectAll => {
                self.anchor = 0;
                self.caret = value.encode_utf16().count();
                true
            }
            Command::SetCaret { at, extend } => {
                if utf16_byte_index(value, at).is_none() {
                    return false;
                }
                self.composition = None;
                self.caret = at;
                if !extend {
                    self.anchor = at;
                }
                true
            }
            Command::Preedit { text, caret } => {
                if text.is_empty() {
                    self.composition = None;
                } else {
                    if caret.is_some_and(|at| utf16_byte_index(&text, at).is_none()) {
                        return false;
                    }
                    let range = self
                        .composition
                        .as_ref()
                        .map_or(self.selection(), |p| p.range);
                    self.composition = Some(Composition { range, text, caret });
                }
                true
            }
            Command::CancelPreedit => self.composition.take().is_some(),
        }
    }
    fn insert(&mut self, value: &mut String, text: &str) -> bool {
        let [a, b] = self
            .composition
            .take()
            .map_or(self.selection(), |p| p.range);
        let start = utf16_byte_index(value, a).expect("editor caret boundary");
        let end = utf16_byte_index(value, b).expect("editor selection boundary");
        value.replace_range(start..end, text);
        self.caret = a + text.encode_utf16().count();
        self.anchor = self.caret;
        true
    }
    fn move_caret(&mut self, value: &str, motion: Motion, extend: bool) -> bool {
        let previous = (self.anchor, self.caret);
        let [a, b] = self.selection();
        let at = utf16_byte_index(value, self.caret).expect("editor caret boundary");
        self.caret = match motion {
            Motion::Start => 0,
            Motion::End => value.encode_utf16().count(),
            Motion::Left if !extend && a != b => a,
            Motion::Right if !extend && a != b => b,
            Motion::Left => self.caret - value[..at].chars().next_back().map_or(0, char::len_utf16),
            Motion::Right => self.caret + value[at..].chars().next().map_or(0, char::len_utf16),
        };
        if !extend {
            self.anchor = self.caret;
        }
        previous != (self.anchor, self.caret)
    }
}
