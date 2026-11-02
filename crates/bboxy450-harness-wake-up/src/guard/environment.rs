// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One session-identity policy for native runtimes, app/backend starts and hosts.
use std::{collections::BTreeSet, ffi::OsStr, process::Command};

/// Generic process-host control key; its value contains names, never secrets.
/// Hosts consume it after launch overrides and before exec.
pub const UNSET_RULES: &str = "BBOXY_LAUNCH_ENV_UNSET";

/// Caller session identities, with exact spellings for older native `env -u`
/// launchers and prefixes for current host exec snapshots. Authentication,
/// homes, provider/model configuration and persistence preferences stay intact.
pub const SESSION_RULES: &str = concat!(
    "CLAUDECODE CLAUDE_PID CLAUDE_CODE_ENTRYPOINT ",
    "CLAUDE_CODE_CHILD_SESSION CLAUDE_CODE_CHILD_* ",
    "CLAUDE_CODE_SESSION_ID CLAUDE_CODE_SESSION_ATTENDED CLAUDE_CODE_SESSION_* ",
    "CLAUDE_CODE_PARENT_SESSION_ID CLAUDE_CODE_PARENT_SESSION_* CLAUDE_CODE_SUBAGENT_* ",
    "CLAUDE_CODE_MESSAGING_SOCKET CLAUDE_CODE_MESSAGING_TOKEN CLAUDE_CODE_MESSAGING_* ",
    "CLAUDE_CODE_BRIDGE_SESSION_ID CLAUDE_CODE_BRIDGE_SESSION_* ",
    "CLAUDE_CODE_REMOTE_SESSION_ID CLAUDE_CODE_REMOTE_SESSION_* ",
    "CLAUDE_CODE_CONTAINER_ID CLAUDE_CODE_WORKER_EPOCH CLAUDE_CODE_EXECPATH ",
    "CLAUDE_CODE_INVOKED_SKILLS AI_AGENT ",
    "CODEX_THREAD_ID CODEX_THREAD_* CODEX_SESSION_ID CODEX_SESSION_* CODEX_TURN_ID ",
    "PI_SESSION_ID PI_SESSION_FILE PI_SESSION_* OMP_SESSION_ID OMP_SESSION_FILE OMP_SESSION_* ",
    "ICQ_CODEX_SOCKET ICQ_HARNESS_DESCRIPTOR",
);

fn selected(key: &OsStr) -> bool {
    let key = key.as_encoded_bytes();
    SESSION_RULES
        .split_ascii_whitespace()
        .any(|rule| match rule.strip_suffix('*') {
            Some(prefix) => key.starts_with(prefix.as_bytes()),
            None => key == rule.as_bytes(),
        })
}

/// Scrub a new app/backend command without changing the process-global
/// environment. Unlike night fixtures, normal launches retain user config.
pub fn scrub_command(command: &mut Command) {
    let keys: Vec<_> = std::env::vars_os()
        .map(|(key, _)| key)
        .chain(command.get_envs().map(|(key, _)| key.to_owned()))
        .filter(|key| selected(key))
        .collect();
    for key in keys {
        command.env_remove(key);
    }
    command.env_remove(UNSET_RULES);
}

/// Exact rule names and current matching names for same-PID native `env -u`.
/// The host also matches prefixes against its own captured exec environment.
pub(crate) fn removal_keys() -> Vec<String> {
    SESSION_RULES
        .split_ascii_whitespace()
        .filter(|key| !key.ends_with('*'))
        .map(str::to_owned)
        .chain(
            std::env::vars_os()
                .filter(|(key, _)| selected(key))
                .filter_map(|(key, _)| key.into_string().ok()),
        )
        .chain([UNSET_RULES.to_owned()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
