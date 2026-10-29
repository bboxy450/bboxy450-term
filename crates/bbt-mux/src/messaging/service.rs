// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The logic owner mediates keys, current grants, inboxes and the local bus.
use bbt_proto::messaging::{MessageRefusal, OperatorReference, OperatorReferenceKind, WakeMode};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ulid::Ulid;

use super::{
    keys::SeatKeys,
    ledger::{Delivery, Ledger, Page, Receipt, StoredMessage, Work, u64_at},
    tree::{Actor, Node, Recipient, Right, Route, Tree},
    wire::{Body, Envelope, EnvelopeExt, MessageKind, SendRequest, SendRequestExt, public},
};
use crate::{
    adapters::{State, WakeOutcome},
    registry::{Class, Registry, SeatRecord},
    store::{
        Result, Store, digest, events::Draft, invalid, object_dir, sqlint, sqlite::SqliteStore,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Grant {
    seat: Ulid,
    generation: u64,
    swarm: Ulid,
    role: crate::registry::Role,
    epoch: u64,
    muted: bool,
    #[serde(default)]
    operator_direct: bool,
    author: String,
}
struct Attention<'a> {
    swarm: Ulid,
    destination: &'a str,
    kind: MessageKind,
    reply_to: Option<Ulid>,
    operator: bool,
    delegated: bool,
}
impl<'a> Attention<'a> {
    fn signed(envelope: &'a Envelope) -> Self {
        Self {
            swarm: envelope.swarm_id,
            destination: &envelope.context,
            kind: envelope.kind,
            reply_to: envelope.reply_to,
            operator: envelope.seat == Ulid::nil(),
            delegated: matches!(&envelope.body, Body::Task { event, .. } if event == "delegate"),
        }
    }
    fn pending(intent: &'a bbt_proto::messaging::PendingIntent) -> Self {
        Self {
            swarm: intent.swarm_id,
            destination: &intent.request.destination,
            kind: intent.request.kind,
            reply_to: intent.request.reply_to,
            operator: !matches!(
                intent.sender,
                bbt_proto::messaging::LogicalSender::Seat { .. }
            ),
            delegated: matches!(&intent.body, Body::Task { event, .. } if event == "delegate"),
        }
    }
    fn message(message: &'a StoredMessage) -> Self {
        if let Some(envelope) = message.envelope() {
            Self::signed(envelope)
        } else {
            Self::pending(message.intent().expect("pending message has its intent"))
        }
    }
}
fn task_event_name<'a>(request: &SendRequest, body: &'a Body) -> Result<&'a str> {
    request.validate()?;
    let Body::Task { event, summary, .. } = body else {
        return Err(invalid("task producer requires a Task body"));
    };
    if request.kind != MessageKind::Task
        || request.text != *summary
        || !matches!(
            event.as_str(),
            "delegate" | "release" | "accept" | "reject" | "report"
        )
    {
        return Err(invalid("invalid task producer request"));
    }
    Ok(event)
}
#[derive(Serialize, Deserialize)]
struct HeldSend {
    seat: Ulid,
    generation: u64,
    swarm: Ulid,
    role: crate::registry::Role,
    epoch: u64,
    issuer_state: Option<Vec<u8>>,
    request: SendRequest,
    operator: bool,
    routes: Vec<Route>,
    /// An admitted local observation, retained before any recipient delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intent: Option<bbt_proto::messaging::PendingIntent>,
    /// Persist the exact completion with its sequence before the cross-store append.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completion: Option<Envelope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operator_context: Option<PendingOperator>,
}
#[derive(Clone, Serialize, Deserialize)]
struct PendingOperator {
    reference: OperatorReference,
    device: Option<Ulid>,
    door: bbt_proto::messaging::Door,
    issued_by: Option<String>,
    epoch: Option<u64>,
}
/// The target policy/version observed by an admitted operator-side control.
pub(crate) struct MuteGuard {
    pub generation: u64,
    pub epoch: u64,
}
fn pin_issuer(account: &SqliteStore, keys: &dyn SeatKeys) -> Result<()> {
    let Some(issuer) = keys.ready_issuer() else {
        return Ok(());
    };
    let issuer = serde_json::to_vec(&issuer)?;
    let mut db = account.connection()?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let old: Option<Vec<u8>> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='messaging/issuer'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(old) = old {
        if old != issuer {
            return Err(invalid(
                "local messaging issuer changed; explicit re-enrollment required",
            ));
        }
    } else {
        tx.execute("INSERT INTO kv VALUES('messaging/issuer',?1)", [&issuer])?;
    }
    tx.commit()?;
    Ok(())
}
struct Swarm {
    tree: Tree,
    ledger: Ledger,
    /// Last reconcile pass: pending rowid -> (stored-bytes hash, message, recipient),
    /// each message's resolution and each recipient's mute. A receipt whose bytes,
    /// message resolution and recipient are all unchanged would reconcile to itself.
    seen: HashMap<u64, (u128, u64, Recipient)>,
    resolved: HashMap<u64, Option<Resolved>>,
    muted: HashMap<Recipient, std::result::Result<bool, String>>,
}

/// A message's sender class, operator-direct flag and routes by (seat, agent), or the reason it cannot route.
type Resolved =
    std::result::Result<(Class, bool, HashMap<(Recipient, Option<String>), Route>), String>;

fn hash128(bytes: &[u8]) -> u128 {
    // Two independent multiply-rotate lanes over 8-byte words, then a
    // splitmix finish: fast enough to scan every pending row each pump.
    fn mix(mut z: u64) -> u64 {
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    let mut a = 0x9e37_79b9_7f4a_7c15 ^ bytes.len() as u64;
    let mut b: u64 = 0xc2b2_ae3d_27d4_eb4f;
    let (words, rest) = bytes.as_chunks::<8>();
    for w in words {
        let w = u64::from_le_bytes(*w);
        a = (a ^ w).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(31);
        b = (b.wrapping_add(w))
            .wrapping_mul(0xff51_afd7_ed55_8ccd)
            .rotate_left(27)
            ^ a;
    }
    let mut tail = [0u8; 8];
    tail[..rest.len()].copy_from_slice(rest);
    let w = u64::from_le_bytes(tail);
    (u128::from(mix(a ^ w)) << 64) | u128::from(mix(b.wrapping_add(w) ^ a))
}
#[derive(Clone, Copy)]
struct OperatorSource<'a> {
    signing: Option<&'a super::keys::OperatorSigning>,
    reference: OperatorReference,
    device: Option<Ulid>,
    origin: &'a crate::mcp::Origin,
}
#[derive(Clone, Copy)]
enum Source<'a> {
    Seat {
        record: &'a SeatRecord,
        operator: bool,
    },
    Operator(OperatorSource<'a>),
}
impl Source<'_> {
    fn actor(self) -> Actor {
        match self {
            Self::Seat {
                record,
                operator: false,
            } => Actor::Seat {
                seat: record.id,
                class: record.role.class,
            },
            _ => Actor::Operator,
        }
    }
    fn inbox(self) -> String {
        match self {
            Self::Seat { record, .. } => format!("seats/{}/inbox", record.id),
            Self::Operator(_) => "operator/inbox".into(),
        }
    }
}

