// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure launcher navigation and editing; the host performs effects.
use super::{FieldKey, Item, Launcher, Source, SpawnField, SpawnParams};
use crate::text_edit::{Command as EditCommand, Editor};
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

pub use crate::text_edit::Motion as CaretMove;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Changed,
    /// The host checks filesystem state off the UI thread before accepting it.
    Directory {
        value: String,
    },
    Parameters {
        id: String,
    },
    LaunchParams {
        id: String,
        modifiers: Modifiers,
    },
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
    editor: Editor,
    parameter_placeholder: Option<String>,
}

impl Controller {
    pub fn new(launcher: Launcher, name_seed: u64) -> Self {
        let editor = Editor::new(&launcher.query);
        let mut result = Self {
            launcher,
            name_seed,
            editor,
            parameter_placeholder: None,
        };
        result.sync_edit();
        result
    }

    pub fn handle(&mut self, command: Command) -> Outcome {
        if self.editor.composing() {
            match command {
                Command::Escape => {
                    self.editor
                        .handle(&mut self.launcher.query, EditCommand::CancelPreedit);
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
            Command::SelectAll => self.edit(EditCommand::SelectAll),
            Command::SetCaret { at, extend } => self.edit(EditCommand::SetCaret { at, extend }),
            Command::Preedit { text, caret } => self.edit(EditCommand::Preedit { text, caret }),
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
            Command::Right if self.launcher.params.is_some() => self.change_parameter(),
            Command::Right => {
                if self.editor.selection()[0] != self.editor.selection()[1] {
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
            Command::Escape if self.launcher.path.is_empty() && self.launcher.params.is_none() => {
                Outcome::Close
            }
            Command::Escape => self.climb(),
            Command::Reroll => {
                self.name_seed = self.name_seed.wrapping_add(1);
                Outcome::Reroll {
                    seed: self.name_seed,
                }
            }
            Command::Activate(modifiers) => self.activate(modifiers),
            Command::ActivateAt { index, modifiers } => self.click_at(index, modifiers),
            Command::Digit { digit, modifiers }
                if self
                    .launcher
                    .params
                    .as_ref()
                    .is_some_and(|p| p.editing.is_some()) =>
            {
                if modifiers.command || modifiers.control || modifiers.alt {
                    Outcome::Local
                } else {
                    self.insert(digit.to_string())
                }
            }
            Command::Digit { digit, modifiers }
                if self.launcher.query.is_empty() && (1..=9).contains(&digit) =>
            {
                self.activate_at(usize::from(digit - 1), modifiers)
            }
            Command::Digit { .. } => Outcome::Local,
        }
    }

    fn sync_edit(&mut self) {
        self.launcher.edit = Some(self.editor.edit());
    }
    fn reset_edit(&mut self) {
        self.editor.reset(&self.launcher.query);
        self.sync_edit();
    }
    fn edit(&mut self, command: EditCommand) -> Outcome {
        if !self.editor.handle(&mut self.launcher.query, command) {
            return Outcome::Local;
        }
        self.sync_edit();
        Outcome::Changed
    }
    fn insert(&mut self, text: String) -> Outcome {
        let result = self.edit(EditCommand::Commit(text));
        if result == Outcome::Changed
            && !self
                .launcher
                .params
                .as_ref()
                .is_some_and(|p| p.editing.is_some())
        {
            self.launcher.selected = Some(0);
        }
        result
    }
    fn move_caret(&mut self, motion: CaretMove, extend: bool) -> Outcome {
        self.edit(EditCommand::Move { motion, extend })
    }
    fn delete(&mut self, backwards: bool) -> Outcome {
        let result = self.edit(EditCommand::Delete { backwards });
        if result == Outcome::Changed
            && !self
                .launcher
                .params
                .as_ref()
                .is_some_and(|p| p.editing.is_some())
        {
            self.launcher.selected = Some(0);
        }
        result
    }

    fn click_at(&mut self, index: usize, modifiers: Modifiers) -> Outcome {
        if self
            .launcher
            .params
            .as_ref()
            .is_some_and(|p| p.choosing.is_none() && p.editing.is_none())
        {
            self.launcher.selected = Some(index);
            return self.change_parameter();
        }
        self.activate_at(index, modifiers)
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
        if self.launcher.params.is_some() {
            return self.activate_parameter(modifiers);
        }
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
        if self
            .launcher
            .params
            .as_ref()
            .is_some_and(|p| p.editing.is_some())
        {
            return Outcome::Local;
        }
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
        if item.parameters {
            return Outcome::Parameters {
                id: item.id.clone(),
            };
        }
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
        if let Some(params) = &mut self.launcher.params {
            if let Some(index) = params.editing.take() {
                self.launcher.selected = Some(index);
            } else if let Some(index) = params.choosing.take() {
                self.launcher.selected = Some(index);
                params.refresh();
            } else {
                self.launcher.params = None;
                self.launcher.selected = Some(0);
            }
            self.launcher.query.clear();
            if let Some(placeholder) = self.parameter_placeholder.take() {
                self.launcher.placeholder = placeholder;
            }
            self.reset_edit();
            return Outcome::Changed;
        }
        if self.launcher.path.pop().is_none() {
            return Outcome::Local;
        }
        self.launcher.query.clear();
        self.reset_edit();
        self.launcher.selected = Some(0);
        Outcome::Changed
    }

    /// The host resolves the selected launch once, then hands us only data.
    pub fn open_parameters(&mut self, id: &str, fields: Vec<SpawnField>) -> bool {
        let view = self.launcher.view();
        let Some(item) = view
            .rows
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .cloned()
        else {
            return false;
        };
        let crumbs = view.crumbs.iter().map(|it| (*it).clone()).collect();
        self.launcher.params = Some(SpawnParams::new(item, fields, crumbs));
        self.launcher.query.clear();
        self.launcher.selected = Some(0);
        self.reset_edit();
        true
    }
    fn change_parameter(&mut self) -> Outcome {
        if self
            .launcher
            .params
            .as_ref()
            .is_some_and(|p| p.editing.is_some())
        {
            return self.move_caret(CaretMove::Right, false);
        }
        let view = self.launcher.view();
        let Some(id) = view.rows.get(view.selected).map(|it| it.id.clone()) else {
            return Outcome::Local;
        };
        let Some(index) = id
            .strip_prefix("param:")
            .and_then(|x| x.parse::<usize>().ok())
        else {
            return Outcome::Local;
        };
        let params = self.launcher.params.as_mut().expect("parameter level");
        let field = params.fields[index].clone();
        if field.fixed {
            return Outcome::Local;
        };
        self.parameter_placeholder
            .get_or_insert_with(|| self.launcher.placeholder.clone());
        if field.choices.is_some() {
            params.choosing = Some(index);
            self.launcher.query.clear();
            self.launcher.selected = Some(0);
            self.launcher.placeholder = if field.key == FieldKey::Directory {
                "Directory · recent folders or type a path"
            } else {
                "Role · choose one"
            }
            .into();
            params.refresh();
            self.reset_edit();
        } else {
            params.editing = Some(index);
            self.launcher.query = field.value.clone();
            self.launcher.placeholder = format!(
                "{} · ↵ sets it · esc keeps the old value",
                field.key.label()
            );
            self.reset_edit();
            self.edit(EditCommand::SelectAll);
        }
        Outcome::Changed
    }
    fn activate_parameter(&mut self, modifiers: Modifiers) -> Outcome {
        let view = self.launcher.view();
        let selected = view.rows.get(view.selected).map(|it| it.id.clone());
        let params = self.launcher.params.as_mut().expect("parameter level");
        let (index, value) = if let Some(index) = params.editing {
            (index, self.launcher.query.clone())
        } else if let Some(index) = params.choosing {
            let choices = params.fields[index].choices.as_ref().expect("value list");
            let value = selected
                .as_deref()
                .and_then(|id| id.strip_prefix("value:"))
                .and_then(|x| x.parse::<usize>().ok())
                .and_then(|i| choices.get(i))
                .map(|c| c.value.clone());
            match value {
                Some(value) => (index, value),
                None if params.fields[index].key == FieldKey::Directory
                    && !self.launcher.query.trim().is_empty() =>
                {
                    (index, self.launcher.query.trim().into())
                }
                _ => return Outcome::Local,
            }
        } else {
            if selected.is_none() {
                return Outcome::Local;
            }
            return if self.launcher.cap.is_some() {
                Outcome::Capped
            } else {
                Outcome::LaunchParams {
                    id: params.launch.id.clone(),
                    modifiers,
                }
            };
        };
        if params.fields[index].key == FieldKey::Directory {
            return Outcome::Directory { value };
        }
        self.accept_parameter(index, value)
    }
    /// Called only after the host has verified that this choice is a directory.
    pub fn accept_directory(&mut self, value: String) -> Outcome {
        let Some(index) = self.launcher.params.as_ref().and_then(|p| p.choosing) else {
            return Outcome::Local;
        };
        if self.launcher.params.as_ref().unwrap().fields[index].key != FieldKey::Directory {
            return Outcome::Local;
        }
        self.accept_parameter(index, value)
    }
    fn accept_parameter(&mut self, index: usize, value: String) -> Outcome {
        let params = self.launcher.params.as_mut().expect("parameter level");
        params.fields[index].value = value;
        params.fields[index].source = Source::ThisLaunch;
        params.editing = None;
        params.choosing = None;
        params.refresh();
        self.launcher.query.clear();
        self.launcher.selected = Some(index);
        if let Some(placeholder) = self.parameter_placeholder.take() {
            self.launcher.placeholder = placeholder;
        }
        self.reset_edit();
        Outcome::Changed
    }

    /// Reconcile a new provider/catalog snapshot without a second item model.
    /// Keep the selected id if it survives; stop an explicit path before any
    /// parent that disappeared or no longer has children (e.g. a down host).
    pub fn replace_items(&mut self, items: Vec<Item>) {
        if self.launcher.params.is_some() {
            self.launcher.items = items;
            return;
        }
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
