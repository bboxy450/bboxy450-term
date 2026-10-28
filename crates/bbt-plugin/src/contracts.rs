// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Contract identity and fixed authority, shared by every suite app.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Refusal,
    policy::{RoleClass, Tier},
};

/// The seven systems briefed in P-0. Describe-only contracts stay at 0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Contract {
    DataSource,
    TaskSystem,
    EventSink,
    Session,
    RepoGraph,
    DecisionModel,
    Provider,
}

impl Contract {
    /// All known system tokens, also used for longest-token filename matching.
    pub const ALL: [Self; 7] = [
        Self::DataSource,
        Self::TaskSystem,
        Self::EventSink,
        Self::Session,
        Self::RepoGraph,
        Self::DecisionModel,
        Self::Provider,
    ];

    /// A stable wire token; it does not depend on the app's filename prefix.
    pub fn name(self) -> &'static str {
        match self {
            Self::DataSource => "data_source",
            Self::TaskSystem => "task_system",
            Self::EventSink => "event_sink",
            Self::Session => "session",
            Self::RepoGraph => "repo_graph",
            Self::DecisionModel => "decision_model",
            Self::Provider => "provider",
        }
    }

    /// Unknown contracts are a named refusal at trust, before a first call.
    pub fn named(name: &str) -> Result<Self, Refusal> {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == name)
            .ok_or_else(|| Refusal::new("unknown_contract", format!("unknown contract {name}")))
    }

    /// The contract version supported by this host.
    pub fn version(self) -> &'static str {
        match self {
            Self::Session | Self::Provider => "0.1",
            _ => "1.0",
        }
    }

    /// A plugin cannot lower its contract's trust tier.
    pub fn tier(self) -> Tier {
        match self {
            Self::RepoGraph => Tier::Read,
            Self::Session | Self::Provider => Tier::Transport,
            _ => Tier::Export,
        }
    }

    /// Canonical method names, without either the wire or host prefix.
    pub fn methods(self) -> &'static [&'static str] {
        match self {
            Self::DataSource => &[
                "resolve",
                "fetch",
                "search",
                "list",
                "links",
                "graph",
                "write",
                "changes_since",
            ],
            Self::TaskSystem => &[
                "list",
                "status",
                "delegate",
                "reassign",
                "report",
                "accept",
                "reject",
                "changes_since",
            ],
            Self::RepoGraph => &["describe", "changes_since"],
            Self::Session | Self::Provider => &["describe"],
            Self::EventSink => &["deliver"],
            Self::DecisionModel => &["choose"],
        }
    }

    /// Authority comes from the contract; descriptor fields cannot override it.
    pub fn role(self, method: &str) -> Result<RoleClass, Refusal> {
        if !self.methods().contains(&method) {
            return Err(Refusal::new(
                "unknown_method",
                format!("{} has no callable method {method}", self.name()),
            ));
        }
        Ok(if self == Self::EventSink {
            RoleClass::HostOnly
        } else if self == Self::DecisionModel
            || self == Self::TaskSystem
                && matches!(method, "delegate" | "reassign" | "accept" | "reject")
        {
            RoleClass::Oracle
        } else {
            RoleClass::Worker
        })
    }

    /// The plugin's MCP tool name is unambiguous across multiple contracts.
    pub fn wire_name(self, method: &str) -> Result<String, Refusal> {
        self.role(method)?;
        Ok(format!("{}_{method}", self.name()))
    }

    /// Select an app-owned schema; the existing data-source schema is reused.
    pub fn method_schema(self, method: &str, result: bool) -> Option<Value> {
        if self == Self::DataSource {
            return crate::data_source::method_schema(method, result);
        }
        let schema: Value = serde_json::from_str(self.schema()).ok()?;
        let selected =
            schema["x-methods"][method][if result { "result" } else { "request" }].as_str()?;
        Some(
            serde_json::json!({"$schema":schema["$schema"],"$defs":schema["$defs"],"$ref":selected}),
        )
    }

    /// The sole JSON Schema document for this contract and version.
    pub fn schema(self) -> &'static str {
        match self {
            Self::DataSource => crate::data_source::SCHEMA,
            Self::TaskSystem => include_str!("../schemas/task_system-1.0.schema.json"),
            Self::EventSink => include_str!("../schemas/event_sink-1.0.schema.json"),
            Self::Session => include_str!("../schemas/session-0.1.schema.json"),
            Self::RepoGraph => include_str!("../schemas/repo_graph-1.0.schema.json"),
            Self::DecisionModel => include_str!("../schemas/decision_model-1.0.schema.json"),
            Self::Provider => include_str!("../schemas/provider-0.1.schema.json"),
        }
    }
}
