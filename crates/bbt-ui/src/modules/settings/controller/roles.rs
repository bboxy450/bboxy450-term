// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained account-role and harness-role controls; the host persists every patch.
//! Adapted from bboxy450-term's web Settings. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::super::Role;
use super::*;

pub(crate) const COLORS: [&str; 8] = [
    "#8B5CF6", "#2E6BFF", "#0EA5E9", "#10B981", "#EAB308", "#F97316", "#EC4899", "#D97757",
];
pub(crate) const MARKS: [&str; 6] = ["♜", "♔", "♟", "♝", "♖", "✦"];
pub(crate) const CLASSES: [&str; 2] = ["worker", "oracle"];

#[derive(Debug, Clone)]
pub struct DraftRole {
    pub name: String,
    pub role: Role,
}
impl Default for DraftRole {
    fn default() -> Self {
        Self {
            name: String::new(),
            role: Role {
                color: COLORS[0].into(),
                mark: MARKS[0].into(),
                cls: "worker".into(),
                custom: true,
                send: class_send("worker").into(),
                ..Default::default()
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct FocusedText {
    pub field: Field,
    pub value: String,
    pub edit: c::Edit,
}
#[derive(Debug, Clone)]
pub struct Picker {
    pub field: Field,
    pub choices: Vec<String>,
    pub selected: usize,
}
pub(super) struct Text {
    pub field: Field,
    pub value: String,
    original: String,
    pub editor: Editor,
}
#[derive(Default, Debug, Clone)]
pub(super) struct Controls {
    pub draft: Option<DraftRole>,
    pub text: Option<Text>,
    pub picker: Option<Picker>,
    pub patches: Vec<Patch>,
}
impl Clone for Text {
    fn clone(&self) -> Self {
        Self {
            field: self.field.clone(),
            value: self.value.clone(),
            original: self.original.clone(),
            editor: self.editor.clone(),
        }
    }
}
impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text")
            .field("field", &self.field)
            .finish_non_exhaustive()
    }
}
pub(super) fn builtin(name: &str) -> bool {
    ["oracle", "architect", "creativity", "executor", "shell"]
        .iter()
        .any(|id| id.eq_ignore_ascii_case(name))
}
pub(super) fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name == name.trim()
        && name.len() <= 128
        && !name.chars().any(char::is_control)
}
pub(super) fn valid_harness(name: &str) -> bool {
    valid_name(name)
}
pub(super) fn valid_role(name: &str, role: &Role) -> bool {
    valid_name(name)
        && !(role.custom && builtin(name))
        && CLASSES.contains(&role.cls.as_str())
        && role.prompt.len() <= 32 * 1024
        && !role.prompt.contains('\0')
        && (!role.mark.is_empty() || (name == "shell" && !role.custom))
        && role.mark.len() <= 32
        && !role.mark.chars().any(char::is_control)
        && role.send.len() <= 512
        && !role.send.chars().any(char::is_control)
        && (role.color.starts_with("var(--role-")
            || (matches!(role.color.len(), 7 | 9)
                && role.color.starts_with('#')
                && role.color[1..].bytes().all(|b| b.is_ascii_hexdigit())))
}
/// Stable control names keep arbitrary account role names out of the draft namespace.
pub fn role_region(name: &str, control: &str) -> String {
    let mut id = name.replace('%', "%25").replace('/', "%2F");
    if id == "draft" {
        id = "%64raft".into();
    }
    format!("role/{id}/{control}")
}
fn class_send(class: &str) -> &'static str {
    if class == "oracle" {
        "all channels · all DMs"
    } else {
        "own swarm · operator DM"
    }
}
fn is_text(field: &Field) -> bool {
    matches!(
        field,
        Field::RoleDraftName
            | Field::RoleDraftPrompt
            | Field::RolePrompt(_)
            | Field::Machine(_, _)
            | Field::MachineNew
    )
}

