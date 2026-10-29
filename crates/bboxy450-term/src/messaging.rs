// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
mod history;
pub use history::{History, HistoryAction};

pub const MAX_RESULTS: usize = 64;
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;
pub const MAX_DESTINATIONS: usize = 32;

#[cfg(test)]
mod empty_swarm_reads {
    use super::*;

    #[test]
    fn tree_reads_wait_for_membership_without_blocking_or_losing_task_reads() {
        let mut records = Records::default();
        records.select(SwarmId::from_u128(1));
        let tasks = records.commands_with_messaging(false);
        assert_eq!(tasks.len(), 2);
        assert!(
            tasks
                .iter()
                .all(|command| matches!(command, UiMsg::Task { .. }))
        );
        assert!(records.commands_with_messaging(false).is_empty());
        let trees = records.commands_with_messaging(true);
        assert_eq!(trees.len(), 2);
        assert!(
            trees
                .iter()
                .all(|command| matches!(command, UiMsg::Messaging { .. }))
        );
        assert!(records.commands_with_messaging(true).is_empty());
    }
}

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
        message_ids: vec![row.id.to_string()],
        author: Some(Author {
            kind: match row.author.kind {
                wire::AuthorKind::Seat => "seat",
                wire::AuthorKind::Operator | wire::AuthorKind::OperatorLocal => "operator",
                wire::AuthorKind::Observer => "observer",
            }
            .into(),
            id: row.author.id.clone(),
            name: row.author.name.clone(),
        }),
        issuer_kind: row.issuer_kind.map(|kind| {
            match kind {
                wire::IssuerKind::LocalDev => "local-dev",
                wire::IssuerKind::GpgRoot => "gpg-root",
            }
            .into()
        }),
        signature: (row.signature == wire::SignatureState::Pending || row.upgrade_of.is_some())
            .then_some(
                match row.signature {
                    wire::SignatureState::Signed => "signed",
                    wire::SignatureState::Pending => "pending",
                }
                .into(),
            ),
        signature_reason: row.signature_reason.clone(),
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
                MessageKind::SlotClaimed => "slot_claimed",
                MessageKind::SlotReleased => "slot_released",
                MessageKind::CutClaimed => "cut_claimed",
                MessageKind::Landed => "landed",
                MessageKind::GateResult => "gate_result",
                MessageKind::TaskTransition => "task_transition",
                MessageKind::Withdrawn => "withdrawn",
            }
            .into(),
        ),
        text: row.text.clone(),
        ts: Some(row.ts),
        time: crate::local_time::hm(row.ts),
        role,
        // v0.1 has one operator per account; other rows' receipt aggregates are
        // not displayed as the operator's own delivery evidence.
        receipts: matches!(
            row.author.kind,
            wire::AuthorKind::Operator | wire::AuthorKind::OperatorLocal
        )
        .then_some(Receipts {
            delivered: row.receipts.delivered,
            acked: row.receipts.acked,
            held: row.receipts.held,
        }),
        ..Default::default()
    }
}

