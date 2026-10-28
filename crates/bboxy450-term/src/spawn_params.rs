// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Local launch resolution. All values are captured before the IO owner sees a spawn.
use bbt_ui::modules::launcher::{FieldKey, Source, SpawnField, ValueChoice};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HarnessDefaults {
    pub role: Option<String>,
    pub allowed_roles: Option<Vec<String>>,
    pub command: Option<String>,
    pub arguments: Option<Vec<String>>,
}
pub type Harnesses = BTreeMap<String, HarnessDefaults>;

pub fn resolve(
    harness: &str,
    settings: &Harnesses,
    swarm: Option<&bbt_ui_model::Swarm>,
    fallback: &str,
) -> Vec<SpawnField> {
    let preferences = crate::settings::Preferences {
        harnesses: settings.clone(),
        ..Default::default()
    };
    resolve_with_catalog(
        harness,
        settings,
        &preferences.role_catalog(),
        swarm,
        fallback,
    )
}

// Adapted from bboxy450-term's own web launchFields/composeLaunch preview.
// bboxy450-term launcher parameter resolution: https://github.com/bboxy450/bboxy450-term
// Credit copied/adapted/translated reuse accurately. Policy:
// https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
pub fn resolve_with_catalog(
    harness: &str,
    settings: &Harnesses,
    catalog: &bbt_proto::gui::RoleCatalog,
    swarm: Option<&bbt_ui_model::Swarm>,
    fallback: &str,
) -> Vec<SpawnField> {
    let defaults = settings.get(harness);
    let builtin = bbt_mux::adapters::HarnessRecord::builtin(harness).ok();
    let field = |key, value, source, choices, fixed| SpawnField {
        key,
        value,
        source,
        choices,
        fixed,
    };
    let mut out = vec![];
    {
        let eligible = crate::settings::roles::eligible(harness, &catalog.roles);
        let policy = catalog.harnesses.get(harness);
        let available =
            policy.map_or_else(|| eligible.clone(), |policy| policy.available_roles.clone());
        let choices: Vec<_> = available
            .into_iter()
            .filter(|name| eligible.contains(name))
            .map(|value| {
                let (label, purpose) = match value.as_str() {
                    "oracle" => ("Oracle".into(), "supervises, briefs, verifies".into()),
                    "architect" => ("Architect".into(), "specs, invariants, decisions".into()),
                    "creativity" => ("Creativity".into(), "diverges before the pick".into()),
                    "executor" => ("Executor".into(), "implements a scoped brief".into()),
                    _ => (
                        value.clone(),
                        catalog.roles[&value]
                            .prompt
                            .lines()
                            .next()
                            .unwrap_or("")
                            .chars()
                            .take(120)
                            .collect(),
                    ),
                };
                ValueChoice {
                    value,
                    label,
                    purpose,
                }
            })
            .collect();
        let preferred =
            policy
                .map(|policy| policy.default_role.as_str())
                .unwrap_or(if harness == "claude" {
                    "oracle"
                } else {
                    "executor"
                });
        let fallback_role = if harness == "claude" {
            "oracle"
        } else {
            "executor"
        };
        let role = choices
            .iter()
            .find(|choice| choice.value == preferred)
            .or_else(|| choices.first())
            .map_or(fallback_role, |choice| choice.value.as_str());
        let fixed = choices.len() <= 1;
        out.push(field(
            FieldKey::Role,
            role.into(),
            Source::Settings,
            Some(choices),
            fixed,
        ));
    }
    let cwd = swarm.and_then(|s| s.workdir.as_deref()).unwrap_or(fallback);
    let mut recent = vec![cwd.to_owned()];
    for value in swarm
        .into_iter()
        .flat_map(|s| &s.recent_directories)
        .map(String::as_str)
        .chain([fallback])
    {
        if !recent.iter().any(|p| p == value) {
            recent.push(value.into());
        }
    }
    out.push(field(
        FieldKey::Directory,
        cwd.into(),
        if swarm.is_some_and(|s| s.workdir.is_some()) {
            Source::LastChoice
        } else {
            Source::BuiltIn
        },
        Some(
            recent
                .into_iter()
                .map(|value| ValueChoice {
                    label: value.clone(),
                    value,
                    purpose: String::new(),
                })
                .collect(),
        ),
        false,
    ));
    out.push(field(
        FieldKey::Command,
        defaults.and_then(|d| d.command.clone()).unwrap_or_else(|| {
            builtin
                .as_ref()
                .map_or(harness, |h| h.command[0].as_str())
                .into()
        }),
        if defaults.is_some_and(|d| d.command.is_some()) {
            Source::LastChoice
        } else {
            Source::Settings
        },
        None,
        false,
    ));
    out.push(field(
        FieldKey::Arguments,
        defaults
            .and_then(|d| d.arguments.as_ref())
            .map_or(String::new(), |a| join(a)),
        if defaults.is_some_and(|d| d.arguments.is_some()) {
            Source::LastChoice
        } else {
            Source::BuiltIn
        },
        None,
        false,
    ));
    out.push(field(
        FieldKey::Environment,
        swarm
            .and_then(|s| s.launch_environment.as_ref())
            .map_or(String::new(), |e| {
                join(
                    &e.iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>(),
                )
            }),
        if swarm.is_some_and(|s| s.launch_environment.is_some()) {
            Source::LastChoice
        } else {
            Source::BuiltIn
        },
        None,
        false,
    ));
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub role: Option<String>,
    pub cwd: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}
