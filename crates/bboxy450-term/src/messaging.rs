//! UI glue for the shared messaging protocol. No IO, signing, routing policy,
//! unread authority or reusable widgets live in the app.
use crate::cockpit::UiMsg;
use bbt_mux::Ulid;
use bbt_proto::{
    SwarmId,
    messaging::{self as wire, MessageKind, Request, Response, TaskRequest, TaskResponse},
};
use bbt_ui::modules::composer::{Address, PickGroup, Submission};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

pub const MAX_RESULTS: usize = 64;
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
pub const MAX_DESTINATIONS: usize = 32;

/// Built-in destinations are W2.4's documented id-based paths. Additional
/// channel/group choices must come from tree pages, never guessed labels.
pub fn catalog(
    model: &crate::model::Model,
    stage: &bbt_ui_model::Stage,
) -> Option<(Address, Vec<PickGroup>)> {
    let swarm = stage.swarm?;
    let owner = model
        .ui
        .window
        .sidebar
        .swarms
        .iter()
        .find(|s| s.id == swarm)?;
    let channel = Address {
        kind: "channel".into(),
        id: swarm.to_string(),
        label: owner.name.clone(),
        destination: format!("swarms/{swarm}"),
        ..Default::default()
    };
    let mut seats = vec![];
    let mut agents = vec![];
    for (id, seat) in &model.ui.seats {
        if seat.swarm != swarm || seat.lifecycle != bbt_ui_model::Lifecycle::Open {
            continue;
        }
        let h = seat.color.resolve(&id.to_string(), model.ui.window.theme);
        let rgba = bbt_ui_model::draw::Color::hsl(h.hue, h.saturation / 100., h.lightness / 100.).0;
        let color = format!(
            "#{:02X}{:02X}{:02X}",
            (rgba[0] * 255.).round() as u8,
            (rgba[1] * 255.).round() as u8,
            (rgba[2] * 255.).round() as u8
        );
        seats.push(Address {
            kind: "seat".into(),
            label: seat.name.clone(),
            sub: seat.role.as_ref().map_or_else(
                || seat.harness.0.clone(),
                |role| format!("{} · {}", role.name, seat.harness.0),
            ),
            role: seat
                .role
                .as_ref()
                .map_or_else(String::new, |role| role.name.clone()),
            id: id.to_string(),
            destination: format!("seats/{id}/inbox"),
            color: Some(color),
            ..Default::default()
        });
        if let Some(state) = model.states.get(id) {
            for agent in &state.subagents {
                agents.push(Address {
                    kind: "subagent".into(),
                    id: agent.id.clone(),
                    parent: seat.name.clone(),
                    label: agent.title.clone(),
                    destination: format!("seats/{id}/agents/{}", agent.id),
                    ..Default::default()
                });
            }
        }
    }
    let mut roles: BTreeMap<String, usize> = BTreeMap::new();
    for seat in model
        .ui
        .seats
        .values()
        .filter(|s| s.swarm == swarm && s.lifecycle == bbt_ui_model::Lifecycle::Open)
    {
        if let Some(role) = &seat.role {
            *roles.entry(role.name.clone()).or_default() += 1;
        }
    }
    let roles = roles
        .into_iter()
        .map(|(role, n)| Address {
            kind: "role".into(),
            id: role.clone(),
            label: role.clone(),
            role,
            n: Some(n),
            ..Default::default()
        })
        .collect();
    Some((
        channel.clone(),
        vec![
            PickGroup {
                title: "Seats".into(),
                items: seats,
            },
            PickGroup {
                title: "Roles".into(),
                items: roles,
            },
            PickGroup {
                title: "Channels".into(),
                items: vec![channel],
            },
            PickGroup {
                title: "Subagents".into(),
                items: agents,
            },
        ],
    ))
}

