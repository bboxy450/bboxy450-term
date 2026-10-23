//! Pure launcher control over the existing module's item/filter projection.
//! The app supplies catalog updates and text edits, and performs all effects.

use bbt_ui::modules::launcher::{Item, Launcher};
use bbt_ui_model::input::Modifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Query(String),
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
}

impl Controller {
    pub fn new(launcher: Launcher, name_seed: u64) -> Self {
        Self {
            launcher,
            name_seed,
        }
    }

    pub fn handle(&mut self, command: Command) -> Outcome {
        match command {
            Command::Query(query) => {
                if self.launcher.query == query {
                    return Outcome::Local;
                }
                self.launcher.query = query;
                self.launcher.selected = Some(0);
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
            Command::Right => self.dive(),
            Command::Left | Command::Backspace if self.launcher.query.is_empty() => self.climb(),
            Command::Left | Command::Backspace => Outcome::Local,
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
        self.launcher.selected = Some(0);
        Outcome::Changed
    }

    fn climb(&mut self) -> Outcome {
        if self.launcher.path.pop().is_none() {
            return Outcome::Local;
        }
        self.launcher.query.clear();
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
