// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Trust is operator-owned data. This kit never reads a store or grants trust.

use crate::{
    Refusal,
    policy::{FailurePolicy, Tier},
    shape::{PluginName, Shape},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Payload and blocking authority are separate from a contract's base tier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Grants {
    #[serde(default)]
    pub payload_events: BTreeSet<String>,
    #[serde(default)]
    pub block: Option<FailurePolicy>,
    /// Host API methods the operator approved (see `shape::HOST_API`).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub host_api: BTreeSet<String>,
    /// Message destinations (`seats/<seat>` or a `swarms/<swarm>/…` channel)
    /// a plugin granted `messages.send` may post to, matched exactly.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub messages_to: BTreeSet<String>,
}

/// `payload_events` entry granting the payload of every payload-carrying hook
/// the trusted shape declares (the operator's `--grant payload_events`).
pub const ALL_REQUESTED: &str = "*";

impl Grants {
    /// Every extra authority `shape` asks for: the payload of each
    /// payload-carrying hook (`bus` for bus hooks) and each requested host
    /// API method. A block grant and message destinations are never
    /// requested by a shape; only the operator chooses them.
    pub fn requested_by(shape: &crate::shape::Shape) -> Self {
        Self {
            payload_events: shape
                .hooks
                .iter()
                .filter(|hook| hook.is_bus() || hook.carries_payload().unwrap_or(false))
                .map(|hook| hook.event.clone())
                .collect(),
            block: None,
            host_api: shape.host_api.iter().cloned().collect(),
            messages_to: BTreeSet::new(),
        }
    }

    /// Grants are only ever a subset of what the trusted shape requests: a
    /// payload event of a hook it declares, a host API method it asks for,
    /// a block grant only with a `before` hook, and message destinations
    /// only with `messages.send`.
    pub fn within(&self, shape: &crate::shape::Shape) -> Result<(), Refusal> {
        let mut requested = Self::requested_by(shape);
        requested.payload_events.insert(ALL_REQUESTED.into());
        if let Some(event) = self
            .payload_events
            .difference(&requested.payload_events)
            .next()
        {
            return Err(Refusal::new(
                "grant_not_requested",
                format!("payload_events:{event}"),
            ));
        }
        if let Some(method) = self.host_api.difference(&requested.host_api).next() {
            return Err(Refusal::new(
                "grant_not_requested",
                format!("host_api:{method}"),
            ));
        }
        if self.block.is_some()
            && !shape
                .hooks
                .iter()
                .any(|hook| hook.phase == crate::policy::Phase::Before)
        {
            return Err(Refusal::new("grant_not_requested", "block: no before hook"));
        }
        if !self.messages_to.is_empty() && !self.host_api.contains("messages.send") {
            return Err(Refusal::new(
                "grant_not_requested",
                "messages_to needs the messages.send grant",
            ));
        }
        Ok(())
    }
}

/// The account record pins one executable, its approved policy and cached shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustRecord {
    pub sha256: String,
    pub source: String,
    pub copy: String,
    pub data_dir: String,
    pub name: PluginName,
    pub scope: String,
    pub approved_tier: Tier,
    #[serde(default)]
    pub grants: Grants,
    #[serde(default)]
    pub shape: Option<Shape>,
}

impl TrustRecord {
    /// A descriptor cannot quietly exceed what the operator approved.
    pub fn validate_shape(&self, shape: &Shape) -> Result<(), Refusal> {
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Refusal::new("invalid_digest", "expected lowercase SHA-256"));
        }
        if shape.validate(&self.name)? > self.approved_tier {
            return Err(Refusal::new(
                "tier_not_approved",
                "effective tier exceeds the operator grant",
            ));
        }
        self.grants.within(shape)?;
        for hook in &shape.hooks {
            if hook.carries_payload()? && !self.payload_granted(hook) {
                return Err(Refusal::new("payload_grant_required", &hook.event));
            }
            // Bus payloads are opt-in per grant; with it the plugin exports them.
            if hook.is_bus() && self.payload_granted(hook) && self.approved_tier < Tier::Export {
                return Err(Refusal::new(
                    "tier_not_approved",
                    "a bus payload grant needs the export tier",
                ));
            }
        }
        Ok(())
    }

    /// Whether the plugin may call host API `method`: the trusted shape
    /// requests it and the operator granted it.
    pub fn host_api_granted(&self, method: &str) -> bool {
        self.grants.host_api.contains(method)
            && self
                .shape
                .as_ref()
                .is_some_and(|shape| shape.host_api.iter().any(|m| m == method))
    }

    /// Whether `hook`'s event payload may reach the plugin.
    pub fn payload_granted(&self, hook: &crate::policy::Hook) -> bool {
        self.grants.payload_events.contains(&hook.event)
            || self.grants.payload_events.contains(ALL_REQUESTED)
    }
}
