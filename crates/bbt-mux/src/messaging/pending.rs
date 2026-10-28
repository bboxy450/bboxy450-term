// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Local unsigned intents and their immutable upgrade bindings (decision #285).
//! These records are observations at inferred grade, never signing authority.
use bbt_proto::messaging::{IntentBinding, LogicalSender, PendingIntent};
use ulid::Ulid;

use super::wire::SendRequestExt;
use crate::store::{Result, digest, invalid};

/// Bind the full original intent; no signer, sender or delivery is admitted here.
pub fn binding(intent: &PendingIntent) -> Result<IntentBinding> {
    intent.request.validate()?;
    if intent.request.kind == bbt_proto::messaging::MessageKind::Withdrawn
        && intent.body != (bbt_proto::messaging::Body::Withdrawn {})
    {
        return Err(invalid("withdrawal intent must have a content-free body"));
    }
    if intent.version != 1
        || intent.id == Ulid::nil()
        || intent.id != intent.request.id
        || intent.swarm_id == Ulid::nil()
        || intent.admission_order == 0
        || intent.role_epoch == Some(0)
        || intent.role.name.is_empty()
        || intent.role.name.len() > 128
        || intent.role.name.chars().any(char::is_control)
        || intent.hold_reason.is_empty()
        || intent.hold_reason.len() > 512
    {
        return Err(invalid("invalid local pending intent"));
    }
    match intent.sender {
        LogicalSender::Seat {
            device: Some(device),
            ..
        } if device == Ulid::nil() => {
            return Err(invalid("invalid pending device reference"));
        }
        LogicalSender::Seat {
            seat,
            process_generation,
            ..
        } if seat == Ulid::nil() || process_generation == 0 || intent.role_epoch.is_none() => {
            return Err(invalid("invalid pending seat identity"));
        }
        LogicalSender::Operator { reference, .. }
            if reference.id == Ulid::nil()
                || reference.kind != bbt_proto::messaging::OperatorReferenceKind::DeviceDerived =>
        {
            return Err(invalid("invalid device-derived operator reference"));
        }
        LogicalSender::Operator {
            device: Some(device),
            ..
        } if device == Ulid::nil() => return Err(invalid("invalid pending operator reference")),
        LogicalSender::OperatorLocal {
            reference, device, ..
        } if reference.id == Ulid::nil()
            || reference.kind != bbt_proto::messaging::OperatorReferenceKind::OperatorLocal
            || device == Some(Ulid::nil()) =>
        {
            return Err(invalid("invalid local operator reference"));
        }
        _ => {}
    }
    let bytes = serde_json::to_vec(intent)?;
    if bytes.len() > bbt_proto::messaging::MAX_ENVELOPE {
        return Err(invalid("pending intent exceeds the message bound"));
    }
    Ok(IntentBinding {
        id: intent.id,
        digest: digest(&bytes),
    })
}

/// Check that a completion signs the admitted content. The caller must separately
/// verify the signature and its current account/seat authority before admitting it.
pub fn check_upgrade_content(
    intent: &PendingIntent,
    envelope: &bbt_proto::messaging::Envelope,
) -> Result<()> {
    binding(intent)?;
    let (seat, generation, origin) = match &intent.sender {
        LogicalSender::Seat {
            seat,
            process_generation,
            ..
        } => (*seat, *process_generation, true),
        LogicalSender::Operator {
            door, issued_by, ..
        }
        | LogicalSender::OperatorLocal {
            door, issued_by, ..
        } => (
            Ulid::nil(),
            0,
            envelope.door == Some(*door) && envelope.issued_by == *issued_by,
        ),
    };
    if envelope.id != intent.id
        || envelope.swarm_id != intent.swarm_id
        || envelope.seat != seat
        || envelope.process_generation != generation
        || envelope.role != intent.role
        || intent
            .role_epoch
            .is_some_and(|epoch| envelope.role_epoch != epoch)
        || envelope.priority_class != intent.role.class
        || envelope.context != intent.request.destination
        || envelope.kind != intent.request.kind
        || envelope.body != intent.body
        || envelope.priority != intent.request.priority
        || envelope.thread_id != intent.request.thread_id.unwrap_or(intent.id)
        || envelope.reply_to != intent.request.reply_to
        || envelope.supersedes != intent.request.supersedes
        || envelope.attachments != intent.request.attachments
        || envelope.timestamp != intent.admitted_ms / 1000
        || !origin
    {
        return Err(invalid(
            "signed completion differs from its original pending intent",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::{
        messaging::{Body, Door, PendingSignature, SendRequest},
        registry::{Class, Role},
    };

    // keeps: spec messaging
    #[test]
    fn pending_binding_covers_sender_content_and_admission_order_without_an_author() {
        let request = SendRequest::text("operator/inbox", "local pending observation");
        let intent = PendingIntent {
            version: 1,
            id: request.id,
            sender: LogicalSender::OperatorLocal {
                reference: bbt_proto::messaging::OperatorReference {
                    id: Ulid::new(),
                    kind: bbt_proto::messaging::OperatorReferenceKind::OperatorLocal,
                },
                device: None,
                door: Door::Gui,
                issued_by: None,
            },
            swarm_id: Ulid::new(),
            role: Role {
                name: "operator".into(),
                class: Class::Oracle,
            },
            role_epoch: None,
            body: Body::Text {
                text: request.text.clone(),
            },
            request,
            admission_order: 1,
            admitted_ms: 1000,
            signature: PendingSignature::Pending,
            hold_reason: "fixture custody unavailable".into(),
        };
        let original = binding(&intent).unwrap();
        let reopened: PendingIntent =
            serde_json::from_slice(&serde_json::to_vec(&intent).unwrap()).unwrap();
        assert_eq!(binding(&reopened).unwrap(), original);
        let mut changed = intent.clone();
        changed.admission_order += 1;
        assert_ne!(binding(&changed).unwrap().digest, original.digest);
        changed = intent.clone();
        changed.request.text.push('!');
        changed.body = Body::Text {
            text: changed.request.text.clone(),
        };
        assert_ne!(binding(&changed).unwrap().digest, original.digest);
        changed = intent.clone();
        changed.sender = LogicalSender::Seat {
            device: None,
            seat: Ulid::new(),
            process_generation: 1,
        };
        changed.role_epoch = Some(1);
        assert_ne!(binding(&changed).unwrap().digest, original.digest);
        changed = intent;
        changed.request.id = Ulid::new();
        assert!(binding(&changed).is_err());
    }
}
