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
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Insert(String),
    Commit(String),
    Delete { backwards: bool },
    Move { motion: Motion, extend: bool },
    Undo,
    Redo,
    HistoryPrevious,
    HistoryNext,
    Complete { request: Completion, text: String },
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

#[derive(Debug, Clone)]
struct Snapshot {
    value: String,
    caret: usize,
    anchor: usize,
}
/// A captured popover request. Offsets are UTF-16 and include the trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub trigger: char,
    pub query: String,
    pub range: [usize; 2],
}
const MAX_HISTORY: usize = 100;

/// Indices are UTF-16 offsets at Unicode scalar boundaries, as in the lab.
/// The value stays in the caller's view record; preedit never enters it.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    caret: usize,
    anchor: usize,
    composition: Option<Composition>,
    // Keep the public editor compact when embedded in overlay enums.
    memory: Box<Memory>,
}

#[derive(Debug, Clone, Default)]
struct Memory {
    vertical_column: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    history: Vec<String>,
    history_at: Option<usize>,
    draft: Option<Snapshot>,
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
        self.memory.vertical_column = None;
        self.memory.undo.clear();
        self.memory.redo.clear();
        self.memory.history_at = None;
        self.memory.draft = None;
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
            && !matches!(
                command,
                Command::Preedit { .. }
                    | Command::Commit(_)
                    | Command::CancelPreedit
                    | Command::SetCaret { .. }
            )
        {
            return false;
        }
        let before = self.snapshot(value);
        let changed = match command {
            Command::Undo => {
                let Some(previous) = self.memory.undo.pop() else {
                    return false;
                };
                self.memory.redo.push(before);
                self.restore(value, previous);
                return true;
            }
            Command::Redo => {
                let Some(next) = self.memory.redo.pop() else {
                    return false;
                };
                self.memory.undo.push(before);
                self.restore(value, next);
                return true;
            }
            Command::HistoryPrevious => self.history_move(value, true),
            Command::HistoryNext => self.history_move(value, false),
            Command::Complete { request, text } => {
                if self.completion(value).as_ref() != Some(&request) {
                    return false;
                }
                self.anchor = request.range[0];
                self.insert(value, &text)
            }
            other => self.handle_inner(value, other),
        };
        if *value != before.value {
            self.memory.undo.push(before);
            if self.memory.undo.len() > MAX_HISTORY {
                self.memory.undo.remove(0);
            }
            self.memory.redo.clear();
            self.memory.vertical_column = None;
        }
        changed
    }
    fn handle_inner(&mut self, value: &mut String, command: Command) -> bool {
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
                self.memory.vertical_column = None;
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
            Command::Undo
            | Command::Redo
            | Command::HistoryPrevious
            | Command::HistoryNext
            | Command::Complete { .. } => unreachable!("handled by editing transaction"),
        }
    }
    fn snapshot(&self, value: &str) -> Snapshot {
        Snapshot {
            value: value.into(),
            caret: self.caret,
            anchor: self.anchor,
        }
    }
    fn restore(&mut self, value: &mut String, s: Snapshot) {
        *value = s.value;
        self.caret = s.caret;
        self.anchor = s.anchor;
        self.composition = None;
        self.memory.vertical_column = None;
    }
    /// Record a submitted input, deduplicating adjacent entries. Drafts and
    /// completions are UI state; submission/IO remain host-owned.
    pub fn remember_input(&mut self, value: &str) {
        if !value.is_empty() && self.memory.history.last().is_none_or(|s| s != value) {
            self.memory.history.push(value.into());
            if self.memory.history.len() > MAX_HISTORY {
                self.memory.history.remove(0);
            }
        }
        self.memory.history_at = None;
        self.memory.draft = None;
    }
    fn history_move(&mut self, value: &mut String, previous: bool) -> bool {
        if previous {
            if self.memory.history.is_empty() || self.memory.history_at == Some(0) {
                return false;
            }
            let at = self
                .memory
                .history_at
                .map_or(self.memory.history.len() - 1, |i| i - 1);
            if self.memory.history_at.is_none() {
                self.memory.draft = Some(self.snapshot(value));
            }
            *value = self.memory.history[at].clone();
            self.memory.history_at = Some(at);
        } else {
            let Some(at) = self.memory.history_at else {
                return false;
            };
            if at + 1 == self.memory.history.len() {
                let draft = self.memory.draft.take().expect("history draft");
                self.memory.history_at = None;
                self.restore(value, draft);
                return true;
            }
            *value = self.memory.history[at + 1].clone();
            self.memory.history_at = Some(at + 1);
        }
        self.caret = value.encode_utf16().count();
        self.anchor = self.caret;
        self.memory.vertical_column = None;
        true
    }
    /// Pure hook for `/` and `@` providers. Hosts fetch candidates off-thread
    /// and retain this request; Complete rejects stale query/caret captures.
    pub fn completion(&self, value: &str) -> Option<Completion> {
        if self.composing() || self.caret != self.anchor {
            return None;
        }
        let at = utf16_byte_index(value, self.caret)?;
        let start = value[..at]
            .char_indices()
            .rev()
            .find(|(_, ch)| ch.is_whitespace())
            .map_or(0, |(i, ch)| i + ch.len_utf8());
        let token = &value[start..at];
        let trigger = token.chars().next()?;
        if !matches!(trigger, '/' | '@') {
            return None;
        }
        Some(Completion {
            trigger,
            query: token[1..].into(),
            range: [value[..start].encode_utf16().count(), self.caret],
        })
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
        let line_start = value[..at].rfind('\n').map_or(0, |i| i + 1);
        let line_end = at + value[at..].find('\n').unwrap_or(value.len() - at);
        let vertical = matches!(motion, Motion::Up | Motion::Down);
        let column = if vertical {
            *self
                .memory
                .vertical_column
                .get_or_insert_with(|| value[line_start..at].encode_utf16().count())
        } else {
            self.memory.vertical_column = None;
            0
        };
        let units = |byte| value[..byte].encode_utf16().count();
        self.caret = match motion {
            Motion::LineStart => units(line_start),
            Motion::LineEnd => units(line_end),
            Motion::Up | Motion::Down => {
                let range = if motion == Motion::Up {
                    let end = line_start.saturating_sub(1);
                    let start = value[..end].rfind('\n').map_or(0, |n| n + 1);
                    start..end
                } else if line_end < value.len() {
                    let start = line_end + 1;
                    let end = start + value[start..].find('\n').unwrap_or(value.len() - start);
                    start..end
                } else {
                    line_start..line_end
                };
                let mut count = 0;
                for ch in value[range.clone()].chars() {
                    if count + ch.len_utf16() > column {
                        break;
                    }
                    count += ch.len_utf16();
                }
                units(range.start) + count
            }
            Motion::WordLeft => units(word_boundary(value, at, true)),
            Motion::WordRight => units(word_boundary(value, at, false)),
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

fn word_boundary(value: &str, at: usize, backwards: bool) -> usize {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let chars: Vec<_> = if backwards {
        value[..at].char_indices().rev().collect()
    } else {
        value[at..]
            .char_indices()
            .map(|(i, c)| (at + i, c))
            .collect()
    };
    let mut result = at;
    let mut class = None;
    for (i, c) in chars {
        if class.is_none() && c.is_whitespace() {
            result = if backwards { i } else { i + c.len_utf8() };
            continue;
        }
        if c.is_whitespace() || class.is_some_and(|k| k != word(c)) {
            break;
        }
        class = Some(word(c));
        result = if backwards { i } else { i + c.len_utf8() };
    }
    result
}

/// Candidate popover built on the shared keyboard choice control. Candidate
/// strings are insertion text, including a trigger if the provider wants one.
pub struct CompletionPopover {
    pub request: Completion,
    pub choices: crate::modules::approval::Approval,
}
impl CompletionPopover {
    pub fn new(request: Completion, candidates: Vec<String>) -> Self {
        Self {
            request,
            choices: crate::modules::approval::Approval::new(
                "completion",
                "Complete input",
                candidates
                    .into_iter()
                    .enumerate()
                    .map(|(i, label)| crate::modules::approval::OptionItem {
                        key: i.to_string(),
                        label,
                        enabled: true,
                    })
                    .collect(),
            ),
        }
    }
    pub fn handle(&mut self, key: crate::modules::approval::Key) -> Option<Command> {
        let i = self.choices.handle(key)?.parse::<usize>().ok()?;
        Some(Command::Complete {
            request: self.request.clone(),
            text: self.choices.options.get(i)?.label.clone(),
        })
    }
    pub fn element(&self, ctx: crate::Context) -> crate::components::Element {
        self.choices.element(ctx)
    }
}
