// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Account activation is separate from trust; discovery never grants either.

use serde::{Deserialize, Serialize};

use crate::{
    Refusal,
    policy::Tier,
    shape::{PluginName, Shape},
    trust::{Grants, TrustRecord},
};

/// The only automatically discovered locations. Neither is an authority source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Application,
    Repository,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Repository => "repository",
        }
    }
}

/// A file observation, including refused entries so the registry explains them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub source: String,
    pub scope: Scope,
    pub name: Option<PluginName>,
    pub sha256: Option<String>,
    pub refusal: Option<Refusal>,
}

/// Explicit approval of policy; a shape can never widen this value itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub tier: Tier,
    #[serde(default)]
    pub grants: Grants,
}

impl Approval {
    /// New trust starts at the named contract's fixed minimum, with no extras.
    pub fn minimum(name: &PluginName) -> Self {
        Self {
            tier: name.system.tier(),
            grants: Grants::default(),
        }
    }

    /// The operator's grants at the contract minimum raised by what they
    /// expose: a payload or a `records.*` grant needs export, a write host
    /// API method (or message destinations) write.
    pub fn implied(name: &PluginName, grants: Grants) -> Self {
        let mut tier = name.system.tier();
        if !grants.payload_events.is_empty() {
            tier = tier.max(Tier::Export);
        }
        for method in &grants.host_api {
            if let Some((_, raised)) = crate::shape::HOST_API.iter().find(|(m, _)| m == method) {
                tier = tier.max(*raised);
            }
        }
        if !grants.messages_to.is_empty() {
            tier = tier.max(Tier::Write);
        }
        Self { tier, grants }
    }
}

/// Actual descriptor inspection, not a production conformance-test claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inspection {
    pub shape: Option<Shape>,
    pub refusal: Option<Refusal>,
    pub elapsed_ms: u64,
    pub stderr: String,
    pub stderr_truncated: bool,
}

/// Isolated contract checks are observations, never production trust or enable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conformance {
    pub sha256: String,
    pub ok: bool,
    pub checked_at: u64,
    pub checks: Vec<String>,
    pub refusal: Option<Refusal>,
}

/// Persisted in the operator account. A digest alone is never an activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activation {
    pub candidate: String,
    pub source: String,
    pub scope: Scope,
    pub name: PluginName,
    pub sha256: String,
    pub generation: u64,
    pub desired_enabled: bool,
    pub needs_retrust: bool,
    /// An admitted inspection without a recorded outcome; never implies health.
    #[serde(default)]
    pub inspection_started: bool,
    pub inspection: Option<Inspection>,
}

/// A read projection. Hosts must recheck account authority before executing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub candidate: Candidate,
    pub activation: Option<Activation>,
    pub trust: Option<TrustRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conformance: Option<Conformance>,
    /// Host-side measurements of this plugin's recent calls and hook
    /// deliveries, oldest first; present only in a live inventory listing,
    /// never in the account record. Empty means nothing was measured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measurements: Vec<Sample>,
}

/// How one measured call or hook delivery ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The plugin answered and the host accepted the result.
    Ok,
    /// The plugin answered with a named refusal (`code`).
    Refused,
    /// The host failed the call (`code`): transport, deadline, schema, authority.
    Failed,
    /// The host did not deliver it (`code`): queue full, stale activation, busy.
    Skipped,
}

/// Plugin trace spans the host accepts with one result
/// (`result._meta.bboxy450.spans`, only when the trusted shape declares
/// `trace: true`).
pub const TRACE_SPANS: usize = 32;
/// Longest accepted plugin span name, in characters.
pub const TRACE_SPAN_NAME: usize = 64;

/// Plugin trace spans the host refused for one call, with the first reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSkip {
    /// `trace_spans_invalid`, `trace_spans_oversized`, `trace_span_invalid`
    /// or `trace_span_outside_window`.
    pub code: String,
    pub count: u32,
}

/// One phase of a measured call, relative to the call's start: the host's
/// own phases, and accepted plugin spans (depth 2) inside the `plugin` phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub name: String,
    pub start_ms: f64,
    pub ms: f64,
    /// 0 is the whole call; 1 the host's phases; 2 the plugin's own spans.
    pub depth: u32,
}

/// One measured plugin call or hook delivery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Milliseconds between the call's end and the listing snapshot.
    pub ago_ms: f64,
    /// The wire method (`<contract>_<method>`, `tool_<name>` or `hook`).
    pub method: String,
    /// The harness event or bus subject of a hook delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    pub ms: f64,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<Span>,
    /// Plugin trace spans dropped as malformed or outside the plugin phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_skip: Option<SpanSkip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Refused,
    Untrusted,
    NeedsRetrust,
    Unchecked,
    Disabled,
    Enabled,
}

impl Entry {
    pub fn state(&self) -> State {
        if self.candidate.refusal.is_some() {
            return State::Refused;
        }
        let Some(activation) = &self.activation else {
            return State::Untrusted;
        };
        if activation.needs_retrust {
            return State::NeedsRetrust;
        }
        let Some(trust) = &self.trust else {
            return State::Untrusted;
        };
        if activation
            .inspection
            .as_ref()
            .is_some_and(|i| i.refusal.is_some())
        {
            return State::Refused;
        }
        if trust.shape.is_none() {
            return State::Unchecked;
        }
        if activation.desired_enabled {
            State::Enabled
        } else {
            State::Disabled
        }
    }
}
