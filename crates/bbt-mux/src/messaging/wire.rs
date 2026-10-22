use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::registry::{Class, Role};
use crate::store::{Result, invalid};

pub const MAX_TEXT: usize = 16 * 1024;
pub const MAX_ENVELOPE: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssuerKind {
    LocalDev,
    GpgRoot,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issuer {
    pub kind: IssuerKind,
    pub key: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Text,
    Chunk,
    Decision,
    Presence,
    Task,
}

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
        state: crate::adapters::State,
        muted: bool,
        #[serde(with = "decimal")]
        policy_epoch: u64,
    },
    Task {
        task_id: Ulid,
        event: String,
        path: String,
        summary: String,
    },
}
impl Body {
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::Task { summary, .. } => Some(summary),
            Self::Presence { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendRequest {
    pub id: Ulid,
    pub destination: String,
    pub thread_id: Option<Ulid>,
    pub reply_to: Option<Ulid>,
    pub priority: u8,
    pub kind: MessageKind,
    pub text: String,
    pub attachments: Vec<String>,
}
impl SendRequest {
    pub fn text(destination: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: Ulid::new(),
            destination: destination.into(),
            thread_id: None,
            reply_to: None,
            priority: 0,
            kind: MessageKind::Text,
            text: text.into(),
            attachments: Vec::new(),
        }
    }
    pub fn validate(&self) -> Result<()> {
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

/// Every field except signature is covered by the fixed-order v1 tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub version: u16,
    pub id: Ulid,
    pub issuer_kind: IssuerKind,
    pub issuer: String,
    pub account: String,
    pub device: String,
    pub seat: Ulid,
    pub author: String,
    pub swarm_id: Ulid,
    #[serde(with = "decimal")]
    pub process_generation: u64,
    pub context: String,
    pub role: Role,
    pub role_grant_id: String,
    #[serde(with = "decimal")]
    pub role_epoch: u64,
    pub priority_class: Class,
    #[serde(with = "decimal")]
    pub sender_sequence: u64,
    pub thread_id: Ulid,
    pub reply_to: Option<Ulid>,
    pub timestamp: u64,
    pub priority: u8,
    pub kind: MessageKind,
    pub body: Body,
    pub attachments: Vec<String>,
    pub signature: String,
}
impl Envelope {
    pub fn signing_bytes(&self) -> Result<Vec<u8>> {
        // Nest two fixed tuples to stay within serde's tuple implementation bound.
        let payload = serde_json::to_vec(&(
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
        ))?;
        let mut bytes = b"bboxy450/message/v1\0".to_vec();
        bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        bytes.extend(payload);
        Ok(bytes)
    }
    pub fn sign(&mut self, key: &SigningKey) -> Result<()> {
        if self.author != public(&key.verifying_key().to_bytes()) {
            return Err(invalid("seat signing key mismatch"));
        }
        self.signature = URL_SAFE_NO_PAD.encode(key.sign(&self.signing_bytes()?).to_bytes());
        self.verify_signature()
    }
    pub fn verify_signature(&self) -> Result<()> {
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
        if self.version != 1
            || self.seat == Ulid::nil()
            || self.swarm_id == Ulid::nil()
            || self.process_generation == 0
            || self.sender_sequence == 0
            || self.role_epoch == 0
            || self.role.name.is_empty()
            || self.role.name.len() > 128
            || self.role.name.chars().any(char::is_control)
            || self.priority_class != self.role.class
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
mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let value = String::deserialize(d)?;
        let n = value.parse::<u64>().map_err(serde::de::Error::custom)?;
        if n.to_string() != value {
            return Err(serde::de::Error::custom("noncanonical counter"));
        }
        Ok(n)
    }
}
