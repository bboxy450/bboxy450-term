// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained answer controls shared by the desktop and mobile hosts.
use crate::{
    self as ui, components as c,
    overlays::dialog::{Control, ControlKind},
    text_edit::{Command, Editor},
    widgets::{Button, ButtonKind, ButtonState, Input, InputState},
};
use serde_json::{Map, Value, json};

pub const SUBMIT: &str = "question-form/submit";
const FIELD: &str = "question-form/field/";
const CHOICE: &str = "question-form/choice/";

enum Entry {
    Choices {
        options: Vec<(String, Value)>,
        selected: Option<usize>,
    },
    Input {
        value: String,
        number: bool,
        editor: Editor,
    },
}

struct Field {
    key: String,
    label: String,
    required: bool,
    entry: Entry,
}
impl Field {
    fn from_schema(key: &str, schema: &Value, required: bool) -> Option<Self> {
        let label = schema["title"].as_str().unwrap_or(key).to_owned();
        let options = if let Some(values) = schema["enum"].as_array() {
            Some(
                values
                    .iter()
                    .cloned()
                    .map(|value| {
                        let label = value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string());
                        (label, value)
                    })
                    .collect::<Vec<_>>(),
            )
        } else if let Some(values) = schema["oneOf"].as_array() {
            Some(
                values
                    .iter()
                    .map(|value| {
                        let option = value.get("const")?;
                        let label = value["title"]
                            .as_str()
                            .or_else(|| option.as_str())
                            .map(str::to_owned)
                            .unwrap_or_else(|| option.to_string());
                        Some((label, option.clone()))
                    })
                    .collect::<Option<Vec<_>>>()?,
            )
        } else if schema["type"] == "boolean" {
            Some(vec![
                ("Yes".into(), Value::Bool(true)),
                ("No".into(), Value::Bool(false)),
            ])
        } else {
            None
        };
        let entry = if let Some(options) = options.filter(|options| !options.is_empty()) {
            Entry::Choices {
                options,
                selected: None,
            }
        } else {
            let number = match schema["type"].as_str()? {
                "string" => false,
                "number" => true,
                _ => return None,
            };
            Entry::Input {
                value: String::new(),
                number,
                editor: Editor::default(),
            }
        };
        Some(Self {
            key: key.into(),
            label,
            required,
            entry,
        })
    }
    fn value(&self) -> Result<Option<Value>, &'static str> {
        let value = match &self.entry {
            Entry::Choices { options, selected } => selected.map(|i| options[i].1.clone()),
            Entry::Input {
                value,
                number: false,
                ..
            } => (self.required || !value.is_empty()).then(|| Value::String(value.clone())),
            Entry::Input { value, .. } if value.is_empty() => None,
            Entry::Input {
                value,
                number: true,
                ..
            } => Some(Value::Number(
                serde_json::from_str(value.trim()).map_err(|_| "Enter a number")?,
            )),
        };
        if value.is_none() && self.required {
            Err("Complete each required field")
        } else {
            Ok(value)
        }
    }
}

enum Shape {
    Unsupported,
    Scalar,
    Object,
    Approval,
}