pub fn assignee_seat(reference: &str) -> Option<bbt_proto::SeatId> {
    reference
        .strip_prefix("seat:")
        .unwrap_or(reference)
        .parse()
        .ok()
}
fn assignee_name(reference: &str, model: &crate::model::Model) -> String {
    assignee_seat(reference)
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
// Keep the provider path intact: Delegate consumes a task path. Only the
// message-building seam below converts that locator to a wire wikilink.
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
        reject: None,
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
    RolesConfigured,
    Messaging(Response),
    Task(TaskResponse),
    Attention,
    Plugins,
    KnowledgeControl,
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
        let pending: Vec<_> = std::mem::take(&mut self.pending).into_iter().collect();
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

fn attachment_reference(value: &str) -> Result<String, &'static str> {
    if value.is_empty() {
        return Err("attachment_unresolved");
    }
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err("attachment_reference_invalid");
    }
    let (inner, wrapped) =
        if let Some(inner) = value.strip_prefix("[[").and_then(|s| s.strip_suffix("]]")) {
            (inner, true)
        } else {
            (value, false)
        };
    if inner.contains(['[', ']', '{', '}']) {
        return Err("attachment_reference_unknown_shape");
    }
    let (target, alias) = inner
        .split_once('|')
        .map_or((inner, None), |(target, alias)| {
            (target.strip_suffix('\\').unwrap_or(target), Some(alias))
        });
    if alias.is_some_and(|alias| alias.trim().is_empty()) {
        return Err("attachment_reference_invalid");
    }
    if target.is_empty()
        || target.trim() != target
        || target.contains([':', '\\'])
        || target.ends_with('/')
    {
        return Err("attachment_reference_unknown_shape");
    }
    let known = if target.starts_with('#') {
        bbt_ui::modules::document_modal::controller::numbered(target).is_some()
    } else {
        std::path::Path::new(target)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    };
    if !known {
        return Err("attachment_reference_unknown_shape");
    }
    let reference = if wrapped {
        value.to_owned()
    } else {
        format!("[[{value}]]")
    };
    if reference.len() > 512 {
        return Err("attachment_reference_too_large");
    }
    Ok(reference)
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
    images_committed: bool,
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
    groups: VecDeque<SendGroup>,
}
/// Presentation metadata binds a role pick to its actual journaled sends.
/// Receipts always come from the shared MessageRow projections.
pub struct SendGroup {
    pub swarm: SwarmId,
    pub label: String,
    pub ids: Vec<Ulid>,
}

