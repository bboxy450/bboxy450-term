//! Operator-authored role settings supplied to the runtime as a derived snapshot.
//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
            (
                "claude",
                "oracle",
                vec!["oracle", "architect", "creativity", "executor"],
            ),
            ("codex", "executor", vec!["creativity", "executor"]),
            ("pi", "executor", vec!["creativity", "executor"]),
            ("omp", "executor", vec!["creativity", "executor"]),
            ("zsh", "shell", vec!["shell"]),
        ]
        .into_iter()
        .map(|(kind, default, available)| {
            (
                kind.into(),
                HarnessRoles {
                    default_role: default.into(),
                    available_roles: available.into_iter().map(String::from).collect(),
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
        for (name, definition) in &builtin.roles {
            if self
                .roles
                .get(name)
                .is_none_or(|role| role.class != definition.class)
            {
                return Err(format!("builtin_role_class_required: {name}"));
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
                let role = self
                    .roles
                    .get(name)
                    .ok_or_else(|| format!("unknown_available_role: {name}"))?;
                if !seen.insert(name) {
                    return Err(format!("duplicate_available_role: {name}"));
                }
                if kind == "zsh" {
                    if name != "shell" {
                        return Err("zsh_requires_shell_role".into());
                    }
                } else if name == "shell" {
                    return Err("shell_role_requires_zsh".into());
                } else if kind != "claude" && role.class == Class::Oracle {
                    return Err(if kind == "codex" {
                        "codex_oracle_role_forbidden"
                    } else {
                        "harness_oracle_role_forbidden"
                    }
                    .into());
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
    fn builtin_roles_and_custom_classes_are_explicit_and_harness_bounded() {
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
        assert_eq!(
            catalog.validate().unwrap_err(),
            "codex_oracle_role_forbidden"
        );
    }
    #[test]
    fn role_catalog_rejects_missing_defaults_duplicates_and_builtin_class_changes() {
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
        assert!(
            catalog
                .validate()
                .unwrap_err()
                .starts_with("builtin_role_class_required")
        );
        assert!(
            serde_json::from_value::<RoleDefinition>(
                serde_json::json!({"prompt":"No implicit class"})
            )
            .is_err()
        );
    }
}
