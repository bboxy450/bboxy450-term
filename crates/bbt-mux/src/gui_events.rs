//! The GUI receives selected projections and canonical proof references, never
//! arbitrary event payloads or a new signature over a display summary.

use std::path::{Component, Path};

use base64::Engine;
use bbt_proto::events::{
    ActivitySummary, EventMetadata, EventPosition, EventProof, EventReference, EventSignatureScope,
    GuiEventCursor, GuiEventEnvelope, GuiEventPayload,
};
use serde_json::Value;

use crate::{
    Ulid,
    bus::GuiCursor,
    logic::LogicClient,
    store::{Result, digest, events::Envelope, invalid, now_ms},
};

pub(crate) const PAGE_RECORDS: usize = 128;
pub(crate) const PAGE_BYTES: usize = 256 * 1024;
pub(crate) const LIVE_BYTES: usize = 16 * 1024;

pub(crate) fn filters(mut messaging: Vec<String>) -> Vec<String> {
    messaging.extend(
        [
            "seat/*/state",
            "account/identity/state",
            "account/providers/transition",
            "doc/>",
            "activity/>",
            "task/*/status",
            "artifact/>",
            "account/control",
        ]
        .map(str::to_owned),
    );
    messaging.sort();
    messaging.dedup();
    messaging
}

pub(crate) fn cursor(value: GuiCursor) -> GuiEventCursor {
    GuiEventCursor {
        stream: value.stream,
        admitted: value.admitted,
        acknowledged: value.published,
        read: value.read,
        high: value.high,
        unread: value.unread,
    }
}

pub(crate) fn metadata(stream: &str, envelope: &Envelope) -> Result<EventMetadata> {
    envelope.verify()?;
    if envelope.seq == 0
        || stream.len() > 128
        || envelope.event.subject.len() > 256
        || envelope.event.source.len() > 512
    {
        return Err(invalid("GUI event metadata exceeds projection bounds"));
    }
    Ok(EventMetadata {
        reference: EventReference {
            position: EventPosition {
                stream: stream.into(),
                sequence: envelope.seq,
            },
            hash: envelope.hash()?,
        },
        id: envelope.event.id,
        ts: envelope.event.ts,
        subject: envelope.event.subject.clone(),
        source: envelope.event.source.clone(),
        seat: envelope.event.seat,
        generation: envelope.event.generation,
        proof: EventProof {
            scope: EventSignatureScope::OriginalRetainedEnvelope,
            public_key: base64::engine::general_purpose::STANDARD
                .encode(envelope.provenance.signer),
            signature: base64::engine::general_purpose::STANDARD.encode(&envelope.provenance.sig),
            prev: envelope.prev.clone(),
        },
    })
}

fn text(value: &Value, limit: usize) -> Option<String> {
    value
        .as_str()
        .filter(|value| value.len() <= limit)
        .map(str::to_owned)
}
fn id(value: &Value) -> Option<Ulid> {
    value.as_str()?.parse().ok()
}
fn hash(value: &Value) -> Option<String> {
    let value = value.as_str()?;
    let bytes = value.strip_prefix("sha256:").unwrap_or(value);
    (bytes.len() == 64 && bytes.bytes().all(|b| b.is_ascii_hexdigit())).then(|| value.to_owned())
}
fn path(value: &Value) -> (Option<String>, bool) {
    let Some(value) = value.as_str() else {
        return (None, false);
    };
    let safe = value.len() <= 512
        && !value.contains([':', '?', '#'])
        && !Path::new(value).is_absolute()
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
    if safe {
        (Some(value.into()), false)
    } else {
        (None, true)
    }
}

pub(crate) fn summary(envelope: &Envelope) -> Result<ActivitySummary> {
    let payload = &envelope.event.payload;
    let mut result = ActivitySummary {
        action: envelope
            .event
            .subject
            .rsplit('/')
            .next()
            .unwrap_or("event")
            .into(),
        id: None,
        path: None,
        path_redacted: false,
        digest: None,
        before_digest: None,
        grade: None,
        producer: None,
        candidates: vec![],
        status: None,
        previous_status: None,
        outcome: None,
        reason: None,
        position: None,
        payload_digest: digest(&serde_json::to_vec(payload)?),
    };
    if envelope.event.subject.starts_with("artifact/") || payload["artifact"].is_object() {
        let item = payload
            .get("observation")
            .or_else(|| payload.get("artifact"))
            .unwrap_or(payload);
        if envelope.event.subject.starts_with("doc/")
            && envelope.event.subject.ends_with("/written")
            && item["latest"]["kind"] == "claimed"
        {
            result.action = "claimed".into();
        }
        result.id = id(&item["artifact_id"]).or_else(|| id(&item["id"]));
        (result.path, result.path_redacted) = path(item.get("reference").unwrap_or(&item["ref"]));
        result.digest = hash(&item["digest"]);
        result.before_digest = hash(&item["before_digest"]);
        result.grade = text(
            item.get("grade").unwrap_or(&item["attribution"]["grade"]),
            32,
        );
        result.producer = id(&item["attribution"]["seat"])
            .map(|seat| format!("seat:{seat}"))
            .or_else(|| text(&item["produced_by"], 128));
        if let Some(candidates) = item["attribution"]["candidates"].as_array() {
            result.candidates = candidates.iter().filter_map(id).take(16).collect();
        }
        if result.candidates.len() > 1 {
            result.reason = Some("ambiguous attribution; original detail retained".into());
        } else if result.producer.is_none() {
            result.reason = Some("attribution unavailable; original detail retained".into());
        }
    } else if envelope.event.subject.starts_with("task/") {
        result.id = envelope
            .event
            .subject
            .split('/')
            .nth(1)
            .and_then(|value| value.parse().ok());
        (result.path, result.path_redacted) = path(&payload["path"]);
        // This is the canonical status string, with no native alias table.
        result.status = text(&payload["to"], 64);
        result.previous_status = text(&payload["from"], 64);
    } else if envelope.event.subject == "account/control" {
        result.id = id(&payload["op_id"]);
        result.outcome = text(&payload["outcome"], 32);
        if payload["activity"] == "undeliverable" {
            result.action = "undeliverable".into();
            result.reason = Some("model publication not acknowledged".into());
            if let (Some(stream), Some(sequence)) = (
                text(&payload["stream"], 128),
                payload["seq"].as_str().and_then(|value| value.parse().ok()),
            ) {
                result.position = Some(EventPosition { stream, sequence });
            }
        }
    } else if envelope.event.subject.starts_with("doc/") {
        result.id = envelope
            .event
            .subject
            .split('/')
            .nth(1)
            .and_then(|value| value.parse().ok());
        (result.path, result.path_redacted) = path(&payload["path"]);
        result.digest = hash(payload.get("body_digest").unwrap_or(&payload["digest"]));
    }
    if result.action.len() > 64 {
        result.action = "event".into();
    }
    Ok(result)
}

