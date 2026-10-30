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
