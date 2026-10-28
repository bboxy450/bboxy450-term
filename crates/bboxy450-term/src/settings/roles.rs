// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Settings role definitions and harness membership share one typed catalog.
use super::{Patch, Preferences};
use bbt_proto::{
    gui::{HarnessRoles, RoleCatalog, RoleDefinition},
    registry::Class,
};
use bbt_ui::modules::settings::Role;
use std::collections::BTreeMap;
use toml_edit::{DocumentMut, Item, Value};

pub const BUILTINS: [&str; 5] = ["oracle", "architect", "creativity", "executor", "shell"];
pub const HARNESSES: [&str; 5] = ["claude", "codex", "pi", "omp", "zsh"];

pub fn builtin() -> BTreeMap<String, RoleDefinition> {
    RoleCatalog::builtin().roles
}

pub fn names(roles: &BTreeMap<String, RoleDefinition>) -> Vec<String> {
    BUILTINS
        .into_iter()
        .filter(|name| roles.contains_key(*name))
        .map(str::to_owned)
        .chain(
            roles
                .keys()
                .filter(|name| !BUILTINS.contains(&name.as_str()))
                .cloned(),
        )
        .collect()
}

pub fn eligible(_harness: &str, roles: &BTreeMap<String, RoleDefinition>) -> Vec<String> {
    names(roles)
}

pub fn view(name: &str, role: &RoleDefinition) -> Role {
    Role {
        prompt: role.prompt.clone(),
        send: role.send.clone(),
        color: role.color.clone(),
        mark: role.mark.clone(),
        cls: class_name(role.class).into(),
        custom: !BUILTINS.contains(&name),
    }
}

pub fn class_name(class: Class) -> &'static str {
    match class {
        Class::Oracle => "oracle",
        Class::Worker => "worker",
    }
}

pub fn definition(role: &Role) -> Result<RoleDefinition, String> {
    Ok(RoleDefinition {
        class: match role.cls.as_str() {
            "oracle" => Class::Oracle,
            "worker" => Class::Worker,
            _ => return Err("Role class must be oracle or worker".into()),
        },
        prompt: role.prompt.clone(),
        send: role.send.clone(),
        color: role.color.clone(),
        mark: role.mark.clone(),
    })
}

pub(super) fn read(doc: &DocumentMut, preferences: &mut Preferences) -> Result<(), String> {
    let Some(roles) = doc.get("roles") else {
        return Ok(());
    };
    let roles = roles
        .as_table_like()
        .ok_or("Settings [roles] must be a table")?;
    for (name, item) in roles.iter() {
        let fields = item
            .as_table_like()
            .ok_or("Role settings must be a table")?;
        let mut role = preferences
            .roles
            .get(name)
            .cloned()
            .unwrap_or(RoleDefinition {
                class: Class::Worker,
                prompt: String::new(),
                send: "own swarm · operator DM".into(),
                color: "#7397d4".into(),
                mark: "♜".into(),
            });
        if let Some(class) = fields.get("class") {
            role.class = match class.as_str() {
                Some("oracle") => Class::Oracle,
                Some("worker") => Class::Worker,
                _ => return Err("Role class must be oracle or worker".into()),
            };
        }
        for (key, value) in [
            ("prompt", &mut role.prompt),
            ("send", &mut role.send),
            ("color", &mut role.color),
            ("mark", &mut role.mark),
        ] {
            if let Some(item) = fields.get(key) {
                *value = item
                    .as_str()
                    .ok_or_else(|| format!("Role {key} must be text"))?
                    .into();
            }
        }
        preferences.roles.insert(name.into(), role);
    }
    Ok(())
}

pub(super) fn replace(slot: &mut Item, mut value: Value) {
    if let Some(old) = slot.as_value() {
        *value.decor_mut() = old.decor().clone();
    }
    *slot = Item::Value(value);
}

pub(super) fn write(doc: &mut DocumentMut, patch: &Patch) -> Result<bool, String> {
    match patch {
        Patch::RoleUpsert { name, role } => {
            let role = definition(role)?;
            for (key, value) in [
                ("class", class_name(role.class)),
                ("prompt", role.prompt.as_str()),
                ("send", role.send.as_str()),
                ("color", role.color.as_str()),
                ("mark", role.mark.as_str()),
            ] {
                replace(&mut doc["roles"][name][key], Value::from(value));
            }
        }
        Patch::RoleDelete(name) => {
            if BUILTINS.contains(&name.as_str()) {
                return Err("Built-in roles cannot be deleted".into());
            }
            if let Some(roles) = doc.get_mut("roles").and_then(Item::as_table_like_mut) {
                roles.remove(name);
            }
        }
        Patch::HarnessRole { harness, role } => replace(
            &mut doc["harnesses"][harness]["role"],
            Value::from(role.clone()),
        ),
        Patch::HarnessAvailableRoles { harness, roles } => {
            let slot = &mut doc["harnesses"][harness]["allowed_roles"];
            let array = slot.as_array().map_or_else(
                || roles.iter().map(String::as_str).collect(),
                |old| super::harness_array(old, roles),
            );
            replace(slot, Value::Array(array));
        }
        _ => return Ok(false),
    }
    Ok(true)
}

impl Preferences {
    /// Effective policies are captured with definitions before backend admission.
    pub fn role_catalog(&self) -> RoleCatalog {
        let mut harnesses = BTreeMap::new();
        for harness in HARNESSES.into_iter().map(str::to_owned).chain(
            self.harnesses
                .keys()
                .filter(|name| !HARNESSES.contains(&name.as_str()))
                .cloned(),
        ) {
            let defaults = self.harnesses.get(&harness);
            let default_role = defaults
                .and_then(|entry| entry.role.clone())
                .unwrap_or_else(|| {
                    match harness.as_str() {
                        "claude" => "oracle",
                        "zsh" => "shell",
                        _ => "executor",
                    }
                    .into()
                });
            let available_roles = defaults
                .and_then(|entry| entry.allowed_roles.clone())
                .unwrap_or_else(|| eligible(&harness, &self.roles));
            harnesses.insert(
                harness,
                HarnessRoles {
                    default_role,
                    available_roles,
                },
            );
        }
        RoleCatalog {
            roles: self.roles.clone(),
            harnesses,
        }
    }

    pub(super) fn apply_role_patch(&mut self, patch: &Patch) -> Result<(), String> {
        match patch {
            Patch::RoleUpsert { name, role } => {
                self.roles.insert(name.clone(), definition(role)?);
            }
            Patch::RoleDelete(name) => {
                if BUILTINS.contains(&name.as_str()) {
                    return Err("Built-in roles cannot be deleted".into());
                }
                if !self.roles.contains_key(name) {
                    return Err("Unknown role".into());
                }
                if self.harnesses.values().any(|harness| {
                    harness.role.as_ref() == Some(name)
                        || harness
                            .allowed_roles
                            .as_ref()
                            .is_some_and(|roles| roles.contains(name))
                }) {
                    return Err("Role is referenced by a harness; change its defaults and availability first".into());
                }
                self.roles.remove(name);
            }
            Patch::HarnessRole { harness, role } => {
                self.harnesses.entry(harness.clone()).or_default().role = Some(role.clone());
            }
            Patch::HarnessAvailableRoles { harness, roles } => {
                self.harnesses
                    .entry(harness.clone())
                    .or_default()
                    .allowed_roles = Some(roles.iter().cloned().collect());
            }
            _ => {}
        }
        Ok(())
    }
}
