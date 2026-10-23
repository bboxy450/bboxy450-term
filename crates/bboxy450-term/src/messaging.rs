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