/// This is a field-for-field view adapter, not a second author/priority
/// projection. Only the seat registry may supply a role-name mark.
pub fn message_view(
    row: &wire::MessageRow,
    model: &crate::model::Model,
) -> bbt_ui::modules::stream_dock::Message {
    use bbt_ui::modules::stream_dock::{Author, Message, Receipts};
    let role = if row.author.kind == wire::AuthorKind::Seat {
        row.author
            .id
            .parse::<bbt_proto::SeatId>()
            .ok()
            .and_then(|id| model.ui.seats.get(&id))
            .and_then(|s| s.role.as_ref())
            .map_or_else(String::new, |r| r.name.clone())
    } else {
        String::new()
    };
    Message {
        author: Some(Author {
            kind: match row.author.kind {
                wire::AuthorKind::Seat => "seat",
                wire::AuthorKind::Operator => "operator",
                wire::AuthorKind::Observer => "observer",
            }
            .into(),
            id: row.author.id.clone(),
            name: row.author.name.clone(),
        }),
        issuer_kind: Some(
            match row.issuer_kind {
                wire::IssuerKind::LocalDev => "local-dev",
                wire::IssuerKind::GpgRoot => "gpg-root",
            }
            .into(),
        ),
        priority_class: Some(
            match row.priority_class {
                bbt_proto::registry::Class::Oracle => "oracle",
                bbt_proto::registry::Class::Worker => "worker",
            }
            .into(),
        ),
        priority: row.priority,
        kind: Some(
            match row.kind {
                MessageKind::Text => "text",
                MessageKind::Chunk => "chunk",
                MessageKind::Decision => "decision",
                MessageKind::Presence => "presence",
                MessageKind::Task => "task",
            }
            .into(),
        ),
        text: row.text.clone(),
        ts: Some(row.ts),
        role,
        // v0.1 has one operator per account; other rows' receipt aggregates are
        // not displayed as the operator's own delivery evidence.
        receipts: (row.author.kind == wire::AuthorKind::Operator).then_some(Receipts {
            delivered: row.receipts.delivered,
            acked: row.receipts.acked,
            held: row.receipts.held,
        }),
        ..Default::default()
    }
}

fn assignee_name(reference: &str, model: &crate::model::Model) -> String {
    reference
        .strip_prefix("seat:")
        .and_then(|id| id.parse::<bbt_ui_model::SeatId>().ok())
        .and_then(|id| model.ui.seats.get(&id))
        .map_or_else(|| reference.to_owned(), |seat| seat.name.clone())
}

/// TaskCard carries the canonical status. The UI adds names from the registry,
/// never a status translation or an acceptance claim.
pub fn task_item(
    card: &wire::TaskCard,
    model: &crate::model::Model,
) -> bbt_ui::modules::stream_dock::WorkItem {
    bbt_ui::modules::stream_dock::WorkItem {
        id: card.id.to_string(),
        path: card.path.clone(),
        title: card.title.clone(),
        state: card.status.clone(),
        assignee: card
            .assignee
            .as_deref()
            .map_or_else(String::new, |id| assignee_name(id, model)),
        delegated_by: card.delegated_by.clone(),
        kind: "task".into(),
        ..Default::default()
    }
}
pub fn task_attachment(card: &wire::TaskCard) -> bbt_ui::modules::composer::Attachment {
    bbt_ui::modules::composer::Attachment {
        id: card.id.to_string(),
        path: card.path.clone(),
        kind: "task".into(),
        title: card.title.clone(),
        status: card.status.clone(),
        ..Default::default()
    }
}
pub fn task_row(card: &wire::TaskCard) -> (bbt_ui::modules::seat_row::TaskRow, Option<i32>) {
    let exit = card.verify.as_ref().map(|v| v.exit_code);
    (
        bbt_ui::modules::seat_row::TaskRow {
            title: card.title.clone(),
            state: card.status.clone(),
            bug: false,
            violates: None,
            constrains: None,
            waits: None,
            may_accept: card.status == "reported" && exit == Some(0),
        },
        exit,
    )
}
pub fn task_report(
    card: &wire::TaskCard,
    doc: &wire::TaskDoc,
) -> Result<Option<bbt_ui::modules::composer::Report>, &'static str> {
    if doc.id != card.id || doc.path != card.path {
        return Err("task_document_mismatch");
    }
    if doc
        .metadata
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status != card.status)
    {
        return Err("task_status_changed_refresh_required");
    }
    let Some(report) = doc
        .metadata
        .get("report")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(None);
    };
    let summary = report
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .ok_or("task_report_summary_missing")?;
    let verify = doc
        .metadata
        .get("verify_result")
        .filter(|v| !v.is_null())
        .map(|v| {
            serde_json::from_value::<wire::TaskVerification>(v.clone())
                .map_err(|_| "task_verify_result_invalid")
        })
        .transpose()?;
    Ok(Some(bbt_ui::modules::composer::Report {
        task: card.title.clone(),
        status: card.status.clone(),
        summary: summary.into(),
        commit: report
            .get("commit")
            .and_then(serde_json::Value::as_str)
            .map(String::from),
        verify_result: verify.map(|v| bbt_ui::modules::composer::Verification {
            command: v.command,
            exit_code: v.exit_code,
            evidence: v
                .evidence
                .into_iter()
                .map(|e| bbt_ui::modules::composer::Criterion {
                    criterion: e.criterion,
                    reference: e.reference,
                })
                .collect(),
        }),
    }))
}

