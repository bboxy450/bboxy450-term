//! Caller-resolved spawn fields. This level owns edits, never process or disk IO.
use super::{Item, View, hits};

pub use bbt_ui_model::{FieldKey, Source, SpawnField, ValueChoice};
#[derive(Debug, Clone)]
pub struct SpawnParams {
    pub launch: Item,
    pub state: bbt_ui_model::SpawnParams,
    pub runs: String,
    pub(crate) crumbs: Vec<Item>,
    pub(crate) rows: Vec<Item>,
    choice_crumb: Item,
}
impl SpawnParams {
    pub(crate) fn new(launch: Item, fields: Vec<SpawnField>, mut crumbs: Vec<Item>) -> Self {
        crumbs.push(launch.clone());
        let state = bbt_ui_model::SpawnParams {
            launch: launch.id.clone(),
            fields,
            editing: None,
            choosing: None,
        };
        let mut params = Self {
            launch,
            state,
            runs: String::new(),
            crumbs,
            rows: vec![],
            choice_crumb: Item::default(),
        };
        params.refresh();
        params
    }
    pub(crate) fn refresh(&mut self) {
        if let Some(index) = self.choosing {
            self.choice_crumb = Item {
                label: self.fields[index].key.label().into(),
                ..Default::default()
            };
        }
        self.rows = if let Some(index) = self.choosing {
            self.fields[index]
                .choices
                .as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(i, choice)| Item {
                    id: format!("value:{i}"),
                    harness: "zsh".into(),
                    label: choice.label.clone(),
                    sub: choice.purpose.clone(),
                    kind: if choice.value == self.fields[index].value {
                        "current".into()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                })
                .collect()
        } else {
            self.fields
                .iter()
                .enumerate()
                .map(|(i, field)| Item {
                    id: format!("param:{i}"),
                    harness: "zsh".into(),
                    label: field.key.label().into(),
                    sub: if field.value.is_empty() {
                        "—".into()
                    } else {
                        field.value.clone()
                    },
                    kind: if field.fixed {
                        "fixed"
                    } else {
                        field.source.label()
                    }
                    .into(),
                    // A parameter has the same chevron as any other level.
                    children: (!field.fixed).then(Vec::new),
                    ..Default::default()
                })
                .collect()
        };
    }
    pub(crate) fn view<'a>(&'a self, query: &str, selected: Option<usize>) -> View<'a> {
        let words: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
        let rows = self
            .rows
            .iter()
            .filter(|it| self.editing.is_some() || words.iter().all(|w| hits(it, w)))
            .collect();
        let mut crumbs: Vec<_> = self.crumbs.iter().collect();
        if self.choosing.is_some() {
            crumbs.push(&self.choice_crumb);
        }
        View {
            crumbs,
            rows,
            selected: selected.unwrap_or(0),
        }
    }
}

impl std::ops::Deref for SpawnParams {
    type Target = bbt_ui_model::SpawnParams;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
impl std::ops::DerefMut for SpawnParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
