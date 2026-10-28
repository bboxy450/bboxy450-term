// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Operator-authored role settings supplied to the runtime as a derived snapshot.
use super::*;
use crate::registry::Class;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDefinition {
    pub class: Class,
    #[serde(default)]
    pub prompt: String,
    /// Descriptive Settings metadata; destination grants remain separately authoritative.
    #[serde(default)]
    pub send: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub mark: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessRoles {
    pub default_role: String,
    pub available_roles: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// bboxy450-term role-catalog authority: https://github.com/bboxy450/bboxy450-term
// Credit copied/adapted/translated reuse accurately. Policy:
// https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
pub struct RoleCatalog {
    pub roles: BTreeMap<String, RoleDefinition>,
    pub harnesses: BTreeMap<String, HarnessRoles>,
}
impl Default for RoleCatalog {
    fn default() -> Self {
        Self::builtin()
    }
}
impl RoleCatalog {
    pub fn builtin() -> Self {
        use Class::{Oracle, Worker};
        let roles = [
            ("oracle", Oracle, "Supervise the swarm. Brief executors, verify evidence, and coordinate work.", "#F7C442", "♔"),
            ("architect", Oracle, "Define interfaces, specifications, invariants, and the evidence that verifies them.", "#A78BFA", "♜"),
            ("creativity", Worker, "Explore alternatives and supply creative possibilities within the assigned brief.", "#FF69B4", "♝"),
            ("executor", Worker, "Implement scoped assignments, run verification, and report evidence to the Oracle.", "#43D9A3", "♟"),
            ("shell", Worker, "", "#7397D4", ""),
        ].into_iter().map(|(name, class, prompt, color, mark)| (name.into(), RoleDefinition {
            class, prompt: prompt.into(), send: String::new(), color: color.into(), mark: mark.into(),
        })).collect();
        let harnesses = [
            ("claude", "oracle"),
            ("codex", "executor"),
            ("pi", "executor"),
            ("omp", "executor"),
            ("zsh", "shell"),
        ]
        .into_iter()
        .map(|(kind, default)| {
            (
                kind.into(),
                HarnessRoles {
                    default_role: default.into(),
                    available_roles: ["oracle", "architect", "creativity", "executor", "shell"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                },
            )
        })
        .collect();
        Self { roles, harnesses }
    }
    pub fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 128
            && name.trim() == name
            && !name.chars().any(char::is_control)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.roles.len() > 128 || self.harnesses.len() > 128 {
            return Err("role_catalog_too_large".into());
        }
        let builtin = Self::builtin();
        for name in builtin.roles.keys() {
            if !self.roles.contains_key(name) {
                return Err(format!("builtin_role_required: {name}"));
            }
        }
        for (name, role) in &self.roles {
            if !Self::valid_name(name)
                || builtin
                    .roles
                    .keys()
                    .any(|key| key != name && key.eq_ignore_ascii_case(name))
                || role.prompt.len() > 32 * 1024
                || role.prompt.contains('\0')
                || role.send.len() > 4096
                || role.send.contains('\0')
                || role.color.len() > 128
                || role.color.contains('\0')
                || role.mark.len() > 128
                || role.mark.contains('\0')
            {
                return Err(format!("invalid_role_definition: {name}"));
            }
        }
        for kind in builtin.harnesses.keys() {
            if !self.harnesses.contains_key(kind) {
                return Err(format!("builtin_harness_roles_required: {kind}"));
            }
        }
        for (kind, policy) in &self.harnesses {
            if !Self::valid_name(kind)
                || policy.available_roles.is_empty()
                || policy.available_roles.len() > 128
            {
                return Err(format!("invalid_harness_roles: {kind}"));
            }
            let mut seen = BTreeSet::new();
            for name in &policy.available_roles {
                if !self.roles.contains_key(name) {
                    return Err(format!("unknown_available_role: {name}"));
                }
                if !seen.insert(name) {
                    return Err(format!("duplicate_available_role: {name}"));
                }
            }
            if !policy.available_roles.contains(&policy.default_role) {
                return Err(format!("default_role_not_available: {kind}"));
            }
        }
        if serde_json::to_vec(self)
            .map_err(|error| error.to_string())?
            .len()
            > 512 * 1024
        {
            return Err("role_catalog_too_large".into());
        }
        Ok(())
    }
}

macro_rules! role_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            ConfigureRoles { id: Ulid, catalog: RoleCatalog },
        ] $($head)*);
    };
}
pub(super) use role_actions;
macro_rules! role_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            RolesConfigured { id: Ulid },
        ] $($head)*);
    };
}
pub(super) use role_replies;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_harness_accepts_explicit_builtin_and_custom_role_classes() {
        let mut catalog = RoleCatalog::default();
        catalog.validate().unwrap();
        assert_eq!(catalog.roles["oracle"].class, Class::Oracle);
        let mut custom = catalog.roles["executor"].clone();
        custom.prompt = "Review changes and cite evidence.".into();
        catalog.roles.insert("Code reviewer".into(), custom.clone());
        catalog
            .harnesses
            .get_mut("codex")
            .unwrap()
            .available_roles
            .push("Code reviewer".into());
        catalog.harnesses.get_mut("codex").unwrap().default_role = "Code reviewer".into();
        catalog.validate().unwrap();
        custom.class = Class::Oracle;
        catalog.roles.insert("Coordinator".into(), custom);
        catalog
            .harnesses
            .get_mut("claude")
            .unwrap()
            .available_roles
            .push("Coordinator".into());
        catalog.validate().unwrap();
        catalog
            .harnesses
            .get_mut("codex")
            .unwrap()
            .available_roles
            .push("Coordinator".into());
        catalog.harnesses.get_mut("codex").unwrap().default_role = "Coordinator".into();
        catalog.validate().unwrap();
        for kind in ["pi", "omp", "custom"] {
            let mut configured = catalog.clone();
            configured.harnesses.insert(
                kind.into(),
                HarnessRoles {
                    default_role: "Coordinator".into(),
                    available_roles: vec!["Coordinator".into()],
                },
            );
            configured.validate().unwrap();
        }
    }
    #[test]
    fn role_catalog_rejects_missing_defaults_duplicates_and_missing_builtin_roles() {
        let mut catalog = RoleCatalog::default();
        catalog.harnesses.get_mut("claude").unwrap().default_role = "absent".into();
        assert!(
            catalog
                .validate()
                .unwrap_err()
                .starts_with("default_role_not_available")
        );
        let mut catalog = RoleCatalog::default();
        catalog
            .harnesses
            .get_mut("claude")
            .unwrap()
            .available_roles
            .push("oracle".into());
        assert!(
            catalog
                .validate()
                .unwrap_err()
                .starts_with("duplicate_available_role")
        );
        let mut catalog = RoleCatalog::default();
        catalog.roles.get_mut("executor").unwrap().class = Class::Oracle;
        catalog.roles.get_mut("oracle").unwrap().class = Class::Worker;
        catalog.validate().unwrap();
        catalog.roles.remove("executor");
        assert!(
            catalog
                .validate()
                .unwrap_err()
                .starts_with("builtin_role_required")
        );
        assert!(
            serde_json::from_value::<RoleDefinition>(
                serde_json::json!({"prompt":"No implicit class"})
            )
            .is_err()
        );
    }
}