impl Controller {
    pub fn can_cancel(&self) -> bool {
        self.roles.text.is_some() || self.roles.picker.is_some() || self.roles.draft.is_some()
    }
    /// A single blur plus a clicked control may produce two independent edits.
    pub fn take_patches(&mut self) -> Vec<Patch> {
        std::mem::take(&mut self.roles.patches)
    }
    pub(super) fn role_editable(&self, field: &Field, values: &Records) -> bool {
        match field {
            Field::MachineNew => self.section == "Machines" && values.machine_controls,
            Field::Machine(id, key) => {
                self.section == "Machines"
                    && values.machine_controls
                    && values.machine_defaults.contains_key(id)
                    && super::super::machines::FIELDS
                        .iter()
                        .any(|(name, _)| *name == key)
            }
            Field::RoleNew => self.section == "Roles" || !self.query.trim().is_empty(),
            Field::RoleAdd
            | Field::RoleCancel
            | Field::RoleDraftName
            | Field::RoleDraftPrompt
            | Field::RoleDraftColor
            | Field::RoleDraftMark
            | Field::RoleDraftClass => self.roles.draft.is_some(),
            Field::RolePrompt(name) | Field::RoleClass(name) => {
                values.roles.iter().any(|(id, _)| id == name)
            }
            Field::RoleDelete(name) => values
                .roles
                .iter()
                .any(|(id, role)| id == name && role.custom && !builtin(id)),
            Field::HarnessRole(name) => values.harness.iter().any(|(id, _)| id == name),
            Field::HarnessAvailableRole { harness, role } => values
                .harness
                .iter()
                .any(|(id, h)| id == harness && h.role != *role && h.eligible_roles.contains(role)),
            Field::Choice(index) => self
                .roles
                .picker
                .as_ref()
                .is_some_and(|picker| *index < picker.choices.len()),
            _ => false,
        }
    }
    pub(super) fn role_fields(&self, values: &Records) -> Vec<Field> {
        if self.section == "Machines" {
            if !values.machine_controls { return vec![]; }
            return std::iter::once(Field::MachineNew)
                .chain(values.machine_defaults.keys().flat_map(|id| {
                    super::super::machines::FIELDS
                        .iter()
                        .map(move |(key, _)| Field::Machine(id.clone(), (*key).into()))
                }))
                .collect();
        }
        if self.section == "Roles" {
            let mut out = vec![];
            for (name, role) in &values.roles {
                out.push(Field::RolePrompt(name.clone()));
                out.push(Field::RoleClass(name.clone()));
                if role.custom {
                    out.push(Field::RoleDelete(name.clone()));
                }
            }
            if self.roles.draft.is_some() {
                out.extend([
                    Field::RoleDraftColor,
                    Field::RoleDraftName,
                    Field::RoleDraftMark,
                    Field::RoleDraftPrompt,
                    Field::RoleDraftClass,
                    Field::RoleAdd,
                    Field::RoleCancel,
                ]);
            } else {
                out.push(Field::RoleNew);
            }
            out
        } else {
            values
                .harness
                .iter()
                .flat_map(|(name, h)| {
                    std::iter::once(Field::HarnessRole(name.clone())).chain(
                        h.eligible_roles
                            .iter()
                            .filter(|role| **role != h.role)
                            .map(|role| Field::HarnessAvailableRole {
                                harness: name.clone(),
                                role: role.clone(),
                            }),
                    )
                })
                .collect()
        }
    }
    fn begin_role_text(&mut self, field: Field, values: &Records) {
        if self
            .roles
            .text
            .as_ref()
            .is_some_and(|text| text.field == field)
        {
            return;
        }
        let value = match &field {
            Field::MachineNew => Some(String::new()),
            Field::Machine(id, key) => values
                .machine_defaults
                .get(id)
                .map(|machine| machine.value(key)),
            Field::RoleDraftName => self.roles.draft.as_ref().map(|d| d.name.clone()),
            Field::RoleDraftPrompt => self.roles.draft.as_ref().map(|d| d.role.prompt.clone()),
            Field::RolePrompt(name) => values
                .roles
                .iter()
                .find(|(id, _)| id == name)
                .map(|(_, role)| role.prompt.clone()),
            _ => None,
        };
        if let Some(value) = value {
            self.roles.text = Some(Text {
                field: field.clone(),
                original: value.clone(),
                editor: Editor::new(&value),
                value,
            });
            self.focus = Focus::Control(field);
        }
    }
    fn draft_text(&mut self, field: &Field, value: &str) {
        if let Some(draft) = &mut self.roles.draft {
            match field {
                Field::RoleDraftName => draft.name = value.into(),
                Field::RoleDraftPrompt => draft.role.prompt = value.into(),
                _ => {}
            }
        }
    }
    fn commit_role_text(&mut self, values: &mut Records) -> Option<Patch> {
        let mut text = self.roles.text.take()?;
        text.editor
            .handle(&mut text.value, EditCommand::CancelPreedit);
        self.draft_text(&text.field, &text.value);
        if text.field == Field::MachineNew {
            let route = text.value.trim().to_owned();
            let machine = super::super::Machine {
                connect: route.clone(),
                ..Default::default()
            };
            return self.role_patch(
                Patch::MachineLaunch {
                    id: format!("ssh:{route}"),
                    machine,
                },
                values,
            );
        }
        if let Field::Machine(id, key) = &text.field {
            let mut machine = values.machine_defaults.get(id)?.clone();
            machine.set(key, text.value);
            return self.role_patch(
                Patch::MachineLaunch {
                    id: id.clone(),
                    machine,
                },
                values,
            );
        }
        if let Field::RolePrompt(name) = text.field {
            let (_, old) = values.roles.iter().find(|(id, _)| *id == name)?;
            if old.prompt == text.value {
                return None;
            }
            let mut role = old.clone();
            role.prompt = text.value;
            return self.role_patch(Patch::RoleUpsert { name, role }, values);
        }
        None
    }
    pub(super) fn prepare_role_action(
        &mut self,
        action: &Action,
        values: &mut Records,
    ) -> Option<Patch> {
        // Every native click ends with EndDrag, including picker/editor clicks.
        // Release only ends a drag; it is not a focus change or an edit commit.
        if matches!(action, Action::EndDrag) {
            return None;
        }
        if !matches!(action, Action::Search(_)) {
            self.editor
                .handle(&mut self.query, EditCommand::CancelPreedit);
        }
        let same_text = self.roles.text.as_ref().is_some_and(|text| match action {
            Action::Focus(Focus::Control(field)) | Action::Toggle(field) => *field == text.field,
            _ => false,
        });
        let commit =
            !same_text && !matches!(action, Action::Search(_) | Action::Commit | Action::Cancel);
        if !matches!(
            action,
            Action::Nudge(_) | Action::Commit | Action::Cancel | Action::Toggle(Field::Choice(_))
        ) && !self
            .roles
            .picker
            .as_ref()
            .is_some_and(|p| matches!(action, Action::Toggle(field) if *field == p.field))
        {
            self.roles.picker = None;
        }
        if commit {
            self.commit_role_text(values)
        } else {
            None
        }
    }
    fn role_patch(&mut self, patch: Patch, values: &mut Records) -> Option<Patch> {
        if let Err(error) = patch.validate() {
            self.error = Some(error);
            return None;
        }
        match &patch {
            Patch::MachineLaunch { id, machine } => {
                values.machine_defaults.insert(id.clone(), machine.clone());
            }
            Patch::RoleUpsert { name, role } => {
                if let Some((_, current)) = values.roles.iter_mut().find(|(id, _)| id == name) {
                    if !current.custom && role.custom {
                        self.error = Some("Built-in roles cannot become custom roles".into());
                        return None;
                    }
                    *current = role.clone();
                } else {
                    values.roles.push((name.clone(), role.clone()));
                }
            }
            Patch::RoleDelete(name) => values.roles.retain(|(id, _)| id != name),
            Patch::HarnessRole { harness, role } => {
                let current = &mut values.harness.iter_mut().find(|(id, _)| id == harness)?.1;
                if !current.available_roles.contains(role) || !current.eligible_roles.contains(role)
                {
                    self.error = Some("Choose an available role for this harness".into());
                    return None;
                }
                current.role = role.clone();
            }
            Patch::HarnessAvailableRoles { harness, roles } => {
                let current = &mut values.harness.iter_mut().find(|(id, _)| id == harness)?.1;
                if !roles.contains(&current.role)
                    || roles
                        .iter()
                        .any(|role| !current.eligible_roles.contains(role))
                {
                    self.error = Some("Available roles must include the default and stay eligible for this harness".into());
                    return None;
                }
                current.available_roles = roles.iter().cloned().collect();
            }
            _ => return None,
        }
        self.error = None;
        Some(patch)
    }
    fn choices(&self, field: &Field, values: &Records) -> Option<(Vec<String>, String)> {
        let draft = self.roles.draft.as_ref();
        let (choices, value) = match field {
            Field::RoleDraftColor => (
                COLORS.iter().map(|s| (*s).into()).collect(),
                draft?.role.color.clone(),
            ),
            Field::RoleDraftMark => (
                MARKS.iter().map(|s| (*s).into()).collect(),
                draft?.role.mark.clone(),
            ),
            Field::RoleDraftClass => (
                CLASSES.iter().map(|s| (*s).into()).collect(),
                draft?.role.cls.clone(),
            ),
            Field::RoleClass(name) => (
                CLASSES.iter().map(|s| (*s).into()).collect(),
                values
                    .roles
                    .iter()
                    .find(|(id, _)| id == name)?
                    .1
                    .cls
                    .clone(),
            ),
            Field::HarnessRole(name) => {
                let h = &values.harness.iter().find(|(id, _)| id == name)?.1;
                (
                    h.available_roles
                        .iter()
                        .filter(|id| h.eligible_roles.contains(id))
                        .cloned()
                        .collect(),
                    h.role.clone(),
                )
            }
            _ => return None,
        };
        Some((choices, value))
    }
    fn choose(&mut self, index: usize, values: &mut Records) -> Option<Patch> {
        let picker = self.roles.picker.take()?;
        let value = picker.choices.get(index)?.clone();
        self.focus = Focus::Control(picker.field.clone());
        match picker.field {
            Field::RoleDraftColor => self.roles.draft.as_mut()?.role.color = value,
            Field::RoleDraftMark => self.roles.draft.as_mut()?.role.mark = value,
            Field::RoleDraftClass => {
                let role = &mut self.roles.draft.as_mut()?.role;
                role.cls = value;
                role.send = class_send(&role.cls).into();
            }
            Field::RoleClass(name) => {
                let mut role = values.roles.iter().find(|(id, _)| *id == name)?.1.clone();
                role.cls = value;
                role.send = class_send(&role.cls).into();
                return self.role_patch(Patch::RoleUpsert { name, role }, values);
            }
            Field::HarnessRole(harness) => {
                return self.role_patch(
                    Patch::HarnessRole {
                        harness,
                        role: value,
                    },
                    values,
                );
            }
            _ => {}
        }
        self.error = None;
        None
    }
    pub(super) fn handle_role_action(
        &mut self,
        action: &Action,
        values: &mut Records,
        defaults: &Records,
    ) -> Option<Option<Patch>> {
        match action {
            Action::Commit => {
                if let Some(picker) = &self.roles.picker {
                    return Some(self.choose(picker.selected, values));
                }
                if self.roles.text.is_some() {
                    return Some(self.commit_role_text(values));
                }
                if let Focus::Control(field) = self.focus.clone() {
                    let activate = Action::Toggle(field);
                    return Some(
                        self.handle_role_action(&activate, values, defaults)
                            .unwrap_or_else(|| self.handle_base(activate, values, defaults)),
                    );
                }
                return Some(None);
            }
            Action::Cancel => {
                if self.roles.picker.take().is_some() {
                    return Some(None);
                }
                if let Some(text) = self.roles.text.take() {
                    self.draft_text(&text.field, &text.original);
                    self.error = None;
                    return Some(None);
                }
                self.roles.draft = None;
                self.focus = Focus::Control(Field::RoleNew);
                self.error = None;
                return Some(None);
            }
            Action::Search(command) if self.roles.text.is_some() => {
                let text = self.roles.text.as_mut().unwrap();
                let previous = text.value.clone();
                let editor = text.editor.clone();
                text.editor.handle(&mut text.value, command.clone());
                let limit = if text.field == Field::RoleDraftName {
                    128
                } else {
                    32 * 1024
                };
                if text.value.len() > limit || text.value.contains('\0') {
                    text.value = previous;
                    text.editor = editor;
                    self.error = Some(format!(
                        "This field is limited to {limit} bytes and cannot contain NUL"
                    ));
                } else {
                    self.error = None;
                }
                let field = text.field.clone();
                let value = text.value.clone();
                self.draft_text(&field, &value);
                return Some(None);
            }
            Action::Focus(Focus::Control(field)) if is_text(field) => {
                if self.role_editable(field, values) {
                    self.begin_role_text(field.clone(), values);
                }
                return Some(None);
            }
            Action::NextFocus(back) if matches!(self.section.as_str(), "Roles" | "Harnesses") => {
                let mut order = vec![Focus::Search, Focus::Navigation];
                order.extend(self.role_fields(values).into_iter().map(Focus::Control));
                let at = order.iter().position(|f| *f == self.focus).unwrap_or(0);
                self.focus =
                    order[(at + if *back { order.len() - 1 } else { 1 }) % order.len()].clone();
                if let Focus::Control(field) = &self.focus {
                    self.begin_role_text(field.clone(), values);
                }
                return Some(None);
            }
            Action::Nudge(delta) if self.roles.picker.is_some() => {
                let picker = self.roles.picker.as_mut().unwrap();
                if !picker.choices.is_empty() {
                    picker.selected = (picker.selected as i32 + delta)
                        .rem_euclid(picker.choices.len() as i32)
                        as usize;
                }
                return Some(None);
            }
            Action::Nudge(delta) => {
                let Focus::Control(field) = &self.focus else {
                    return None;
                };
                if let Some((choices, value)) = self.choices(field, values) {
                    if choices.is_empty() {
                        return Some(None);
                    }
                    let at = choices.iter().position(|v| *v == value).unwrap_or(0);
                    self.roles.picker = Some(Picker {
                        field: field.clone(),
                        choices,
                        selected: at,
                    });
                    let picker = self.roles.picker.as_mut().unwrap();
                    picker.selected = (picker.selected as i32 + delta)
                        .rem_euclid(picker.choices.len() as i32)
                        as usize;
                    return Some(None);
                }
            }
            Action::Toggle(field) if self.role_editable(field, values) => {
                self.focus = Focus::Control(field.clone());
                match field {
                    Field::RoleNew => {
                        self.roles.draft = Some(DraftRole::default());
                        self.begin_role_text(Field::RoleDraftName, values);
                    }
                    Field::RoleCancel => {
                        self.roles.text = None;
                        self.roles.draft = None;
                        self.focus = Focus::Control(Field::RoleNew);
                    }
                    Field::RoleAdd => {
                        let draft = self.roles.draft.as_ref()?;
                        let name = draft.name.trim().to_owned();
                        if !valid_name(&name)
                            || builtin(&name)
                            || values.roles.iter().any(|(id, _)| *id == name)
                        {
                            self.error = Some("Choose a unique role name (1–128 bytes); built-in names are reserved".into());
                            self.begin_role_text(Field::RoleDraftName, values);
                            return Some(None);
                        }
                        let patch = self.role_patch(
                            Patch::RoleUpsert {
                                name,
                                role: draft.role.clone(),
                            },
                            values,
                        );
                        if patch.is_some() {
                            self.roles.draft = None;
                            self.roles.text = None;
                            self.focus = Focus::Control(Field::RoleNew);
                        }
                        return Some(patch);
                    }
                    Field::RoleDelete(name) => {
                        return Some(self.role_patch(Patch::RoleDelete(name.clone()), values));
                    }
                    Field::RoleDraftName
                    | Field::RoleDraftPrompt
                    | Field::RolePrompt(_)
                    | Field::Machine(_, _)
                    | Field::MachineNew => self.begin_role_text(field.clone(), values),
                    Field::Choice(index) => return Some(self.choose(*index, values)),
                    Field::HarnessAvailableRole { harness, role } => {
                        let h = &values.harness.iter().find(|(id, _)| id == harness)?.1;
                        let mut allowed: BTreeSet<_> = h.available_roles.iter().cloned().collect();
                        allowed.insert(h.role.clone());
                        if !allowed.remove(role) {
                            allowed.insert(role.clone());
                        }
                        return Some(self.role_patch(
                            Patch::HarnessAvailableRoles {
                                harness: harness.clone(),
                                roles: allowed,
                            },
                            values,
                        ));
                    }
                    _ => {
                        if let Some(picker) = &self.roles.picker
                            && picker.field == *field
                        {
                            return Some(self.choose(picker.selected, values));
                        }
                        if let Some((choices, value)) = self.choices(field, values) {
                            let selected = choices.iter().position(|v| *v == value).unwrap_or(0);
                            self.roles.picker = Some(Picker {
                                field: field.clone(),
                                choices,
                                selected,
                            });
                        }
                    }
                }
                self.error = None;
                return Some(None);
            }
            Action::Reset(Field::RolePrompt(name)) => {
                if let Some((_, role)) = defaults.roles.iter().find(|(id, _)| id == name) {
                    let mut current = values.roles.iter().find(|(id, _)| id == name)?.1.clone();
                    current.prompt = role.prompt.clone();
                    return Some(self.role_patch(
                        Patch::RoleUpsert {
                            name: name.clone(),
                            role: current,
                        },
                        values,
                    ));
                }
                return Some(None);
            }
            Action::Reset(Field::HarnessRole(name)) => {
                if let Some((_, h)) = defaults.harness.iter().find(|(id, _)| id == name)
                    && values.harness.iter().any(|(id, current)| {
                        id == name && current.available_roles.contains(&h.role)
                    })
                {
                    return Some(self.role_patch(
                        Patch::HarnessRole {
                            harness: name.clone(),
                            role: h.role.clone(),
                        },
                        values,
                    ));
                }
                return Some(None);
            }
            Action::Set(
                patch @ (Patch::MachineLaunch { .. }
                | Patch::RoleUpsert { .. }
                | Patch::RoleDelete(_)
                | Patch::HarnessRole { .. }
                | Patch::HarnessAvailableRoles { .. }),
            ) => return Some(self.role_patch(patch.clone(), values)),
            _ => {}
        }
        None
    }
}