/// Keeps selection and editor state, independent of a native window or wire connection.
pub struct Form {
    shape: Shape,
    fields: Vec<Field>,
}
impl Form {
    pub fn question(schema: &Value) -> Self {
        let fields = if schema["type"] == "object" {
            schema["properties"]
                .as_object()
                .and_then(|properties| {
                    properties
                        .iter()
                        .map(|(key, value)| {
                            let required = schema["required"]
                                .as_array()
                                .is_some_and(|keys| keys.iter().any(|name| name == key));
                            Field::from_schema(key, value, required)
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .filter(|fields| !fields.is_empty())
                .map(|fields| (Shape::Object, fields))
        } else {
            Field::from_schema("", schema, true).map(|field| (Shape::Scalar, vec![field]))
        };
        let (shape, fields) = fields.unwrap_or((Shape::Unsupported, Vec::new()));
        Self { shape, fields }
    }

    pub fn approval(options: Vec<(String, Value)>) -> Self {
        Self {
            shape: Shape::Approval,
            fields: vec![Field {
                key: String::new(),
                label: String::new(),
                required: true,
                entry: Entry::Choices {
                    options,
                    selected: None,
                },
            }],
        }
    }

    /// A choice submits immediately for scalar questions and approvals.
    pub fn act(&mut self, id: &str) -> Option<Result<Value, &'static str>> {
        if id == SUBMIT {
            return Some(self.reply());
        }
        let (field, option) = id.strip_prefix(CHOICE)?.split_once('/')?;
        let field: usize = field.parse().ok()?;
        let option: usize = option.parse().ok()?;
        let Entry::Choices { options, selected } = &mut self.fields.get_mut(field)?.entry else {
            return None;
        };
        options.get(option)?;
        *selected = Some(option);
        (!matches!(self.shape, Shape::Object)).then(|| self.reply())
    }

    pub fn edit(&mut self, id: &str, command: Command) -> bool {
        let Some(i) = id.strip_prefix(FIELD).and_then(|i| i.parse::<usize>().ok()) else {
            return false;
        };
        let Some(Field {
            entry: Entry::Input { value, editor, .. },
            ..
        }) = self.fields.get_mut(i)
        else {
            return false;
        };
        editor.handle(value, command);
        editor.composing()
    }

    pub fn compose(
        &self,
        ctx: ui::Context,
        focused: Option<&str>,
        enabled: bool,
        body: &mut Vec<c::Item>,
        controls: &mut Vec<Control>,
    ) {
        if matches!(self.shape, Shape::Unsupported) {
            body.push(text("Answer on the desktop").into());
            return;
        }
        for (i, field) in self.fields.iter().enumerate() {
            if !field.label.is_empty() {
                body.push(text(&field.label).into());
            }
            match &field.entry {
                Entry::Choices { options, selected } => {
                    for (j, (label, _)) in options.iter().enumerate() {
                        let id = format!("{CHOICE}{i}/{j}");
                        body.push(button(ctx, &id, label, enabled, *selected == Some(j)).into());
                        controls.push(Control {
                            id,
                            kind: ControlKind::Action,
                            enabled,
                        });
                    }
                }
                Entry::Input {
                    value,
                    number,
                    editor,
                } => {
                    let id = format!("{FIELD}{i}");
                    let focus = enabled && focused == Some(id.as_str());
                    let input = Input {
                        value: value.clone(),
                        placeholder: if *number {
                            "Number".into()
                        } else {
                            "Answer".into()
                        },
                        state: if focus {
                            InputState::Focus
                        } else if value.is_empty() {
                            InputState::Empty
                        } else {
                            InputState::Value
                        },
                        icon: None,
                        error: None,
                        edit: focus.then(|| editor.edit()),
                    }
                    .element(ctx);
                    body.push(named(&id, input).into());
                    controls.push(Control {
                        id,
                        kind: ControlKind::Field,
                        enabled,
                    });
                }
            }
        }
        if matches!(self.shape, Shape::Object)
            || self
                .fields
                .iter()
                .any(|f| matches!(f.entry, Entry::Input { .. }))
        {
            body.push(button(ctx, SUBMIT, "Send answer", enabled, true).into());
            controls.push(Control {
                id: SUBMIT.into(),
                kind: ControlKind::Action,
                enabled,
            });
        }
    }

    pub fn reply(&self) -> Result<Value, &'static str> {
        let content = match self.shape {
            Shape::Unsupported => return Err("Answer on the desktop"),
            Shape::Approval => return self.fields[0].value()?.ok_or("Choose an option"),
            Shape::Scalar => self.fields[0].value()?.ok_or("Enter an answer")?,
            Shape::Object => {
                let mut values = Map::new();
                for field in &self.fields {
                    if let Some(value) = field.value()? {
                        values.insert(field.key.clone(), value);
                    }
                }
                if values.is_empty() {
                    return Err("Enter an answer");
                }
                Value::Object(values)
            }
        };
        Ok(json!({"action":"accept","content":content}))
    }
}

pub fn text(value: impl Into<String>) -> c::Element {
    c::Element::Text(c::Text::new(value, ui::Text::default()))
}

pub fn button(
    ctx: ui::Context,
    id: &str,
    label: &str,
    enabled: bool,
    selected: bool,
) -> c::Element {
    let content = Button {
        label: label.into(),
        kind: if selected {
            ButtonKind::Primary
        } else {
            ButtonKind::Ghost
        },
        state: if enabled {
            ButtonState::Idle
        } else {
            ButtonState::Disabled
        },
        icon: None,
        key: None,
    }
    .element(ctx)
    .expect("answer form button style");
    named(id, content)
}

fn named(id: &str, content: c::Element) -> c::Element {
    c::Element::Named {
        name: id.into(),
        content: Box::new(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_forms_submit_the_schema_values() {
        let mut enumeration = Form::question(&json!({"enum":[1,"two"]}));
        assert_eq!(
            enumeration
                .act("question-form/choice/0/0")
                .unwrap()
                .unwrap()["content"],
            1
        );
        let mut boolean = Form::question(&json!({"type":"boolean"}));
        assert_eq!(
            boolean.act("question-form/choice/0/1").unwrap().unwrap()["content"],
            false
        );
        let mut form = Form::question(&json!({"type":"object","required":["n","text"],
            "properties":{"n":{"type":"number"},"text":{"type":"string"}}}));
        assert!(form.reply().is_err());
        form.edit("question-form/field/0", Command::Commit("42.5".into()));
        form.edit("question-form/field/1", Command::Commit("hello".into()));
        assert_eq!(
            form.reply().unwrap()["content"],
            json!({"n":42.5,"text":"hello"})
        );
        let mut approval = Form::approval(vec![("Deny".into(), json!("deny"))]);
        assert_eq!(
            approval.act("question-form/choice/0/0").unwrap().unwrap(),
            "deny"
        );
        assert!(Form::question(&json!({"type":"array"})).reply().is_err());
    }
}