impl Resolved {
    pub fn from_fields(fields: &[SpawnField], home: &str) -> Result<Self, String> {
        let get = |key| {
            fields
                .iter()
                .find(|f| f.key == key)
                .map(|f| f.value.as_str())
                .unwrap_or("")
        };
        let expand = |value: &str| {
            if value == "~" {
                home.into()
            } else if let Some(rest) = value.strip_prefix("~/") {
                format!("{home}/{rest}")
            } else {
                value.into()
            }
        };
        let command = expand(get(FieldKey::Command).trim());
        if command.is_empty() || command.contains('\0') {
            return Err("Command must name an executable".into());
        }
        let cwd = expand(get(FieldKey::Directory).trim());
        if !bbt_proto::gui::valid_workdir(&cwd) {
            return Err("Directory must be an absolute path".into());
        }
        let args = shell_words::split(get(FieldKey::Arguments))
            .map_err(|_| "Arguments contain an unfinished quote")?;
        let env = shell_words::split(get(FieldKey::Environment))
            .map_err(|_| "Environment contains an unfinished quote")?
            .into_iter()
            .map(|pair| {
                let (key, value) = pair.split_once('=').ok_or("Environment uses KEY=value")?;
                if key.is_empty()
                    || key.starts_with("BBOXY_")
                    || key.starts_with(|c: char| c.is_ascii_digit())
                    || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || pair.contains('\0')
                {
                    return Err("Environment uses KEY=value".to_owned());
                }
                Ok((key.into(), value.into()))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if args.iter().any(|v| v.contains('\0')) {
            return Err("Arguments contain a NUL".into());
        }
        Ok(Self {
            role: fields
                .iter()
                .find(|f| f.key == FieldKey::Role)
                .map(|f| f.value.clone()),
            cwd,
            command,
            args,
            env,
        })
    }
    pub fn argv(&self) -> Vec<String> {
        std::iter::once(self.command.clone())
            .chain(self.args.clone())
            .collect()
    }
    pub fn runs(&self) -> String {
        let env = if self.env.is_empty() {
            String::new()
        } else {
            format!(
                "env {} ",
                join(
                    &self
                        .env
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                )
            )
        };
        format!("cd {} && {env}{}", quote(&self.cwd), join(&self.argv()))
    }
    pub fn remember(
        &self,
        harness: &str,
        swarm: &mut bbt_ui_model::Swarm,
        settings: &mut Harnesses,
    ) {
        swarm.workdir = Some(self.cwd.clone());
        swarm.launch_environment = Some(self.env.clone());
        swarm.recent_directories.retain(|p| p != &self.cwd);
        swarm.recent_directories.insert(0, self.cwd.clone());
        swarm.recent_directories.truncate(6);
        let defaults = settings.entry(harness.into()).or_default();
        defaults.command = Some(self.command.clone());
        defaults.arguments = Some(self.args.clone());
        // Role belongs to Settings; a launch never changes it.
    }
}
pub fn quote(value: &str) -> String {
    shell_words::quote(value).into_owned()
}
pub fn join(values: &[String]) -> String {
    values
        .iter()
        .map(|v| quote(v))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui_model::{Swarm, SwarmId};
    #[test]
    fn configured_catalog_drives_role_prefill_and_membership_for_each_harness() {
        use bbt_proto::{gui::RoleDefinition, registry::Class};
        use bbt_ui::modules::settings::controller::Patch;
        use std::collections::BTreeSet;
        for harness in ["claude", "codex", "pi", "omp", "zsh", "custom"] {
            for explicit in [false, true] {
                let mut preferences = crate::settings::Preferences::default();
                for (name, class) in [("Reviewer Δ", Class::Worker), ("planner", Class::Oracle)] {
                    preferences.roles.insert(
                        name.into(),
                        RoleDefinition {
                            class,
                            prompt: "Check the diff.\nReturn evidence.".into(),
                            send: "own swarm · operator DM".into(),
                            color: "#7397d4".into(),
                            mark: "♜".into(),
                        },
                    );
                }
                preferences
                    .apply(&Patch::HarnessRole {
                        harness: harness.into(),
                        role: "Reviewer Δ".into(),
                    })
                    .unwrap();
                if explicit {
                    preferences
                        .apply(&Patch::HarnessAvailableRoles {
                            harness: harness.into(),
                            roles: BTreeSet::from(["Reviewer Δ".into()]),
                        })
                        .unwrap();
                }
                let catalog = preferences.role_catalog();
                catalog.validate().unwrap();
                let fields = resolve_with_catalog(
                    harness,
                    &preferences.harnesses,
                    &catalog,
                    None,
                    "/workspace",
                );
                let role = &fields[0];
                assert_eq!(
                    (role.key, role.value.as_str(), role.source),
                    (FieldKey::Role, "Reviewer Δ", Source::Settings)
                );
                assert_eq!(role.fixed, explicit);
                let choices = role.choices.as_ref().unwrap();
                assert_eq!(
                    choices.iter().any(|choice| choice.value == "planner"),
                    !explicit
                );
                assert_eq!(
                    choices
                        .iter()
                        .find(|choice| choice.value == "Reviewer Δ")
                        .unwrap()
                        .purpose,
                    "Check the diff."
                );
                assert_eq!(
                    choices.iter().any(|choice| choice.value == "shell"),
                    !explicit
                );
            }
        }
        let mut configured = bbt_proto::gui::RoleCatalog::builtin();
        let codex = configured.harnesses.get_mut("codex").unwrap();
        codex.default_role = "oracle".into();
        codex.available_roles = vec!["oracle".into()];
        configured.validate().unwrap();
        let fields =
            resolve_with_catalog("codex", &BTreeMap::new(), &configured, None, "/workspace");
        assert_eq!(fields[0].value, "oracle");
        assert!(fields[0].fixed);
        assert_eq!(fields[0].choices.as_ref().unwrap()[0].value, "oracle");
        let shell = resolve_with_catalog(
            "zsh",
            &BTreeMap::new(),
            &bbt_proto::gui::RoleCatalog::builtin(),
            None,
            "/workspace",
        );
        assert_eq!(shell[0].key, FieldKey::Role);
        assert_eq!(shell[0].value, "shell");
    }
    #[test]
    fn resolver_table_prefills_every_local_row_and_names_its_source() {
        for harness in ["claude", "codex", "pi", "omp", "zsh"] {
            for has_folder in [false, true] {
                for has_last in [false, true] {
                    let mut swarm = Swarm::new(SwarmId::from_u128(1), "test", vec![]);
                    if has_folder {
                        swarm.workdir = Some("/swarm".into());
                    }
                    swarm.recent_directories = vec!["/older".into(), "/swarm".into()];
                    if has_last {
                        swarm.launch_environment = Some(vec![("A".into(), "two words".into())]);
                    }
                    let settings = if has_last {
                        BTreeMap::from([(
                            harness.into(),
                            HarnessDefaults {
                                role: Some("creativity".into()),
                                command: Some("/tool".into()),
                                arguments: Some(vec!["--literal".into(), "two words".into()]),
                                ..Default::default()
                            },
                        )])
                    } else {
                        BTreeMap::new()
                    };
                    let rows = resolve(harness, &settings, Some(&swarm), "/app");
                    let mut expected = vec![(
                        FieldKey::Role,
                        if has_last {
                            "creativity"
                        } else if harness == "zsh" {
                            "shell"
                        } else if harness == "claude" {
                            "oracle"
                        } else {
                            "executor"
                        }
                        .into(),
                        Source::Settings,
                    )];
                    expected.extend([
                        (
                            FieldKey::Directory,
                            if has_folder { "/swarm" } else { "/app" }.into(),
                            if has_folder {
                                Source::LastChoice
                            } else {
                                Source::BuiltIn
                            },
                        ),
                        (
                            FieldKey::Command,
                            if has_last {
                                "/tool"
                            } else if harness == "zsh" {
                                "/bin/zsh"
                            } else {
                                harness
                            }
                            .into(),
                            if has_last {
                                Source::LastChoice
                            } else {
                                Source::Settings
                            },
                        ),
                        (
                            FieldKey::Arguments,
                            if has_last {
                                "--literal 'two words'"
                            } else {
                                ""
                            }
                            .into(),
                            if has_last {
                                Source::LastChoice
                            } else {
                                Source::BuiltIn
                            },
                        ),
                        (
                            FieldKey::Environment,
                            if has_last { "'A=two words'" } else { "" }.into(),
                            if has_last {
                                Source::LastChoice
                            } else {
                                Source::BuiltIn
                            },
                        ),
                    ]);
                    assert_eq!(
                        rows.iter()
                            .map(|f| (f.key, f.value.clone(), f.source))
                            .collect::<Vec<_>>(),
                        expected,
                        "{harness} folder={has_folder} last={has_last}"
                    );
                    assert_eq!(
                        rows.iter()
                            .find(|f| f.key == FieldKey::Role)
                            .map(|f| f.fixed),
                        Some(false)
                    );
                    let directories = rows
                        .iter()
                        .find(|f| f.key == FieldKey::Directory)
                        .unwrap()
                        .choices
                        .as_ref()
                        .unwrap();
                    assert_eq!(
                        directories[0].value,
                        if has_folder { "/swarm" } else { "/app" }
                    );
                    assert_eq!(
                        directories.iter().filter(|d| d.value == "/swarm").count(),
                        1
                    );
                }
            }
            let no_swarm = resolve(harness, &BTreeMap::new(), None, "/fallback");
            let folder = no_swarm
                .iter()
                .find(|f| f.key == FieldKey::Directory)
                .unwrap();
            assert_eq!(
                (&folder.value, folder.source),
                (&"/fallback".into(), Source::BuiltIn)
            );
        }
    }
    #[test]
    fn runs_quotes_literal_argv_and_remember_keeps_role_at_settings_home() {
        let mut swarm = Swarm::new(SwarmId::from_u128(1), "test", vec![]);
        let mut settings = BTreeMap::from([(
            "claude".into(),
            HarnessDefaults {
                role: Some("oracle".into()),
                allowed_roles: Some(vec!["oracle".into(), "architect".into()]),
                ..Default::default()
            },
        )]);
        let mut fields = resolve("claude", &settings, Some(&swarm), "/project with spaces");
        for field in &mut fields {
            field.value = match field.key {
                FieldKey::Role => "architect",
                FieldKey::Arguments => "--flag '$(must-stay-literal)' ''",
                FieldKey::Environment => "A='two words' B=",
                _ => continue,
            }
            .into();
        }
        let resolved = Resolved::from_fields(&fields, "/home").unwrap();
        assert_eq!(resolved.args, ["--flag", "$(must-stay-literal)", ""]);
        assert_eq!(
            resolved.runs(),
            "cd '/project with spaces' && env 'A=two words' 'B=' claude --flag '$(must-stay-literal)' ''"
        );
        resolved.remember("claude", &mut swarm, &mut settings);
        assert_eq!(settings["claude"].role.as_deref(), Some("oracle"));
        assert_eq!(
            settings["claude"].allowed_roles.as_deref(),
            Some(["oracle".to_owned(), "architect".to_owned()].as_slice())
        );
        let next = resolve("claude", &settings, Some(&swarm), "/app");
        assert_eq!(next[0].value, "oracle");
        assert!(next[1..].iter().all(|f| f.source == Source::LastChoice));
        fields
            .iter_mut()
            .find(|f| f.key == FieldKey::Environment)
            .unwrap()
            .value = "bad-key=value".into();
        assert!(Resolved::from_fields(&fields, "/home").is_err());
    }
}
