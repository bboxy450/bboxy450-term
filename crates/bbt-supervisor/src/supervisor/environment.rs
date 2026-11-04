// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Caller-selected exact/prefix environment removals at the exec boundary.

use std::ffi::OsStr;
use std::process::Command;

/// Ordinary launch overrides may carry space-separated names or trailing-`*`
/// prefixes here. The host consumes this control value before exec; it never
/// records the inherited environment or chooses program-specific rules.
pub const UNSET_RULES: &str = "BBOXY_LAUNCH_ENV_UNSET";

fn selected(key: &OsStr, rules: &str) -> bool {
    let key = key.as_encoded_bytes();
    rules
        .split_ascii_whitespace()
        .any(|rule| match rule.strip_suffix('*') {
            Some(prefix) => key.starts_with(prefix.as_bytes()),
            None => key == rule.as_bytes(),
        })
}

/// Apply the caller's removal rules after all explicit overrides, consuming
/// the control value. An explicit removal of the control overrides inheritance.
pub fn scrub_command(command: &mut Command) {
    let rules = match command.get_envs().find(|(key, _)| *key == UNSET_RULES) {
        Some((_, value)) => value
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_default(),
        None => std::env::var(UNSET_RULES).unwrap_or_default(),
    };
    let keys: Vec<_> = std::env::vars_os()
        .map(|(key, _)| key)
        .chain(command.get_envs().map(|(key, _)| key.to_owned()))
        .filter(|key| selected(key, &rules))
        .collect();
    for key in keys {
        command.env_remove(key);
    }
    command.env_remove(UNSET_RULES);
}

pub(super) fn scrub_pty(command: &mut portable_pty::CommandBuilder) {
    let rules = command
        .get_env(UNSET_RULES)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let keys: Vec<_> = std::env::vars_os()
        .map(|(key, _)| key)
        .chain(command.iter_full_env_as_str().map(|(key, _)| key.into()))
        .filter(|key| selected(key, &rules))
        .collect();
    for key in keys {
        command.env_remove(key);
    }
    command.env_remove(UNSET_RULES);
}
