//! Pure launcher control over the existing module's item/filter projection.
//! The app supplies catalog updates and text edits, and performs all effects.

use bbt_ui::components::{Edit, Preedit, utf16_byte_index};
use bbt_ui::modules::launcher::{Item, Launcher};
use bbt_ui_model::input::Modifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Query(String),
    Insert(String),
    Commit(String),
    Delete,
    MoveCaret { motion: CaretMove, extend: bool },
    SelectAll,
    SetCaret { at: usize, extend: bool },
    Preedit { text: String, caret: Option<usize> },
    Up,
    Down,
    Right,
    Left,
    Backspace,
    Escape,
    Hover(usize),
    Reroll,
    Activate(Modifiers),
    ActivateAt { index: usize, modifiers: Modifiers },
    Digit { digit: u8, modifiers: Modifiers },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretMove {
    Left,
    Right,
    Start,
    End,
}

#[derive(Debug, Clone)]
struct Composition {
    range: [usize; 2],
    text: String,
    caret: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Changed,
    /// The focused query field may handle this key; it never goes to a PTY.
    Local,
    Close,
    /// Name generation remains the caller's job, as it is for the view module.
    Reroll {
        seed: u64,
    },
    HostUnavailable,
    Capped,
    /// The caller translates this id and chord into its bounded app dispatch.
    Pick {
        id: String,
        modifiers: Modifiers,
    },
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub launcher: Launcher,
    name_seed: u64,
    caret: usize,
    anchor: usize,
    composition: Option<Composition>,
}

impl Controller {
    pub fn new(launcher: Launcher, name_seed: u64) -> Self {
        let caret = launcher.query.encode_utf16().count();
        let mut result = Self {
            launcher,
            name_seed,
            caret,
            anchor: caret,
            composition: None,
        };
        result.sync_edit();
        result
    }

    pub fn handle(&mut self, command: Command) -> Outcome {
        if self.composition.is_some() {
            match command {
                Command::Escape => {
                    self.composition = None;
                    self.sync_edit();
                    return Outcome::Changed;
                }
                Command::Activate(_)
                | Command::ActivateAt { .. }
                | Command::Digit { .. }
                | Command::Up
                | Command::Down
                | Command::Right
                | Command::Left
                | Command::MoveCaret { .. }
                | Command::Backspace
                | Command::Delete
                | Command::Insert(_)
                | Command::SelectAll => return Outcome::Local,
                _ => {}
            }
        }
        match command {
            Command::Query(query) => {
                if self.launcher.query == query {
                    return Outcome::Local;
                }
                self.launcher.query = query;
                self.launcher.selected = Some(0);
                self.reset_edit();
                Outcome::Changed
            }
            Command::Insert(text) | Command::Commit(text) => self.insert(text),
            Command::Delete => self.delete(false),
            Command::MoveCaret { motion, extend } => self.move_caret(motion, extend),
            Command::SelectAll => {
                self.anchor = 0;
                self.caret = self.launcher.query.encode_utf16().count();
                self.sync_edit();
                Outcome::Changed
            }
            Command::SetCaret { at, extend } => {
                if utf16_byte_index(&self.launcher.query, at).is_none() {
                    return Outcome::Local;
                }
                self.composition = None;
                self.caret = at;
                if !extend {
                    self.anchor = at;
                }
                self.sync_edit();
                Outcome::Changed
            }
            Command::Preedit { text, caret } => {
                if text.is_empty() {
                    self.composition = None;
                } else {
                    if caret.is_some_and(|at| utf16_byte_index(&text, at).is_none()) {
                        return Outcome::Local;
                    }
                    let range = self
                        .composition
                        .as_ref()
                        .map_or(self.selection(), |p| p.range);
                    self.composition = Some(Composition { range, text, caret });
                }
                self.sync_edit();
                Outcome::Changed
            }
            Command::Up => self.move_selection(true),
            Command::Down => self.move_selection(false),
            Command::Hover(index) => {
                let view = self.launcher.view();
                if index >= view.rows.len() || index == view.selected {
                    return Outcome::Local;
                }
                self.launcher.selected = Some(index);
                Outcome::Changed
            }
            Command::Right => {
                if self.anchor != self.caret {
                    return self.move_caret(CaretMove::Right, false);
                }
                let outcome = self.dive();
                if outcome == Outcome::Local {
                    self.move_caret(CaretMove::Right, false)
                } else {
                    outcome
                }
            }
            Command::Left | Command::Backspace if self.launcher.query.is_empty() => self.climb(),
            Command::Left => self.move_caret(CaretMove::Left, false),
            Command::Backspace => self.delete(true),
            Command::Escape if self.launcher.path.is_empty() => Outcome::Close,
            Command::Escape => self.climb(),
            Command::Reroll => {
                self.name_seed = self.name_seed.wrapping_add(1);
                Outcome::Reroll {
                    seed: self.name_seed,
                }
            }
            Command::Activate(modifiers) => self.activate(modifiers),
            Command::ActivateAt { index, modifiers } => self.activate_at(index, modifiers),
            Command::Digit { digit, modifiers }
                if self.launcher.query.is_empty() && (1..=9).contains(&digit) =>
            {
                self.activate_at(usize::from(digit - 1), modifiers)
            }
            Command::Digit { .. } => Outcome::Local,
        }
    }

