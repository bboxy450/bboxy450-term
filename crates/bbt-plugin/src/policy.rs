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

/// A hook request is data; P-0 never installs or executes hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hook {
    pub event: String,
    pub phase: Phase,
    pub budget_ms: u64,
}

impl Hook {
    fn category(&self) -> Result<(bool, bool), Refusal> {
        let normalized: String = self
            .event
            .bytes()
            .filter(|b| *b != b'_' && *b != b'-')
            .map(|b| char::from(b.to_ascii_lowercase()))
            .collect();
        match normalized.as_str() {
            "pretooluse" | "posttooluse" | "permission" | "permissionrequest" => Ok((true, false)),
            "sessionend" | "interrupt" => Ok((false, true)),
            "sessionstart" | "seatstate" => Ok((false, false)),
            "promptsubmit" | "userpromptsubmit" => Ok((true, false)),
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
    pub fn validate(&self) -> Result<(), Refusal> {
        let ceiling = self.ceiling_ms()?;
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
