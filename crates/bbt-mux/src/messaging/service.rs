//! The logic owner mediates keys, current grants, inboxes and the local bus.
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ulid::Ulid;

use super::{
    keys::SeatKeys,
    ledger::{Ledger, Page, Receipt, StoredMessage, Work, u64_at},
    tree::{Actor, Node, Right, Route, Tree},
    wire::{Body, Envelope, MessageKind, SendRequest, public},
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
struct Swarm {
    tree: Tree,
    ledger: Ledger,
    /// Last reconcile pass: pending rowid -> (stored-bytes hash, message, recipient),
    /// each message's resolution and each recipient's mute. A receipt whose bytes,
    /// message resolution and recipient are all unchanged would reconcile to itself.
    seen: HashMap<u64, (u128, u64, Ulid)>,
    resolved: HashMap<u64, Option<Resolved>>,
    muted: HashMap<Ulid, std::result::Result<bool, String>>,
}

/// A message's sender class, operator-direct flag and routes by (seat, agent), or the reason it cannot route.
type Resolved = std::result::Result<(Class, bool, HashMap<(Ulid, Option<String>), Route>), String>;

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
pub struct Messaging {
    root: PathBuf,
    account: SqliteStore,
    registry: Registry,
    keys: Arc<dyn SeatKeys>,
    swarms: BTreeMap<Ulid, Swarm>,
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
        use super::api::{Request, Response};
        let (swarm, actor) = self.actor(seat, generation)?;
        let actor = if operator { Actor::Operator } else { actor };
        match request {
            Request::Send { message } => Ok(Response::Stored {
                message: Box::new(self.send_inner(seat, generation, message, now, operator)?),
            }),
            Request::List {
                thread,
                channel,
                after,
                limit,
            } => Ok(Response::Messages {
                page: self.list(seat, thread, channel.as_deref(), after, limit)?,
            }),
            Request::Receipts {
                message,
                after,
                limit,
            } => {
                let m = self.swarm(swarm)?.ledger.by_id(message)?;
                self.can_read(seat, &m)?;
                let recipient = (!actor.controls() && m.envelope.seat != seat).then_some(seat);
                Ok(Response::Receipts {
                    page: self
                        .swarm(swarm)?
                        .ledger
                        .receipts_page(message, recipient, after, limit)?,
                })
            }
            Request::Acknowledge { message, agent } => {
                self.acknowledge(seat, message, agent.as_deref(), now)?;
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
                let mut node = Node::link(path, format!("seats/{seat}/inbox"));
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
            CREATE TABLE IF NOT EXISTS msg_notices(id TEXT PRIMARY KEY,envelope BLOB NOT NULL,request BLOB NOT NULL,sent INTEGER NOT NULL DEFAULT 0);")?;
        let issuer = serde_json::to_vec(&keys.issuer())?;
        if let Some(old) = account.get("messaging/issuer")? {
            if old != issuer {
                return Err(invalid(
                    "local messaging issuer changed; explicit re-enrollment required",
                ));
            }
        } else {
            account.put("messaging/issuer", &issuer)?;
        }
        let registry = Registry::new(account.clone());
        let mut messaging = Self {
            root: root.into(),
            account,
            registry,
            keys,
            swarms: BTreeMap::new(),
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
    fn swarm(&mut self, id: Ulid) -> Result<&mut Swarm> {
        if !self.swarms.contains_key(&id) {
            let store = SqliteStore::open(&object_dir(&self.root, &format!("swarm-{id}"))?)?;
            self.swarms.insert(
                id,
                Swarm {
                    tree: Tree::open(store.clone(), id)?,
                    ledger: Ledger::open(store, id)?,
                    seen: HashMap::new(),
                    resolved: HashMap::new(),
                    muted: HashMap::new(),
                },
            );
        }
        Ok(self.swarms.get_mut(&id).unwrap())
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
        self.send_inner(from, generation, request, now, false)
    }
    fn send_inner(
        &mut self,
        from: Ulid,
        generation: u64,
        mut request: SendRequest,
        now: u64,
        operator: bool,
    ) -> Result<StoredMessage> {
        request.validate()?;
        if matches!(request.kind, MessageKind::Presence | MessageKind::Task) {
            return Err(invalid(
                "typed presence/task messages require their producer API",
            ));
        }
        request.destination = super::tree::normalize(&request.destination, true)?;
        let record = self.record(from, generation)?;
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
            return Ok(old);
        }
        if let Some(reply) = request.reply_to {
            let old = self.swarm(record.swarm)?.ledger.by_id(reply)?;
            self.can_read(from, &old)?;
            if request
                .thread_id
                .is_some_and(|id| id != old.envelope.thread_id)
            {
                return Err(invalid("reply thread mismatch"));
            }
            request.thread_id = Some(old.envelope.thread_id);
        }
        let actor = if operator {
            Actor::Operator
        } else {
            Actor::Seat {
                seat: from,
                class: record.role.class,
            }
        };
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
        Ok(message)
    }
    fn check_recipients(&self, swarm: Ulid, routes: &[Route]) -> Result<()> {
        for route in routes {
            if self.registry.get(route.seat)?.swarm != swarm {
                return Err(invalid("destination seat is outside this swarm scope"));
            }
        }
        Ok(())
    }
    fn validate(&self, message: &Envelope) -> Result<(SeatRecord, Class, bool)> {
        self.keys.verify(message)?;
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
            record,
            class,
            grant.operator_direct && super::tree::sink(&message.context).is_ok(),
        ))
    }
    pub fn can_read(&mut self, seat: Ulid, message: &StoredMessage) -> Result<()> {
        let record = self.registry.get(seat)?;
        if record.swarm != message.envelope.swarm_id {
            return Err(invalid("message outside caller swarm"));
        }
        if seat == message.envelope.seat
            || record.role.class == Class::Oracle
            || self
                .swarm(record.swarm)?
                .ledger
                .receipts(message.envelope.id)?
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
            if stored.envelope.seat == seat || self.registry.get(seat)?.role.class == Class::Oracle
            {
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
        if !actor.controls() {
            return Err(invalid("mute requires operator or oracle class"));
        }
        let mut record = self.registry.get(seat)?;
        if let Some(controller) = actor.seat()
            && self.registry.get(controller)?.swarm != record.swarm
        {
            return Err(invalid("mute outside caller swarm"));
        }
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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
            state,
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
                        let (sender, class, direct) = self.validate(&message.envelope)?;
                        if sender.muted && !direct {
                            return Err(invalid("sender muted"));
                        }
                        self.swarm(swarm_id)?.tree.resolve(
                            if direct {
                                Actor::Operator
                            } else {
                                Actor::Seat {
                                    seat: sender.id,
                                    class: sender.role.class,
                                }
                            },
                            &message.envelope.context,
                            message.envelope.kind,
                            class,
                            message.envelope.priority,
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
            let mut targets: HashMap<Ulid, (std::result::Result<bool, String>, bool)> =
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
        let entry = if matches!(message.envelope.body, Body::Presence { .. }) {
            None
        } else {
            Some(
                (|| -> Result<_> {
                    let (sender, class, direct) = self.validate(&message.envelope)?;
                    if sender.muted && !direct {
                        return Err(invalid("sender muted"));
                    }
                    let routes = self.swarm(swarm_id)?.tree.resolve(
                        if direct {
                            Actor::Operator
                        } else {
                            Actor::Seat {
                                seat: sender.id,
                                class: sender.role.class,
                            }
                        },
                        &message.envelope.context,
                        message.envelope.kind,
                        class,
                        message.envelope.priority,
                    )?;
                    // First route per (seat, agent), as a linear `find` picks.
                    let mut by = HashMap::new();
                    for r in routes {
                        by.entry((r.seat, r.agent.clone())).or_insert(r);
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
        recipient: Ulid,
        memo: &mut HashMap<Ulid, (std::result::Result<bool, String>, bool)>,
    ) -> (std::result::Result<bool, String>, bool) {
        if let Some(hit) = memo.get(&recipient) {
            return hit.clone();
        }
        let muted = self
            .registry
            .get(recipient)
            .map(|t| t.muted)
            .map_err(|e| e.to_string());
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
        for (&swarm, s) in &self.swarms {
            for draft in s.ledger.bus_outbox()? {
                events.push((format!("swarm-{swarm}"), draft));
            }
        }
        Ok(events)
    }
    pub fn event_sent(&self, stream: &str, id: Ulid) -> Result<()> {
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
        signature: String::new(),
    };
    keys.finalize(&mut envelope, record, tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO msg_grants VALUES(?1,?2)",
        params![envelope.role_grant_id, raw_grant],
    )?;
    envelope.sign(&leaf)?;
    tx.execute("INSERT INTO msg_counters VALUES(?1,?2,?3) ON CONFLICT(seat,generation) DO UPDATE SET sequence=excluded.sequence",params![record.id.to_string(),sqlint(record.generation)?,sqlint(sequence)?])?;
    Ok(envelope)
}