/// Resolve chrome input in the selected swarm before the composer admits effects.
pub fn ui_compose_draft(
    model: &crate::model::Model,
    stage: &bbt_ui_model::Stage,
    request: &bbt_proto::gui::UiCompose,
) -> Result<Submission, &'static str> {
    use bbt_proto::gui::UiComposeTarget;
    request.validate()?;
    let (_, choices) = catalog(model, stage).ok_or("swarm_required")?;
    let address = match &request.to {
        UiComposeTarget::Seat(id) => choices
            .iter()
            .flat_map(|group| &group.items)
            .find(|address| address.kind == "seat" && address.id == id.to_string())
            .cloned()
            .ok_or("composer_seat_not_in_swarm")?,
        UiComposeTarget::Role(role) => choices
            .iter()
            .flat_map(|group| &group.items)
            .find(|address| address.kind == "role" && address.role == *role)
            .cloned()
            .ok_or("role_has_no_members")?,
        UiComposeTarget::Channel(name) => {
            let name = name.trim_start_matches("channels/");
            Address {
                kind: "channel".into(),
                id: format!("channels/{name}"),
                label: name.into(),
                destination: format!("channels/{name}"),
                ..Default::default()
            }
        }
    };
    Ok(Submission {
        delegate: false,
        to: vec![address],
        attach: vec![],
        text: request.text.clone(),
        kind: "text".into(),
        priority: 0,
    })
}
impl Client {
    pub fn has_pending(&self) -> bool { self.pending.is_some() }
    /// At most the current batch's 32 image references; no completed-send history.
    pub fn protected_image_paths(&self) -> Vec<String> {
        self.pending.as_ref().map_or_else(Vec::new, |pending| {
            pending.draft.attach.iter().filter(|a| a.kind == "image").map(|a| a.path.clone()).collect()
        })
    }
    pub fn groups(&self) -> &VecDeque<SendGroup> {
        &self.groups
    }
    /// The caller reserves the whole bounded UI queue batch before `admitted`.
    /// Scope is captured from the selected stage; no first/default swarm exists.
    pub fn prepare(
        &self,
        swarm: Option<SwarmId>,
        draft: Submission,
    ) -> Result<Batch, &'static str> {
        let recipients = draft.to.clone();
        self.prepare_recipients(swarm, draft, recipients, None)
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
        let workspace = if draft.delegate {
            Some(model.swarm_scope(swarm)?)
        } else {
            None
        };
        self.prepare_recipients(Some(swarm), draft, recipients, workspace)
    }
    fn prepare_recipients(
        &self,
        swarm: Option<SwarmId>,
        draft: Submission,
        recipients: Vec<Address>,
        workspace: Option<bbt_proto::gui::SwarmScope>,
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
        if draft.attach.len() > 32 {
            return Err("attachment_count");
        }
        let attachments = draft
            .attach
            .iter()
            .map(|a| if a.kind == "image" { crate::composer_images::reference(a) } else { attachment_reference(&a.path) })
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
                scope: workspace,
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
                images_committed: false,
            },
        })
    }
    pub fn admitted(&mut self, mut batch: Batch) -> Vec<UiMsg> {
        self.status = Status::Pending;
        self.seen.clear();
        let commands = std::mem::take(&mut batch.commands);
        if !batch.pending.draft.delegate
            && batch
                .pending
                .draft
                .to
                .iter()
                .any(|address| address.kind == "role")
        {
            self.groups.push_back(SendGroup {
                swarm: batch.pending.swarm,
                label: batch
                    .pending
                    .draft
                    .to
                    .iter()
                    .map(|address| address.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                ids: commands
                    .iter()
                    .filter_map(|message| {
                        if let UiMsg::Messaging { id, .. } = message {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
            while self.groups.len() > 32 {
                self.groups.pop_front();
            }
        }
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
        let (completed, follow_up, _) = self.replies_with_images(results);
        (completed, follow_up)
    }
    /// The window consumes newly durable image paths in this same update. Each batch
    /// emits at most 32 paths once, even for fan-out or replayed/late receipts.
    /// No committed path queue or sent-image index survives the reply operation.
    pub fn replies_with_images(&mut self, results: &Results) -> (Option<Submission>, Vec<UiMsg>, Vec<String>) {
        let Some(pending) = &mut self.pending else {
            return (None, vec![], vec![]);
        };
        let mut follow_up = vec![];
        let mut committed_images = vec![];
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
            if !task && !pending.images_committed && matches!(&result.reply, Reply::Messaging(Response::Stored { .. } | Response::Held { .. })) {
                committed_images.extend(pending.draft.attach.iter().filter(|a| a.kind == "image").map(|a| a.path.clone()));
                pending.images_committed = true;
            }
            match &result.reply {
                Reply::Unknown(reason) => {
                    self.status = Status::Unknown(reason.clone());
                    continue;
                }
                Reply::Messaging(Response::Held { reason, .. }) if !task => {
                    pending.held.push(reason.clone());
                }
                Reply::Messaging(Response::Stored { .. }) if !task => {
                    pending.stored += 1;
                }
                Reply::Messaging(Response::Refused { error }) if !task => {
                    pending
                        .failures
                        .push(format!("{}: {}; {}", error.kind, error.message, error.hint));
                }
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
            return (None, follow_up, committed_images);
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
            (Some(pending.draft), follow_up, committed_images)
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
            (None, follow_up, committed_images)
        }
    }
}

/// Original retained attachment references, with no inferred record type or
/// rewritten target. Bounds match the admitted W2.4 attachment vocabulary.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageAttachments {
    pub references: Vec<String>,
    pub omitted: usize,
}
impl MessageAttachments {
    pub fn bounded(references: &[String]) -> Self {
        let kept: Vec<_> = references
            .iter()
            .take(32)
            .filter(|s| s.len() <= 512)
            .cloned()
            .collect();
        Self {
            omitted: references.len() - kept.len(),
            references: kept,
        }
    }
}
pub fn message_entries(
    rows: &[wire::MessageRow],
    contexts: &BTreeMap<Ulid, String>,
    groups: &VecDeque<SendGroup>,
    swarm: SwarmId,
    model: &crate::model::Model,
    read: Option<(&str, u64)>,
) -> Vec<bbt_ui::modules::stream_dock::Entry> {
    message_entries_with_attachments(rows, contexts, groups, swarm, model, read, &BTreeMap::new())
}

pub fn message_entries_with_attachments(
    rows: &[wire::MessageRow],
    contexts: &BTreeMap<Ulid, String>,
    groups: &VecDeque<SendGroup>,
    swarm: SwarmId,
    model: &crate::model::Model,
    read: Option<(&str, u64)>,
    attachments: &BTreeMap<Ulid, MessageAttachments>,
) -> Vec<bbt_ui::modules::stream_dock::Entry> {
    use bbt_ui::modules::stream_dock::{Entry, Receipts};
    let mut out = vec![];
    let mut shown = BTreeSet::new();
    for row in rows {
        if !shown.insert(row.id) {
            continue;
        }
        let mut view = message_view(row, model);
        view.ch = contexts
            .get(&row.id)
            .and_then(|path| path.strip_prefix("channels/"))
            .map(String::from);
        view.unread = read.is_some_and(|(stream, read)| {
            row.bus
                .as_ref()
                .is_some_and(|bus| bus.stream == stream && bus.seq > read)
        });
        let mut gap = None;
        if let Some(attachments) = attachments.get(&row.id) {
            view.attach = attachments
                .references
                .iter()
                .map(|reference| bbt_ui::modules::composer::Attachment {
                    kind: "record".into(),
                    title: reference.clone(),
                    path: reference.clone(),
                    ..Default::default()
                })
                .collect();
            if attachments.omitted > 0 {
                gap = Some(format!(
                    "attachment_view_limit: {} references not shown; originals retained",
                    attachments.omitted
                ));
            }
        }
        if let Some(group) = groups
            .iter()
            .find(|group| group.swarm == swarm && group.ids.contains(&row.id))
        {
            let members: Vec<_> = group
                .ids
                .iter()
                .filter_map(|id| rows.iter().find(|row| row.id == *id))
                .collect();
            shown.extend(group.ids.iter().copied());
            view.to_label = Some(format!("{} · {} recipients", group.label, group.ids.len()));
            if members.len() == group.ids.len() {
                view.message_ids = group.ids.iter().map(ToString::to_string).collect();
                view.receipts = Some(Receipts {
                    delivered: members.iter().map(|row| row.receipts.delivered).sum(),
                    acked: members.iter().map(|row| row.receipts.acked).sum(),
                    held: members.iter().map(|row| row.receipts.held).sum(),
                });
                view.unread = members.iter().any(|row| {
                    read.is_some_and(|(stream, read)| {
                        row.bus
                            .as_ref()
                            .is_some_and(|bus| bus.stream == stream && bus.seq > read)
                    })
                });
            } else {
                view.receipts = None;
                let receipts = format!(
                    "receipt coverage incomplete · {} of {} message projections",
                    members.len(),
                    group.ids.len()
                );
                gap = Some(gap.map_or_else(
                    || receipts.clone(),
                    |attachments| format!("{attachments} · {receipts}"),
                ));
            }
        }
        out.push(Entry::Message {
            data: Box::new(view),
            show_channel: Some(true),
        });
        if let Some(text) = gap {
            out.push(Entry::Note {
                text,
                padding: [4.; 4],
            });
        }
    }
    out
}

#[derive(Clone)]
enum Saved {
    Message {
        swarm: SwarmId,
        request: Request,
    },
    Task {
        request: TaskRequest,
        scope: Option<bbt_proto::gui::SwarmScope>,
    },
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
            UiMsg::Task { id, request, scope } => (
                *id,
                Self::Task {
                    request: request.clone(),
                    scope: *scope,
                },
            ),
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
            Self::Task { request, scope } => UiMsg::Task {
                id,
                request: request.clone(),
                scope: *scope,
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
                supersedes: None,
            },
        },
    }
}

/// Read-through task/tree snapshots for the selected swarm. These are bounded
/// display caches; the backend journal, bus positions and read marks stay authority.
#[derive(Default)]
pub struct Records {
    scope: Option<SwarmId>,
    workspace: Option<bbt_proto::gui::SwarmScope>,
    workspace_required: bool,
    retry_epoch: u64,
    retry_blocked: bool,
    retry_delay: Option<std::time::Duration>,
    retry_started: Option<std::time::Instant>,
    retries: u8,
    pending: BTreeMap<Ulid, RecordRequest>,
    waiting: VecDeque<Read>,
    tasks: VecDeque<wire::TaskCard>,
    unassigned: VecDeque<wire::TaskCard>,
    docs: VecDeque<wire::TaskDoc>,
    nodes: Vec<wire::Node>,
    pub task_next: Option<Ulid>,
    pub unassigned_next: Option<Ulid>,
    pub tree_next: BTreeMap<String, Option<String>>,
    pub issue: Option<String>,
    pub omitted: usize,
    pub selected: Option<Ulid>,
    pub review_issue: Option<(Ulid, String)>,
    review_completed: Option<(Ulid, Option<String>)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Read {
    Tasks(Option<Ulid>),
    Unassigned(Option<Ulid>),
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
    pub fn bind_workspace(
        &mut self,
        binding: Result<bbt_proto::gui::SwarmScope, &'static str>,
    ) -> bool {
        let next = binding.ok();
        if self.workspace_required && self.workspace == next {
            return false;
        }
        self.workspace_required = true;
        self.workspace = next;
        self.retry_epoch = self.retry_epoch.wrapping_add(1);
        self.retry_blocked = false;
        self.retry_delay = None;
        self.retry_started = None;
        self.retries = 0;
        self.pending.retain(|_, request| {
            request.read.is_none() || matches!(request.read, Some(Read::Tree(..)))
        });
        self.waiting.retain(|read| matches!(read, Read::Tree(..)));
        self.tasks.clear();
        self.unassigned.clear();
        self.docs.clear();
        self.task_next = None;
        self.unassigned_next = None;
        self.selected = None;
        self.review_issue = None;
        self.review_completed = None;
        self.issue = binding.err().map(Into::into);
        if next.is_some() {
            self.refresh();
        }
        true
    }
    pub fn take_retry(&mut self) -> Option<(u64, std::time::Duration)> {
        self.retry_delay
            .take()
            .map(|delay| (self.retry_epoch, delay))
    }
    pub fn retry_ready(&mut self, epoch: u64) -> bool {
        if epoch == self.retry_epoch && self.retry_blocked {
            self.retry_blocked = false;
            true
        } else {
            false
        }
    }
    pub fn scope(&self) -> Option<SwarmId> {
        self.scope
    }
    pub fn tasks(&self) -> &VecDeque<wire::TaskCard> {
        &self.tasks
    }
    pub fn unassigned(&self) -> &VecDeque<wire::TaskCard> {
        &self.unassigned
    }
    pub fn task(&self, id: Ulid) -> Option<&wire::TaskCard> {
        self.tasks
            .iter()
            .chain(&self.unassigned)
            .find(|t| t.id == id)
    }
    pub fn doc(&self, id: Ulid) -> Option<&wire::TaskDoc> {
        self.docs.iter().find(|t| t.id == id)
    }
    pub fn take_review_completion(&mut self) -> Option<(Ulid, Option<String>)> {
        self.review_completed.take()
    }
    pub fn nodes(&self) -> &[wire::Node] {
        &self.nodes
    }
    pub fn entries(
        &self,
        model: &crate::model::Model,
        reject: Option<(Ulid, &bbt_ui::modules::composer::Reject)>,
    ) -> Vec<bbt_ui::modules::stream_dock::Entry> {
        use bbt_ui::modules::stream_dock::Entry;
        let mut out: Vec<_> = self
            .tasks
            .iter()
            .map(|card| Entry::Task {
                data: task_item(card, model),
                delegate: true,
            })
            .collect();
        if !self.unassigned.is_empty() {
            out.push(Entry::Note {
                text: "Unassigned tasks · account".into(),
                padding: [4.; 4],
            });
            out.extend(self.unassigned.iter().map(|card| Entry::Task {
                data: task_item(card, model),
                delegate: true,
            }));
        }
        if let Some(card) = self.selected.and_then(|id| self.task(id))
            && let Some(doc) = self.doc(card.id)
        {
            match task_report(card, doc) {
                Ok(Some(mut report)) => {
                    if let Some((id, reason)) = reject
                        && id == card.id
                    {
                        report.reject = Some(reason.clone());
                    }
                    out.push(Entry::Report {
                        id: card.id.to_string(),
                        data: report,
                        review: true,
                    });
                }
                Err(reason) => out.push(Entry::Note {
                    text: reason.into(),
                    padding: [4.; 4],
                }),
                _ => {}
            }
        }
        if let Some(issue) = &self.issue {
            out.push(Entry::Note {
                text: issue.clone(),
                padding: [4.; 4],
            });
        }
        if self.omitted > 0 {
            out.push(Entry::Note {
                text: format!(
                    "{} records outside this window · retained on the backend",
                    self.omitted
                ),
                padding: [4.; 4],
            });
        }
        out
    }
    pub fn select(&mut self, scope: SwarmId) {
        if self.scope == Some(scope) {
            return;
        }
        let pending = std::mem::take(&mut self.pending)
            .into_iter()
            .filter(|(_, p)| p.read.is_none())
            .collect();
        let retry_epoch = self.retry_epoch.wrapping_add(1);
        *self = Self {
            scope: Some(scope),
            retry_epoch,
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
        self.enqueue(Read::Unassigned(None));
        for path in ["channels", "groups"] {
            self.enqueue(Read::Tree(path.into(), None));
        }
    }
    pub fn task_notice(&mut self, id: Option<Ulid>) {
        self.enqueue(Read::Tasks(None));
        self.enqueue(Read::Unassigned(None));
        if let Some(id) = id {
            self.docs.retain(|doc| doc.id != id);
            if self.task(id).is_some() {
                self.enqueue(Read::Detail(id));
            }
        }
    }
    pub fn more_tasks(&mut self) {
        if let Some(next) = self.task_next {
            self.enqueue(Read::Tasks(Some(next)));
        }
        if let Some(next) = self.unassigned_next {
            self.enqueue(Read::Unassigned(Some(next)));
        }
    }
    pub fn more_tree(&mut self, path: &str) {
        if let Some(Some(next)) = self.tree_next.get(path) {
            self.enqueue(Read::Tree(path.into(), Some(next.clone())));
        }
    }
    pub fn detail(&mut self, id: Ulid) {
        if let Some(task) = self.task(id) {
            let scoped = task.assignee.is_some();
            self.selected = Some(id);
            if scoped {
                self.enqueue(Read::Detail(id));
            }
        }
    }
    pub fn commands(&mut self) -> Vec<UiMsg> {
        self.commands_with_messaging(true)
    }
    pub fn commands_with_messaging(&mut self, messaging_ready: bool) -> Vec<UiMsg> {
        let Some(scope) = self.scope else {
            return vec![];
        };
        let mut commands = vec![];
        let mut remaining = self.waiting.len();
        while self.pending.len() < 8 && remaining > 0 {
            remaining -= 1;
            let Some(read) = self.waiting.pop_front() else {
                break;
            };
            if ((self.workspace_required && self.workspace.is_none() || self.retry_blocked)
                && !matches!(read, Read::Tree(..)))
                || (!messaging_ready && matches!(read, Read::Tree(..)))
            {
                self.waiting.push_back(read);
                continue;
            }
            let id = Ulid::new();
            let command = match &read {
                Read::Tasks(after) | Read::Unassigned(after) => UiMsg::Task {
                    id,
                    scope: self.workspace,
                    request: TaskRequest::List {
                        unassigned: matches!(&read, Read::Unassigned(_)),
                        swarm_id: matches!(&read, Read::Tasks(_)).then(|| Ulid::from(scope.0.0)),
                        status: matches!(&read, Read::Unassigned(_)).then(|| "open".into()),
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
                        scope: self.workspace,
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
        if self.workspace_required && self.workspace.is_none() {
            return Err("workspace_metadata_pending");
        }
        if let Some((&original_id, p)) = self.pending.iter_mut().find(|(_, p)| {
            p.task == Some(task)
                && (!self.workspace_required
                    || matches!(&p.command,Saved::Task{scope,..}if *scope==self.workspace))
        }) {
            if !p.unknown {
                return Err("task_operation_in_flight");
            }
            let Saved::Task { request, .. } = &p.command else {
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
        let command = UiMsg::Task {
            id,
            request,
            scope: self.workspace,
        };
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
            if self.workspace_required
                && matches!(&p.command,Saved::Task{scope,..}if *scope!=self.workspace)
            {
                if let Reply::Unknown(_) = &row.reply {
                    p.unknown = true;
                    p.serial = row.serial;
                } else {
                    self.pending.remove(&row.op);
                }
                continue;
            }
            p.serial = row.serial;
            if let Reply::Unknown(reason) = &row.reply {
                p.unknown = true;
                self.issue = Some(format!("task/read outcome unknown · {reason}"));
                if let Some(task) = p.task {
                    self.review_issue = Some((
                        task,
                        format!("outcome unknown · repeat the same action to retry · {reason}"),
                    ));
                }
                changed = true;
                continue;
            }
            let p = self.pending.remove(&row.op).unwrap();
            if let (Some(read), Reply::Refused(reason)) = (&p.read, &row.reply)
                && !matches!(read, Read::Tree(..))
                && reason.starts_with("workspace_pending")
            {
                let now = std::time::Instant::now();
                let began = self.retry_started.get_or_insert(now);
                if self.retries < 50
                    && now.saturating_duration_since(*began) < std::time::Duration::from_secs(10)
                {
                    self.retries += 1;
                    self.enqueue(read.clone());
                    if !self.retry_blocked {
                        self.retry_blocked = true;
                        self.retry_delay = Some(std::time::Duration::from_millis(100));
                    }
                    changed = true;
                    continue;
                }
            }
            match (&p.read, &row.reply) {
                (
                    Some(Read::Tasks(after) | Read::Unassigned(after)),
                    Reply::Task(TaskResponse::Items { items, next }),
                ) => {
                    let unassigned = matches!(&p.read, Some(Read::Unassigned(_)));
                    if unassigned
                        && items
                            .iter()
                            .any(|card| card.status != "open" || card.assignee.is_some())
                    {
                        self.issue = Some("unassigned_inventory_mismatch".into());
                        changed = true;
                        continue;
                    }
                    for card in items {
                        if self.task(card.id) != Some(card) {
                            self.docs.retain(|doc| doc.id != card.id);
                        }
                    }
                    let target = if unassigned {
                        &mut self.unassigned
                    } else {
                        &mut self.tasks
                    };
                    if after.is_none() {
                        target.clear();
                        self.omitted = 0;
                    }
                    if unassigned {
                        self.unassigned_next = *next;
                    } else {
                        self.task_next = *next;
                    }
                    for card in items {
                        if let Some(i) = target.iter().position(|t| t.id == card.id) {
                            target.remove(i);
                        }
                        target.push_back(card.clone());
                    }
                    while target.len() > 128
                        || serde_json::to_vec(&target).map_or(true, |b| b.len() > 256 * 1024)
                    {
                        if target.pop_front().is_none() {
                            break;
                        }
                        self.omitted += 1;
                    }
                    for card in items {
                        if card.status == "reported" && self.docs.iter().all(|d| d.id != card.id) {
                            self.enqueue(Read::Detail(card.id));
                        }
                    }
                    self.docs.retain(|d| {
                        self.tasks
                            .iter()
                            .chain(&self.unassigned)
                            .any(|t| t.id == d.id)
                    });
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
                    self.review_issue = None;
                    self.enqueue(Read::Tasks(None));
                    if let Some(task) = p.task {
                        self.review_completed = Some((
                            task,
                            if let Saved::Task {
                                request: TaskRequest::Reject { reason, .. },
                                ..
                            } = &p.command
                            {
                                Some(reason.clone())
                            } else {
                                None
                            },
                        ));
                        self.docs.retain(|d| d.id != task);
                        self.enqueue(Read::Detail(task));
                    }
                }
                (_, Reply::Refused(reason)) => {
                    self.issue = Some(reason.chars().take(512).collect());
                    if let Some(task) = p.task {
                        self.review_issue = Some((task, reason.chars().take(512).collect()));
                    }
                }
                _ => self.issue = Some("unexpected_task_or_tree_reply".into()),
            }
            changed = true;
        }
        changed
    }
}
