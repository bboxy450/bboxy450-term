//! Shared messaging request and wire records.
//! Routing, persistence, authority resolution and signing remain in logic.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::registry::{Class, Role};

pub const MAX_TEXT: usize = 16 * 1024;
/// The longest `issued_by` label an operator envelope carries (decision #269).
pub const MAX_ISSUER: usize = 128;

/// The operator's two doors on `gui.sock` (decisions #260, #269): the app's own window, or
/// `bboxy450-term ctl`. An operator envelope names the one it came through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    #[default]
    Gui,
    Ctl,
}
impl Door {
    pub fn is_gui(&self) -> bool {
        *self == Self::Gui
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gui => "gui",
            Self::Ctl => "ctl",
        }
    }
}
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
    /// Operator envelopes only (decision #269): the door the operator came through and who
    /// issued it there, attribution and never authority. Signed when present; absent on
    /// seat envelopes and on operator envelopes from before #269.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<Door>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued_by: Option<String>,
    pub signature: String,
}

/// Cryptographic evidence on a locally stored message, independently of delivery receipts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureState {
    #[default]
    Signed,
    Pending,
}
impl SignatureState {
    pub fn is_signed(&self) -> bool {
        *self == Self::Signed
    }
}

/// An intent can carry only this marker; a signed envelope is a distinct record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingSignature {
    #[default]
    Pending,
}

/// The locally admitted speaker of an unsigned intent, never a public-key claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperatorReferenceKind {
    OperatorLocal,
    DeviceDerived,
}
/// Immutable account-local continuity identity. Device attribution lives apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorReference {
    pub id: Ulid,
    pub kind: OperatorReferenceKind,
}
impl OperatorReference {
    pub fn recipient(self) -> Recipient {
        match self.kind {
            OperatorReferenceKind::OperatorLocal => Recipient::OperatorLocal(self.id),
            OperatorReferenceKind::DeviceDerived => Recipient::Operator(self.id),
        }
    }
    pub fn from_recipient(recipient: Recipient) -> Option<Self> {
        match recipient {
            Recipient::Seat(_) => None,
            Recipient::Operator(id) => Some(Self {
                id,
                kind: OperatorReferenceKind::DeviceDerived,
            }),
            Recipient::OperatorLocal(id) => Some(Self {
                id,
                kind: OperatorReferenceKind::OperatorLocal,
            }),
        }
    }
}

/// The locally admitted speaker of an unsigned intent, never a public-key claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogicalSender {
    Seat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device: Option<Ulid>,
        seat: Ulid,
        #[serde(with = "decimal")]
        process_generation: u64,
    },
    Operator {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device: Option<Ulid>,
        door: Door,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        issued_by: Option<String>,
    },
    #[serde(rename = "operator-local")]
    OperatorLocal {
        reference: OperatorReference,
        device: Option<Ulid>,
        door: Door,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        issued_by: Option<String>,
    },
}

/// Same-device delivery admitted while custody is unavailable (decision #285).
///
/// There is deliberately no cryptographic author, issuer or fallback signature here.
/// A pending intent cannot be submitted to a pinned remote receiver as an Envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingIntent {
    pub version: u16,
    pub id: Ulid,
    pub sender: LogicalSender,
    pub swarm_id: Ulid,
    pub role: Role,
    /// A first-ever local operator has no signed grant epoch to claim.
    #[serde(
        default,
        with = "optional_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub role_epoch: Option<u64>,
    pub request: SendRequest,
    pub body: Body,
    #[serde(with = "decimal")]
    pub admission_order: u64,
    #[serde(with = "decimal")]
    pub admitted_ms: u64,
    pub signature: PendingSignature,
    pub hold_reason: String,
}

/// An appended signature upgrade binds the immutable original intent by ID and digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentBinding {
    pub id: Ulid,
    pub digest: String,
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
    #[serde(flatten)]
    pub content: StoredContent,
    pub held_out: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus: Option<BusPosition>,
}

/// Signed records retain their original wire shape; pending records have no envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StoredContent {
    Signed {
        envelope: Box<Envelope>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upgrade_of: Option<IntentBinding>,
    },
    Pending {
        intent: Box<PendingIntent>,
    },
}
impl From<Envelope> for StoredContent {
    fn from(envelope: Envelope) -> Self {
        Self::Signed {
            envelope: Box::new(envelope),
            upgrade_of: None,
        }
    }
}

/// Borrowed message content shared by delivery and projection, never signing authority.
pub struct MessageView<'a> {
    pub id: Ulid,
    pub seat: Ulid,
    pub process_generation: u64,
    pub swarm_id: Ulid,
    pub role: &'a Role,
    pub role_epoch: Option<u64>,
    pub kind: MessageKind,
    pub context: &'a str,
    pub body: &'a Body,
    pub priority_class: Class,
    pub priority: u8,
    pub thread_id: Ulid,
    pub reply_to: Option<Ulid>,
    pub attachments: &'a [String],
    pub timestamp: u64,
    pub issuer_kind: Option<IssuerKind>,
    pub signature: SignatureState,
    pub signature_reason: Option<&'a str>,
}