    fn selection(&self) -> [usize; 2] {
        [self.anchor.min(self.caret), self.anchor.max(self.caret)]
    }
    fn sync_edit(&mut self) {
        let range = self.selection();
        self.launcher.edit = Some(if let Some(p) = &self.composition {
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
        });
    }
    fn reset_edit(&mut self) {
        self.caret = self.launcher.query.encode_utf16().count();
        self.anchor = self.caret;
        self.composition = None;
        self.sync_edit();
    }
    fn insert(&mut self, text: String) -> Outcome {
        let [a, b] = self
            .composition
            .take()
            .map_or(self.selection(), |p| p.range);
        let start = utf16_byte_index(&self.launcher.query, a).unwrap();
        let end = utf16_byte_index(&self.launcher.query, b).unwrap();
        self.launcher.query.replace_range(start..end, &text);
        self.caret = a + text.encode_utf16().count();
        self.anchor = self.caret;
        self.launcher.selected = Some(0);
        self.sync_edit();
        Outcome::Changed
    }
    fn move_caret(&mut self, motion: CaretMove, extend: bool) -> Outcome {
        let previous = (self.anchor, self.caret);
        let [a, b] = self.selection();
        let at = utf16_byte_index(&self.launcher.query, self.caret).unwrap();
        self.caret = match motion {
            CaretMove::Start => 0,
            CaretMove::End => self.launcher.query.encode_utf16().count(),
            CaretMove::Left if !extend && a != b => a,
            CaretMove::Right if !extend && a != b => b,
            CaretMove::Left => {
                self.caret
                    - self.launcher.query[..at]
                        .chars()
                        .next_back()
                        .map_or(0, char::len_utf16)
            }
            CaretMove::Right => {
                self.caret
                    + self.launcher.query[at..]
                        .chars()
                        .next()
                        .map_or(0, char::len_utf16)
            }
        };
        if !extend {
            self.anchor = self.caret;
        }
        if previous == (self.anchor, self.caret) {
            return Outcome::Local;
        }
        self.sync_edit();
        Outcome::Changed
    }
    fn delete(&mut self, backwards: bool) -> Outcome {
        if self.anchor == self.caret {
            let outcome = self.move_caret(
                if backwards {
                    CaretMove::Left
                } else {
                    CaretMove::Right
                },
                true,
            );
            if outcome == Outcome::Local {
                return Outcome::Local;
            }
        }
        self.insert(String::new())
    }

    fn activate_at(&mut self, index: usize, modifiers: Modifiers) -> Outcome {
        if index >= self.launcher.view().rows.len() {
            return Outcome::Local;
        }
        let previous = self.launcher.selected;
        self.launcher.selected = Some(index);
        let outcome = self.activate(modifiers);
        // A direct row pick does not move the highlight. A dive owns a new
        // level and its initial selection, so keep that state when it occurs.
        if outcome != Outcome::Changed {
            self.launcher.selected = previous;
        }
        outcome
    }

    fn unavailable(&self) -> Outcome {
        let view = self.launcher.view();
        if view.rows.get(view.selected).is_some() {
            Outcome::HostUnavailable
        } else {
            Outcome::Local
        }
    }

    fn activate(&mut self, modifiers: Modifiers) -> Outcome {
        let Some(item) = self.launcher.activation() else {
            return self.unavailable();
        };
        let picks = item.children.is_none() || item.enter_picks;
        if picks && self.launcher.cap.is_some() && !item.navigation {
            return Outcome::Capped;
        }
        if !picks {
            return self.dive();
        }
        Outcome::Pick {
            id: item.id.clone(),
            modifiers,
        }
    }

    fn move_selection(&mut self, backwards: bool) -> Outcome {
        let view = self.launcher.view();
        let count = view.rows.len();
        if count == 0 {
            return Outcome::Local;
        }
        let index = view.selected % count;
        self.launcher.selected = Some(if backwards {
            if index == 0 { count - 1 } else { index - 1 }
        } else {
            (index + 1) % count
        });
        Outcome::Changed
    }

    fn dive(&mut self) -> Outcome {
        let Some(item) = self.launcher.activation() else {
            return self.unavailable();
        };
        if item.children.is_none() {
            return Outcome::Local;
        }
        // Include virtual ancestors produced by typed descent before committing
        // the explicit path, so "ssh rig cod" then Right retains all three.
        let mut path: Vec<_> = self
            .launcher
            .view()
            .crumbs
            .iter()
            .map(|it| it.id.clone())
            .collect();
        path.push(item.id.clone());
        self.launcher.path = path;
        self.launcher.query.clear();
        self.reset_edit();
        self.launcher.selected = Some(0);
        Outcome::Changed
    }

    fn climb(&mut self) -> Outcome {
        if self.launcher.path.pop().is_none() {
            return Outcome::Local;
        }
        self.launcher.query.clear();
        self.reset_edit();
        self.launcher.selected = Some(0);
        Outcome::Changed
    }

    /// Reconcile a new provider/catalog snapshot without a second item model.
    /// Keep the selected id if it survives; stop an explicit path before any
    /// parent that disappeared or no longer has children (e.g. a down host).
    pub fn replace_items(&mut self, items: Vec<Item>) {
        let view = self.launcher.view();
        let old_index = view.selected;
        let selected = view.rows.get(old_index).map(|it| it.id.clone());
        self.launcher.items = items;
        let mut level = self.launcher.items.as_slice();
        let mut keep = 0;
        for id in &self.launcher.path {
            let Some(children) = level
                .iter()
                .find(|it| &it.id == id)
                .and_then(|it| it.children.as_deref())
            else {
                break;
            };
            keep += 1;
            level = children;
        }
        self.launcher.path.truncate(keep);
        let view = self.launcher.view();
        self.launcher.selected = Some(
            selected
                .and_then(|id| view.rows.iter().position(|it| it.id == id))
                .unwrap_or_else(|| old_index.min(view.rows.len().saturating_sub(1))),
        );
    }
}
