// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public references to retained events. Display projections do not replace
//! the original signed envelope or certify its account authority.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// A sequence belongs to one object's stream; it is never a global cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPosition {
    pub stream: String,
    #[serde(with = "decimal")]
    pub sequence: u64,
}

/// The canonical retained row that a display projection refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReference {
    #[serde(flatten)]
    pub position: EventPosition,
    pub hash: String,
}

/// These signatures cover the original event, including its original payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventSignatureScope {
    OriginalRetainedEnvelope,
}

/// Original proof bytes, encoded as base64 for the public GUI wire.
/// Integrity verification is separate from root, grant or signer authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventProof {
    pub scope: EventSignatureScope,
    pub public_key: String,
    pub signature: String,
    pub prev: String,
}

/// Original event metadata plus the reference and proof for its full payload.
/// `seat` is attribution; it does not identify the key which signed the row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub reference: EventReference,
    pub id: Ulid,
    #[serde(with = "decimal")]
    pub ts: u64,
    pub subject: String,
    pub source: String,
    pub seat: Option<Ulid>,
    #[serde(default, with = "optional_decimal")]
    pub generation: Option<u64>,
    pub proof: EventProof,
}

/// A visible omitted range in one stream, not a fabricated signed event.
/// The reason distinguishes projection or window bounds from retained history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventGap {
    pub stream: String,
    #[serde(with = "decimal")]
    pub first: u64,
    #[serde(with = "decimal")]
    pub last: u64,
    pub reason: String,
}

/// Selected activity fields. Raw hooks, terminal bytes and command arguments
/// are not part of this projection; the digest refers to the original payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub action: String,
    pub id: Option<Ulid>,
    pub path: Option<String>,
    pub path_redacted: bool,
    pub digest: Option<String>,
    pub before_digest: Option<String>,
    pub grade: Option<String>,
    pub producer: Option<String>,
    pub candidates: Vec<Ulid>,
    pub status: Option<String>,
    pub previous_status: Option<String>,
    pub outcome: Option<String>,
    pub reason: Option<String>,
    pub position: Option<EventPosition>,
    pub payload_digest: String,
}

/// State payloads are sampled after the original notice. Their current state
/// does not retroactively certify that notice's signer or original payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum GuiEventPayload {
    Seat {
        seat: Box<crate::GuiSeat>,
    },
    Identity {
        identity: crate::GuiIdentity,
    },
    Host {
        host: crate::GuiHost,
        transition: Option<crate::HostState>,
    },
    Activity {
        summary: ActivitySummary,
    },
    /// Shared W2.4 projection, resolved under the GUI's operator authority.
    /// A receipt push carries the same row with its current receipt summary.
    Message {
        row: crate::messaging::MessageRow,
    },
}

/// A public display projection and the proof/reference for its retained source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiEventEnvelope {
    pub metadata: EventMetadata,
    #[serde(with = "decimal")]
    pub projected_at: u64,
    /// A derived transport/read snapshot, not part of the original signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<GuiEventCursor>,
    pub payload: GuiEventPayload,
}

/// Retained pages and the app's bounded window use the same row/gap vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum GuiEventRecord {
    Event(Box<GuiEventEnvelope>),
    Gap(EventGap),
}

/// Transport positions are distinct from operator-read state. The ACK anchor
/// starts at the head for a new consumer and then follows published events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiEventCursor {
    pub stream: String,
    #[serde(with = "decimal")]
    pub admitted: u64,
    #[serde(with = "decimal")]
    pub acknowledged: u64,
    #[serde(with = "decimal")]
    pub read: u64,
    #[serde(with = "decimal")]
    pub high: u64,
    #[serde(with = "decimal")]
    pub unread: u64,
}

/// An ordered, byte-bounded page from one retained stream. It creates no
/// consumer and does not acknowledge delivery or mark anything as read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiEventPage {
    pub records: Vec<GuiEventRecord>,
    #[serde(default, with = "optional_decimal")]
    pub next: Option<u64>,
    pub cursor: GuiEventCursor,
}

pub(crate) mod decimal {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let value = String::deserialize(deserializer)?;
        parse(&value).map_err(D::Error::custom)
    }

    pub(super) fn parse(value: &str) -> Result<u64, &'static str> {
        if value.is_empty()
            || !value.bytes().all(|byte| byte.is_ascii_digit())
            || (value.len() > 1 && value.starts_with('0'))
        {
            return Err("event position must be a canonical unsigned decimal string");
        }
        value.parse().map_err(|_| "event position exceeds u64")
    }
}

mod optional_decimal {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|value| super::decimal::parse(&value).map_err(D::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn positions_keep_every_u64_bit_and_their_stream() {
        for sequence in [0, 9_007_199_254_740_993, u64::MAX] {
            let position = EventPosition {
                stream: "index".into(),
                sequence,
            };
            let value = serde_json::to_value(&position).unwrap();
            assert_eq!(value["sequence"], sequence.to_string());
            assert_eq!(
                serde_json::from_value::<EventPosition>(value).unwrap(),
                position
            );
            assert_ne!(
                position,
                EventPosition {
                    stream: "account".into(),
                    sequence,
                }
            );
        }
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn numeric_loss_and_ambiguous_positions_are_rejected() {
        for value in [
            json!(1),
            json!(""),
            json!("01"),
            json!("+1"),
            json!("-1"),
            json!("1.0"),
            json!("18446744073709551616"),
        ] {
            assert!(
                serde_json::from_value::<EventPosition>(json!({
                    "stream": "index", "sequence": value
                }))
                .is_err()
            );
        }
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn projection_metadata_names_the_original_signature_scope() {
        let metadata = EventMetadata {
            reference: EventReference {
                position: EventPosition {
                    stream: "index".into(),
                    sequence: 3,
                },
                hash: "canonical-hash".into(),
            },
            id: Ulid::from(3_u128),
            ts: u64::MAX,
            subject: "artifact/fixture/created".into(),
            source: "ephemeral-hold/fixture/provenance".into(),
            seat: Some(Ulid::from(4_u128)),
            generation: Some(u64::MAX),
            proof: EventProof {
                scope: EventSignatureScope::OriginalRetainedEnvelope,
                public_key: "public-key".into(),
                signature: "original-signature".into(),
                prev: "previous-hash".into(),
            },
        };
        let value = serde_json::to_value(&metadata).unwrap();
        assert_eq!(value["reference"]["stream"], "index");
        assert_eq!(value["reference"]["sequence"], "3");
        assert_eq!(value["generation"], u64::MAX.to_string());
        assert_eq!(value["proof"]["scope"], "original-retained-envelope");
        assert_eq!(
            serde_json::from_value::<EventMetadata>(value).unwrap(),
            metadata
        );
        let mut unbound = metadata;
        unbound.generation = None;
        let value = serde_json::to_value(&unbound).unwrap();
        assert!(value["generation"].is_null());
        assert_eq!(
            serde_json::from_value::<EventMetadata>(value).unwrap(),
            unbound
        );
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn gaps_are_stream_ranges_without_a_signature() {
        let gap = EventGap {
            stream: "index".into(),
            first: 2,
            last: 9,
            reason: "app_window_evicted".into(),
        };
        let value = serde_json::to_value(&gap).unwrap();
        assert_eq!(value["first"], "2");
        assert_eq!(value["last"], "9");
        assert!(value.get("proof").is_none());
        assert_eq!(serde_json::from_value::<EventGap>(value).unwrap(), gap);
    }
}