impl Regions {
    pub(super) fn role_controls(
        &mut self,
        layout: &c::Layout,
        section: &str,
        view: &super::super::Settings,
    ) {
        let mut add = |key: String, field: Field| {
            if let Some(node) = find(layout, &key) {
                self.hits.push((node.rect, Hit::Toggle(field), true));
            }
        };
        if section == "Machines" {
            add("machine/new".into(), Field::MachineNew);
            for id in view.values.machine_defaults.keys() {
                for (key, _) in super::super::machines::FIELDS {
                    add(
                        super::super::machines::region(id, key),
                        Field::Machine(id.clone(), key.into()),
                    );
                }
            }
        }
        if section == "Roles" {
            for (name, role) in &view.values.roles {
                add(role_region(name, "prompt"), Field::RolePrompt(name.clone()));
                add(role_region(name, "class"), Field::RoleClass(name.clone()));
                if role.custom {
                    add(role_region(name, "delete"), Field::RoleDelete(name.clone()));
                }
            }
            for (name, field) in [
                ("new", Field::RoleNew),
                ("draft/color", Field::RoleDraftColor),
                ("draft/name", Field::RoleDraftName),
                ("draft/mark", Field::RoleDraftMark),
                ("draft/prompt", Field::RoleDraftPrompt),
                ("draft/class", Field::RoleDraftClass),
                ("draft/add", Field::RoleAdd),
                ("draft/cancel", Field::RoleCancel),
            ] {
                add(format!("role/{name}"), field);
            }
        }
        if section == "Harnesses" {
            for (name, h) in &view.values.harness {
                add(
                    format!("harness/{name}/role"),
                    Field::HarnessRole(name.clone()),
                );
                for role in h.eligible_roles.iter().filter(|role| **role != h.role) {
                    add(
                        format!("harness/{name}/available/{role}"),
                        Field::HarnessAvailableRole {
                            harness: name.clone(),
                            role: role.clone(),
                        },
                    );
                }
            }
        }
        if section == "Roles" {
            for (name, _) in &view.values.roles {
                if let Some(node) = find(layout, &role_region(name, "reset")) {
                    self.hits
                        .push((node.rect, Hit::Reset(Field::RolePrompt(name.clone())), true));
                }
            }
        }
        if section == "Harnesses" {
            for (name, _) in &view.values.harness {
                if let Some(node) = find(layout, &format!("harness/{name}/reset")) {
                    self.hits.push((
                        node.rect,
                        Hit::Reset(Field::HarnessRole(name.clone())),
                        true,
                    ));
                }
            }
        }
    }
    pub(super) fn role_picker_controls(&mut self, layout: &c::Layout) {
        if let Some(menu) = find(layout, "settings/choices") {
            for index in 0..256 {
                if let Some(node) = find(menu, &format!("settings/choice/{index}")) {
                    let r = node.rect;
                    let y = r.y.max(menu.rect.y);
                    let bottom = r.bottom().min(menu.rect.bottom());
                    if bottom > y {
                        self.hits.push((
                            Rect::new(r.x, y, r.w, bottom - y),
                            Hit::Toggle(Field::Choice(index)),
                            false,
                        ));
                    }
                }
            }
        }
    }
}
