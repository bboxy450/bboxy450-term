//! The logic owner mediates keys, current grants, inboxes and the local bus.
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ulid::Ulid;

use super::{
    keys::SeatKeys,
    ledger::{Ledger, Page, Receipt, StoredMessage, Work, u64_at},
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
enum Source<'a> {
    Seat {
        record: &'a SeatRecord,
        operator: bool,
    },
    Operator(&'a super::keys::OperatorSigning),
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
    operator_device: Option<Ulid>,
}
impl Messaging {
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
        self.request_from(swarm, Source::Operator(authority), request, mute_state, now)
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
        let actor = source.actor();
        match request {
            Request::Send { message } => match source {
                Source::Seat { record, operator } => {
                    self.send_inner(record.id, record.generation, message, now, operator, None)
                }
                Source::Operator(authority) => {
                    self.send_operator(authority, swarm, message, None, now)
                }
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
                Source::Operator(authority) => {
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
                        channel
                            .is_none()
                            .then_some(Recipient::Operator(authority.device_id)),
                        thread,
                        channel.as_deref(),
                        after,
                        limit,
                    )?;
                    Ok(Response::Messages { page })
                }
            },
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
                    Source::Operator(authority) => self.swarm(swarm)?.ledger.acknowledge(
                        Recipient::Operator(authority.device_id),
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
            CREATE TABLE IF NOT EXISTS msg_held(position INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT UNIQUE NOT NULL,data BLOB NOT NULL,reason TEXT NOT NULL,state TEXT NOT NULL);")?;
        pin_issuer(&account, keys.as_ref())?;
        let registry = Registry::new(account.clone());
        let mut messaging = Self {
            root: root.into(),
            account,
            registry,
            keys,
            swarms: BTreeMap::new(),
            operator_device: None,
        };
        for record in messaging.registry.list()? {
            messaging.sync_seat(&record)?;
        }
        messaging.recover_notices()?;
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
            if let Some(device_id) = self.operator_device {
                tree.set_operator(device_id);
            }
            self.swarms.insert(
                id,
                Swarm {
                    tree,
                    ledger: Ledger::open(store, id)?
                        .with_test_context(self.keys.test_account_label().map(str::to_owned)),
                    seen: HashMap::new(),
                    resolved: HashMap::new(),
                    muted: HashMap::new(),
                },
            );
        }
        Ok(self.swarms.get_mut(&id).unwrap())
    }
    pub(crate) fn set_operator_device(&mut self, device_id: Ulid) -> Result<()> {
        if device_id == Ulid::nil() {
            return Err(invalid("operator device must be non-nil"));
        }
        if self.operator_device != Some(device_id) {
            self.operator_device = Some(device_id);
            for swarm in self.swarms.values_mut() {
                swarm.tree.set_operator(device_id);
                swarm.seen.clear();
                swarm.resolved.clear();
                swarm.muted.clear();
            }
        }
        Ok(())
    }
    pub(crate) fn operator_subjects(&self) -> Result<Vec<String>> {
        if self.operator_device.is_none() {
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
            issuer_kind: message
                .view()
                .issuer_kind
                .ok_or_else(|| invalid("pending message projection is not yet installed"))?,
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
        let message = self
            .swarm(record.swarm)?
            .ledger
            .accept(&request, &envelope, &routes, held, now)?;
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
        let body = body.unwrap_or_else(|| Body::Text {
            text: request.text.clone(),
        });
        let sender = Recipient::Operator(authority.device_id);
        if let Some(old) = self.swarm(swarm)?.ledger.existing(&request, sender)? {
            if old.view().body != &body {
                return Err(invalid("message ID body conflict"));
            }
            return Ok(super::api::Response::Stored {
                message: Box::new(old),
            });
        }
        if let Some(reply) = request.reply_to {
            let old = self.swarm(swarm)?.ledger.by_id(reply)?;
            if request
                .thread_id
                .is_some_and(|thread| thread != old.view().thread_id)
            {
                return Err(invalid("reply thread mismatch"));
            }
            request.thread_id = Some(old.view().thread_id);
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
        let counter = sender.to_string();
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
            timestamp: now / 1000,
            priority: request.priority,
            kind: request.kind,
            body,
            attachments: request.attachments.clone(),
            door: Some(authority.origin.door),
            issued_by: authority.origin.issued_by.clone(),
            signature: String::new(),
        };
        envelope.sign(&authority.key)?;
        self.validate(&envelope)?;
        tx.execute("INSERT INTO msg_counters VALUES(?1,0,?2) ON CONFLICT(seat,generation) DO UPDATE SET sequence=excluded.sequence",
            params![counter, sqlint(sequence)?])?;
        tx.commit()?;
        let message = self
            .swarm(swarm)?
            .ledger
            .accept_from(&request, &envelope, &routes, None, now, sender)
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
        // Decision #264: a reply without a destination goes to the sender of the message
        // it answers, the operator's inbox for the operator's.
        if request.destination.is_empty() {
            let reply = request.reply_to.ok_or_else(|| {
                invalid(
                    "message_send needs a destination, or reply_to to answer a message's sender",
                )
            })?;
            request.destination = sender_inbox(&self.swarm(record.swarm)?.ledger.by_id(reply)?);
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
        if let Some(reply) = request.reply_to {
            let old = self.swarm(record.swarm)?.ledger.by_id(reply)?;
            self.can_read(from, &old)?;
            if request
                .thread_id
                .is_some_and(|id| id != old.view().thread_id)
            {
                return Err(invalid("reply thread mismatch"));
            }
            request.thread_id = Some(old.view().thread_id);
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
        let pending = HeldSend {
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
                return Ok(super::api::Response::Held {
                    id: request.id,
                    reason,
                });
            }
            let queued: i64 = tx.query_row(
                "SELECT count(*) FROM msg_held WHERE state='held'",
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
                .or_else(|| (queued > 0).then(|| "earlier signing work held".into()));
            if let Some(reason) = reason {
                if queued >= 128 {
                    return Err(invalid("held signing queue is full"));
                }
                tx.execute(
                    "INSERT INTO msg_held(id,data,reason,state) VALUES(?1,?2,?3,'held')",
                    params![
                        request.id.to_string(),
                        serde_json::to_vec(&pending)?,
                        reason
                    ],
                )?;
                tx.commit()?;
                return Ok(super::api::Response::Held {
                    id: request.id,
                    reason,
                });
            }
        }
        let envelope = mint(
            &tx,
            self.keys.as_ref(),
            &record,
            &request,
            Body::Text {
                text: request.text.clone(),
            },
            Issue {
                epoch,
                now,
                operator_direct,
            },
        )?;
        tx.commit()?;
        let held = (record.muted && !operator_direct).then_some("sender muted");
        let message = self
            .swarm(record.swarm)?
            .ledger
            .accept(&request, &envelope, &routes, held, now)?;
        // Recipient mutes and paused subscriptions apply at once (held before the next pump).
        self.reconcile(now)?;
        Ok(super::api::Response::Stored {
            message: Box::new(message),
        })
    }
    /// Install a verified ready provider, then consume durable unsigned work in
    /// admission order. Changed authority is refused by name, never upgraded.
    pub fn resume_signing(&mut self, keys: Arc<dyn SeatKeys>, now: u64) -> Result<usize> {
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
        loop {
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
            let result = self.send_inner(
                queued.seat,
                queued.generation,
                queued.request.clone(),
                now,
                queued.operator,
                Some(&queued),
            );
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
                Recipient::Operator(device) if self.operator_device == Some(device) => {}
                _ => return Err(invalid("destination is outside this account/swarm scope")),
            }
        }
        Ok(())
    }
    fn validate_message(
        &self,
        message: &StoredMessage,
    ) -> Result<(Option<SeatRecord>, Class, bool)> {
        let envelope = message
            .envelope()
            .ok_or_else(|| invalid("pending message validation is not yet installed"))?;
        self.validate(envelope)
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
            || record.role.class == Class::Oracle
            || self
                .swarm(record.swarm)?
                .ledger
                .receipts(message.view().id)?
                .iter()
                .any(|r| r.recipient == seat)
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
            self.swarm(envelope.swarm_id)?.ledger.accept(
                &request,
                &envelope,
                &routes,
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
                        Ok(routes) => self
                            .swarm(swarm_id)?
                            .ledger
                            .release_out(message, &routes, now)?,
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
                                    let held = if muted && !*direct {
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
        let entry = if matches!(message.view().body, Body::Presence { .. }) {
            None
        } else {
            Some(
                (|| -> Result<_> {
                    let (sender, class, direct) = self.validate_message(&message)?;
                    if sender.as_ref().is_some_and(|sender| sender.muted) && !direct {
                        return Err(invalid("sender muted"));
                    }
                    let routes = self.swarm(swarm_id)?.tree.resolve(
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
                    )?;
                    // First route per (seat, agent), as a linear `find` picks.
                    let mut by = HashMap::new();
                    for r in routes {
                        by.entry((r.recipient, r.agent.clone())).or_insert(r);
                    }
                    Ok((class, direct, by))
                })()
                .map_err(|e| e.to_string()),
            )
        };
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
            Recipient::Operator(device) if self.operator_device == Some(device) => Ok(false),
            Recipient::Operator(_) => {
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
