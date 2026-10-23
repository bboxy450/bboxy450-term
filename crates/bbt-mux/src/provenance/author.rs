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
        let (kind, id) = if let Some(device) = status.device_id {
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
