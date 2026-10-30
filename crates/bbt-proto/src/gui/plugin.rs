// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Operator controls for the account-owned production plugin registry.
use super::*;
pub use bbt_plugin::registry::{Entry as PluginEntry, State as PluginState};
pub use bbt_plugin::{
    policy::{FailurePolicy as PluginFailurePolicy, Phase as PluginPhase, Tier as PluginTier},
    shape::Shape as PluginShape,
    trust::{
        ALL_REQUESTED as PLUGIN_ALL_REQUESTED, Grants as PluginGrants,
        TrustRecord as PluginTrustRecord,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginRequest {
    List,
    /// Only a discovered path is eligible; this never imports an arbitrary executable.
    /// Trusting again with different grants is a new trust generation.
    Trust {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
        /// Grants the operator approves; only ever a subset of what the
        /// trusted shape requests (refused otherwise at inspection).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        grants: Option<bbt_plugin::trust::Grants>,
        /// Approved tier; when absent, the contract minimum raised by the grants.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tier: Option<bbt_plugin::policy::Tier>,
    },
    Enable {
        candidate: String,
    },
    Disable {
        candidate: String,
    },
    /// Check a trusted pin in a disposable fixture; never grants trust or enable.
    Check {
        candidate: String,
    },
}

impl PluginRequest {
    pub fn mutation(&self) -> bool {
        !matches!(self, Self::List)
    }
}

macro_rules! plugin_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Plugin {
                id: Ulid,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                scope: Option<SwarmScope>,
                request: PluginRequest,
            },
        ] $($head)*);
    };
}
pub(super) use plugin_actions;

macro_rules! plugin_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Plugin { op: Ulid, entries: Vec<bbt_plugin::registry::Entry> },
        ] $($head)*);
    };
}
pub(super) use plugin_replies;