pub struct Messaging {
    root: PathBuf,
    account: SqliteStore,
    registry: Registry,
    keys: Arc<dyn SeatKeys>,
    swarms: BTreeMap<Ulid, Swarm>,
    operator_reference: Option<OperatorReference>,
    operator_device_attribution: Option<Ulid>,
}
impl Messaging {
    fn supersedes_target(
        &mut self,
        swarm: Ulid,
        sender: Recipient,
        request: &mut SendRequest,
    ) -> Result<Option<MessageRefusal>> {
        let Some(id) = request.supersedes else {
            return Ok(None);
        };
        let ledger = &self.swarm(swarm)?.ledger;
        let owner = match ledger.sender(id) {
            Ok(owner) => Some(owner),
            Err(error)
                if error
                    .downcast_ref::<rusqlite::Error>()
                    .is_some_and(|e| matches!(e, rusqlite::Error::QueryReturnedNoRows)) =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        if owner != Some(sender) {
            return Ok(Some(MessageRefusal {
                kind: "invalid_supersedes".into(),
                message: "supersedes must name a message from the same sender".into(),
                hint: "Use one of your own retained message ids.".into(),
            }));
        }
        if request.destination.is_empty() {
            request.destination = ledger.by_id(id)?.view().context.into();
        }
        Ok(None)
    }

    /// Task authority comes from the existing Index, not a message's prose.
    #[cfg(test)]
    pub(crate) fn task_state(&mut self, task: Ulid, status: &str, now: u64) -> Result<()> {
        let key = format!("messaging/task-state/{task}");
        if self.account.get(&key)?.as_deref() != Some(status.as_bytes()) {
            self.account.put(&key, status.as_bytes())?;
        }
        for swarm in self.swarms.values() {
            swarm.ledger.task_state(task, status, now)?;
        }
        Ok(())
    }
    /// Workspace projection is scoped by canonical root and admitted swarm bindings.
    pub(crate) fn task_state_root(
        &mut self,
        root: &str,
        swarms: &[Ulid],
        task: Ulid,
        status: &str,
        now: u64,
    ) -> Result<()> {
        let key = format!("messaging/task-state-root/{root}/{task}");
        if self.account.get(&key)?.as_deref() != Some(status.as_bytes()) {
            self.account.put(&key, status.as_bytes())?;
        }
        let prefix = format!("messaging/task-root-swarm/{root}/");
        let mut targets: std::collections::BTreeSet<Ulid> = swarms.iter().copied().collect();
        for (key, _) in self.account.list(&prefix)? {
            if let Some(swarm) = key.strip_prefix(&prefix).and_then(|id| id.parse().ok()) {
                targets.insert(swarm);
            }
        }
        for swarm in targets {
            self.swarm(swarm)?
                .ledger
                .task_state_root(root, task, status, now)?;
        }
        Ok(())
    }
    pub(crate) fn startup_workspace(&self, root: &str) -> Result<()> {
        self.account.put("messaging/startup-root", root.as_bytes())
    }
    pub(crate) fn bind_workspace(&self, swarm: Ulid, root: &str) -> Result<()> {
        let key = format!("messaging/swarm-root/{swarm}");
        if self.account.get(&key)?.as_deref() != Some(root.as_bytes()) {
            self.account.put(&key, root.as_bytes())?;
        }
        Ok(())
    }
    pub(crate) fn task_root(&mut self, swarm: Ulid, message: Ulid, root: &str) -> Result<()> {
        self.swarm(swarm)?.ledger.task_root(message, root)?;
        self.account
            .put(&format!("messaging/task-root-swarm/{root}/{swarm}"), b"1")
    }
    fn sync_task(&mut self, swarm: Ulid, message: Ulid, body: &Body, now: u64) -> Result<()> {
        if let Body::Task { task_id, event, .. } = body
            && event == "delegate"
        {
            let mut root = self.swarm(swarm)?.ledger.task_root_id(message)?;
            if root.is_none()
                && let Some(startup) = self.account.get("messaging/startup-root")?
            {
                let startup = std::str::from_utf8(&startup)?.to_owned();
                self.swarm(swarm)?.ledger.task_root(message, &startup)?;
                root = Some(startup);
            }
            let key = root.as_ref().map_or_else(
                || format!("messaging/task-state/{task_id}"),
                |root| format!("messaging/task-state-root/{root}/{task_id}"),
            );
            if let Some(status) = self.account.get(&key)? {
                let status = std::str::from_utf8(&status)?;
                match root {
                    Some(root) => self
                        .swarm(swarm)?
                        .ledger
                        .task_state_root(&root, *task_id, status, now)?,
                    None => self
                        .swarm(swarm)?
                        .ledger
                        .task_state(*task_id, status, now)?,
                }
            }
        }
        Ok(())
    }
    fn reply_parent(
        &mut self,
        swarm: Ulid,
        reader: Recipient,
        id: Option<Ulid>,
    ) -> Result<Option<StoredMessage>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let message = match self.swarm(swarm)?.ledger.by_id(id) {
            Ok(message) => message,
            Err(error)
                if error
                    .downcast_ref::<rusqlite::Error>()
                    .is_some_and(|e| matches!(e, rusqlite::Error::QueryReturnedNoRows)) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if let Recipient::Seat(seat) = reader {
            match self.can_read(seat, &message) {
                Ok(()) => {}
                Err(error)
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::InvalidData) =>
                {
                    return Ok(None);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(Some(message))
    }
    fn thread_refusal(&mut self, swarm: Ulid, reader: Recipient) -> Result<MessageRefusal> {
        let tips = self.swarm(swarm)?.ledger.visible_thread_tips(reader)?;
        let hint = if tips.is_empty() {
            "No recent visible thread tips; list your inbox before replying, or start a new message.".into()
        } else {
            format!(
                "Use reply_to with a visible thread tip: {}. Omit thread_id to inherit it.",
                tips.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        Ok(MessageRefusal {
            kind: "unknown_thread".into(),
            message: "reply_to must name a message in a visible thread with a matching thread_id"
                .into(),
            hint,
        })
    }
    fn inherit_reply(
        &mut self,
        swarm: Ulid,
        reader: Recipient,
        parent: Option<&StoredMessage>,
        request: &mut SendRequest,
    ) -> Result<Option<MessageRefusal>> {
        let Some(parent) = parent else {
            return Ok(None);
        };
        let root = self.swarm(swarm)?.ledger.thread_root(parent.view().id)?.1;
        // The root's retained label is inherited. A branch's caller-supplied
        // label never changes which reply_to chain owns the conversation.
        let thread = self.swarm(swarm)?.ledger.by_id(root)?.view().thread_id;
        if request.thread_id.is_some_and(|id| id != thread) {
            return self.thread_refusal(swarm, reader).map(Some);
        }
        request.thread_id = Some(thread);
        Ok(None)
    }
    fn attention_modes(
        &mut self,
        input: Attention<'_>,
        routes: &[Route],
    ) -> Result<BTreeMap<Recipient, WakeMode>> {
        // The ledger derives this owner from the actual reply_to chain, never
        // a caller-supplied thread_id or the sender's priority class.
        let root_sender = if let Some(parent) = input.reply_to {
            Some(self.swarm(input.swarm)?.ledger.thread_sender(parent)?)
        } else {
            None
        };
        let delegated_to = input
            .delegated
            .then(|| super::tree::sink(input.destination).ok())
            .flatten()
            .map(|(seat, _)| seat);
        let mut modes = BTreeMap::new();
        for route in routes {
            if modes.contains_key(&route.recipient) {
                continue;
            }
            let mode = if let Recipient::Seat(seat) = route.recipient {
                super::attention::decide(
                    &self.registry.get(seat)?,
                    input
                        .destination
                        .trim_end_matches("/**")
                        .trim_end_matches("/*"),
                    input.kind,
                    super::attention::Facts {
                        operator: input.operator,
                        delegated: delegated_to == Some(seat),
                        own_thread_reply: root_sender == Some(route.recipient),
                        addressed_by_name: false,
                    },
                )?
            } else {
                WakeMode::Inbox
            };
            modes.insert(route.recipient, mode);
        }
        Ok(modes)
    }
    pub fn request(
        &mut self,
        seat: Ulid,
        generation: u64,
        request: super::api::Request,
        operator: bool,
        mute_state: Option<State>,
        now: u64,
    ) -> Result<super::api::Response> {
        let record = self.record(seat, generation)?;
        self.request_from(
            record.swarm,
            Source::Seat {
                record: &record,
                operator,
            },
            request,
            mute_state,
            now,
        )
    }
    pub(crate) fn request_operator(
        &mut self,
        authority: &super::keys::OperatorSigning,
        swarm: Ulid,
        request: super::api::Request,
        mute_state: Option<State>,
        now: u64,
    ) -> Result<super::api::Response> {
        if swarm == Ulid::nil() || !self.registry.list()?.iter().any(|seat| seat.swarm == swarm) {
            return Err(invalid("unknown operator messaging swarm"));
        }
        self.set_operator_device(authority.device_id)?;
        self.request_from(
            swarm,
            Source::Operator(OperatorSource {
                signing: Some(authority),
                reference: self.operator_reference.expect("reference admitted"),
                device: Some(authority.device_id),
                origin: &authority.origin,
            }),
            request,
            mute_state,
            now,
        )
    }
    /// Only the admitted local messaging door may use this inferred principal.
    /// Other operator mutations still require their existing signing authority.
    pub(crate) fn request_operator_pending(
        &mut self,
        device: Option<Ulid>,
        origin: &crate::mcp::Origin,
        swarm: Ulid,
        request: super::api::Request,
        now: u64,
    ) -> Result<super::api::Response> {
        if swarm == Ulid::nil() || !self.registry.list()?.iter().any(|seat| seat.swarm == swarm) {
            return Err(invalid("unknown operator messaging swarm"));
        }
        let reference = self.operator_reference(device)?;
        self.request_from(
            swarm,
            Source::Operator(OperatorSource {
                signing: None,
                reference,
                device: self.operator_device_attribution,
                origin,
            }),
            request,
            None,
            now,
        )
    }
    fn request_from(
        &mut self,
        swarm: Ulid,
        source: Source<'_>,
        request: super::api::Request,
        mute_state: Option<State>,
        now: u64,
    ) -> Result<super::api::Response> {
        use super::api::{Request, Response};
        if matches!(
            source,
            Source::Operator(OperatorSource { signing: None, .. })
        ) && !matches!(
            &request,
            Request::Send { .. }
                | Request::List { .. }
                | Request::Recent { .. }
                | Request::Receipts { .. }
                | Request::Acknowledge { .. }
                | Request::Ls { .. }
        ) {
            return Err(invalid(
                "held: this operator mutation requires signing custody",
            ));
        }
        let actor = source.actor();
        match request {
            Request::Send { message } => match source {
                Source::Seat { record, operator } => {
                    self.send_inner(record.id, record.generation, message, now, operator, None)
                }
                Source::Operator(source) => match source.signing {
                    Some(authority) => self.send_operator(authority, swarm, message, None, now),
                    None => self.send_operator_pending(source, swarm, message, now),
                },
            },
            Request::List {
                thread,
                channel,
                after,
                limit,
            } => match source {
                Source::Seat { record, .. } => Ok(Response::Messages {
                    page: self.list(record.id, thread, channel.as_deref(), after, limit)?,
                }),
                Source::Operator(source) => {
                    let channel = channel
                        .map(|path| super::tree::normalize(&path, true))
                        .transpose()?;
                    if let Some(path) = &channel {
                        self.swarm(swarm)?.tree.check(
                            Actor::Operator,
                            path.trim_end_matches("/**").trim_end_matches("/*"),
                            Right::Read,
                        )?;
                    }
                    let page = self.swarm(swarm)?.ledger.list_for(
                        channel.is_none().then_some(source.reference.recipient()),
                        thread,
                        channel.as_deref(),
                        after,
                        limit,
                    )?;
                    Ok(Response::Messages { page })
                }
            },
            Request::Recent {
                channel,
                before,
                limit,
            } => {
                let Source::Operator(source) = source else {
                    return Err(invalid("recent messages require operator authority"));
                };
                let channel = channel
                    .map(|path| super::tree::normalize(&path, true))
                    .transpose()?;
                if let Some(path) = &channel {
                    self.swarm(swarm)?.tree.check(
                        Actor::Operator,
                        path.trim_end_matches("/**").trim_end_matches("/*"),
                        Right::Read,
                    )?;
                }
                let page = self.swarm(swarm)?.ledger.recent_for(
                    channel.is_none().then_some(source.reference.recipient()),
                    channel.as_deref(),
                    before,
                    limit,
                )?;
                let rows = page
                    .items
                    .iter()
                    .map(|message| self.operator_message_row(swarm, message.view().id))
                    .collect::<Result<Vec<_>>>()?;
                let mut response = Response::Recent {
                    page,
                    rows,
                    projected_at: crate::store::now_ms(),
                };
                while serde_json::to_vec(&response)?.len() > 512 * 1024 {
                    let Response::Recent { page, rows, .. } = &mut response else {
                        unreachable!()
                    };
                    if page.items.len() <= 1 {
                        return Err(invalid(
                            "recent message exceeds response byte limit; original retained",
                        ));
                    }
                    // Keep the newest suffix; the exclusive older cursor includes
                    // every whole signed message trimmed from this response.
                    page.items.remove(0);
                    rows.remove(0);
                    page.next = Some(page.items[0].sequence);
                }
                Ok(response)
            }
            Request::Receipts {
                message,
                after,
                limit,
            } => {
                let stored = self
                    .swarm(swarm)?
                    .ledger
                    .by_id(message)
                    .map_err(|_| invalid("message is not in the requested swarm"))?;
                if let Source::Seat { record, .. } = source {
                    self.can_read(record.id, &stored)?;
                }
                let recipient = match source {
                    Source::Seat { record, .. }
                        if !actor.controls() && stored.view().seat != record.id =>
                    {
                        Some(record.id)
                    }
                    _ => None,
                };
                Ok(Response::Receipts {
                    page: self
                        .swarm(swarm)?
                        .ledger
                        .receipts_page(message, recipient, after, limit)?,
                })
            }
            Request::Acknowledge { message, agent } => {
                match source {
                    Source::Seat { record, .. } => {
                        self.acknowledge(record.id, message, agent.as_deref(), now)?
                    }
                    Source::Operator(source) => self.swarm(swarm)?.ledger.acknowledge(
                        source.reference.recipient(),
                        message,
                        agent.as_deref(),
                        now,
                    )?,
                }
                Ok(Response::Applied)
            }
            Request::Ls { path, after, limit } => {
                if limit == 0 || limit > 128 {
                    return Err(invalid("tree page limit must be 1..128"));
                }
                let nodes = self.swarm(swarm)?.tree.ls(actor, &path)?;
                let mut out = Vec::new();
                let mut size = 0;
                let mut next = None;
                for node in nodes
                    .into_iter()
                    .filter(|n| after.as_ref().is_none_or(|a| n.path > *a))
                {
                    let n = serde_json::to_vec(&node)?.len();
                    if out.len() >= limit || (!out.is_empty() && size + n > 512 * 1024) {
                        next = out.last().map(|n: &Node| n.path.clone());
                        break;
                    }
                    size += n;
                    out.push(node);
                }
                Ok(Response::Tree { nodes: out, next })
            }
            Request::Subscribe {
                folder,
                name,
                filter,
                rate,
            } => {
                if name.is_empty() || name.contains('/') {
                    return Err(invalid("subscription name must be one path token"));
                }
                let path = format!("{}/{}", super::tree::normalize(&folder, false)?, name);
                let mut node = Node::link(path, source.inbox());
                node.filter = filter;
                node.rate = rate;
                self.swarm(swarm)?.tree.put(actor, node)?;
                Ok(Response::Applied)
            }
            Request::Unsubscribe { path } => {
                self.swarm(swarm)?.tree.remove(actor, &path)?;
                Ok(Response::Applied)
            }
            Request::Link { node } => {
                self.swarm(swarm)?.tree.put(actor, node)?;
                Ok(Response::Applied)
            }
            Request::Pause { path, paused } => {
                self.swarm(swarm)?.tree.pause(actor, &path, paused)?;
                Ok(Response::Applied)
            }
            Request::Grant {
                path,
                seat: target,
                rights,
            } => {
                if self.registry.get(target)?.swarm != swarm {
                    return Err(invalid("grant outside caller swarm"));
                }
                self.swarm(swarm)?
                    .tree
                    .grant(actor, &path, target, rights)?;
                Ok(Response::Applied)
            }
            Request::Mute {
                seat: target,
                muted,
            } => {
                if self.registry.get(target)?.swarm != swarm {
                    return Err(invalid("mute outside caller swarm"));
                }
                Ok(Response::Muted {
                    notice: self.set_mute(
                        actor,
                        target,
                        muted,
                        mute_state
                            .ok_or_else(|| invalid("target has no live state observation"))?,
                        now,
                    )?,
                })
            }
        }
    }
    pub fn open(root: &Path, keys: Arc<dyn SeatKeys>) -> Result<Self> {
        let account = SqliteStore::open(&object_dir(root, "account")?)?;
        account.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS msg_counters(seat TEXT NOT NULL,generation INTEGER NOT NULL,sequence INTEGER NOT NULL,PRIMARY KEY(seat,generation));
            CREATE TABLE IF NOT EXISTS msg_policies(seat TEXT PRIMARY KEY,epoch INTEGER NOT NULL,role BLOB NOT NULL,muted INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS msg_grants(id TEXT PRIMARY KEY,data BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS msg_notices(id TEXT PRIMARY KEY,envelope BLOB NOT NULL,request BLOB NOT NULL,sent INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS msg_held(position INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT UNIQUE NOT NULL,data BLOB NOT NULL,reason TEXT NOT NULL,state TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS msg_held_fifo ON msg_held(state,position);")?;
        pin_issuer(&account, keys.as_ref())?;
        let registry = Registry::new(account.clone());
        let mut messaging = Self {
            root: root.into(),
            account,
            registry,
            keys,
            swarms: BTreeMap::new(),
            operator_reference: None,
            operator_device_attribution: None,
        };
        if messaging
            .account
            .get("messaging/operator-reference")?
            .is_some()
        {
            messaging.operator_reference(None)?;
        }
        for record in messaging.registry.list()? {
            messaging.sync_seat(&record)?;
        }
        messaging.recover_notices()?;
        messaging.recover_pending_intents()?;
        Ok(messaging)
    }
    pub fn record(&self, id: Ulid, generation: u64) -> Result<SeatRecord> {
        let record = self.registry.get(id)?;
        if record.generation != generation {
            return Err(invalid("stale messaging process generation"));
        }
        Ok(record)
    }
    pub fn actor(&self, id: Ulid, generation: u64) -> Result<(Ulid, Actor)> {
        let record = self.record(id, generation)?;
        Ok((
            record.swarm,
            Actor::Seat {
                seat: id,
                class: record.role.class,
            },
        ))
    }
    /// Current local authority seam shared with control. ID1 replaces the
    /// issuer/grant verifier; a caller can never supply its role or epoch.
    pub fn authority(
        &self,
        id: Ulid,
        generation: u64,
    ) -> Result<(SeatRecord, u64, super::wire::IssuerKind)> {
        let record = self.record(id, generation)?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let epoch = policy(&tx, &record)?;
        tx.commit()?;
        Ok((record, epoch, self.keys.issuer().kind))
    }
    pub fn pending_application_ack(&mut self, seat: Ulid, generation: u64) -> Result<u64> {
        let record = self.record(seat, generation)?;
        self.swarm(record.swarm)?
            .ledger
            .pending_application_ack(seat)
    }

    pub(crate) fn inbox_summary(
        &mut self,
        seat: Ulid,
        generation: u64,
    ) -> Result<super::ledger::InboxSummary> {
        let record = self.record(seat, generation)?;
        self.swarm(record.swarm)?.ledger.inbox_summary(seat, 128)
    }

    pub(crate) fn destinations(
        &mut self,
        seat: Ulid,
        generation: u64,
    ) -> Result<Vec<super::tree::Destination>> {
        let record = self.record(seat, generation)?;
        self.sync_seat(&record)?;
        Ok(self.swarm(record.swarm)?.tree.destinations(Actor::Seat {
            seat,
            class: record.role.class,
        }))
    }

    fn swarm(&mut self, id: Ulid) -> Result<&mut Swarm> {
        if !self.swarms.contains_key(&id) {
            let store = SqliteStore::open(&object_dir(&self.root, &format!("swarm-{id}"))?)?;
            let mut tree = Tree::open(store.clone(), id)?;
            if let Some(reference) = self.operator_reference {
                tree.set_operator_reference(reference);
            }
            let ledger = Ledger::open(store, id)?
                .with_test_context(self.keys.test_account_label().map(str::to_owned));
            let startup = self.account.get("messaging/startup-root")?;
            if let Some(startup) = &startup {
                ledger.bind_legacy_task_roots(std::str::from_utf8(startup)?)?;
            }
            for (root, task) in ledger.pending_task_roots()? {
                let key = match &root {
                    Some(root) => {
                        self.account
                            .put(&format!("messaging/task-root-swarm/{root}/{id}"), b"1")?;
                        format!("messaging/task-state-root/{root}/{task}")
                    }
                    None => format!("messaging/task-state/{task}"),
                };
                let mut status = self.account.get(&key)?;
                if status.is_none()
                    && root
                        .as_ref()
                        .is_some_and(|root| startup.as_deref() == Some(root.as_bytes()))
                {
                    status = self.account.get(&format!("messaging/task-state/{task}"))?;
                }
                if let Some(status) = status {
                    match root {
                        Some(root) => {
                            ledger.task_state_root(&root, task, std::str::from_utf8(&status)?, 0)?
                        }
                        None => ledger.task_state(task, std::str::from_utf8(&status)?, 0)?,
                    }
                }
            }
            self.swarms.insert(
                id,
                Swarm {
                    tree,
                    ledger,
                    seen: HashMap::new(),
                    resolved: HashMap::new(),
                    muted: HashMap::new(),
                },
            );
        }
        Ok(self.swarms.get_mut(&id).unwrap())
    }
    pub(crate) fn set_operator_device(&mut self, device_id: Ulid) -> Result<()> {
        self.operator_reference(Some(device_id)).map(|_| ())
    }
    /// Persist a principal once. Only the admitted IO owner supplies validated
    /// device facts; no request DTO can choose or change the continuity key.
    pub(crate) fn operator_reference(&mut self, device: Option<Ulid>) -> Result<OperatorReference> {
        if device == Some(Ulid::nil()) {
            return Err(invalid("operator device must be non-nil"));
        }
        if let Some(reference) = self.operator_reference
            && device.is_none_or(|device| self.operator_device_attribution == Some(device))
        {
            return Ok(reference);
        }
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let old: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='messaging/operator-reference'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let reference = match old {
            Some(bytes) => serde_json::from_slice::<OperatorReference>(&bytes)?,
            None => {
                let reference = OperatorReference {
                    id: match device {
                        Some(device) => device,
                        None => Ulid::new(),
                    },
                    kind: if device.is_some() {
                        OperatorReferenceKind::DeviceDerived
                    } else {
                        OperatorReferenceKind::OperatorLocal
                    },
                };
                tx.execute(
                    "INSERT INTO kv VALUES('messaging/operator-reference',?1)",
                    [serde_json::to_vec(&reference)?],
                )?;
                reference
            }
        };
        if reference.id == Ulid::nil() {
            return Err(invalid("invalid persisted operator reference"));
        }
        if let Some(device) = device {
            let binding = serde_json::json!({"reference":reference,"device":device});
            tx.execute(
                "INSERT OR IGNORE INTO kv VALUES(?1,?2)",
                params![
                    format!("messaging/operator-bindings/{}/{}", reference.id, device),
                    serde_json::to_vec(&binding)?
                ],
            )?;
            tx.execute("INSERT INTO kv VALUES('messaging/operator-attribution',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_vec(&device)?])?;
        }
        let attribution: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='messaging/operator-attribution'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        tx.commit()?;
        self.operator_device_attribution = attribution
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?;
        if self.operator_reference != Some(reference) {
            self.operator_reference = Some(reference);
            for swarm in self.swarms.values_mut() {
                swarm.tree.set_operator_reference(reference);
                swarm.seen.clear();
                swarm.resolved.clear();
                swarm.muted.clear();
            }
        }
        Ok(reference)
    }
    pub(crate) fn operator_subjects(&self) -> Result<Vec<String>> {
        if self.operator_reference.is_none() {
            return Err(invalid("held: operator device custody is unavailable"));
        }
        // Enumerate retained subjects from the current tree, rather than a
        // wildcard that would admit new subchannels or unrelated bus traffic.
        // State/task/artifact/audit subjects and consumer versions belong to the lane.
        let mut subjects = BTreeSet::from([
            "operator/inbox/message".to_owned(),
            "operator/inbox/receipt".to_owned(),
        ]);
        for swarm in self.swarms.values() {
            let mut folders = vec![
                "channels".to_owned(),
                "swarms".to_owned(),
                "groups".to_owned(),
                "ext".to_owned(),
            ];
            while let Some(path) = folders.pop() {
                subjects.insert(format!("{path}/message"));
                for node in swarm.tree.ls(Actor::Operator, &path)? {
                    subjects.insert(format!("{}/message", node.path));
                    if node.kind == super::tree::EntryKind::Folder {
                        folders.push(node.path);
                    }
                }
            }
        }
        Ok(subjects.into_iter().collect())
    }
    pub(crate) fn operator_message_row(
        &mut self,
        swarm: Ulid,
        id: Ulid,
    ) -> Result<bbt_proto::messaging::MessageRow> {
        use bbt_proto::messaging::{AuthorKind, AuthorRef, MessageRow, ReceiptSummary};
        let message = self
            .swarm(swarm)?
            .ledger
            .by_id(id)
            .map_err(|_| invalid("message is not in the requested swarm"))?;
        if message.view().swarm_id != swarm {
            return Err(invalid("messaging swarm mismatch: retained message"));
        }
        let sender = self.swarm(swarm)?.ledger.sender(id)?;
        let author = match sender {
            Recipient::Seat(seat) => AuthorRef {
                kind: AuthorKind::Seat,
                id: seat.to_string(),
                name: self
                    .registry
                    .get(seat)
                    .map(|seat| seat.name)
                    .unwrap_or_else(|_| format!("seat {seat}")),
            },
            Recipient::Operator(device) => AuthorRef {
                kind: AuthorKind::Operator,
                id: device.to_string(),
                name: "operator".into(),
            },
            Recipient::OperatorLocal(reference) => AuthorRef {
                kind: AuthorKind::OperatorLocal,
                id: reference.to_string(),
                name: "operator".into(),
            },
        };
        let raw = self.swarm(swarm)?.ledger.receipts(id)?;
        let receipts = ReceiptSummary {
            // Inbox commit is delivery; a native write remains its own evidence rung.
            delivered: raw
                .iter()
                .filter(|receipt| receipt.evidence != crate::adapters::Evidence::SenderStored)
                .count(),
            acked: raw
                .iter()
                .filter(|receipt| {
                    receipt.evidence == crate::adapters::Evidence::ApplicationAcknowledged
                })
                .count(),
            held: raw.iter().filter(|receipt| receipt.held.is_some()).count()
                + usize::from(message.held_out.is_some()),
        };
        Ok(MessageRow {
            id,
            bus: message.bus.clone(),
            author,
            issuer_kind: message.view().issuer_kind,
            signature: message.view().signature,
            signature_reason: message.view().signature_reason.map(str::to_owned),
            upgrade_of: message.upgrade_of().cloned(),
            kind: message.view().kind,
            priority_class: message.view().priority_class,
            priority: message.view().priority,
            text: message.view().body.text().unwrap_or_default().into(),
            ts: message.view().timestamp,
            receipts,
        })
    }
    pub fn sync_seat(&mut self, record: &SeatRecord) -> Result<()> {
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        policy(&tx, record)?;
        tx.commit()?;
        let swarm = self.swarm(record.swarm)?;
        let folder = format!("seats/{}", record.id);
        let newly_registered = swarm.tree.get(&folder).is_none();
        if newly_registered {
            let mut node = Node::folder(&folder);
            node.grants
                .insert(record.id, vec![Right::Read, Right::Write]);
            swarm.tree.put(Actor::Operator, node)?;
            let mut sink = Node::sink(record.id);
            sink.grants
                .insert(record.id, vec![Right::Read, Right::Write]);
            swarm.tree.put(Actor::Operator, sink)?;
        }
        // One account/scope per tree. Defaults are installed only once per seat;
        // an explicit empty grant is a revocation and is never recreated here.
        let swarm_path = format!("swarms/{}", record.swarm);
        for (path, rights) in [
            ("seats", vec![Right::Write]),
            ("channels", vec![Right::Read]),
            ("groups", vec![Right::Read]),
            ("operator", vec![Right::Write]),
            (
                swarm_path.as_str(),
                vec![Right::Read, Right::Write, Right::Link],
            ),
        ] {
            let node = swarm.tree.get(path).unwrap();
            if !node.grants.contains_key(&record.id) {
                swarm.tree.grant(Actor::Operator, path, record.id, rights)?;
            }
        }
        let membership = format!("{swarm_path}/{}", record.id);
        if newly_registered && swarm.tree.get(&membership).is_none() {
            swarm.tree.put(
                Actor::Operator,
                Node::link(membership, format!("seats/{}/inbox", record.id)),
            )?;
        }
        Ok(())
    }
    pub fn tree(&mut self, swarm: Ulid) -> Result<&mut Tree> {
        Ok(&mut self.swarm(swarm)?.tree)
    }
    pub fn send(
        &mut self,
        from: Ulid,
        generation: u64,
        request: SendRequest,
        now: u64,
    ) -> Result<StoredMessage> {
        match self.send_inner(from, generation, request, now, false, None)? {
            super::api::Response::Stored { message } => Ok(*message),
            super::api::Response::Held { reason, .. } => {
                Err(invalid(format!("signing held: {reason}")))
            }
            super::api::Response::Refused { error } => Err(invalid(format!(
                "{}: {}; {}",
                error.kind, error.message, error.hint
            ))),
            _ => unreachable!(),
        }
    }
    /// Terminal-authored task wakes retain the admitted sender's role and all
    /// W2.4 destination/receipt rules; ordinary Send cannot construct this body.
    pub(crate) fn task_event(
        &mut self,
        from: Ulid,
        generation: u64,
        mut request: SendRequest,
        body: Body,
        now: u64,
    ) -> Result<StoredMessage> {
        let event = task_event_name(&request, &body)?;
        let record = self.record(from, generation)?;
        if event != "report" && record.role.class != Class::Oracle {
            return Err(invalid("task control requires oracle-class authority"));
        }
        request.destination = super::tree::normalize(&request.destination, false)?;
        if request.destination != "operator/inbox" {
            super::tree::sink(&request.destination)?;
        }
        self.sync_seat(&record)?;
        self.sync_task(record.swarm, request.id, &body, now)?;
        if let Some(old) = self.swarm(record.swarm)?.ledger.existing(&request, from)? {
            if old.view().body != &body {
                return Err(invalid("task message ID body conflict"));
            }
            return Ok(old);
        }
        let routes = self.swarm(record.swarm)?.tree.resolve(
            Actor::Seat {
                seat: from,
                class: record.role.class,
            },
            &request.destination,
            MessageKind::Task,
            record.role.class,
            request.priority,
        )?;
        self.check_recipients(record.swarm, &routes)?;
        if let Some(reason) = self.keys.signing_hold() {
            return Err(invalid(format!("task signing held: {reason}")));
        }
        if self.keys.ready_issuer().is_none() {
            return Err(invalid("task signing held: identity pending"));
        }
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let epoch = policy(&tx, &record)?;
        let envelope = mint(
            &tx,
            self.keys.as_ref(),
            &record,
            &request,
            body,
            Issue {
                epoch,
                now,
                operator_direct: false,
            },
        )?;
        tx.commit()?;
        let held = record.muted.then_some("sender muted");
        let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
        let message = self.swarm(record.swarm)?.ledger.accept_with_attention(
            &request,
            &envelope,
            Delivery {
                routes: &routes,
                wake: Some(&wake),
            },
            held,
            now,
        )?;
        self.reconcile(now)?;
        Ok(message)
    }
    pub(crate) fn task_event_operator(
        &mut self,
        authority: &super::keys::OperatorSigning,
        swarm: Ulid,
        request: SendRequest,
        body: Body,
        now: u64,
    ) -> Result<StoredMessage> {
        task_event_name(&request, &body)?;
        if request.destination != "operator/inbox" {
            let (target, _) = super::tree::sink(&request.destination)?;
            if self.registry.get(target)?.swarm != swarm {
                return Err(invalid("task target outside assignee swarm"));
            }
        }
        match self.send_operator(authority, swarm, request, Some(body), now)? {
            super::api::Response::Stored { message } => Ok(*message),
            super::api::Response::Held { reason, .. } => Err(invalid(format!("held: {reason}"))),
            _ => unreachable!(),
        }
    }
    fn send_operator(
        &mut self,
        authority: &super::keys::OperatorSigning,
        swarm: Ulid,
        mut request: SendRequest,
        body: Option<Body>,
        now: u64,
    ) -> Result<super::api::Response> {
        request.validate()?;
        if body.is_none() && matches!(request.kind, MessageKind::Presence | MessageKind::Task) {
            return Err(invalid(
                "typed presence/task messages require their producer API",
            ));
        }
        if let Some(reason) = self.keys.signing_hold() {
            return Err(invalid(format!("held: {reason}")));
        }
        self.set_operator_device(authority.device_id)?;
        let sender = self
            .operator_reference
            .expect("operator reference admitted")
            .recipient();
        if let Some(error) = self.supersedes_target(swarm, sender, &mut request)? {
            return Ok(super::api::Response::Refused { error });
        }
        let parent = self.reply_parent(swarm, sender, request.reply_to)?;
        if request.reply_to.is_some() && parent.is_none() {
            return Ok(super::api::Response::Refused {
                error: self.thread_refusal(swarm, sender)?,
            });
        }
        if request.destination.is_empty()
            && let Some(parent) = &parent
        {
            request.destination = sender_inbox(parent);
        }
        request.destination = super::tree::normalize(&request.destination, true)?;
        let destination_swarm = request
            .destination
            .split_once('/')
            .and_then(|(kind, tail)| {
                tail.split('/')
                    .next()?
                    .parse::<Ulid>()
                    .ok()
                    .map(|id| (kind, id))
            });
        let destination_swarm = match destination_swarm {
            Some(("seats", seat)) => Some(self.registry.get(seat)?.swarm),
            Some(("swarms", swarm)) => Some(swarm),
            _ => None, // Other channel paths are resolved in the explicitly named swarm tree.
        };
        if destination_swarm.is_some_and(|destination| destination != swarm) {
            return Err(invalid("messaging swarm mismatch: destination"));
        }
        if request.destination == format!("swarms/{swarm}") {
            request.destination.push_str("/*");
        }
        let body = body.unwrap_or_else(|| request_body(&request));
        self.sync_task(swarm, request.id, &body, now)?;
        if let Some(old) = self.swarm(swarm)?.ledger.existing(&request, sender)? {
            if old.view().body != &body {
                return Err(invalid("message ID body conflict"));
            }
            return Ok(super::api::Response::Stored {
                message: Box::new(old),
            });
        }
        if let Some(error) = self.inherit_reply(swarm, sender, parent.as_ref(), &mut request)? {
            return Ok(super::api::Response::Refused { error });
        }
        let routes = self.swarm(swarm)?.tree.resolve(
            Actor::Operator,
            &request.destination,
            request.kind,
            Class::Oracle,
            request.priority,
        )?;
        self.check_recipients(swarm, &routes)?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let envelope = mint_operator(
            &tx,
            authority,
            swarm,
            &request,
            body,
            now,
            &authority.origin,
        )?;
        self.validate(&envelope)?;
        tx.commit()?;
        let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
        let message = self
            .swarm(swarm)?
            .ledger
            .accept_from(
                &request,
                &envelope,
                Delivery {
                    routes: &routes,
                    wake: Some(&wake),
                },
                None,
                now,
                super::ledger::SenderAttribution {
                    recipient: sender,
                    device: Some(authority.device_id),
                },
            )
            .map_err(|error| {
                invalid(format!(
                    "unknown: message {} store outcome unavailable: {error}",
                    request.id
                ))
            })?;
        self.reconcile(now).map_err(|error| {
            invalid(format!(
                "unknown: message {} stored; delivery reconciliation failed: {error}",
                request.id
            ))
        })?;
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    fn send_operator_pending(
        &mut self,
        source: OperatorSource<'_>,
        swarm: Ulid,
        mut request: SendRequest,
        now: u64,
    ) -> Result<super::api::Response> {
        use bbt_proto::messaging::{LogicalSender, PendingIntent, PendingSignature};
        request.validate()?;
        if matches!(request.kind, MessageKind::Presence | MessageKind::Task) {
            return Err(invalid(
                "typed presence/task messages require their signed producer API",
            ));
        }
        let sender = source.reference.recipient();
        if let Some(error) = self.supersedes_target(swarm, sender, &mut request)? {
            return Ok(super::api::Response::Refused { error });
        }
        let parent = self.reply_parent(swarm, sender, request.reply_to)?;
        if request.reply_to.is_some() && parent.is_none() {
            return Ok(super::api::Response::Refused {
                error: self.thread_refusal(swarm, sender)?,
            });
        }
        if request.destination.is_empty()
            && let Some(parent) = &parent
        {
            request.destination = sender_inbox(parent);
        }
        request.destination = super::tree::normalize(&request.destination, true)?;
        let destination = request
            .destination
            .split_once('/')
            .and_then(|(kind, tail)| {
                tail.split('/')
                    .next()?
                    .parse::<Ulid>()
                    .ok()
                    .map(|id| (kind, id))
            });
        let destination_swarm = match destination {
            Some(("seats", seat)) => Some(self.registry.get(seat)?.swarm),
            Some(("swarms", swarm)) => Some(swarm),
            _ => None,
        };
        if destination_swarm.is_some_and(|target| target != swarm) {
            return Err(invalid("messaging swarm mismatch: destination"));
        }
        if request.destination == format!("swarms/{swarm}") {
            request.destination.push_str("/*");
        }
        if let Some(old) = self.swarm(swarm)?.ledger.existing(&request, sender)? {
            return Ok(super::api::Response::Stored {
                message: Box::new(old),
            });
        }
        if let Some(error) = self.inherit_reply(swarm, sender, parent.as_ref(), &mut request)? {
            return Ok(super::api::Response::Refused { error });
        }
        let routes = self.swarm(swarm)?.tree.resolve(
            Actor::Operator,
            &request.destination,
            request.kind,
            Class::Oracle,
            request.priority,
        )?;
        self.check_recipients(swarm, &routes)?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous: Option<Vec<u8>> = tx
            .query_row(
                "SELECT data FROM msg_held WHERE id=?1",
                [request.id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(bytes) = previous {
            let held: HeldSend = serde_json::from_slice(&bytes)?;
            if held.request != request
                || held
                    .operator_context
                    .as_ref()
                    .is_none_or(|old| old.reference != source.reference)
            {
                return Err(invalid(
                    "pending operator id conflicts with its original admission",
                ));
            }
            tx.commit()?;
            return self.store_pending(&held, now);
        }
        let issuer_record: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='identity/local-issuer'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let epoch = issuer_record
            .map(|bytes| super::keys::local_operator_grant(&bytes).map(|(_, epoch)| epoch))
            .transpose()?;
        let reason = self
            .keys
            .signing_hold()
            .unwrap_or_else(|| "operator signing unavailable".into());
        let context = PendingOperator {
            reference: source.reference,
            device: source.device,
            door: source.origin.door,
            issued_by: source.origin.issued_by.clone(),
            epoch,
        };
        let mut held = HeldSend {
            seat: Ulid::nil(),
            generation: 0,
            swarm,
            role: crate::registry::Role {
                name: "operator".into(),
                class: Class::Oracle,
            },
            epoch: epoch.unwrap_or(0),
            issuer_state: tx
                .query_row(
                    "SELECT value FROM kv WHERE key='identity/issuer'",
                    [],
                    |r| r.get(0),
                )
                .optional()?,
            request: request.clone(),
            operator: true,
            routes,
            intent: None,
            completion: None,
            operator_context: Some(context),
        };
        tx.execute(
            "INSERT INTO msg_held(id,data,reason,state) VALUES(?1,?2,?3,'held')",
            params![request.id.to_string(), serde_json::to_vec(&held)?, reason],
        )?;
        let sender = match source.reference.kind {
            OperatorReferenceKind::OperatorLocal => LogicalSender::OperatorLocal {
                reference: source.reference,
                device: source.device,
                door: source.origin.door,
                issued_by: source.origin.issued_by.clone(),
            },
            OperatorReferenceKind::DeviceDerived => LogicalSender::Operator {
                reference: source.reference,
                device: source.device,
                door: source.origin.door,
                issued_by: source.origin.issued_by.clone(),
            },
        };
        held.intent = Some(PendingIntent {
            version: 1,
            id: request.id,
            sender,
            swarm_id: swarm,
            role: held.role.clone(),
            role_epoch: epoch,
            body: request_body(&request),
            request,
            admission_order: tx.last_insert_rowid() as u64,
            admitted_ms: now,
            signature: PendingSignature::Pending,
            hold_reason: reason,
        });
        super::pending::binding(held.intent.as_ref().unwrap())?;
        tx.execute(
            "UPDATE msg_held SET data=?2 WHERE id=?1",
            params![held.request.id.to_string(), serde_json::to_vec(&held)?],
        )?;
        tx.commit()?;
        self.store_pending(&held, now)
    }
    fn send_inner(
        &mut self,
        from: Ulid,
        generation: u64,
        mut request: SendRequest,
        now: u64,
        operator: bool,
        releasing: Option<&HeldSend>,
    ) -> Result<super::api::Response> {
        request.validate()?;
        if matches!(request.kind, MessageKind::Presence | MessageKind::Task) {
            return Err(invalid(
                "typed presence/task messages require their producer API",
            ));
        }
        let record = self.record(from, generation)?;
        let reader = Recipient::Seat(from);
        if let Some(error) = self.supersedes_target(record.swarm, reader, &mut request)? {
            return Ok(super::api::Response::Refused { error });
        }
        let parent = self.reply_parent(record.swarm, reader, request.reply_to)?;
        if request.reply_to.is_some() && parent.is_none() {
            return Ok(super::api::Response::Refused {
                error: self.thread_refusal(record.swarm, reader)?,
            });
        }
        if request.destination.is_empty() {
            let parent = parent.as_ref().ok_or_else(|| {
                invalid(
                    "message_send needs a destination, or reply_to to answer a message's sender",
                )
            })?;
            request.destination = sender_inbox(parent);
        }
        request.destination = super::tree::normalize(&request.destination, true)?;
        self.sync_seat(&record)?;
        if request.destination == format!("swarms/{}", record.swarm) {
            request.destination.push_str("/*");
        }
        if !operator
            && record.role.class == Class::Worker
            && request.destination.starts_with("swarms/")
            && request.kind == MessageKind::Text
        {
            return Err(invalid(
                "swarm stream requires chunk or decision; send progress as a DM to your delegator",
            ));
        }
        if let Some(old) = self.swarm(record.swarm)?.ledger.existing(&request, from)? {
            return Ok(super::api::Response::Stored {
                message: Box::new(old),
            });
        }
        if let Some(error) =
            self.inherit_reply(record.swarm, reader, parent.as_ref(), &mut request)?
        {
            return Ok(super::api::Response::Refused { error });
        }
        let actor = if operator {
            Actor::Operator
        } else {
            Actor::Seat {
                seat: from,
                class: record.role.class,
            }
        };
        let base = request
            .destination
            .trim_end_matches("/**")
            .trim_end_matches("/*");
        if !self.swarm(record.swarm)?.tree.contains(base) {
            let hint = match request.reply_to {
                Some(reply) => format!(
                    "the message you answer came from {}; send there, or omit destination and keep reply_to",
                    sender_inbox(&self.swarm(record.swarm)?.ledger.by_id(reply)?)
                ),
                None => format!(
                    "you may address: {}; to reach the operator, send to operator/inbox",
                    self.swarm(record.swarm)?
                        .tree
                        .writable(actor, 16)
                        .join(", ")
                ),
            };
            return Err(invalid(format!(
                "unknown destination {}: {hint}",
                request.destination
            )));
        }
        let routes = self.swarm(record.swarm)?.tree.resolve(
            actor,
            &request.destination,
            request.kind,
            record.role.class,
            request.priority,
        )?;
        self.check_recipients(record.swarm, &routes)?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let epoch = policy(&tx, &record)?;
        let operator_direct = operator && super::tree::sink(&request.destination).is_ok();
        let mut pending = HeldSend {
            seat: from,
            generation,
            swarm: record.swarm,
            role: record.role.clone(),
            epoch,
            issuer_state: tx
                .query_row(
                    "SELECT value FROM kv WHERE key='identity/issuer'",
                    [],
                    |r| r.get(0),
                )
                .optional()?,
            request: request.clone(),
            operator,
            routes: routes.clone(),
            intent: None,
            completion: None,
            operator_context: None,
        };
        if let Some(original) = releasing {
            if original.swarm != pending.swarm
                || original.role != pending.role
                || original.epoch != epoch
                || original.issuer_state != pending.issuer_state
                || original.routes != routes
            {
                return Err(invalid("held send authority changed; resubmit explicitly"));
            }
        } else {
            let old: Option<(Vec<u8>, String)> = tx
                .query_row(
                    "SELECT data,reason FROM msg_held WHERE id=?1",
                    [request.id.to_string()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some((bytes, reason)) = old {
                let old: HeldSend = serde_json::from_slice(&bytes)?;
                if old.seat != from
                    || old.generation != generation
                    || old.operator != operator
                    || serde_json::to_vec(&old.request)? != serde_json::to_vec(&request)?
                {
                    return Err(invalid(
                        "held send id reused with different content or authority",
                    ));
                }
                if old.intent.is_some() {
                    tx.commit()?;
                    return self.store_pending(&old, now);
                }
                return Ok(super::api::Response::Held {
                    id: request.id,
                    reason,
                });
            }
            let queued: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM msg_held WHERE state='held')",
                [],
                |r| r.get(0),
            )?;
            let reason = self
                .keys
                .signing_hold()
                .or_else(|| {
                    self.keys
                        .ready_issuer()
                        .is_none()
                        .then(|| "identity pending".into())
                })
                .or_else(|| queued.then(|| "earlier signing work held".into()));
            if let Some(reason) = reason {
                tx.execute(
                    "INSERT INTO msg_held(id,data,reason,state) VALUES(?1,?2,?3,'held')",
                    params![
                        request.id.to_string(),
                        serde_json::to_vec(&pending)?,
                        reason
                    ],
                )?;
                // The durable admission order is independent of a signing key.
                // A custody outage must not turn a signing queue bound into a
                // local-delivery gate. Resume drains a bounded page instead.
                pending.intent = Some(bbt_proto::messaging::PendingIntent {
                    version: 1,
                    id: request.id,
                    sender: bbt_proto::messaging::LogicalSender::Seat {
                        device: self.operator_device_attribution,
                        seat: from,
                        process_generation: generation,
                    },
                    swarm_id: record.swarm,
                    role: record.role.clone(),
                    role_epoch: Some(epoch),
                    body: request_body(&request),
                    request: request.clone(),
                    admission_order: tx.last_insert_rowid() as u64,
                    admitted_ms: now,
                    signature: bbt_proto::messaging::PendingSignature::Pending,
                    hold_reason: reason,
                });
                super::pending::binding(pending.intent.as_ref().unwrap())?;
                tx.execute(
                    "UPDATE msg_held SET data=?2 WHERE id=?1",
                    params![request.id.to_string(), serde_json::to_vec(&pending)?],
                )?;
                tx.commit()?;
                return self.store_pending(&pending, now);
            }
        }
        let envelope = mint(
            &tx,
            self.keys.as_ref(),
            &record,
            &request,
            request_body(&request),
            Issue {
                epoch,
                now,
                operator_direct,
            },
        )?;
        tx.commit()?;
        let held = (record.muted && !operator_direct).then_some("sender muted");
        let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
        let message = self.swarm(record.swarm)?.ledger.accept_with_attention(
            &request,
            &envelope,
            Delivery {
                routes: &routes,
                wake: Some(&wake),
            },
            held,
            now,
        )?;
        // Recipient mutes and paused subscriptions apply at once (held before the next pump).
        self.reconcile(now)?;
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    fn recover_pending_intents(&mut self) -> Result<()> {
        let mut after = 0;
        loop {
            let page: Vec<(u64, Vec<u8>)> = {
                let db = self.account.connection()?;
                let mut query = db.prepare_cached("SELECT position,data FROM msg_held WHERE state='held' AND position>?1 ORDER BY position LIMIT 128")?;
                query
                    .query_map([sqlint(after)?], |row| Ok((u64_at(row, 0)?, row.get(1)?)))?
                    .collect::<std::result::Result<_, _>>()?
            };
            if page.is_empty() {
                break;
            }
            for (position, bytes) in page {
                after = position;
                let held: HeldSend = serde_json::from_slice(&bytes)?;
                let Some(intent) = &held.intent else {
                    continue;
                };
                if let Err(error) = self.store_pending(&held, intent.admitted_ms) {
                    if !error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::InvalidData)
                    {
                        return Err(error);
                    }
                    self.account.connection()?.execute(
                        "UPDATE msg_held SET state='refused',reason=?2 WHERE position=?1",
                        params![
                            sqlint(position)?,
                            format!("pending delivery refused: {error}")
                        ],
                    )?;
                }
            }
        }
        Ok(())
    }
    fn local_recipients(&self, routes: &[Route]) -> Result<BTreeSet<Recipient>> {
        let mut local = BTreeSet::new();
        for route in routes {
            if self.recipient_is_local(route.recipient)? {
                local.insert(route.recipient);
            }
        }
        Ok(local)
    }
    fn recipient_is_local(&self, recipient: Recipient) -> Result<bool> {
        Ok(match recipient {
            Recipient::Seat(seat) => {
                self.registry.get(seat)?.location == crate::registry::Location::Local
            }
            recipient @ (Recipient::Operator(_) | Recipient::OperatorLocal(_)) => {
                self.operator_reference.map(OperatorReference::recipient) == Some(recipient)
            }
        })
    }
    fn store_pending(&mut self, held: &HeldSend, now: u64) -> Result<super::api::Response> {
        let intent = held
            .intent
            .as_ref()
            .ok_or_else(|| invalid("missing pending intent"))?;
        let local = self.local_recipients(&held.routes)?;
        let (sender, sender_muted) = if let Some(operator) = &held.operator_context {
            if self.operator_reference != Some(operator.reference) {
                return Err(invalid("pending operator reference mismatch"));
            }
            (operator.reference.recipient(), None)
        } else {
            let record = self.record(held.seat, held.generation)?;
            (
                Recipient::Seat(held.seat),
                (record.muted && !held.operator).then_some("sender muted"),
            )
        };
        let wake = self.attention_modes(Attention::pending(intent), &held.routes)?;
        let message = self.swarm(held.swarm)?.ledger.accept_pending(
            intent,
            Delivery {
                routes: &held.routes,
                wake: Some(&wake),
            },
            &local,
            sender_muted,
            now,
            sender,
        )?;
        self.reconcile(now)?;
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    fn release_operator_intent(
        &mut self,
        mut held: HeldSend,
        authority: Option<&super::keys::OperatorSigning>,
        now: u64,
    ) -> Result<super::api::Response> {
        let Some(authority) = authority else {
            return Ok(super::api::Response::Held {
                id: held.request.id,
                reason: "operator signing unavailable; local delivery remains pending".into(),
            });
        };
        let context = held
            .operator_context
            .clone()
            .ok_or_else(|| invalid("missing pending operator context"))?;
        let intent = held
            .intent
            .clone()
            .ok_or_else(|| invalid("missing pending operator intent"))?;
        if self.operator_reference != Some(context.reference)
            || context.epoch.is_some_and(|epoch| epoch != authority.epoch)
        {
            return Err(invalid(
                "held operator authority changed; original intent retained",
            ));
        }
        let routes = self.swarm(held.swarm)?.tree.resolve(
            Actor::Operator,
            &held.request.destination,
            held.request.kind,
            Class::Oracle,
            held.request.priority,
        )?;
        self.check_recipients(held.swarm, &routes)?;
        if routes != held.routes {
            return Err(invalid("held send authority changed; resubmit explicitly"));
        }
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let issuer: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='identity/issuer'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if held
            .issuer_state
            .as_ref()
            .is_some_and(|old| Some(old) != issuer.as_ref())
        {
            return Err(invalid(
                "held operator issuer changed; original intent retained",
            ));
        }
        let envelope = match &held.completion {
            Some(envelope) => envelope.clone(),
            None => {
                let origin = crate::mcp::Origin {
                    door: context.door,
                    issued_by: context.issued_by.clone(),
                };
                mint_operator(
                    &tx,
                    authority,
                    held.swarm,
                    &held.request,
                    intent.body.clone(),
                    intent.admitted_ms,
                    &origin,
                )?
            }
        };
        super::pending::check_upgrade_content(&intent, &envelope)?;
        held.completion = Some(envelope.clone());
        tx.execute(
            "UPDATE msg_held SET data=?2 WHERE id=?1",
            params![held.request.id.to_string(), serde_json::to_vec(&held)?],
        )?;
        tx.commit()?;
        self.validate(&envelope)?;
        if self.operator_reference(Some(authority.device_id))? != context.reference {
            return Err(invalid(
                "operator reference changed during signature completion",
            ));
        }
        self.store_pending(&held, now)?;
        let local = self.local_recipients(&routes)?;
        let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
        let message = self.swarm(held.swarm)?.ledger.upgrade_pending(
            &intent,
            &envelope,
            Delivery {
                routes: &routes,
                wake: Some(&wake),
            },
            &local,
            now,
            Some(authority.device_id),
        )?;
        if let Some(swarm) = self.swarms.get_mut(&held.swarm) {
            swarm.seen.clear();
            swarm.resolved.clear();
        }
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    fn release_held_send(
        &mut self,
        mut held: HeldSend,
        authority: Option<&super::keys::OperatorSigning>,
        now: u64,
    ) -> Result<super::api::Response> {
        if held.operator_context.is_some() {
            return self.release_operator_intent(held, authority, now);
        }
        let Some(intent) = held.intent.clone() else {
            return self.send_inner(
                held.seat,
                held.generation,
                held.request.clone(),
                now,
                held.operator,
                Some(&held),
            );
        };
        let record = self.record(held.seat, held.generation)?;
        let actor = if held.operator {
            Actor::Operator
        } else {
            Actor::Seat {
                seat: record.id,
                class: record.role.class,
            }
        };
        let routes = self.swarm(held.swarm)?.tree.resolve(
            actor,
            &held.request.destination,
            held.request.kind,
            record.role.class,
            held.request.priority,
        )?;
        self.check_recipients(held.swarm, &routes)?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let epoch = policy(&tx, &record)?;
        let issuer: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='identity/issuer'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if record.swarm != held.swarm
            || record.role != held.role
            || epoch != held.epoch
            || held
                .issuer_state
                .as_ref()
                .is_some_and(|old| Some(old) != issuer.as_ref())
            || routes != held.routes
        {
            return Err(invalid("held send authority changed; resubmit explicitly"));
        }
        let envelope = match &held.completion {
            Some(envelope) => envelope.clone(),
            None => mint(
                &tx,
                self.keys.as_ref(),
                &record,
                &held.request,
                intent.body.clone(),
                Issue {
                    epoch,
                    now: intent.admitted_ms,
                    operator_direct: held.operator
                        && super::tree::sink(&held.request.destination).is_ok(),
                },
            )?,
        };
        super::pending::check_upgrade_content(&intent, &envelope)?;
        held.completion = Some(envelope.clone());
        tx.execute(
            "UPDATE msg_held SET data=?2 WHERE id=?1",
            params![held.request.id.to_string(), serde_json::to_vec(&held)?],
        )?;
        tx.commit()?;
        self.validate(&envelope)?;
        // A crash between the account transaction and this append reuses the
        // stored envelope, including its sender sequence and signature.
        self.store_pending(&held, now)?;
        let local = self.local_recipients(&routes)?;
        let device = self.operator_device_attribution;
        let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
        let message = self.swarm(held.swarm)?.ledger.upgrade_pending(
            &intent,
            &envelope,
            Delivery {
                routes: &routes,
                wake: Some(&wake),
            },
            &local,
            now,
            device,
        )?;
        if let Some(swarm) = self.swarms.get_mut(&held.swarm) {
            swarm.seen.clear();
            swarm.resolved.clear();
        }
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    /// Install a verified ready provider, then consume durable unsigned work in
    /// admission order. Changed authority is refused by name, never upgraded.
    pub fn resume_signing(&mut self, keys: Arc<dyn SeatKeys>, now: u64) -> Result<usize> {
        self.resume_signing_with_operator(keys, None, now)
    }
    pub(crate) fn resume_signing_with_operator(
        &mut self,
        keys: Arc<dyn SeatKeys>,
        authority: Option<&super::keys::OperatorSigning>,
        now: u64,
    ) -> Result<usize> {
        pin_issuer(&self.account, keys.as_ref())?;
        let changed = !Arc::ptr_eq(&self.keys, &keys);
        self.keys = keys;
        // W2.4b memoization must not retain resolutions from a held verifier.
        if changed {
            for swarm in self.swarms.values_mut() {
                swarm.seen.clear();
                swarm.resolved.clear();
            }
        }
        if self.keys.ready_issuer().is_none() || self.keys.signing_hold().is_some() {
            return Ok(0);
        }
        let mut released = 0;
        for _ in 0..64 {
            let next: Option<(String, Vec<u8>)> = self
                .account
                .connection()?
                .query_row(
                    "SELECT id,data FROM msg_held WHERE state='held' ORDER BY position LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((id, bytes)) = next else {
                break;
            };
            let queued: HeldSend = serde_json::from_slice(&bytes)?;
            let result = self.release_held_send(queued, authority, now);
            let (state, reason) = match result {
                Ok(super::api::Response::Stored { .. }) => {
                    released += 1;
                    ("released", "custody restored".to_owned())
                }
                Ok(_) => break,
                Err(error) => {
                    if self.keys.signing_hold().is_some() {
                        break;
                    }
                    if !error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::InvalidData)
                    {
                        return Err(error); // Storage failure retains the unsigned queue entry.
                    }
                    ("refused", format!("held send refused: {error}"))
                }
            };
            self.account.connection()?.execute(
                "UPDATE msg_held SET state=?2,reason=?3 WHERE id=?1",
                params![id, state, reason],
            )?;
        }
        self.reconcile(now)?;
        Ok(released)
    }
    fn check_recipients(&self, swarm: Ulid, routes: &[Route]) -> Result<()> {
        for route in routes {
            match route.recipient {
                Recipient::Seat(seat) if self.registry.get(seat)?.swarm == swarm => {}
                recipient @ (Recipient::Operator(_) | Recipient::OperatorLocal(_))
                    if self.operator_reference.map(OperatorReference::recipient)
                        == Some(recipient) => {}
                _ => return Err(invalid("destination is outside this account/swarm scope")),
            }
        }
        Ok(())
    }
    fn validate_message(
        &self,
        message: &StoredMessage,
    ) -> Result<(Option<SeatRecord>, Class, bool)> {
        if let Some(envelope) = message.envelope() {
            return self.validate(envelope);
        }
        let intent = message
            .intent()
            .ok_or_else(|| invalid("missing pending content"))?;
        let bytes: Vec<u8> = self.account.connection()?.query_row(
            "SELECT data FROM msg_held WHERE id=?1",
            [intent.id.to_string()],
            |r| r.get(0),
        )?;
        let admitted: HeldSend = serde_json::from_slice(&bytes)?;
        if admitted.intent.as_ref() != Some(intent) {
            return Err(invalid(
                "pending intent does not match its local admission journal",
            ));
        }
        super::pending::binding(intent)?;
        if let Some(operator) = &admitted.operator_context {
            if self.operator_reference != Some(operator.reference) {
                return Err(invalid("pending operator reference changed"));
            }
            return Ok((None, Class::Oracle, true));
        }
        let record = self.record(admitted.seat, admitted.generation)?;
        if record.swarm != admitted.swarm || record.role != admitted.role {
            return Err(invalid("pending sender authority changed"));
        }
        Ok((
            Some(record),
            admitted.role.class,
            admitted.operator && super::tree::sink(&intent.request.destination).is_ok(),
        ))
    }
    fn validate(&self, message: &Envelope) -> Result<(Option<SeatRecord>, Class, bool)> {
        self.keys.verify(message)?;
        if message.seat == Ulid::nil() {
            if message.issuer_kind == super::wire::IssuerKind::LocalDev {
                let bytes = self
                    .account
                    .get("identity/local-issuer")?
                    .ok_or_else(|| invalid("missing local operator custody record"))?;
                let (public_key, epoch) = super::keys::local_operator_grant(&bytes)?;
                if public_key != message.issuer
                    || digest(&bytes) != message.role_grant_id
                    || epoch != message.role_epoch
                {
                    return Err(invalid(
                        "operator grant does not match the local issuer journal",
                    ));
                }
            }
            return Ok((None, Class::Oracle, true));
        }
        let record = self.record(message.seat, message.process_generation)?;
        let grant: Option<Vec<u8>> = self
            .account
            .connection()?
            .query_row(
                "SELECT data FROM msg_grants WHERE id=?1",
                [&message.role_grant_id],
                |r| r.get(0),
            )
            .optional()?;
        let grant: Grant = serde_json::from_slice(
            &grant.ok_or_else(|| invalid("unknown terminal-issued seat grant"))?,
        )?;
        if grant.seat != message.seat
            || grant.generation != message.process_generation
            || grant.swarm != message.swarm_id
            || grant.role != message.role
            || grant.author != message.author
            || grant.epoch != message.role_epoch
            || record.swarm != message.swarm_id
        {
            return Err(invalid(
                "signed role/seat fields do not match the issued grant",
            ));
        }
        let class = if grant.role.class == Class::Oracle && record.role.class == Class::Oracle {
            Class::Oracle
        } else {
            Class::Worker
        };
        Ok((
            Some(record),
            class,
            grant.operator_direct && super::tree::sink(&message.context).is_ok(),
        ))
    }
    pub fn can_read(&mut self, seat: Ulid, message: &StoredMessage) -> Result<()> {
        let record = self.registry.get(seat)?;
        if record.swarm != message.view().swarm_id {
            return Err(invalid("message outside caller swarm"));
        }
        if seat == message.view().seat
            || (record.role.class == Class::Oracle
                && message.view().context != "operator"
                && !message.view().context.starts_with("operator/"))
            || self
                .swarm(record.swarm)?
                .ledger
                .receipts(message.view().id)?
                .iter()
                .any(|r| {
                    r.recipient == seat && r.evidence != crate::adapters::Evidence::SenderStored
                })
        {
            return Ok(());
        }
        Err(invalid("message is not in caller inbox/outbox"))
    }
    pub fn receipts(&mut self, seat: Ulid, message: Ulid) -> Result<Vec<Receipt>> {
        let swarm = self.registry.get(seat)?.swarm;
        let stored = self.swarm(swarm)?.ledger.by_id(message)?;
        self.can_read(seat, &stored)?;
        let all = self.swarm(swarm)?.ledger.receipts(message)?;
        Ok(
            if stored.view().seat == seat || self.registry.get(seat)?.role.class == Class::Oracle {
                all
            } else {
                all.into_iter().filter(|r| r.recipient == seat).collect()
            },
        )
    }
    pub fn list(
        &mut self,
        seat: Ulid,
        thread: Option<Ulid>,
        channel: Option<&str>,
        after: u64,
        limit: usize,
    ) -> Result<Page<StoredMessage>> {
        let record = self.registry.get(seat)?;
        let channel = channel
            .map(|p| super::tree::normalize(p, true))
            .transpose()?;
        if let Some(path) = &channel {
            self.swarm(record.swarm)?.tree.check(
                Actor::Seat {
                    seat,
                    class: record.role.class,
                },
                path.trim_end_matches("/**").trim_end_matches("/*"),
                Right::Read,
            )?;
        }
        let page = self.swarm(record.swarm)?.ledger.list(
            (record.role.class != Class::Oracle || channel.is_none()).then_some(seat),
            thread,
            channel.as_deref(),
            after,
            limit,
        )?;
        Ok(page)
    }
    pub fn set_mute(
        &mut self,
        actor: Actor,
        seat: Ulid,
        muted: bool,
        state: State,
        now: u64,
    ) -> Result<Option<Ulid>> {
        self.change_mute(actor, seat, (muted, state), now, None)
    }
    /// Compare the process and policy versions in the same transaction that
    /// writes mute state and its one signed presence notice.
    pub(crate) fn set_mute_checked(
        &mut self,
        actor: Actor,
        seat: Ulid,
        muted: bool,
        state: State,
        now: u64,
        expected: (u64, u64),
    ) -> Result<Option<Ulid>> {
        self.change_mute(
            actor,
            seat,
            (muted, state),
            now,
            Some(MuteGuard {
                generation: expected.0,
                epoch: expected.1,
            }),
        )
    }
    fn change_mute(
        &mut self,
        actor: Actor,
        seat: Ulid,
        change: (bool, State),
        now: u64,
        expected: Option<MuteGuard>,
    ) -> Result<Option<Ulid>> {
        let (muted, state) = change;
        if !actor.controls() {
            return Err(invalid("mute requires operator or oracle class"));
        }
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let bytes: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("seat/{seat}")],
                |row| row.get(0),
            )
            .optional()?;
        let mut record: SeatRecord =
            serde_json::from_slice(&bytes.ok_or_else(|| invalid("unknown seat"))?)?;
        if let Some(controller) = actor.seat() {
            let bytes: Vec<u8> = tx.query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("seat/{controller}")],
                |row| row.get(0),
            )?;
            let controller: SeatRecord = serde_json::from_slice(&bytes)?;
            if controller.swarm != record.swarm {
                return Err(invalid("mute outside caller swarm"));
            }
            if expected.is_some() && controller.role.class != Class::Oracle {
                return Err(invalid("mute caller authority changed"));
            }
        }
        if let Some(expected) = expected {
            if record.generation != expected.generation {
                return Err(invalid("stale target process generation"));
            }
            if policy(&tx, &record)? != expected.epoch {
                return Err(invalid("stale target policy epoch"));
            }
        }
        if record.muted == muted {
            let old:Option<String>=tx.query_row("SELECT id FROM msg_notices WHERE json_extract(CAST(envelope AS TEXT),'$.seat')=?1 ORDER BY rowid DESC LIMIT 1",[seat.to_string()],|r|r.get(0)).optional()?;
            return old.map(|id| Ok(id.parse()?)).transpose();
        }
        record.muted = muted;
        let epoch = policy(&tx, &record)?;
        let mut request = SendRequest::text(
            format!("swarms/{}/*", record.swarm),
            if muted { "muted" } else { "unmuted" },
        );
        request.kind = MessageKind::Presence;
        let body = Body::Presence {
            event: if muted { "mute" } else { "unmute" }.into(),
            name: record.name.clone(),
            harness: record.harness_kind.clone(),
            state: crate::adapters::mapping::state(state),
            muted,
            policy_epoch: epoch,
        };
        let envelope = mint(
            &tx,
            self.keys.as_ref(),
            &record,
            &request,
            body,
            Issue {
                epoch,
                now,
                operator_direct: false,
            },
        )?;
        tx.execute("INSERT INTO kv(key,value) VALUES(?1,?2)",params![format!("messaging/control/{}",envelope.id),serde_json::to_vec(&serde_json::json!({"seat":seat,"muted":muted,"initiator":actor.seat(),"operator":matches!(actor,Actor::Operator),"at_ms":now,"notice":envelope.id}))?])?;
        tx.execute(
            "UPDATE kv SET value=?2 WHERE key=?1",
            params![format!("seat/{seat}"), serde_json::to_vec(&record)?],
        )?;
        tx.execute(
            "INSERT INTO msg_notices(id,envelope,request) VALUES(?1,?2,?3)",
            params![
                envelope.id.to_string(),
                serde_json::to_vec(&envelope)?,
                serde_json::to_vec(&request)?
            ],
        )?;
        // Metadata changes are visible on the same seat-state lane as rename
        // and adapter changes, even if the process remains idle. Persist the
        // draft with the policy transaction; the existing bus pump signs and
        // publishes it, then removes this outbox entry only after acceptance.
        let mut changed = Draft::new(
            format!("seat/{seat}/state"),
            "mute",
            serde_json::json!({"muted":muted,"policy_epoch":epoch.to_string()}),
        );
        changed.seat = Some(seat);
        changed.generation = Some(record.generation);
        changed.ts = now;
        tx.execute(
            "INSERT INTO kv(key,value) VALUES(?1,?2)",
            params![
                format!("messaging/seat-state/{}", changed.id),
                serde_json::to_vec(&changed)?
            ],
        )?;
        tx.commit()?;
        self.recover_notices()?;
        self.reconcile(now)?;
        Ok(Some(envelope.id))
    }
    fn recover_notices(&mut self) -> Result<()> {
        let notices = {
            let db = self.account.connection()?;
            let mut q = db.prepare(
                "SELECT envelope,request FROM msg_notices WHERE sent=0 ORDER BY rowid LIMIT 4096",
            )?;
            q.query_map([], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (bytes, raw) in notices {
            let envelope: Envelope = serde_json::from_slice(&bytes)?;
            self.keys.verify(&envelope)?;
            let request: SendRequest = serde_json::from_slice(&raw)?;
            let routes = self.swarm(envelope.swarm_id)?.tree.resolve(
                Actor::Operator,
                &envelope.context,
                envelope.kind,
                envelope.priority_class,
                0,
            )?;
            let wake = self.attention_modes(Attention::signed(&envelope), &routes)?;
            self.swarm(envelope.swarm_id)?
                .ledger
                .accept_with_attention(
                    &request,
                    &envelope,
                    Delivery {
                        routes: &routes,
                        wake: Some(&wake),
                    },
                    None,
                    envelope.timestamp * 1000,
                )?;
            self.account.connection()?.execute(
                "UPDATE msg_notices SET sent=1 WHERE id=?1",
                [envelope.id.to_string()],
            )?;
        }
        Ok(())
    }
    pub fn reconcile(&mut self, now: u64) -> Result<()> {
        for swarm_id in self.swarms.keys().copied().collect::<Vec<_>>() {
            let mut out_after = 0;
            loop {
                let held = self.swarm(swarm_id)?.ledger.held_out(out_after)?;
                if held.is_empty() {
                    break;
                }
                for message in &held {
                    let routes = (|| -> Result<Vec<Route>> {
                        let (sender, class, direct) = self.validate_message(message)?;
                        if sender.as_ref().is_some_and(|sender| sender.muted) && !direct {
                            return Err(invalid("sender muted"));
                        }
                        self.swarm(swarm_id)?.tree.resolve(
                            if direct {
                                Actor::Operator
                            } else {
                                sender
                                    .as_ref()
                                    .map_or(Actor::Operator, |sender| Actor::Seat {
                                        seat: sender.id,
                                        class: sender.role.class,
                                    })
                            },
                            message.view().context,
                            message.view().kind,
                            class,
                            message.view().priority,
                        )
                    })();
                    match routes {
                        Ok(routes) => {
                            let local = message
                                .intent()
                                .map(|_| self.local_recipients(&routes))
                                .transpose()?;
                            let wake =
                                self.attention_modes(Attention::message(message), &routes)?;
                            self.swarm(swarm_id)?.ledger.release_out(
                                message,
                                Delivery {
                                    routes: &routes,
                                    wake: Some(&wake),
                                },
                                now,
                                local.as_ref(),
                            )?;
                        }
                        Err(e) => self
                            .swarm(swarm_id)?
                            .ledger
                            .hold_out(message.sequence, &e.to_string())?,
                    }
                }
                out_after = held.last().unwrap().sequence;
                if held.len() < 256 {
                    break;
                }
            }
            // One pass sees one tree, registry and key set (only receipts change
            // below), so each message is validated and resolved once and each
            // recipient looked up once, not once per pending receipt. A receipt
            // whose stored bytes, message resolution and recipient mute all match
            // the previous pass would reconcile to itself, so it is not decoded.
            let mut messages: HashMap<u64, (Option<Resolved>, bool)> = HashMap::new();
            let mut targets: HashMap<Recipient, (std::result::Result<bool, String>, bool)> =
                HashMap::new();
            let mut seen = HashMap::new();
            let mut after = 0;
            loop {
                let pending = self.swarm(swarm_id)?.ledger.pending_raw(after, 4096)?;
                if pending.is_empty() {
                    break;
                }
                for (rowid, bytes) in &pending {
                    let hash = hash128(bytes);
                    let prev = self.swarm(swarm_id)?.seen.get(rowid).copied();
                    if let Some((h, sequence, recipient)) = prev
                        && h == hash
                    {
                        let message_same = self.resolve_once(swarm_id, sequence, &mut messages)?;
                        let recipient_same = self.muted_once(swarm_id, recipient, &mut targets).1;
                        if message_same && recipient_same {
                            seen.insert(*rowid, (hash, sequence, recipient));
                            continue;
                        }
                    }
                    let receipt: Receipt = serde_json::from_slice(bytes)?;
                    seen.insert(*rowid, (hash, receipt.message_sequence, receipt.recipient));
                    self.resolve_once(swarm_id, receipt.message_sequence, &mut messages)?;
                    let Some(resolved) = &messages[&receipt.message_sequence].0 else {
                        continue;
                    };
                    let remote_pending = receipt.signature
                        == bbt_proto::messaging::SignatureState::Pending
                        && !self.recipient_is_local(receipt.recipient)?;
                    let resolved = match resolved {
                        Err(e) => Err(e.clone()),
                        Ok((class, direct, by)) => match by
                            .get(&(receipt.recipient, receipt.agent.clone()))
                        {
                            None => Err(invalid("route removed or no longer eligible").to_string()),
                            Some(route) => self
                                .muted_once(swarm_id, receipt.recipient, &mut targets)
                                .0
                                .map(|muted| {
                                    let held = if remote_pending {
                                        Some(
                                            "signature pending; pinned remote delivery waits"
                                                .into(),
                                        )
                                    } else if muted && !*direct {
                                        Some("recipient muted".into())
                                    } else if route.arrivals.iter().all(|a| a.paused) {
                                        Some("subscription paused".into())
                                    } else {
                                        None
                                    };
                                    (route.clone(), *class, held)
                                }),
                        },
                    };
                    match resolved {
                        Ok((route, class, held)) => self.swarm(swarm_id)?.ledger.update_policy(
                            receipt,
                            Some(&route),
                            held,
                            class,
                        )?,
                        Err(e) => self.swarm(swarm_id)?.ledger.update_policy(
                            receipt,
                            None,
                            Some(e),
                            Class::Worker,
                        )?,
                    }
                }
                after = pending.last().unwrap().0;
                if pending.len() < 4096 {
                    break;
                }
            }
            let sw = self.swarm(swarm_id)?;
            sw.seen = seen;
            sw.resolved = messages.into_iter().map(|(k, (r, _))| (k, r)).collect();
            sw.muted = targets.into_iter().map(|(k, (m, _))| (k, m)).collect();
        }
        Ok(())
    }
    /// Resolves message `sequence` into `memo` once per pass; true when it equals the last pass's.
    fn resolve_once(
        &mut self,
        swarm_id: Ulid,
        sequence: u64,
        memo: &mut HashMap<u64, (Option<Resolved>, bool)>,
    ) -> Result<bool> {
        if let Some(hit) = memo.get(&sequence) {
            return Ok(hit.1);
        }
        let message = self.swarm(swarm_id)?.ledger.message(sequence)?;
        let entry = Some(
            (|| -> Result<_> {
                let (sender, class, direct) = self.validate_message(&message)?;
                // A local mute notice is signed as its subject, but its routing
                // authority came from the retained operator transaction. Match
                // the exact journaled bytes; a seat cannot forge this exception.
                let notice = if let Some(envelope) = message.envelope()
                    && matches!(envelope.body, Body::Presence { .. })
                {
                    let bytes: Option<Vec<u8>> = self
                        .account
                        .connection()?
                        .query_row(
                            "SELECT envelope FROM msg_notices WHERE id=?1",
                            [envelope.id.to_string()],
                            |row| row.get(0),
                        )
                        .optional()?;
                    bytes.as_deref() == Some(serde_json::to_vec(envelope)?.as_slice())
                } else {
                    false
                };
                if sender.as_ref().is_some_and(|sender| sender.muted) && !direct && !notice {
                    return Err(invalid("sender muted"));
                }
                let routes = self.swarm(swarm_id)?.tree.resolve(
                    if direct || notice {
                        Actor::Operator
                    } else {
                        sender
                            .as_ref()
                            .map_or(Actor::Operator, |sender| Actor::Seat {
                                seat: sender.id,
                                class: sender.role.class,
                            })
                    },
                    message.view().context,
                    message.view().kind,
                    class,
                    message.view().priority,
                )?;
                let mut by = HashMap::new();
                for r in routes {
                    by.entry((r.recipient, r.agent.clone())).or_insert(r);
                }
                Ok((class, direct, by))
            })()
            .map_err(|e| e.to_string()),
        );
        let same = self.swarm(swarm_id)?.resolved.get(&sequence) == Some(&entry);
        memo.insert(sequence, (entry, same));
        Ok(same)
    }
    /// This pass's mute of `recipient`, and whether it equals the last pass's.
    fn muted_once(
        &mut self,
        swarm_id: Ulid,
        recipient: Recipient,
        memo: &mut HashMap<Recipient, (std::result::Result<bool, String>, bool)>,
    ) -> (std::result::Result<bool, String>, bool) {
        if let Some(hit) = memo.get(&recipient) {
            return hit.clone();
        }
        let muted = match recipient {
            Recipient::Seat(seat) => self
                .registry
                .get(seat)
                .map(|t| t.muted)
                .map_err(|e| e.to_string()),
            recipient @ (Recipient::Operator(_) | Recipient::OperatorLocal(_))
                if self.operator_reference.map(OperatorReference::recipient) == Some(recipient) =>
            {
                Ok(false)
            }
            Recipient::Operator(_) | Recipient::OperatorLocal(_) => {
                Err("operator device is not the active account device".into())
            }
        };
        let same = self
            .swarm(swarm_id)
            .ok()
            .and_then(|s| s.muted.get(&recipient))
            == Some(&muted);
        memo.insert(recipient, (muted.clone(), same));
        (muted, same)
    }
    /// Small ordinary observation: no channel list, role or policy duplication per seat.
    pub fn deferred_view(&self, record: &SeatRecord) -> Result<Option<serde_json::Value>> {
        let Some(swarm) = self.swarms.get(&record.swarm) else {
            return Ok(Some(
                serde_json::json!({"deferred_error":"messaging observation unavailable"}),
            ));
        };
        Ok(swarm
            .ledger
            .digest_summary(record.id)?
            .map(|summary| serde_json::json!({"deferred":summary})))
    }
    /// One explicitly selected seat, read without another consumer or scheduler mutation.
    pub fn attention_view(&self, record: &SeatRecord) -> Result<serde_json::Value> {
        let mut value = crate::mcp::attention::view(record).map_err(invalid)?;
        let summary = self.deferred_view(record)?;
        value["deferred"] = summary
            .as_ref()
            .and_then(|v| v.get("deferred"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if let Some(reason) = summary.as_ref().and_then(|v| v.get("deferred_error")) {
            value["deferred_error"] = reason.clone();
        }
        let (channels, complete) = self
            .swarms
            .get(&record.swarm)
            .map(|swarm| swarm.tree.attention_channels(128))
            .unwrap_or_default();
        value["channels"] = serde_json::json!(channels);
        value["channels_complete"] = complete.into();
        Ok(value)
    }
    pub fn ready(&mut self, now: u64) -> Result<Vec<(Ulid, Ulid)>> {
        self.recover_notices()?;
        self.reconcile(now)?;
        let mut targets = Vec::new();
        for (&id, swarm) in &self.swarms {
            for seat in swarm.ledger.targets(now)? {
                targets.push((id, seat));
            }
        }
        Ok(targets)
    }
    pub fn prepare(&mut self, swarm: Ulid, seat: Ulid, now: u64) -> Result<Option<Work>> {
        self.swarm(swarm)?.ledger.prepare(seat, now)
    }
    pub fn finish(
        &mut self,
        swarm: Ulid,
        work: &Work,
        outcome: &WakeOutcome,
        now: u64,
        relayed: bool,
    ) -> Result<()> {
        self.swarm(swarm)?
            .ledger
            .finish(work, outcome, now, relayed)
    }
    pub fn acknowledge(
        &mut self,
        seat: Ulid,
        id: Ulid,
        agent: Option<&str>,
        now: u64,
    ) -> Result<()> {
        let swarm = self.registry.get(seat)?.swarm;
        self.swarm(swarm)?.ledger.acknowledge(seat, id, agent, now)
    }
    pub fn events(&self) -> Result<Vec<(String, Draft)>> {
        let mut events = Vec::new();
        {
            let db = self.account.connection()?;
            // A bounded indexed range over pending policy events, not a scan of
            // historical messaging/control audit records.
            let mut query = db.prepare_cached(
                "SELECT value FROM kv WHERE key >= 'messaging/seat-state/' AND key < 'messaging/seat-state0' ORDER BY key LIMIT 128",
            )?;
            for bytes in query.query_map([], |row| row.get::<_, Vec<u8>>(0))? {
                let draft: Draft = serde_json::from_slice(&bytes?)?;
                let seat = draft
                    .seat
                    .ok_or_else(|| invalid("mute state event has no seat"))?;
                events.push((format!("seat-{seat}"), draft));
            }
        }
        for (&swarm, s) in &self.swarms {
            for draft in s.ledger.bus_outbox()? {
                events.push((format!("swarm-{swarm}"), draft));
            }
        }
        Ok(events)
    }
    pub fn event_sent(&self, stream: &str, id: Ulid) -> Result<()> {
        if stream
            .strip_prefix("seat-")
            .is_some_and(|seat| seat.parse::<Ulid>().is_ok())
        {
            self.account.connection()?.execute(
                "DELETE FROM kv WHERE key=?1",
                [format!("messaging/seat-state/{id}")],
            )?;
            return Ok(());
        }
        let swarm = stream
            .strip_prefix("swarm-")
            .ok_or_else(|| invalid("invalid messaging stream"))?
            .parse::<Ulid>()?;
        self.swarms
            .get(&swarm)
            .ok_or_else(|| invalid("unknown swarm stream"))?
            .ledger
            .bus_sent(id)
    }
    pub fn event_published(&self, stream: &str, id: Ulid, sequence: u64) -> Result<()> {
        if stream.starts_with("seat-") {
            return self.event_sent(stream, id);
        }
        let swarm = stream
            .strip_prefix("swarm-")
            .ok_or_else(|| invalid("invalid messaging stream"))?
            .parse::<Ulid>()?;
        let ledger = &self
            .swarms
            .get(&swarm)
            .ok_or_else(|| invalid("unknown swarm stream"))?
            .ledger;
        if sequence > 0 {
            ledger.record_publication(
                id,
                &bbt_proto::messaging::BusPosition {
                    stream: stream.into(),
                    seq: sequence,
                },
            )?;
        }
        ledger.bus_sent(id)
    }
}
fn policy(tx: &rusqlite::Transaction<'_>, record: &SeatRecord) -> Result<u64> {
    let old: Option<(u64, Vec<u8>, bool)> = tx
        .query_row(
            "SELECT epoch,role,muted FROM msg_policies WHERE seat=?1",
            [record.id.to_string()],
            |r| Ok((u64_at(r, 0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let role = serde_json::to_vec(&record.role)?;
    let epoch = match old {
        Some((epoch, old_role, muted)) if old_role == role && muted == record.muted => {
            return Ok(epoch);
        }
        Some((epoch, _, _)) => epoch
            .checked_add(1)
            .ok_or_else(|| invalid("seat policy epoch exhausted"))?,
        None => 1,
    };
    tx.execute("INSERT INTO msg_policies VALUES(?1,?2,?3,?4) ON CONFLICT(seat) DO UPDATE SET epoch=excluded.epoch,role=excluded.role,muted=excluded.muted",params![record.id.to_string(),sqlint(epoch)?,role,record.muted])?;
    Ok(epoch)
}
fn mint_operator(
    tx: &rusqlite::Transaction<'_>,
    authority: &super::keys::OperatorSigning,
    swarm: Ulid,
    request: &SendRequest,
    body: Body,
    now: u64,
    origin: &crate::mcp::Origin,
) -> Result<Envelope> {
    let counter = Recipient::Operator(authority.device_id).to_string();
    let last: Option<u64> = tx
        .query_row(
            "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=0",
            [&counter],
            |row| u64_at(row, 0),
        )
        .optional()?;
    let sequence = last
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| invalid("sender sequence exhausted"))?;
    let mut envelope = Envelope {
        version: 1,
        id: request.id,
        issuer_kind: authority.issuer.kind,
        issuer: authority.issuer.key.clone(),
        account: authority.issuer.key.clone(),
        device: authority.device.clone(),
        seat: Ulid::nil(),
        author: authority.device.clone(),
        swarm_id: swarm,
        process_generation: 0,
        context: request.destination.clone(),
        role: crate::registry::Role {
            name: "operator".into(),
            class: Class::Oracle,
        },
        role_grant_id: authority.grant.clone(),
        role_epoch: authority.epoch,
        priority_class: Class::Oracle,
        sender_sequence: sequence,
        thread_id: request.thread_id.unwrap_or(request.id),
        reply_to: request.reply_to,
        supersedes: request.supersedes,
        timestamp: now / 1000,
        priority: request.priority,
        kind: request.kind,
        body,
        attachments: request.attachments.clone(),
        door: Some(origin.door),
        issued_by: origin.issued_by.clone(),
        signature: String::new(),
    };
    envelope.sign(&authority.key)?;
    tx.execute("INSERT INTO msg_counters VALUES(?1,0,?2) ON CONFLICT(seat,generation) DO UPDATE SET sequence=excluded.sequence",
        params![counter, sqlint(sequence)?])?;
    Ok(envelope)
}

struct Issue {
    epoch: u64,
    now: u64,
    operator_direct: bool,
}
fn mint(
    tx: &rusqlite::Transaction<'_>,
    keys: &dyn SeatKeys,
    record: &SeatRecord,
    request: &SendRequest,
    body: Body,
    issue: Issue,
) -> Result<Envelope> {
    let Issue {
        epoch,
        now,
        operator_direct,
    } = issue;
    let last: Option<u64> = tx
        .query_row(
            "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=?2",
            params![record.id.to_string(), sqlint(record.generation)?],
            |r| u64_at(r, 0),
        )
        .optional()?;
    let sequence = last
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| invalid("sender sequence exhausted"))?;
    let issuer = keys.issuer();
    let leaf = keys.leaf(record.id, record.generation)?;
    let grant = Grant {
        seat: record.id,
        generation: record.generation,
        swarm: record.swarm,
        role: record.role.clone(),
        epoch,
        muted: record.muted,
        operator_direct,
        author: public(&leaf.verifying_key().to_bytes()),
    };
    let raw_grant = serde_json::to_vec(&grant)?;
    let grant_id = digest(&raw_grant);
    let mut envelope = Envelope {
        version: 1,
        id: request.id,
        issuer_kind: issuer.kind,
        issuer: issuer.key.clone(),
        account: issuer.key.clone(),
        device: issuer.key,
        seat: record.id,
        author: grant.author,
        swarm_id: record.swarm,
        process_generation: record.generation,
        context: request.destination.clone(),
        role: record.role.clone(),
        role_grant_id: grant_id,
        role_epoch: epoch,
        priority_class: record.role.class,
        sender_sequence: sequence,
        thread_id: request.thread_id.unwrap_or(request.id),
        reply_to: request.reply_to,
        supersedes: request.supersedes,
        timestamp: now / 1000,
        priority: request.priority,
        kind: request.kind,
        body,
        attachments: request.attachments.clone(),
        door: None,
        issued_by: None,
        signature: String::new(),
    };
    keys.finalize(&mut envelope, record, tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO msg_grants VALUES(?1,?2)",
        params![envelope.role_grant_id, raw_grant],
    )?;
    keys.sign_envelope(&mut envelope, &leaf)?;
    tx.execute("INSERT INTO msg_counters VALUES(?1,?2,?3) ON CONFLICT(seat,generation) DO UPDATE SET sequence=excluded.sequence",params![record.id.to_string(),sqlint(record.generation)?,sqlint(sequence)?])?;
    Ok(envelope)
}

/// Where a reply to `message` goes: its seat's inbox, or the operator's (decision #264).
fn sender_inbox(message: &super::ledger::StoredMessage) -> String {
    if message.view().seat == Ulid::nil() {
        "operator/inbox".into()
    } else {
        format!("seats/{}/inbox", message.view().seat)
    }
}

#[cfg(test)]
mod reference_tests {
    use super::super::{
        keys::LocalSeatKeys,
        wire::{Issuer, IssuerKind},
    };
    use super::*;
    use ed25519_dalek::SigningKey;

    struct NoAuthor;
    impl SeatKeys for NoAuthor {
        fn issuer(&self) -> Issuer {
            Issuer {
                kind: IssuerKind::LocalDev,
                key: String::new(),
            }
        }
        fn ready_issuer(&self) -> Option<Issuer> {
            None
        }
        fn signing_hold(&self) -> Option<String> {
            Some("fixture custody unavailable".into())
        }
        fn leaf(&self, _: Ulid, _: u64) -> Result<SigningKey> {
            panic!("pending local messaging must not ask for a signing key")
        }
        fn verify(&self, _: &Envelope) -> Result<()> {
            Err(invalid("pending has no author"))
        }
    }

    fn recent_fixture() -> (tempfile::TempDir, Messaging, SeatRecord) {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let recipient = SeatRecord::new(Ulid::new(), Ulid::new(), "recent fixture", "zsh");
        registry.insert(&recipient).unwrap();
        let messaging = Messaging::open(root.path(), Arc::new(NoAuthor)).unwrap();
        (root, messaging, recipient)
    }
    fn recent(
        messaging: &mut Messaging,
        swarm: Ulid,
        channel: Option<String>,
        before: Option<u64>,
        limit: usize,
    ) -> super::super::api::Response {
        messaging
            .request_operator_pending(
                None,
                &Default::default(),
                swarm,
                super::super::api::Request::Recent {
                    channel,
                    before,
                    limit,
                },
                1_000_000,
            )
            .unwrap()
    }
    fn pending_text(messaging: &mut Messaging, recipient: &SeatRecord, text: &str) -> Ulid {
        let request = SendRequest::text(format!("seats/{}/inbox", recipient.id), text);
        let id = request.id;
        assert!(matches!(
            messaging
                .request_operator_pending(
                    None,
                    &Default::default(),
                    recipient.swarm,
                    super::super::api::Request::Send { message: request },
                    1000
                )
                .unwrap(),
            super::super::api::Response::Stored { .. }
        ));
        id
    }

    #[test]
    fn recent_operator_read_pages_latest_messages_despite_receipt_traffic_and_refuses_seats() {
        use super::super::api::{Request, Response};
        let (_root, mut messaging, recipient) = recent_fixture();
        let ids: Vec<_> = (0..4)
            .map(|number| pending_text(&mut messaging, &recipient, &format!("recent-{number}")))
            .collect();
        // Repeated native recipient-store outcomes create real receipt history;
        // newest messages remain in their own ledger index rather than the bus tail.
        for attempt in 0..140 {
            let now = 10_000 + attempt * 10_000;
            let work = messaging
                .prepare(recipient.swarm, recipient.id, now)
                .unwrap()
                .unwrap();
            messaging
                .finish(
                    recipient.swarm,
                    &work,
                    &WakeOutcome {
                        evidence: crate::adapters::Evidence::RecipientStored,
                        native_id: None,
                        note: "receipt-heavy fixture".into(),
                    },
                    now + 1,
                    false,
                )
                .unwrap();
        }
        assert!(
            messaging
                .swarm(recipient.swarm)
                .unwrap()
                .ledger
                .receipt_events(ids[0], recipient.id, None)
                .unwrap()
                .len()
                > 128
        );
        let Response::Recent {
            page,
            rows,
            projected_at,
        } = recent(&mut messaging, recipient.swarm, None, None, 2)
        else {
            panic!("recent response")
        };
        assert!(projected_at > 0);
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.view().id)
                .collect::<Vec<_>>(),
            ids[2..]
        );
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), ids[2..]);
        assert!(
            rows.iter()
                .all(|row| row.signature == bbt_proto::messaging::SignatureState::Pending)
        );
        let before = page.next.expect("older messages remain");
        assert_eq!(before, page.items[0].sequence);
        let Response::Recent { page, rows, .. } =
            recent(&mut messaging, recipient.swarm, None, Some(before), 2)
        else {
            unreachable!()
        };
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.view().id)
                .collect::<Vec<_>>(),
            ids[..2]
        );
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), ids[..2]);
        assert!(page.items.iter().all(|item| item.sequence < before));
        assert_eq!(page.next, None);
        let channel = format!("seats/{}/inbox", recipient.id);
        let Response::Recent { page, .. } =
            recent(&mut messaging, recipient.swarm, Some(channel), None, 128)
        else {
            unreachable!()
        };
        assert_eq!(page.items.len(), 4);
        for operator in [false, true] {
            let error = messaging
                .request(
                    recipient.id,
                    1,
                    Request::Recent {
                        channel: None,
                        before: None,
                        limit: 2,
                    },
                    operator,
                    None,
                    1_000_000,
                )
                .unwrap_err();
            assert!(error.to_string().contains("operator authority"));
        }
        for limit in [0, 129] {
            assert!(
                messaging
                    .request_operator_pending(
                        None,
                        &Default::default(),
                        recipient.swarm,
                        Request::Recent {
                            channel: None,
                            before: None,
                            limit
                        },
                        1_000_000
                    )
                    .is_err()
            );
        }
        assert!(
            messaging
                .request_operator_pending(
                    None,
                    &Default::default(),
                    recipient.swarm,
                    Request::Recent {
                        channel: Some("../private".into()),
                        before: None,
                        limit: 2
                    },
                    1_000_000
                )
                .is_err()
        );
    }

    #[test]
    fn recent_complete_response_fits_bytes_without_truncating_messages_or_losing_older_cursor() {
        use super::super::api::Response;
        let (_root, mut messaging, recipient) = recent_fixture();
        let text = "x".repeat(bbt_proto::messaging::MAX_TEXT);
        let ids: Vec<_> = (0..36)
            .map(|_| pending_text(&mut messaging, &recipient, &text))
            .collect();
        let response = recent(&mut messaging, recipient.swarm, None, None, 128);
        assert!(serde_json::to_vec(&response).unwrap().len() <= 512 * 1024);
        let Response::Recent { page, rows, .. } = response else {
            unreachable!()
        };
        assert!(!page.items.is_empty() && page.items.len() < ids.len());
        assert_eq!(rows.len(), page.items.len());
        assert_eq!(page.items.last().unwrap().view().id, *ids.last().unwrap());
        assert_eq!(page.next, Some(page.items[0].sequence));
        assert!(rows.iter().all(|row| row.text == text));
        let mut seen: Vec<_> = page.items.iter().map(|item| item.view().id).collect();
        let mut before = page.next;
        while let Some(cursor) = before {
            let response = recent(&mut messaging, recipient.swarm, None, Some(cursor), 128);
            assert!(serde_json::to_vec(&response).unwrap().len() <= 512 * 1024);
            let Response::Recent { page, rows, .. } = response else {
                unreachable!()
            };
            assert_eq!(rows.len(), page.items.len());
            assert!(page.items.iter().all(|item| item.sequence < cursor));
            let older: Vec<_> = page.items.iter().map(|item| item.view().id).collect();
            assert!(!older.is_empty());
            seen.splice(0..0, older);
            before = page.next;
            assert!(seen.len() <= ids.len());
        }
        assert_eq!(
            seen, ids,
            "exclusive cursor recovers every whole message exactly once"
        );
        let other = SeatRecord::new(Ulid::new(), recipient.swarm, "empty filtered inbox", "zsh");
        messaging.registry.insert(&other).unwrap();
        messaging.sync_seat(&other).unwrap();
        let channel = Some(format!("seats/{}/inbox", other.id));
        let Response::Recent { page, rows, .. } =
            recent(&mut messaging, recipient.swarm, channel.clone(), None, 128)
        else {
            unreachable!()
        };
        assert!(page.items.is_empty() && rows.is_empty());
        let cursor = page
            .next
            .expect("byte-bounded scan must not traverse every unrelated body");
        assert!(cursor > 1 && cursor < 36);
        let Response::Recent { page, rows, .. } =
            recent(&mut messaging, recipient.swarm, channel, Some(cursor), 128)
        else {
            unreachable!()
        };
        assert!(page.items.is_empty() && rows.is_empty());
        assert_eq!(page.next, None);
        let hidden = messaging
            .swarm(recipient.swarm)
            .unwrap()
            .ledger
            .recent_for(Some(Recipient::Seat(other.id)), None, None, 128)
            .unwrap();
        assert!(hidden.items.is_empty());
        assert_eq!(
            hidden.next,
            Some(cursor),
            "visibility filtering also obeys the physical scan budget"
        );
        let older_hidden = messaging
            .swarm(recipient.swarm)
            .unwrap()
            .ledger
            .recent_for(Some(Recipient::Seat(other.id)), None, hidden.next, 128)
            .unwrap();
        assert!(older_hidden.items.is_empty());
        assert_eq!(older_hidden.next, None);
    }

    // keeps: spec messaging
    #[test]
    fn first_held_operator_delivers_an_authorless_intent_but_cannot_mute() {
        use super::super::api::{Request, Response};
        use bbt_proto::messaging::{LogicalSender, SignatureState};
        let root = tempfile::tempdir().unwrap();
        let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
        let registry = Registry::new(account);
        let swarm = Ulid::new();
        let recipient = SeatRecord::new(Ulid::new(), swarm, "recipient", "zsh");
        registry.insert(&recipient).unwrap();
        registry
            .set_attention(
                recipient.id,
                1,
                bbt_proto::registry::AttentionRule {
                    destination: format!("seats/{}/inbox", recipient.id),
                    class: bbt_proto::messaging::MessageClass::Conversation,
                    mode: WakeMode::Inbox,
                },
            )
            .unwrap();

        let mut messaging = Messaging::open(root.path(), Arc::new(NoAuthor)).unwrap();
        let request =
            SendRequest::text(format!("seats/{}/inbox", recipient.id), "first operator DM");
        let id = request.id;
        let Response::Stored { message } = messaging
            .request_operator_pending(
                None,
                &Default::default(),
                swarm,
                Request::Send { message: request },
                1000,
            )
            .unwrap()
        else {
            panic!("local operator message must be stored");
        };
        let intent = message.intent().unwrap();
        assert_eq!(intent.role_epoch, None);
        let LogicalSender::OperatorLocal {
            reference, device, ..
        } = &intent.sender
        else {
            panic!("explicit local principal");
        };
        assert_eq!(*device, None);
        assert_eq!(reference.kind, OperatorReferenceKind::OperatorLocal);
        assert!(
            serde_json::to_value(intent)
                .unwrap()
                .get("author")
                .is_none()
        );
        let inbox = messaging.list(recipient.id, None, None, 0, 16).unwrap();
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(inbox.items[0].view().id, id);
        let receipt = messaging.receipts(recipient.id, id).unwrap().remove(0);
        assert_eq!(receipt.wake, Some(bbt_proto::messaging::ReceiptWake::Now));
        assert_eq!(receipt.signature, SignatureState::Pending);
        assert_eq!(receipt.sender_operator, Some(*reference));
        assert_eq!(receipt.sender_device, None);
        assert_eq!(
            messaging
                .operator_message_row(swarm, id)
                .unwrap()
                .issuer_kind,
            None
        );
        assert!(
            messaging
                .request_operator_pending(
                    None,
                    &Default::default(),
                    swarm,
                    Request::Mute {
                        seat: recipient.id,
                        muted: true
                    },
                    1001
                )
                .unwrap_err()
                .to_string()
                .contains("requires signing custody")
        );
        assert!(!registry.get(recipient.id).unwrap().muted);
        let reference = *reference;
        let binding = super::super::pending::binding(intent).unwrap();
        let original = serde_json::to_vec(intent).unwrap();
        drop(messaging);
        let mut messaging = Messaging::open(root.path(), Arc::new(NoAuthor)).unwrap();
        assert_eq!(messaging.operator_reference(None).unwrap(), reference);
        let work = messaging
            .prepare(swarm, recipient.id, 10000)
            .unwrap()
            .unwrap();
        messaging
            .finish(
                swarm,
                &work,
                &WakeOutcome {
                    evidence: crate::adapters::Evidence::NativeWritten,
                    native_id: Some("operator-write-once".into()),
                    note: "fixture wrote once".into(),
                },
                10001,
                false,
            )
            .unwrap();
        let key = SigningKey::from_bytes(&[63; 32]);
        let public = public(&key.verifying_key().to_bytes());
        let grant =
            serde_json::to_vec(&serde_json::json!({"public_key":public,"epoch":"1"})).unwrap();
        messaging
            .account
            .put("identity/local-issuer", &grant)
            .unwrap();
        let authority = super::super::keys::OperatorSigning {
            device_id: Ulid::from(14u128),
            issuer: Issuer {
                kind: IssuerKind::LocalDev,
                key: public.clone(),
            },
            device: public,
            grant: digest(&grant),
            epoch: 1,
            key: key.clone(),
            origin: Default::default(),
        };
        assert_eq!(
            messaging
                .resume_signing_with_operator(
                    Arc::new(LocalSeatKeys::new(key)),
                    Some(&authority),
                    20000
                )
                .unwrap(),
            1
        );
        assert_eq!(messaging.operator_reference(None).unwrap(), reference);
        let row = messaging
            .list(recipient.id, None, None, 0, 16)
            .unwrap()
            .items
            .remove(0);
        assert_eq!(row.upgrade_of(), Some(&binding));
        row.envelope().unwrap().verify_signature().unwrap();
        assert!(
            messaging
                .prepare(swarm, recipient.id, 20001)
                .unwrap()
                .is_none()
        );
        let receipt = messaging.receipts(recipient.id, id).unwrap().remove(0);
        assert_eq!(receipt.sender_operator, Some(reference));
        assert_eq!(receipt.sender_device, Some(authority.device_id));
        assert_eq!(receipt.attempt, Some(work.id));
        let retained: Vec<u8> = messaging
            .swarm(swarm)
            .unwrap()
            .ledger
            .store
            .connection()
            .unwrap()
            .query_row(
                "SELECT envelope FROM msg_messages WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        let retained: bbt_proto::messaging::StoredContent =
            serde_json::from_slice(&retained).unwrap();
        let bbt_proto::messaging::StoredContent::Pending { intent } = retained else {
            panic!("original intent cannot be replaced");
        };
        assert_eq!(serde_json::to_vec(&intent).unwrap(), original);
        assert!(
            messaging
                .events()
                .unwrap()
                .iter()
                .any(|(_, event)| event.payload["actual_device"]
                    == serde_json::json!(authority.device_id)
                    && event.payload["upgrade_of"]["digest"] == binding.digest)
        );
    }

    // keeps: spec messaging
    #[test]
    fn operator_reference_survives_enrolment_without_moving_its_recipient() {
        let root = tempfile::tempdir().unwrap();
        let keys = Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[61; 32])));
        let mut messaging = Messaging::open(root.path(), keys.clone()).unwrap();
        let reference = messaging.operator_reference(None).unwrap();
        assert_eq!(reference.kind, OperatorReferenceKind::OperatorLocal);
        assert_eq!(messaging.operator_device_attribution, None);
        assert!(matches!(reference.recipient(),Recipient::OperatorLocal(id) if id==reference.id));
        let immutable = messaging
            .account
            .get("messaging/operator-reference")
            .unwrap()
            .unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(&immutable)
                .unwrap()
                .get("device")
                .is_none()
        );
        drop(messaging);

        let mut messaging = Messaging::open(root.path(), keys).unwrap();
        let device = Ulid::from(12u128);
        assert_eq!(
            messaging.operator_reference(Some(device)).unwrap(),
            reference
        );
        assert_eq!(messaging.operator_reference(None).unwrap(), reference);
        assert_eq!(messaging.operator_device_attribution, Some(device));
        assert_eq!(
            messaging
                .account
                .get("messaging/operator-reference")
                .unwrap()
                .unwrap(),
            immutable
        );
        let bytes = messaging
            .account
            .get(&format!(
                "messaging/operator-bindings/{}/{}",
                reference.id, device
            ))
            .unwrap()
            .unwrap();
        let binding: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            binding["reference"],
            serde_json::to_value(reference).unwrap()
        );
        assert_eq!(binding["device"], serde_json::json!(device));
        assert_eq!(
            messaging
                .account
                .list("messaging/operator-bindings/")
                .unwrap()
                .len(),
            1
        );
    }

    // keeps: spec messaging
    #[test]
    fn an_existing_device_keeps_its_original_operator_recipient_encoding() {
        let root = tempfile::tempdir().unwrap();
        let mut messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[62; 32]))),
        )
        .unwrap();
        let device = Ulid::from(13u128);
        let reference = messaging.operator_reference(Some(device)).unwrap();
        assert_eq!(reference.id, device);
        assert_eq!(reference.kind, OperatorReferenceKind::DeviceDerived);
        assert_eq!(
            reference.recipient().to_string(),
            format!("operator:{device}")
        );
        assert!(messaging.operator_reference(Some(Ulid::nil())).is_err());
        assert_eq!(messaging.operator_reference(None).unwrap(), reference);
    }
}

#[cfg(test)]
mod native_attention_tests {
    use super::super::keys::LocalSeatKeys;
    use super::*;
    use bbt_proto::{
        messaging::{MessageClass, ReceiptWake},
        registry::AttentionRule,
    };
    use ed25519_dalek::SigningKey;