#[derive(Debug, Clone, Serialize)]
pub enum Reply {
    Messaging(Response),
    Task(TaskResponse),
    Refused(String),
    Unknown(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationReply {
    pub op: Ulid,
    pub serial: u64,
    pub reply: Reply,
}

/// Latest may replace intermediate publications. Keep correlated results in
/// the immutable model, bounded independently of terminal/grid publications.
#[derive(Debug, Clone, Default)]
pub struct Results {
    rows: VecDeque<(Arc<OperationReply>, usize)>,
    bytes: usize,
    omitted: u64,
    serial: u64,
    pending: BTreeSet<Ulid>,
}
impl Results {
    pub fn rows(&self) -> impl Iterator<Item = &OperationReply> {
        self.rows.iter().map(|(row, _)| row.as_ref())
    }
    pub fn omitted(&self) -> u64 {
        self.omitted
    }
    pub fn bytes(&self) -> usize {
        self.bytes + 2 // enclosing JSON array
    }
    pub fn requested(&mut self, op: Ulid) -> bool {
        if !self.pending.contains(&op) && self.pending.len() >= MAX_RESULTS {
            self.push(op, Reply::Refused("operation_queue_full".into()));
            return false;
        }
        self.pending.insert(op);
        true
    }
    pub fn complete(&mut self, op: Ulid, reply: Reply) -> bool {
        if !self.pending.remove(&op) {
            return false;
        }
        self.push(op, reply);
        true
    }
    pub fn disconnected(&mut self, reason: &str) -> bool {
        let pending: Vec<_> = self.pending.iter().copied().collect();
        for op in &pending {
            self.push(*op, Reply::Unknown(reason.into()));
        }
        !pending.is_empty()
    }
    pub fn push(&mut self, op: Ulid, reply: Reply) {
        self.serial = self
            .serial
            .checked_add(1)
            .expect("operation result serial exhausted");
        let mut row = OperationReply {
            op,
            serial: self.serial,
            reply,
        };
        // Include correlation metadata and separators in the serialized bound.
        let encoded = |row: &OperationReply| {
            serde_json::to_vec(row).map_or(MAX_RESULT_BYTES + 1, |v| v.len() + 1)
        };
        let mut size = encoded(&row);
        if size + 2 > MAX_RESULT_BYTES {
            row.reply = Reply::Refused("operation_result_window_limit: request a smaller read page; mutation retries must keep the operation id".into());
            size = encoded(&row);
        }
        if let Some(index) = self.rows.iter().position(|(row, _)| row.op == op) {
            self.bytes -= self.rows.remove(index).unwrap().1;
        }
        while self.rows.len() >= MAX_RESULTS || self.bytes + size + 2 > MAX_RESULT_BYTES {
            let (_, old_size) = self.rows.pop_front().expect("nonempty bounded results");
            self.bytes -= old_size;
            self.omitted += 1;
        }
        self.bytes += size;
        self.rows.push_back((Arc::new(row), size));
    }
}

pub struct Batch {
    pub commands: Vec<UiMsg>,
    pending: Pending,
}
struct Pending {
    draft: Submission,
    swarm: SwarmId,
    remaining: BTreeMap<Ulid, bool>,
    requests: BTreeMap<Ulid, Saved>,
    stored: usize,
    failures: Vec<String>,
    held: Vec<String>,
    last: Option<Ulid>,
    brief: Option<UiMsg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Idle,
    Pending,
    Stored {
        id: Ulid,
        sends: usize,
    },
    Held(String),
    Refused(String),
    Unknown(String),
    Partial {
        stored: usize,
        reason: String,
    },
}

#[derive(Default)]
pub struct Client {
    pending: Option<Pending>,
    seen: BTreeMap<Ulid, u64>,
    pub status: Status,
}
impl Client {
    /// The caller reserves the whole bounded UI queue batch before `admitted`.
    /// Scope is captured from the selected stage; no first/default swarm exists.
    pub fn prepare(
        &self,
        swarm: Option<SwarmId>,
        draft: Submission,
    ) -> Result<Batch, &'static str> {
        let recipients = draft.to.clone();
        self.prepare_recipients(swarm, draft, recipients)
    }
    pub fn prepare_for(
        &self,
        model: &crate::model::Model,
        stage: &bbt_ui_model::Stage,
        draft: Submission,
    ) -> Result<Batch, &'static str> {
        let swarm = stage.swarm.ok_or("swarm_required")?;
        let mut recipients = vec![];
        let mut paths = BTreeSet::new();
        for address in &draft.to {
            if address.kind == "role" {
                let members: Vec<_> = model
                    .ui
                    .seats
                    .values()
                    .filter(|s| {
                        s.swarm == swarm
                            && s.lifecycle == bbt_ui_model::Lifecycle::Open
                            && s.role.as_ref().is_some_and(|r| r.name == address.role)
                    })
                    .collect();
                if members.len() > 32 {
                    return Err("role_recipient_limit_32");
                }
                if members.is_empty() {
                    return Err("role_has_no_members");
                }
                for seat in members {
                    let destination = format!("seats/{}/inbox", seat.id);
                    if paths.insert(destination.clone()) {
                        recipients.push(Address {
                            kind: "seat".into(),
                            id: seat.id.to_string(),
                            label: seat.name.clone(),
                            destination,
                            ..Default::default()
                        });
                    }
                }
            } else if paths.insert(address.destination.clone()) {
                recipients.push(address.clone());
            }
        }
        self.prepare_recipients(Some(swarm), draft, recipients)
    }
    fn prepare_recipients(
        &self,
        swarm: Option<SwarmId>,
        draft: Submission,
        recipients: Vec<Address>,
    ) -> Result<Batch, &'static str> {
        if self.pending.is_some() {
            return Err("send_in_flight");
        }
        let swarm = swarm.ok_or("swarm_required")?;
        if recipients.is_empty() || recipients.len() > MAX_DESTINATIONS {
            return Err("destination_count");
        }
        if draft.text.len() > wire::MAX_TEXT {
            return Err("message_too_large");
        }
        if draft.priority > 9 {
            return Err("priority_out_of_range");
        }
        let attachments = draft
            .attach
            .iter()
            .map(|a| {
                if a.path.is_empty() {
                    Err("attachment_unresolved")
                } else {
                    Ok(a.path.clone())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let kind = match draft.kind.as_str() {
            "text" => MessageKind::Text,
            "decision" => MessageKind::Decision,
            "chunk" => MessageKind::Chunk,
            _ => return Err("message_kind_not_sendable"),
        };
        let mut commands = vec![];
        let mut remaining = BTreeMap::new();
        let mut brief = None;
        if draft.delegate {
            if draft.to.len() != 1
                || draft.to[0].kind != "seat"
                || draft.attach.len() != 1
                || draft.attach[0].kind != "task"
            {
                return Err("task_and_assignee_required");
            }
            let seat_id = draft.to[0].id.parse().map_err(|_| "assignee_unresolved")?;
            let id = Ulid::new();
            commands.push(UiMsg::Task {
                id,
                request: TaskRequest::Delegate {
                    task_path: draft.attach[0].path.clone(),
                    seat_id,
                    role: None,
                },
            });
            remaining.insert(id, true);
            if !draft.text.trim().is_empty() {
                let destination = &draft.to[0].destination;
                if destination.is_empty() {
                    return Err("destination_unresolved");
                }
                brief = Some(send(
                    swarm,
                    destination,
                    &draft.text,
                    kind,
                    draft.priority,
                    attachments,
                ));
            }
        } else {
            if draft.text.trim().is_empty() {
                return Err("message_required");
            }
            for address in &recipients {
                if address.destination.is_empty() {
                    return Err("destination_unresolved");
                }
                let command = send(
                    swarm,
                    &address.destination,
                    &draft.text,
                    kind,
                    draft.priority,
                    attachments.clone(),
                );
                let UiMsg::Messaging { id, .. } = &command else {
                    unreachable!()
                };
                remaining.insert(*id, false);
                commands.push(command);
            }
        }
        let requests = commands
            .iter()
            .map(|command| {
                let (id, saved) = Saved::capture(command);
                (id, saved)
            })
            .collect();
        Ok(Batch {
            commands,
            pending: Pending {
                draft,
                swarm,
                remaining,
                requests,
                stored: 0,
                failures: vec![],
                held: vec![],
                last: None,
                brief,
            },
        })
    }
    pub fn admitted(&mut self, mut batch: Batch) -> Vec<UiMsg> {
        self.status = Status::Pending;
        self.seen.clear();
        let commands = std::mem::take(&mut batch.commands);
        self.pending = Some(batch.pending);
        commands
    }
    pub fn refused(&mut self, reason: impl Into<String>) {
        self.status = Status::Refused(reason.into());
    }
    pub fn retry(&self) -> Vec<UiMsg> {
        if !matches!(self.status, Status::Unknown(_)) {
            return vec![];
        }
        self.pending.as_ref().map_or_else(Vec::new, |p| {
            p.remaining
                .keys()
                .map(|id| p.requests[id].command(*id))
                .collect()
        })
    }
    pub fn retry_admitted(&mut self) {
        self.status = Status::Pending;
    }
    /// Only replies carrying this batch's operation ids may affect its draft.
    /// A delegate's optional brief is sent after frontmatter delegation succeeds.
    pub fn replies(&mut self, results: &Results) -> (Option<Submission>, Vec<UiMsg>) {
        let Some(pending) = &mut self.pending else {
            return (None, vec![]);
        };
        let mut follow_up = vec![];
        for result in results.rows() {
            let Some(task) = pending.remaining.get(&result.op).copied() else {
                continue;
            };
            if self
                .seen
                .get(&result.op)
                .is_some_and(|serial| *serial >= result.serial)
            {
                continue;
            }
            self.seen.insert(result.op, result.serial);
            match &result.reply {
                Reply::Unknown(reason) => {
                    self.status = Status::Unknown(reason.clone());
                    continue;
                }
                Reply::Messaging(Response::Held { reason, .. }) if !task => {
                    pending.held.push(reason.clone());
                }
                Reply::Messaging(Response::Stored { .. }) if !task => pending.stored += 1,
                Reply::Task(TaskResponse::Applied) if task => {
                    pending.stored += 1;
                    if let Some(command) = pending.brief.take() {
                        let UiMsg::Messaging { id, swarm_id, .. } = &command else {
                            unreachable!()
                        };
                        debug_assert_eq!(*swarm_id, pending.swarm);
                        pending.remaining.insert(*id, false);
                        let (id, saved) = Saved::capture(&command);
                        pending.requests.insert(id, saved);
                        follow_up.push(command);
                    }
                }
                Reply::Refused(reason) => pending.failures.push(reason.clone()),
                _ => pending.failures.push("unexpected_operation_reply".into()),
            }
            pending.last = Some(result.op);
            pending.remaining.remove(&result.op);
        }
        if !pending.remaining.is_empty() {
            return (None, follow_up);
        }
        let pending = self.pending.take().unwrap();
        if pending.failures.is_empty() {
            self.status = if pending.held.is_empty() {
                Status::Stored {
                    id: pending.last.expect("completed operation"),
                    sends: pending.stored,
                }
            } else {
                Status::Held(pending.held.join("; "))
            };
            (Some(pending.draft), follow_up)
        } else {
            let reason = pending.failures.join("; ");
            self.status = if pending.stored > 0 {
                Status::Partial {
                    stored: pending.stored,
                    reason,
                }
            } else {
                Status::Refused(reason)
            };
            (None, follow_up)
        }
    }
}

#[derive(Clone)]
enum Saved {
    Message { swarm: SwarmId, request: Request },
    Task(TaskRequest),
}
impl Saved {
    fn capture(command: &UiMsg) -> (Ulid, Self) {
        match command {
            UiMsg::Messaging {
                id,
                swarm_id,
                request,
            } => (
                *id,
                Self::Message {
                    swarm: *swarm_id,
                    request: request.clone(),
                },
            ),
            UiMsg::Task { id, request } => (*id, Self::Task(request.clone())),
            _ => unreachable!("composer only emits messaging/task operations"),
        }
    }
    fn command(&self, id: Ulid) -> UiMsg {
        match self {
            Self::Message { swarm, request } => UiMsg::Messaging {
                id,
                swarm_id: *swarm,
                request: request.clone(),
            },
            Self::Task(request) => UiMsg::Task {
                id,
                request: request.clone(),
            },
        }
    }
}

fn send(
    swarm: SwarmId,
    destination: &str,
    text: &str,
    kind: MessageKind,
    priority: u8,
    attachments: Vec<String>,
) -> UiMsg {
    let id = Ulid::new();
    UiMsg::Messaging {
        id,
        swarm_id: swarm,
        request: Request::Send {
            message: wire::SendRequest {
                id,
                destination: destination.into(),
                text: text.into(),
                kind,
                priority,
                attachments,
                thread_id: None,
                reply_to: None,
            },
        },
    }
}

/// Read-through task/tree snapshots for the selected swarm. These are bounded
/// display caches; the backend journal, bus positions and read marks stay authority.
#[derive(Default)]
pub struct Records {
    scope: Option<SwarmId>,
    pending: BTreeMap<Ulid, RecordRequest>,
    waiting: VecDeque<Read>,
    tasks: VecDeque<wire::TaskCard>,
    docs: VecDeque<wire::TaskDoc>,
    nodes: Vec<wire::Node>,
    pub task_next: Option<Ulid>,
    pub tree_next: BTreeMap<String, Option<String>>,
    pub issue: Option<String>,
    pub omitted: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Read {
    Tasks(Option<Ulid>),
    Detail(Ulid),
    Tree(String, Option<String>),
}
struct RecordRequest {
    read: Option<Read>,
    task: Option<Ulid>,
    command: Saved,
    serial: u64,
    unknown: bool,
}
impl Records {
    pub fn scope(&self) -> Option<SwarmId> {
        self.scope
    }
    pub fn tasks(&self) -> &VecDeque<wire::TaskCard> {
        &self.tasks
    }
    pub fn task(&self, id: Ulid) -> Option<&wire::TaskCard> {
        self.tasks.iter().find(|t| t.id == id)
    }
    pub fn doc(&self, id: Ulid) -> Option<&wire::TaskDoc> {
        self.docs.iter().find(|t| t.id == id)
    }
    pub fn nodes(&self) -> &[wire::Node] {
        &self.nodes
    }
    pub fn select(&mut self, scope: SwarmId) {
        if self.scope == Some(scope) {
            return;
        }
        let pending = std::mem::take(&mut self.pending)
            .into_iter()
            .filter(|(_, p)| p.read.is_none())
            .collect();
        *self = Self {
            scope: Some(scope),
            pending,
            ..Default::default()
        };
        self.refresh();
    }
    fn enqueue(&mut self, read: Read) {
        if self.waiting.len() < 132
            && !self.waiting.contains(&read)
            && !self
                .pending
                .values()
                .any(|p| p.read.as_ref() == Some(&read))
        {
            self.waiting.push_back(read);
        }
    }
    pub fn refresh(&mut self) {
        // Read retries have no side effect. Unknown mutations keep their original
        // ids until the operator repeats that same action explicitly.
        self.pending.retain(|_, p| p.read.is_none() || !p.unknown);
        self.enqueue(Read::Tasks(None));
        for path in ["channels", "groups"] {
            self.enqueue(Read::Tree(path.into(), None));
        }
    }
    pub fn more_tasks(&mut self) {
        if let Some(next) = self.task_next {
            self.enqueue(Read::Tasks(Some(next)));
        }
    }
    pub fn more_tree(&mut self, path: &str) {
        if let Some(Some(next)) = self.tree_next.get(path) {
            self.enqueue(Read::Tree(path.into(), Some(next.clone())));
        }
    }
    pub fn detail(&mut self, id: Ulid) {
        if self.task(id).is_some() {
            self.enqueue(Read::Detail(id));
        }
    }
    pub fn commands(&mut self) -> Vec<UiMsg> {
        let Some(scope) = self.scope else {
            return vec![];
        };
        let mut commands = vec![];
        while self.pending.len() < 8 {
            let Some(read) = self.waiting.pop_front() else {
                break;
            };
            let id = Ulid::new();
            let command = match &read {
                Read::Tasks(after) => UiMsg::Task {
                    id,
                    request: TaskRequest::List {
                        swarm_id: Some(Ulid::from(scope.0.0)),
                        status: None,
                        assignee: None,
                        after: *after,
                        limit: Some(128),
                    },
                },
                Read::Detail(task) => {
                    let Some(card) = self.task(*task) else {
                        continue;
                    };
                    UiMsg::Task {
                        id,
                        request: TaskRequest::Status {
                            task_path: Some(card.path.clone()),
                            task_id: None,
                        },
                    }
                }
                Read::Tree(path, after) => UiMsg::Messaging {
                    id,
                    swarm_id: scope,
                    request: Request::Ls {
                        path: path.clone(),
                        after: after.clone(),
                        limit: 128,
                    },
                },
            };
            self.pending.insert(
                id,
                RecordRequest {
                    read: Some(read),
                    task: None,
                    command: Saved::capture(&command).1,
                    serial: 0,
                    unknown: false,
                },
            );
            commands.push(command);
        }
        commands
    }
    /// A second click on an unknown task operation reuses its captured request,
    /// never a new mutation id. A changed action is refused until it is resolved.
    pub fn review(&mut self, task: Ulid, reason: Option<String>) -> Result<UiMsg, &'static str> {
        if let Some((&original_id, p)) = self.pending.iter_mut().find(|(_, p)| p.task == Some(task))
        {
            if !p.unknown {
                return Err("task_operation_in_flight");
            }
            let Saved::Task(request) = &p.command else {
                unreachable!();
            };
            let same = match (request, reason.as_deref()) {
                (TaskRequest::Accept { .. }, None) => true,
                (TaskRequest::Reject { reason: old, .. }, Some(new)) => old == new,
                _ => false,
            };
            if !same {
                return Err("task_outcome_unknown_repeat_original_action");
            }
            p.unknown = false;
            return Ok(p.command.command(original_id));
        }
        if self.pending.len() >= 8 {
            return Err("task_operation_queue_full");
        }
        let card = self.task(task).ok_or("task_not_loaded")?;
        let request = if let Some(reason) = reason {
            if reason.trim().is_empty() {
                return Err("rebrief_reason_required");
            }
            TaskRequest::Reject {
                task_path: card.path.clone(),
                reason,
            }
        } else {
            if card.status != "reported" || card.verify.as_ref().is_none_or(|v| v.exit_code != 0) {
                return Err("task_requires_reported_zero_exit");
            }
            TaskRequest::Accept {
                task_path: card.path.clone(),
            }
        };
        let id = Ulid::new();
        let command = UiMsg::Task { id, request };
        self.pending.insert(
            id,
            RecordRequest {
                read: None,
                task: Some(task),
                command: Saved::capture(&command).1,
                serial: 0,
                unknown: false,
            },
        );
        Ok(command)
    }
    pub fn replies(&mut self, results: &Results) -> bool {
        let mut changed = false;
        for row in results.rows() {
            let Some(p) = self.pending.get_mut(&row.op) else {
                continue;
            };
            if row.serial <= p.serial {
                continue;
            }
            p.serial = row.serial;
            if let Reply::Unknown(reason) = &row.reply {
                p.unknown = true;
                self.issue = Some(format!("task/read outcome unknown · {reason}"));
                changed = true;
                continue;
            }
            let p = self.pending.remove(&row.op).unwrap();
            match (&p.read, &row.reply) {
                (Some(Read::Tasks(after)), Reply::Task(TaskResponse::Items { items, next })) => {
                    if after.is_none() {
                        self.tasks.clear();
                        self.omitted = 0;
                    }
                    self.task_next = *next;
                    for card in items {
                        if let Some(i) = self.tasks.iter().position(|t| t.id == card.id) {
                            self.tasks.remove(i);
                        }
                        self.tasks.push_back(card.clone());
                        if card.status == "reported" && self.docs.iter().all(|d| d.id != card.id) {
                            self.enqueue(Read::Detail(card.id));
                        }
                    }
                    while self.tasks.len() > 128
                        || serde_json::to_vec(&self.tasks).map_or(true, |b| b.len() > 256 * 1024)
                    {
                        if self.tasks.pop_front().is_none() {
                            break;
                        }
                        self.omitted += 1;
                    }
                    self.docs
                        .retain(|d| self.tasks.iter().any(|t| t.id == d.id));
                }
                (Some(Read::Detail(id)), Reply::Task(TaskResponse::Item { item }))
                    if item.id == *id =>
                {
                    self.docs.retain(|d| d.id != *id);
                    self.docs.push_back(item.clone());
                    while self.docs.len() > 16
                        || serde_json::to_vec(&self.docs).map_or(true, |b| b.len() > 512 * 1024)
                    {
                        if self.docs.pop_front().is_none() {
                            break;
                        }
                    }
                }
                (
                    Some(Read::Tree(folder, after)),
                    Reply::Messaging(Response::Tree { nodes, next }),
                ) => {
                    if after.is_none() {
                        let prefix = format!("{folder}/");
                        self.nodes.retain(|n| !n.path.starts_with(&prefix));
                    }
                    self.tree_next.insert(folder.clone(), next.clone());
                    for node in nodes {
                        self.nodes.retain(|n| n.path != node.path);
                        self.nodes.push(node.clone());
                    }
                    while self.nodes.len() > 256
                        || serde_json::to_vec(&self.nodes).map_or(true, |b| b.len() > 256 * 1024)
                    {
                        if self.nodes.is_empty() {
                            break;
                        }
                        self.nodes.remove(0);
                        self.omitted += 1;
                    }
                }
                (None, Reply::Task(TaskResponse::Applied)) => {
                    self.issue = None;
                    self.enqueue(Read::Tasks(None));
                    if let Some(task) = p.task {
                        self.docs.retain(|d| d.id != task);
                        self.enqueue(Read::Detail(task));
                    }
                }
                (_, Reply::Refused(reason)) => {
                    self.issue = Some(reason.chars().take(512).collect())
                }
                _ => self.issue = Some("unexpected_task_or_tree_reply".into()),
            }
            changed = true;
        }
        changed
    }
}
