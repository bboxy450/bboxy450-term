//! Shared messaging request and wire records.
//! Routing, persistence, authority resolution and signing remain in logic.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::registry::{Class, Role};

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
        state: State,
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
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Active,
    Idle,
    Waiting,
    Errored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    SenderStored,
    RecipientStored,
    NativeQueued,
    NativeWritten,
    NativeUnknown,
    ApplicationAcknowledged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    Read,
    Write,
    Link,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Folder,
    Sink,
    Link,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub kinds: Vec<MessageKind>,
    pub classes: Vec<Class>,
    pub priorities: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    pub batch_ms: u64,
    pub coalesce: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub path: String,
    pub kind: EntryKind,
    pub target: Option<String>,
    pub filter: Filter,
    pub rate: Rate,
    pub paused: bool,
    pub grants: BTreeMap<Ulid, Vec<Right>>,
}

impl Node {
    pub fn folder(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: EntryKind::Folder,
            target: None,
            filter: Filter::default(),
            rate: Rate::default(),
            paused: false,
            grants: BTreeMap::new(),
        }
    }
    pub fn link(path: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            kind: EntryKind::Link,
            target: Some(target.into()),
            ..Self::folder(path)
        }
    }
    pub fn sink(seat: Ulid) -> Self {
        Self {
            kind: EntryKind::Sink,
            ..Self::folder(format!("seats/{seat}/inbox"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrival {
    pub path: String,
    pub links: Vec<String>,
    pub paused: bool,
    pub batch_ms: u64,
    pub coalesce: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub sequence: u64,
    pub envelope: Envelope,
    pub held_out: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus: Option<BusPosition>,
}

/// A bus position belongs to one retained object stream, never the inbox ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusPosition {
    pub stream: String,
    #[serde(with = "decimal")]
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorKind {
    Seat,
    Operator,
    Observer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorRef {
    pub kind: AuthorKind,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptSummary {
    pub delivered: usize,
    pub acked: usize,
    pub held: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRow {
    pub id: Ulid,
    pub bus: Option<BusPosition>,
    pub author: AuthorRef,
    pub issuer_kind: IssuerKind,
    pub kind: MessageKind,
    pub priority_class: Class,
    pub priority: u8,
    pub text: String,
    pub ts: u64,
    pub receipts: ReceiptSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub message_sequence: u64,
    pub message: Ulid,
    pub sender: Ulid,
    pub recipient: Recipient,
    pub agent: Option<String>,
    pub evidence: Evidence,
    pub note: String,
    pub held: Option<String>,
    pub arrivals: Vec<Arrival>,
    pub received_ms: u64,
    pub eligible_ms: u64,
    pub batch_key: String,
    pub class: Class,
    pub attempt: Option<Ulid>,
    pub native_id: Option<String>,
    pub relayed_via_main: bool,
}

/// Seat receipts retain their existing ULID string; operator receipts name the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Recipient {
    Seat(Ulid),
    Operator(Ulid),
}
impl Recipient {
    pub fn seat(self) -> Option<Ulid> {
        match self {
            Self::Seat(id) => Some(id),
            Self::Operator(_) => None,
        }
    }
}
impl From<Ulid> for Recipient {
    fn from(id: Ulid) -> Self {
        Self::Seat(id)
    }
}
impl From<Recipient> for String {
    fn from(value: Recipient) -> Self {
        match value {
            Recipient::Seat(id) => id.to_string(),
            Recipient::Operator(id) => format!("operator:{id}"),
        }
    }
}
impl std::fmt::Display for Recipient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&String::from(*self))
    }
}
impl TryFrom<String> for Recipient {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (operator, id) = value
            .strip_prefix("operator:")
            .map_or((false, value.as_str()), |id| (true, id));
        let id = id
            .parse::<Ulid>()
            .ok()
            .filter(|id| *id != Ulid::nil())
            .ok_or("invalid message recipient reference")?;
        Ok(if operator {
            Self::Operator(id)
        } else {
            Self::Seat(id)
        })
    }
}
impl PartialEq<Ulid> for Recipient {
    fn eq(&self, id: &Ulid) -> bool {
        matches!(self, Self::Seat(seat) if seat == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Send {
        message: SendRequest,
    },
    List {
        thread: Option<Ulid>,
        channel: Option<String>,
        after: u64,
        limit: usize,
    },
    Receipts {
        message: Ulid,
        after: u64,
        limit: usize,
    },
    Acknowledge {
        message: Ulid,
        agent: Option<String>,
    },
    Ls {
        path: String,
        after: Option<String>,
        limit: usize,
    },
    Subscribe {
        folder: String,
        name: String,
        filter: Filter,
        rate: Rate,
    },
    Unsubscribe {
        path: String,
    },
    Link {
        node: Node,
    },
    Pause {
        path: String,
        paused: bool,
    },
    Grant {
        path: String,
        seat: Ulid,
        rights: Vec<Right>,
    },
    Mute {
        seat: Ulid,
        muted: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Held {
        id: Ulid,
        reason: String,
    },
    Stored {
        message: Box<StoredMessage>,
    },
    Messages {
        page: Page<StoredMessage>,
    },
    Receipts {
        page: Page<Receipt>,
    },
    Tree {
        nodes: Vec<Node>,
        next: Option<String>,
    },
    Muted {
        notice: Option<Ulid>,
    },
    Applied,
}

/// The existing W2.5 task-tool arguments, without the separately correlated op ID.
/// Logic applies the same authorization and validation as the MCP entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskRequest {
    List {
        #[serde(default, skip_serializing_if = "is_false")]
        unassigned: bool,
        swarm_id: Option<Ulid>,
        status: Option<String>,
        assignee: Option<String>,
        after: Option<Ulid>,
        limit: Option<usize>,
    },
    Status {
        task_path: Option<String>,
        task_id: Option<Ulid>,
    },
    Delegate {
        task_path: String,
        seat_id: Ulid,
        role: Option<String>,
    },
    Reassign {
        task_path: String,
        seat_id: Ulid,
    },
    Accept {
        task_path: String,
    },
    Reject {
        task_path: String,
        reason: String,
    },
    Report {
        task_path: String,
        summary: String,
        commit: Option<String>,
        verify_result: Option<TaskVerification>,
    },
}
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskEvidence {
    pub criterion: String,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskVerification {
    pub command: String,
    pub exit_code: i32,
    #[serde(default)]
    pub evidence: Vec<TaskEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskReport {
    pub summary: String,
    pub commit: Option<String>,
    pub verify_result: Option<TaskVerification>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskVerifySummary {
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCard {
    pub id: Ulid,
    pub path: String,
    pub title: String,
    pub status: String,
    pub assignee: Option<String>,
    pub delegated_by: Option<String>,
    pub updated: Option<String>,
    pub verify: Option<TaskVerifySummary>,
}

/// Same document projection returned by task_status today; Markdown remains authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDoc {
    pub id: Ulid,
    pub path: String,
    pub metadata: serde_json::Value,
    pub body: String,
    pub body_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskResponse {
    Items {
        items: Vec<TaskCard>,
        next: Option<Ulid>,
    },
    Item {
        item: TaskDoc,
    },
    Applied,
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
