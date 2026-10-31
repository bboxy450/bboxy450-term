// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the `event_sink` 1.0 contract
//! (`schemas/event_sink-1.0.schema.json`). `deliver` is host-only: the host
//! fans signed envelopes out to a sink; seats never call it. The envelope is
//! strict (no unknown fields) and signed by the host; a sink forwards it and
//! deduplicates by its `id`, never re-signs or edits it.
use serde::{Deserialize, Serialize};

/// `event_sink` 1.0 method names.
pub const METHODS: [&str; 1] = ["deliver"];

/// Who signed the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssuerKind {
    #[serde(rename = "local-dev")]
    LocalDev,
    #[serde(rename = "gpg-root")]
    GpgRoot,
}

/// Worker or oracle authority class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Worker,
    Oracle,
}

/// The sender's role at signing time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub class: Class,
}

/// The kind of message an envelope carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Text,
    Chunk,
    Decision,
    Presence,
    Task,
}

/// A seat's presence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    Active,
    Idle,
    Waiting,
    Errored,
}

/// The envelope body, discriminated by `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    Text {
        text: String,
    },
    Presence {
        event: String,
        name: String,
        harness: String,
        state: PresenceState,
        muted: bool,
        policy_epoch: String,
    },
    Task {
        task_id: String,
        event: String,
        path: String,
        summary: String,
    },
}

/// Which operator door issued it, when one did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    Gui,
    Ctl,
}

/// One host-signed message envelope (strict: unknown fields are refused).
/// Sequence and generation numbers are decimal strings (u64 range).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub version: u16,
    pub id: String,
    pub issuer_kind: IssuerKind,
    pub issuer: String,
    pub account: String,
    pub device: String,
    pub seat: String,
    pub author: String,
    pub swarm_id: String,
    pub process_generation: String,
    pub context: String,
    pub role: Role,
    pub role_grant_id: String,
    pub role_epoch: String,
    pub priority_class: Class,
    pub sender_sequence: String,
    pub thread_id: String,
    pub reply_to: Option<String>,
    pub timestamp: u64,
    pub priority: u8,
    pub kind: Kind,
    pub body: Body,
    pub attachments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<Door>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued_by: Option<String>,
    pub signature: String,
}

/// `deliver` one envelope published on `subject`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverRequest {
    pub subject: String,
    pub envelope: Envelope,
}

/// Whether the sink took it; `id` must be the envelope's `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverResult {
    pub id: String,
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
