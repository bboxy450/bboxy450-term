// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Stable serializable seat metadata; independent of registry storage.
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::messaging::{MessageClass, WakeMode};

/// Account-owned swarm metadata; window layout is a separate projection.
/// An absent workdir retains the startup repository as the default. New paths
/// are canonical existing directories; legacy absolute paths retain their
/// supplied spelling, including unavailable folders that a root mount refuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmRecord {
    pub id: Ulid,
    pub name: Option<String>,
    pub workdir: Option<String>,
    pub revision: u64,
}
impl SwarmRecord {
    pub fn new(id: Ulid) -> Self {
        Self {
            id,
            name: None,
            workdir: None,
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub class: Class,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Oracle,
    Worker,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    Local,
    Ssh { host: String },
    Cloud { provider: String, host: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub swarm: Ulid,
    pub tab: Ulid,
    pub split_path: Vec<usize>,
    pub ratios: Vec<f64>,
    pub layout_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseRecord {
    pub operation: Ulid,
    pub revision: u64,
    pub deadline_ms: u64,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Open,
    ClosedGrace,
    ClosedRetained,
}

/// Receiver overrides; omission on a seat selects its role defaults (#334).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionPolicy {
    pub rules: Vec<AttentionRule>,
}

/// One exact destination and signed message class, validated by the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionRule {
    pub destination: String,
    pub class: MessageClass,
    pub mode: WakeMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatRecord {
    pub id: Ulid,
    pub swarm: Ulid,
    pub name: String,
    pub harness_kind: String,
    pub harness_session_id: Option<String>,
    pub role: Role,
    #[serde(rename = "where")]
    pub location: Location,
    pub muted: bool,
    pub generation: u64,
    pub lifecycle: Lifecycle,
    pub lifecycle_revision: u64,
    pub close: Option<CloseRecord>,
    pub tasks: Vec<Ulid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<AttentionPolicy>,
    /// Durable initial placement for seats launched while no window is attached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_placement: Option<crate::seat_state::Placement>,
}

impl SeatRecord {
    pub fn new(
        id: Ulid,
        swarm: Ulid,
        name: impl Into<String>,
        harness_kind: impl Into<String>,
    ) -> Self {
        Self {
            id,
            swarm,
            name: name.into(),
            harness_kind: harness_kind.into(),
            harness_session_id: None,
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            location: Location::Local,
            muted: false,
            generation: 1,
            lifecycle: Lifecycle::Open,
            lifecycle_revision: 0,
            close: None,
            tasks: Vec::new(),
            attention: None,
            launch_placement: None,
        }
    }
}
