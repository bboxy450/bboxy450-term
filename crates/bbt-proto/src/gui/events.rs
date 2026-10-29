// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The durable events drain and read-only paging of retained streams.
use super::*;

/// A current-state bootstrap or ordered per-seat change, never retained history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttentionUpdate {
    pub epoch: Ulid,
    pub revision: u64,
    pub snapshot: bool,
    pub seats: Vec<GuiSeat>,
    pub removed: Vec<Ulid>,
}

/// One retained event as `EventsRead` pages it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetainedEvent {
    pub seq: u64,
    pub subject: String,
    pub source: String,
    pub ts: u64,
    pub payload: serde_json::Value,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! events_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Subscribe to current waiting state without a durable events cursor.
            ObserveAttention { id: Ulid },
            /// Start the one device-bound durable events drain on this connection.
            AttachEvents {
                id: Ulid,
            },
            GetEvents {
                stream: String,
                #[serde(with = "crate::events::decimal")]
                after: u64,
                limit: usize,
            },
            GetEventCursor {
                stream: String,
            },
            /// ACK follows publication to the model, not a socket write.
            EventAck {
                stream: String,
                #[serde(with = "crate::events::decimal")]
                seq: u64,
            },
            /// Only an operator view/mark-read advances this independent watermark.
            MarkEventsRead {
                stream: String,
                #[serde(with = "crate::events::decimal")]
                seq: u64,
            },
            /// Page a retained stream by the caller's cursor (ctl-cli §1a, #272): read-only, no
            /// attachment, no ack, never a second consumer of the operator's lane; filtered to the
            /// subjects the operator's lane may see.
            EventsRead {
                id: Ulid,
                stream: String,
                after: u64,
                limit: usize,
                /// Wait for an append, at most 60 000ms; omission preserves the old wire.
                #[serde(default, skip_serializing_if = "Option::is_none")]
                wait_ms: Option<u64>,
            },
            /// Discover retained stream names through the same operator admission.
            /// This creates no consumer; EventsRead still filters every returned event.
            EventsStreams {
                id: Ulid,
            },
        ] $($head)*);
    };
}
pub(super) use events_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! events_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Attention { op: Option<Ulid>, update: AttentionUpdate },
            /// Asynchronous attachment transitions, never a terminal action result.
            EventStatus {
                attached: bool,
                reason: Option<String>,
            },
            Event {
                stream: String,
                #[serde(with = "crate::events::decimal")]
                seq: u64,
                subject: String,
                envelope: Box<crate::events::GuiEventEnvelope>,
            },
            Gap {
                stream: String,
                #[serde(with = "crate::events::decimal")]
                first: u64,
                #[serde(with = "crate::events::decimal")]
                last: u64,
                reason: String,
            },
            Events {
                page: crate::events::GuiEventPage,
            },
            EventCursor {
                cursor: crate::events::GuiEventCursor,
            },
            /// A page of retained events, oldest first; `next` is the cursor to continue from.
            EventsRead {
                op: Ulid,
                stream: String,
                events: Vec<RetainedEvent>,
                next: Option<u64>,
                /// Last scanned sequence, including rows hidden by the subject filter.
                #[serde(default)]
                through: u64,
            },
            EventsStreams {
                op: Ulid,
                streams: Vec<String>,
            },
        ] $($head)*);
    };
}
pub(super) use events_replies;
