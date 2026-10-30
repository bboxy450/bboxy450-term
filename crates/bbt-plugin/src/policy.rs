// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App-owned tiers, hook ceilings and the subprocess environment policy.

use serde::{Deserialize, Serialize};

use crate::Refusal;

/// Ordered by the maximum authority/data exposure the operator approves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Read,
    Write,
    Export,
    Transport,
}

/// The minimum authenticated class of a contract method, never a shape claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleClass {
    Worker,
    Oracle,
    HostOnly,
}

/// Hook order is relative to the harness action, not other plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Before,
    After,
}

/// A block grant records the operator's choice when the blocker fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailurePolicy {
    FailOpen,
    FailClosed,
}

/// The hook event that subscribes to bus subjects instead of a harness event.
pub const BUS_EVENT: &str = "bus";

/// First subject tokens a bus hook may never observe: private inboxes, and
/// plugin activity (a hook outcome must not trigger another hook).
const PRIVATE_SUBJECTS: &[&str] = &["inbox", "plugin"];

/// Event names compare without case, `_` or `-`: `PreToolUse` = `pre_tool_use`.
pub fn normalized_event(event: &str) -> String {
    event
        .bytes()
        .filter(|b| *b != b'_' && *b != b'-')
        .map(|b| char::from(b.to_ascii_lowercase()))
        .collect()
}

/// A bus subject or pattern: `/`-separated, at most 256 bytes and 16 tokens;
/// in a pattern `*` is one whole token and `>` the whole last token.
pub fn validate_subject(subject: &str, pattern: bool) -> Result<(), Refusal> {
    let invalid = |why: &str| Refusal::new("invalid_subject", format!("{subject:?}: {why}"));
    let tokens: Vec<_> = subject.split('/').collect();
    if subject.is_empty() || subject.len() > 256 || tokens.len() > 16 {
        return Err(invalid("1..=256 bytes and at most 16 tokens"));
    }
    for (i, token) in tokens.iter().enumerate() {
        if token.is_empty() || token.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(invalid("empty token or whitespace"));
        }
        if token.contains(['*', '>'])
            && !(pattern && (*token == "*" || (*token == ">" && i + 1 == tokens.len())))
        {
            return Err(invalid("misplaced wildcard"));
        }
    }
    Ok(())
}

/// Whether `subject` matches `pattern` (the bus's `*` and `>` wildcards).
pub fn subject_matches(pattern: &str, subject: &str) -> bool {
    let mut subjects = subject.split('/');
    for token in pattern.split('/') {
        if token == ">" {
            return subjects.next().is_some();
        }
        let Some(part) = subjects.next() else {
            return false;
        };
        if token != "*" && token != part {
            return false;
        }
    }
    subjects.next().is_none()
}

/// A hook request: a harness event (`PreToolUse`, ...) at a phase, or the
/// `bus` event with a `subject` pattern. Delivered by the host as the plugin
/// MCP tool `hook` within `min(budget_ms, ceiling_ms)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hook {
    pub event: String,
    pub phase: Phase,
    pub budget_ms: u64,
    /// Bus hooks only: the subject pattern, e.g. `task/*/status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl Hook {
    /// Whether this is a bus-subject hook rather than a harness event hook.
    pub fn is_bus(&self) -> bool {
        normalized_event(&self.event) == BUS_EVENT
    }

    /// Whether a harness `event` (any spelling) is this hook's event.
    pub fn matches_event(&self, event: &str) -> bool {
        !self.is_bus() && normalized_event(&self.event) == normalized_event(event)
    }

    fn category(&self) -> Result<(bool, bool), Refusal> {
        match normalized_event(&self.event).as_str() {
            "pretooluse" | "posttooluse" | "permission" | "permissionrequest" => Ok((true, false)),
            "sessionend" | "interrupt" => Ok((false, true)),
            "sessionstart" | "seatstate" => Ok((false, false)),
            "promptsubmit" | "userpromptsubmit" => Ok((true, false)),
            // A bus envelope's payload is delivered only with the `bus`
            // payload_events grant, so subscribing alone exposes no payload.
            BUS_EVENT => Ok((false, false)),
            _ => Err(Refusal::new(
                "unknown_hook",
                format!("unknown hook event {}", self.event),
            )),
        }
    }

    /// Payload subscriptions require an explicit grant and at least export tier.
    pub fn carries_payload(&self) -> Result<bool, Refusal> {
        Ok(self.category()?.0)
    }

    /// The app's ceiling is independent of any timeout a plugin advertises.
    pub fn ceiling_ms(&self) -> Result<u64, Refusal> {
        let (_, ending) = self.category()?;
        if ending {
            return Ok(1_000);
        }
        let tool = matches!(
            self.event
                .to_ascii_lowercase()
                .replace(['_', '-'], "")
                .as_str(),
            "pretooluse" | "posttooluse" | "permission" | "permissionrequest"
        );
        Ok(if tool && self.phase == Phase::Before {
            2_000
        } else {
            5_000
        })
    }

    /// Validate the default ceiling; longer budgets need a later operator grant.
    /// A bus hook is `after` with a subject pattern that never starts at a
    /// private namespace (`inbox`, `plugin`) or a wildcard; harness hooks
    /// carry no subject.
    pub fn validate(&self) -> Result<(), Refusal> {
        let ceiling = self.ceiling_ms()?;
        match (&self.subject, self.is_bus()) {
            (Some(subject), true) => {
                validate_subject(subject, true)?;
                let first = subject.split('/').next().unwrap_or_default();
                if first == "*" || first == ">" || PRIVATE_SUBJECTS.contains(&first) {
                    return Err(Refusal::new(
                        "invalid_subject",
                        format!("{subject:?} must start with a public namespace"),
                    ));
                }
                if self.phase != Phase::After {
                    return Err(Refusal::new(
                        "invalid_hook",
                        "bus hooks observe published events: phase must be after",
                    ));
                }
            }
            (None, true) => {
                return Err(Refusal::new("invalid_hook", "a bus hook needs a subject"));
            }
            (Some(_), false) => {
                return Err(Refusal::new(
                    "invalid_hook",
                    format!("{} hooks take no subject", self.event),
                ));
            }
            (None, false) => {}
        }
        if self.budget_ms == 0 || self.budget_ms > ceiling {
            return Err(Refusal::new(
                "hook_budget_exceeded",
                format!(
                    "{} {:?}: {} ms exceeds the 1..={ceiling} ms budget",
                    self.event, self.phase, self.budget_ms
                ),
            ));
        }
        Ok(())
    }
}

/// Only these inherited names may reach a plugin; credential-like names lose.
pub fn allowed_env_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    if ["TOKEN", "KEY", "SECRET"]
        .iter()
        .any(|part| upper.contains(part))
        || upper.starts_with("BBOXY_")
        || upper.starts_with("ICQ_")
    {
        return false;
    }
    matches!(name, "PATH" | "HOME" | "LANG" | "TMPDIR" | "TERM") || name.starts_with("LC_")
}

/// Pure filtering; callers own collection and spawning and must not log values.
pub fn filter_environment<'a>(
    values: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> std::collections::BTreeMap<String, String> {
    values
        .into_iter()
        .filter(|(key, _)| allowed_env_name(key))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}