    // keeps: conversation-only task exception; Oracle priority is not operator authority.
    #[test]
    fn task_delegation_forces_wake_but_oracle_seat_prose_does_not() {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let mut sender = SeatRecord::new(Ulid::new(), swarm, "oracle", "zsh");
        sender.role.class = Class::Oracle;
        let receiver = SeatRecord::new(Ulid::new(), swarm, "executor", "zsh");
        registry.insert(&sender).unwrap();
        registry.insert(&receiver).unwrap();
        let destination = format!("seats/{}/inbox", receiver.id);
        registry
            .set_attention(
                receiver.id,
                1,
                AttentionRule {
                    destination: destination.clone(),
                    class: MessageClass::Conversation,
                    mode: WakeMode::Inbox,
                },
            )
            .unwrap();
        let mut messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[67; 32]))),
        )
        .unwrap();
        let prose = messaging
            .send(
                sender.id,
                1,
                SendRequest::text(&destination, "Oracle class is not the operator"),
                1_000,
            )
            .unwrap();
        assert_eq!(
            messaging.receipts(sender.id, prose.view().id).unwrap()[0].wake,
            Some(ReceiptWake::None {
                reason: Some(bbt_proto::messaging::WakeReason::Policy)
            })
        );
        let mut request = SendRequest::text(&destination, "your task brief");
        request.kind = MessageKind::Task;
        let delegated = messaging
            .task_event(
                sender.id,
                1,
                request,
                Body::Task {
                    task_id: Ulid::new(),
                    event: "delegate".into(),
                    path: "tasks/fixture".into(),
                    summary: "your task brief".into(),
                },
                2_000,
            )
            .unwrap();
        assert_eq!(
            messaging.receipts(sender.id, delegated.view().id).unwrap()[0].wake,
            Some(ReceiptWake::Now)
        );
        let work = messaging
            .prepare(swarm, receiver.id, 2_001)
            .unwrap()
            .unwrap();
        assert_eq!(
            work.messages
                .iter()
                .map(|m| m.view().id)
                .collect::<Vec<_>>(),
            [delegated.view().id]
        );
    }
}

