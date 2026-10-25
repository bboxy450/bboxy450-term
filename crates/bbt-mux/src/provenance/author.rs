//! The actual signer is separate from any inferred producer of an artifact.
use crate::Ulid;
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

pub(crate) struct Author<'a> {
    pub id: String,
    pub seat: Option<Ulid>,
    pub generation: Option<u64>,
    pub key: &'a SigningKey,
    pub public: Value,
}
impl<'a> Author<'a> {
    pub fn operator(
        device_id: Ulid,
        issuer: crate::messaging::wire::IssuerKind,
        key: &'a SigningKey,
    ) -> Self {
        use base64::Engine;
        let id = format!("device:{device_id}");
        let kind = if issuer == crate::messaging::wire::IssuerKind::GpgRoot {
            "device"
        } else {
            "local-dev"
        };
        Self {
            id: id.clone(),
            seat: None,
            generation: None,
            key,
            public: json!({"signer":id,"kind":kind,"principal":format!("operator:{device_id}"),
                "device_id":device_id,"public_key":base64::engine::general_purpose::STANDARD.encode(key.verifying_key().to_bytes())}),
        }
    }
    pub fn seat(seat: Ulid, generation: Option<u64>, key: &'a SigningKey) -> Self {
        use base64::Engine;
        let id = format!("seat:{seat}");
        Self {
            id: id.clone(),
            seat: Some(seat),
            generation,
            key,
            public: json!({"signer":id,"kind":"seat","public_key":base64::engine::general_purpose::STANDARD.encode(key.verifying_key().to_bytes()),"process_generation":generation.map(|v|v.to_string())}),
        }
    }
    pub fn observer(status: &crate::identity::IdentityStatus, key: &'a SigningKey) -> Self {
        use base64::Engine;
        let public =
            base64::engine::general_purpose::STANDARD.encode(key.verifying_key().to_bytes());
        let certified = status.bootstrapped
            && status.issuer_kind == crate::messaging::wire::IssuerKind::GpgRoot
            && status.authenticated_signing_ready;
        let (kind, id) = if let Some(device) = status.device_id.filter(|_| certified) {
            ("device", format!("device:{device}"))
        } else {
            ("local-dev", format!("observer:local-dev:{public}"))
        };
        Self {
            id: id.clone(),
            seat: None,
            generation: None,
            key,
            public: json!({"signer":id,"kind":kind,"public_key":public,"device_id":status.device_id,"root_fingerprint":status.root_fingerprint,"custody":status.custody_label}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{IdentityState, IdentityStatus};
    use crate::messaging::wire::IssuerKind;

    #[test]
    fn device_id_is_data_and_does_not_certify_local_or_pending_signers() {
        let key = SigningKey::from_bytes(&[3; 32]);
        let mut status = IdentityStatus {
            state: IdentityState::LocalDev,
            label: "local".into(),
            issuer_kind: IssuerKind::LocalDev,
            bootstrapped: false,
            authenticated_signing_ready: false,
            root_fingerprint: None,
            pending_reason: None,
            reason_label: None,
            custody: Some(crate::identity::Custody::Memory),
            custody_label: Some("memory".into()),
            device_id: Some(Ulid(1)),
            device_grant_expires_at: None,
            device_renewal_due_at: None,
            device_renewal_required: false,
            seats_still_local_dev: 0,
            custody_held_since: None,
            open_ephemeral_holds: 0,
            reopen_failed: None,
        };
        for state in [IdentityState::LocalDev, IdentityState::Pending] {
            status.state = state;
            let author = Author::observer(&status, &key);
            assert_eq!(author.public["kind"], "local-dev");
            assert!(author.id.starts_with("observer:local-dev:"));
            assert_eq!(author.public["device_id"], json!(Ulid(1)));
        }
        // An enrolled account can still be using its old local bus key until
        // activation; public enrollment alone does not change the signer.
        status.bootstrapped = true;
        status.issuer_kind = IssuerKind::GpgRoot;
        status.state = IdentityState::Held;
        assert_eq!(Author::observer(&status, &key).public["kind"], "local-dev");
        status.authenticated_signing_ready = true;
        status.state = IdentityState::Certified;
        let author = Author::observer(&status, &key);
        assert_eq!(author.public["kind"], "device");
        assert_eq!(author.id, format!("device:{}", Ulid(1)));
    }
}
