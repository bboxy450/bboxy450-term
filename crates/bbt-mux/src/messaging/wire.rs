//! Mux-owned validation and signing over shared messaging DTOs.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ulid::Ulid;

use crate::store::{Result, invalid};
pub use bbt_proto::messaging::{
    Body, Envelope, Issuer, IssuerKind, MAX_ENVELOPE, MAX_TEXT, MessageKind, SendRequest,
};

pub trait SendRequestExt {
    fn validate(&self) -> Result<()>;
}
impl SendRequestExt for SendRequest {
    fn validate(&self) -> Result<()> {
        if self.id == Ulid::nil()
            || self.thread_id == Some(Ulid::nil())
            || self.reply_to == Some(Ulid::nil())
            || self.priority > 9
            || self.text.len() > MAX_TEXT
            || self.attachments.len() > 32
            || self
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            || self.attachments.iter().any(|link| {
                link.len() > 512
                    || !link.starts_with("[[")
                    || !link.ends_with("]]")
                    || link[2..link.len().saturating_sub(2)].is_empty()
                    || link.chars().any(char::is_control)
            })
        {
            return Err(invalid(
                "invalid message bounds, control bytes or wikilink attachment",
            ));
        }
        Ok(())
    }
}

pub trait EnvelopeExt {
    fn signing_bytes(&self) -> Result<Vec<u8>>;
    fn sign(&mut self, key: &SigningKey) -> Result<()>;
    fn verify_signature(&self) -> Result<()>;
    /// Signature/shape checking with a label derived from a verified account root.
    /// This check alone never admits a sender or grants delivery authority.
    fn verify_signature_in_account(&self, test_label: Option<&str>) -> Result<()>;
    /// The identity provider supplies the verified test context, never tool input.
    fn sign_in_account(&mut self, key: &SigningKey, test_label: Option<&str>) -> Result<()>;
}
impl EnvelopeExt for Envelope {
    fn signing_bytes(&self) -> Result<Vec<u8>> {
        // Nest two fixed tuples to stay within serde's tuple implementation bound.
        let fields = (
            (
                self.version,
                self.id,
                self.issuer_kind,
                &self.issuer,
                &self.account,
                &self.device,
                self.seat,
                &self.author,
                self.swarm_id,
                self.process_generation.to_string(),
                &self.context,
                &self.role,
                &self.role_grant_id,
            ),
            (
                self.role_epoch.to_string(),
                self.priority_class,
                self.sender_sequence.to_string(),
                self.thread_id,
                self.reply_to,
                self.timestamp,
                self.priority,
                self.kind,
                &self.body,
                &self.attachments,
            ),
        );
        // Decision #269: door and issuer are signed when present; an envelope without them
        // keeps the bytes it was signed with before #269.
        let payload = if self.door.is_none() && self.issued_by.is_none() {
            serde_json::to_vec(&fields)?
        } else {
            serde_json::to_vec(&(fields, (self.door, &self.issued_by)))?
        };
        let mut bytes = b"bboxy450/message/v1\0".to_vec();
        bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        bytes.extend(payload);
        Ok(bytes)
    }
    fn sign(&mut self, key: &SigningKey) -> Result<()> {
        self.sign_in_account(key, None)
    }
    fn sign_in_account(&mut self, key: &SigningKey, test_label: Option<&str>) -> Result<()> {
        if self.author != public(&key.verifying_key().to_bytes()) {
            return Err(invalid("seat signing key mismatch"));
        }
        self.signature = URL_SAFE_NO_PAD.encode(key.sign(&self.signing_bytes()?).to_bytes());
        self.verify_signature_in_account(test_label)
    }
    fn verify_signature(&self) -> Result<()> {
        self.verify_signature_in_account(None)
    }
    fn verify_signature_in_account(&self, test_label: Option<&str>) -> Result<()> {
        if let Some(expected) = test_label
            && (!expected.starts_with("test-account:")
                || self.issued_by.as_deref() != Some(expected))
        {
            return Err(invalid(
                "test_account_label_mismatch: expected the verified run label",
            ));
        }
        SendRequest {
            id: self.id,
            destination: self.context.clone(),
            thread_id: Some(self.thread_id),
            reply_to: self.reply_to,
            priority: self.priority,
            kind: self.kind,
            text: self.body.text().unwrap_or_default().to_string(),
            attachments: self.attachments.clone(),
        }
        .validate()?;
        let operator = self.seat == Ulid::nil();
        if self.version != 1
            || (operator
                && (self.process_generation != 0
                    || self.role.name != "operator"
                    || self.role.class != bbt_proto::registry::Class::Oracle
                    || self.author != self.device))
            || self.swarm_id == Ulid::nil()
            || (!operator && self.process_generation == 0)
            || self.sender_sequence == 0
            || self.role_epoch == 0
            || self.role.name.is_empty()
            || self.role.name.len() > 128
            || self.role.name.chars().any(char::is_control)
            || self.priority_class != self.role.class
            || (!operator && self.door.is_some())
            || (!operator && test_label.is_none() && self.issued_by.is_some())
            || self.issued_by.as_ref().is_some_and(|label| {
                label.trim().is_empty()
                    || label.len() > bbt_proto::messaging::MAX_ISSUER
                    || label.chars().any(char::is_control)
            })
            || self.timestamp > (1u64 << 53) - 1
            || serde_json::to_vec(self)?.len() > MAX_ENVELOPE
        {
            return Err(invalid("invalid signed envelope fields"));
        }
        let body_ok = match &self.body {
            Body::Text { .. } => matches!(
                self.kind,
                MessageKind::Text | MessageKind::Chunk | MessageKind::Decision
            ),
            Body::Presence {
                event,
                name,
                harness,
                policy_epoch,
                ..
            } => {
                self.kind == MessageKind::Presence
                    && matches!(event.as_str(), "mute" | "unmute")
                    && *policy_epoch > 0
                    && !name.is_empty()
                    && name.len() <= 256
                    && harness.len() <= 128
                    && !name.chars().chain(harness.chars()).any(char::is_control)
            }
            Body::Task {
                task_id,
                event,
                path,
                ..
            } => {
                self.kind == MessageKind::Task
                    && *task_id != Ulid::nil()
                    && !event.is_empty()
                    && event.len() <= 64
                    && path.len() <= 4096
                    && !event.chars().chain(path.chars()).any(char::is_control)
            }
        };
        if !body_ok {
            return Err(invalid("message kind/body mismatch or invalid typed body"));
        }
        parse_public(&self.issuer)?;
        parse_public(&self.account)?;
        parse_public(&self.device)?;
        if self.role_grant_id.len() != 64
            || !self
                .role_grant_id
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(invalid("invalid role grant reference"));
        }
        let signature = URL_SAFE_NO_PAD.decode(&self.signature)?;
        VerifyingKey::from_bytes(&parse_public(&self.author)?)?.verify(
            &self.signing_bytes()?,
            &Signature::try_from(signature.as_slice())?,
        )?;
        Ok(())
    }
}
pub fn public(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn parse_public(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(invalid("invalid lowercase public key"));
    }
    let mut key = [0; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * i..2 * i + 2], 16)?;
    }
    Ok(key)
}
