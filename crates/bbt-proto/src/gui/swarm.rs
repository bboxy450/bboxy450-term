// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Authoritative swarm metadata through the admitted operator GUI/ctl door.
/// Independently versions the Swarm* verbs, without changing window layout data.
pub const SWARM_METADATA_VERSION: u16 = 1;
/// Scoped workspace calls require a root-aware control owner.
pub const SWARM_WORKSPACE_VERSION: u16 = 1;

/// Operator-selected swarm context; root identity and paths remain host-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwarmScope {
    pub swarm: ulid::Ulid,
    pub revision: u64,
}

macro_rules! swarm_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            SwarmList { id: Ulid },
            SwarmGet { id: Ulid, swarm: Ulid },
            SwarmRename { id: Ulid, swarm: Ulid, name: String },
            /// A canonical existing directory; None restores the startup fallback.
            SwarmWorkdir { id: Ulid, swarm: Ulid, path: Option<String> },
        ] $($head)*);
    };
}
pub(super) use swarm_actions;

macro_rules! swarm_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Swarms { op: Ulid, records: Vec<crate::registry::SwarmRecord> },
            Swarm { op: Ulid, record: Option<crate::registry::SwarmRecord> },
        ] $($head)*);
    };
}
pub(super) use swarm_replies;
