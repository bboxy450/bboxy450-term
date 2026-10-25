//! Local unsigned intents and their immutable upgrade bindings (decision #285).
//! These records are observations at inferred grade, never signing authority.
use bbt_proto::messaging::{IntentBinding, LogicalSender, PendingIntent};
use ulid::Ulid;

use super::wire::SendRequestExt;
use crate::store::{Result, digest, invalid};

/// Bind the full original intent; no signer, sender or delivery is admitted here.
pub fn binding(intent: &PendingIntent) -> Result<IntentBinding> {
    intent.request.validate()?;
    if intent.version != 1
        || intent.id == Ulid::nil()
        || intent.id != intent.request.id
        || intent.swarm_id == Ulid::nil()
        || intent.admission_order == 0
        || intent.role_epoch == 0
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
            seat,
            process_generation,
        } if seat == Ulid::nil() || process_generation == 0 => {
            return Err(invalid("invalid pending seat identity"));
        }
        LogicalSender::Operator {
            device: Some(device),
            ..
        } if device == Ulid::nil() => return Err(invalid("invalid pending operator reference")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::{
        messaging::{Body, Door, PendingSignature, SendRequest},
        registry::{Class, Role},
    };

    #[test]
    fn pending_binding_covers_sender_content_and_admission_order_without_an_author() {
        let request = SendRequest::text("operator/inbox", "local pending observation");
        let intent = PendingIntent {
            version: 1,
            id: request.id,
            sender: LogicalSender::Operator {
                device: None,
                door: Door::Gui,
                issued_by: None,
            },
            swarm_id: Ulid::new(),
            role: Role {
                name: "operator".into(),
                class: Class::Oracle,
            },
            role_epoch: 1,
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
            seat: Ulid::new(),
            process_generation: 1,
        };
        assert_ne!(binding(&changed).unwrap().digest, original.digest);
        changed = intent;
        changed.request.id = Ulid::new();
        assert!(binding(&changed).is_err());
    }
}
