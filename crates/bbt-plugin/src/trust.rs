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
        for hook in &shape.hooks {
            if hook.carries_payload()? && !self.grants.payload_events.contains(&hook.event) {
                return Err(Refusal::new("payload_grant_required", &hook.event));
            }
        }
        Ok(())
    }
}