pub(crate) async fn project(
    client: &LogicClient,
    stream: &str,
    envelope: Envelope,
    position: Option<GuiEventCursor>,
) -> Result<GuiEventEnvelope> {
    let metadata = metadata(stream, &envelope)?;
    let subject = envelope.event.subject.as_str();
    let payload = if envelope.event.source == "messaging"
        && (subject.ends_with("/message") || subject.ends_with("/receipt"))
    {
        let swarm = stream
            .strip_prefix("swarm-")
            .ok_or_else(|| invalid("message event is outside a swarm stream"))?
            .parse::<Ulid>()?;
        let message = id(if subject.ends_with("/receipt") {
            &envelope.event.payload["message"]
        } else {
            &envelope.event.payload["id"]
        })
        .ok_or_else(|| invalid("message event has no canonical message id"))?;
        GuiEventPayload::Message {
            row: client.gui_message_row(swarm, message).await?,
        }
    } else if subject == "account/identity/state" {
        GuiEventPayload::Identity {
            identity: client.gui_identity().await?,
        }
    } else if subject == crate::provider::fake::STATE_SUBJECT {
        let crate::provider::ProviderEvent::HostState { host, state } =
            serde_json::from_value(envelope.event.payload.clone())?;
        match client
            .gui_hosts()
            .await?
            .into_iter()
            .find(|value| crate::provider::HostId::from(&value.location) == host)
        {
            Some(current) => GuiEventPayload::Host {
                host: current,
                transition: Some(bbt_proto::HostState::from(state)),
            },
            None => GuiEventPayload::Activity {
                summary: summary(&envelope)?,
            },
        }
    } else if subject.starts_with("seat/") && subject.ends_with("/state") {
        let seat = subject
            .split('/')
            .nth(1)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| invalid("state event has no seat"))?;
        match client.gui_seat(seat).await {
            Ok(seat) => GuiEventPayload::Seat {
                seat: Box::new(seat),
            },
            Err(error) if error.to_string() == "seat has no live process view" => {
                let mut summary = summary(&envelope)?;
                summary.reason = Some("no current live seat view; original retained".into());
                GuiEventPayload::Activity { summary }
            }
            Err(error) => return Err(error),
        }
    } else {
        GuiEventPayload::Activity {
            summary: summary(&envelope)?,
        }
    };
    Ok(GuiEventEnvelope {
        metadata,
        projected_at: now_ms(),
        cursor: position,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::events::Draft;
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    fn signed(payload: Value) -> Envelope {
        Envelope::signed(
            Draft::new("artifact/fixture/created", "fixture", payload),
            1,
            String::new(),
            &SigningKey::from_bytes(&[23; 32]),
        )
        .unwrap()
    }
    #[test]
    fn public_activity_fields_exclude_arbitrary_bodies_and_unsafe_refs() {
        let envelope = signed(
            json!({"observation":{"artifact_id":Ulid::from(1_u128),"reference":"https://user:secret@example.test/item?token=hidden","attribution":{"seat":null,"grade":"inferred","candidates":[Ulid::from(2_u128),Ulid::from(3_u128)]}},"hook":{"body":"CREDENTIAL"},"command":"SECRET COMMAND"}),
        );
        let public = summary(&envelope).unwrap();
        assert!(public.path.is_none() && public.path_redacted);
        assert!(public.producer.is_none());
        assert_eq!(public.candidates.len(), 2);
        let json = serde_json::to_string(&public).unwrap();
        for secret in ["secret", "hidden", "CREDENTIAL", "SECRET COMMAND"] {
            assert!(!json.contains(secret));
        }
    }
    #[test]
    fn metadata_preserves_the_original_proof_and_rejects_tampering() {
        let mut envelope = signed(json!({"field":"not displayed"}));
        let public = metadata("index", &envelope).unwrap();
        assert_eq!(public.reference.hash, envelope.hash().unwrap());
        assert_eq!(
            public.proof.scope,
            EventSignatureScope::OriginalRetainedEnvelope
        );
        assert_eq!(public.seat, envelope.event.seat);
        envelope.event.payload = json!({"changed":true});
        assert!(metadata("index", &envelope).is_err());
    }
}