#[cfg(test)]
mod thread_tests {
    use super::super::{
        api::{Request, Response},
        keys::LocalSeatKeys,
    };
    use super::*;
    use ed25519_dalek::SigningKey;

    fn fixture() -> (
        tempfile::TempDir,
        Registry,
        Messaging,
        SeatRecord,
        SeatRecord,
    ) {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let mut caller = SeatRecord::new(Ulid::new(), swarm, "oracle-class caller", "zsh");
        caller.role.class = Class::Oracle;
        let peer = SeatRecord::new(Ulid::new(), swarm, "peer", "zsh");
        registry.insert(&caller).unwrap();
        registry.insert(&peer).unwrap();
        let messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[71; 32]))),
        )
        .unwrap();
        (root, registry, messaging, caller, peer)
    }

    // keeps: admitted local operator errors remain typed before pending/signing effects.
    #[test]
    fn operator_pending_unknown_reply_refuses_before_message_or_signing_effects() {
        let (_root, _registry, mut messaging, caller, _peer) = fixture();
        let mut request = SendRequest::text("", "unknown parent");
        request.reply_to = Some(Ulid::new());
        let response = messaging
            .request_operator_pending(
                None,
                &Default::default(),
                caller.swarm,
                Request::Send { message: request },
                1_000,
            )
            .unwrap();
        let Response::Refused { error } = response else {
            panic!("typed refusal")
        };
        assert_eq!(error.kind, "unknown_thread");
        assert!(!error.hint.is_empty());
        let count: i64 = messaging
            .swarm(caller.swarm)
            .unwrap()
            .ledger
            .store
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let db = messaging.account.connection().unwrap();
        for table in ["msg_counters", "msg_held"] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    // keeps: Oracle priority class never opens another sender's private operator thread.
    #[test]
    fn oracle_class_cannot_reply_to_or_hint_a_private_operator_inbox_thread() {
        let (_root, _registry, mut messaging, caller, peer) = fixture();
        let visible = messaging
            .request_operator_pending(
                None,
                &Default::default(),
                caller.swarm,
                Request::Send {
                    message: SendRequest::text(
                        format!("seats/{}/inbox", caller.id),
                        "visible to caller",
                    ),
                },
                1_000,
            )
            .unwrap();
        let Response::Stored { message } = visible else {
            panic!("local operator inbox delivery")
        };
        let visible = message.view().id;
        let private = messaging
            .send(
                peer.id,
                1,
                SendRequest::text("operator/inbox", "private to operator"),
                2_000,
            )
            .unwrap()
            .view()
            .id;
        let mut request = SendRequest::text("", "cannot answer that private thread");
        request.reply_to = Some(private);
        let response = messaging
            .request(
                caller.id,
                1,
                Request::Send { message: request },
                false,
                None,
                3_000,
            )
            .unwrap();
        let Response::Refused { error } = response else {
            panic!("private operator thread remains hidden")
        };
        assert_eq!(error.kind, "unknown_thread");
        assert!(error.hint.contains(&visible.to_string()));
        assert!(!error.hint.contains(&private.to_string()));
    }
}

#[cfg(test)]
mod suppression_tests;

fn request_body(request: &SendRequest) -> Body {
    if request.kind == MessageKind::Withdrawn {
        Body::Withdrawn {}
    } else {
        Body::Text {
            text: request.text.clone(),
        }
    }
}