impl StoredMessage {
    pub fn envelope(&self) -> Option<&Envelope> {
        match &self.content {
            StoredContent::Signed { envelope, .. } => Some(envelope),
            StoredContent::Pending { .. } => None,
        }
    }
    pub fn intent(&self) -> Option<&PendingIntent> {
        match &self.content {
            StoredContent::Pending { intent } => Some(intent),
            StoredContent::Signed { .. } => None,
        }
    }
    pub fn upgrade_of(&self) -> Option<&IntentBinding> {
        match &self.content {
            StoredContent::Signed { upgrade_of, .. } => upgrade_of.as_ref(),
            StoredContent::Pending { .. } => None,
        }
    }
    pub fn view(&self) -> MessageView<'_> {
        match &self.content {
            StoredContent::Signed { envelope: e, .. } => MessageView {
                id: e.id,
                seat: e.seat,
                process_generation: e.process_generation,
                swarm_id: e.swarm_id,
                role: &e.role,
                role_epoch: Some(e.role_epoch),
                kind: e.kind,
                context: &e.context,
                body: &e.body,
                priority_class: e.priority_class,
                priority: e.priority,
                thread_id: e.thread_id,
                reply_to: e.reply_to,
                attachments: &e.attachments,
                timestamp: e.timestamp,
                issuer_kind: Some(e.issuer_kind),
                signature: SignatureState::Signed,
                signature_reason: None,
            },
            StoredContent::Pending { intent: i } => {
                let (seat, process_generation) = match i.sender {
                    LogicalSender::Seat {
                        seat,
                        process_generation,
                        ..
                    } => (seat, process_generation),
                    LogicalSender::Operator { .. } | LogicalSender::OperatorLocal { .. } => {
                        (Ulid::nil(), 0)
                    }
                };
                MessageView {
                    id: i.id,
                    seat,
                    process_generation,
                    swarm_id: i.swarm_id,
                    role: &i.role,
                    role_epoch: i.role_epoch,
                    kind: i.request.kind,
                    context: &i.request.destination,
                    body: &i.body,
                    priority_class: i.role.class,
                    priority: i.request.priority,
                    thread_id: i.request.thread_id.unwrap_or(i.id),
                    reply_to: i.request.reply_to,
                    attachments: &i.request.attachments,
                    timestamp: i.admitted_ms / 1000,
                    issuer_kind: None,
                    signature: SignatureState::Pending,
                    signature_reason: Some(&i.hold_reason),
                }
            }
        }
    }
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
    #[serde(rename = "operator-local")]
    OperatorLocal,
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
    /// Absent when no cryptographic author exists yet; author is a logical speaker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_kind: Option<IssuerKind>,
    #[serde(default, skip_serializing_if = "SignatureState::is_signed")]
    pub signature: SignatureState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_reason: Option<String>,
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
    #[serde(default, skip_serializing_if = "SignatureState::is_signed")]
    pub signature: SignatureState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_device: Option<Ulid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_device: Option<Ulid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_operator: Option<OperatorReference>,
}

/// Seat receipts retain their existing ULID string; operator receipts name the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Recipient {
    Seat(Ulid),
    Operator(Ulid),
    OperatorLocal(Ulid),
}
impl Recipient {
    pub fn seat(self) -> Option<Ulid> {
        match self {
            Self::Seat(id) => Some(id),
            Self::Operator(_) | Self::OperatorLocal(_) => None,
        }
    }
    pub fn is_operator(self) -> bool {
        matches!(self, Self::Operator(_) | Self::OperatorLocal(_))
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
            Recipient::OperatorLocal(id) => format!("operator-local:{id}"),
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
        let (kind, id) = if let Some(id) = value.strip_prefix("operator-local:") {
            (2, id)
        } else if let Some(id) = value.strip_prefix("operator:") {
            (1, id)
        } else {
            (0, value.as_str())
        };
        let id = id
            .parse::<Ulid>()
            .ok()
            .filter(|id| *id != Ulid::nil())
            .ok_or("invalid message recipient reference")?;
        Ok(match kind {
            2 => Self::OperatorLocal(id),
            1 => Self::Operator(id),
            _ => Self::Seat(id),
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

mod optional_decimal {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
        value.map(|n| n.to_string()).serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|value| {
                let n = value.parse::<u64>().map_err(serde::de::Error::custom)?;
                if n.to_string() != value {
                    return Err(serde::de::Error::custom("noncanonical counter"));
                }
                Ok(n)
            })
            .transpose()
    }
}
