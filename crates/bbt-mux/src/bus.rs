//! One bounded router on a caller-supplied runtime. Blocking SQLite work belongs
//! to its blocking task, never to the UI or an async reactor worker.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use ulid::Ulid;

use crate::store::{
    Events, Result, Store,
    events::{Draft, Envelope, EventLog},
    invalid, now_ms, object_dir, sqlint,
    sqlite::{self, SqliteStore},
};

pub fn validate_subject(subject: &str, pattern: bool) -> Result<()> {
    let tokens: Vec<_> = subject.split('/').collect();
    if subject.is_empty() || subject.len() > 256 || tokens.len() > 16 {
        return Err(invalid("invalid subject length"));
    }
    for (i, token) in tokens.iter().enumerate() {
        if token.is_empty() || token.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(invalid("invalid subject token"));
        }
        if token.contains(['*', '>'])
            && !(pattern && (*token == "*" || (*token == ">" && i + 1 == tokens.len())))
        {
            return Err(invalid("invalid subject wildcard"));
        }
    }
    Ok(())
}

pub fn matches(pattern: &str, subject: &str) -> bool {
    let mut subjects = subject.split('/');
    for token in pattern.split('/') {
        if token == ">" {
            return subjects.next().is_some();
        }
        let Some(part) = subjects.next() else {
            return false;
        };
        if token != "*" && token != part {
            return false;
        }
    }
    subjects.next().is_none()
}

fn fanout(subject: &str) -> bool {
    subject.starts_with("inbox/") || {
        let t: Vec<_> = subject.split('/').collect();
        t.len() == 3 && t[0] == "seat" && t[2] == "output"
    }
}
fn coalesces(subject: &str) -> bool {
    matches!(
        subject.rsplit('/').next(),
        Some("state" | "modified" | "title" | "cwd" | "presence")
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Start {
    New,
    All,
    LastPerSubject,
    Sequence(u64),
    Time(u64),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ack {
    None,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consumer {
    pub name: Option<String>,
    pub filter: Vec<String>,
    pub start: Start,
    pub ack: Ack,
    pub deliver: String,
    pub queue_max: usize,
    pub ack_wait_ms: u64,
    pub max_deliver: u32,
}

impl Consumer {
    pub fn gui(filter: impl Into<String>) -> Self {
        Self {
            name: None,
            filter: vec![filter.into()],
            start: Start::New,
            ack: Ack::None,
            deliver: "gui".into(),
            queue_max: 128,
            ack_wait_ms: 30_000,
            max_deliver: 3,
        }
    }
    fn accepts(&self, subject: &str) -> bool {
        self.filter.iter().any(|p| matches(p, subject))
    }
    fn validate(&self) -> Result<()> {
        if self.filter.is_empty()
            || self.filter.len() > 64
            || !(1..=4096).contains(&self.queue_max)
            || self.ack_wait_ms == 0
            || self.max_deliver == 0
            || self.max_deliver > 100
        {
            return Err(invalid("invalid consumer bounds"));
        }
        for pattern in &self.filter {
            validate_subject(pattern, true)?;
        }
        if self
            .name
            .as_ref()
            .is_some_and(|n| n.is_empty() || n.len() > 128)
        {
            return Err(invalid("invalid consumer name"));
        }
        if self.ack == Ack::Explicit && self.name.is_none() {
            return Err(invalid("explicit ack needs a durable consumer name"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Delivery {
    Event {
        stream: String,
        envelope: Box<Envelope>,
        deliveries: u32,
    },
    Gap {
        stream: String,
        first: u64,
        last: u64,
        reason: String,
    },
}

struct Subscriber {
    config: Consumer,
    queue: VecDeque<Envelope>,
    gap: Option<(u64, u64)>,
    cursor: u64,
    gui: Option<GuiPending>,
}

// GUI replay is read through in finite quanta. No full envelopes are retained
// in this attachment state; the signed log and pending table own the history.
struct GuiPending {
    policy: GuiPolicy,
    after: u64,
    replay_through: u64,
    next_due: u64,
    sweep_due: Option<u64>,
}

// GUI-only metadata in the existing consumers.filter column. Each subject's
// activation head survives policy versions; the Consumer policy stays intact.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuiPolicy {
    starts: BTreeMap<String, u64>,
    inherited_from: Option<String>,
}
impl GuiPolicy {
    fn new(config: &Consumer, head: u64) -> Self {
        Self {
            starts: config
                .filter
                .iter()
                .map(|filter| (filter.clone(), head))
                .collect(),
            inherited_from: None,
        }
    }
    fn accepts(&self, subject: &str, sequence: u64) -> bool {
        self.starts
            .iter()
            .any(|(filter, start)| sequence > *start && matches(filter, subject))
    }
}

fn gui_consumer(config: &Consumer) -> bool {
    config.deliver == "gui"
        && config.ack == Ack::Explicit
        && config
            .name
            .as_deref()
            .is_some_and(|name| name.starts_with("gui-"))
}

fn gui_uint(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    u64::try_from(row.get::<_, i64>(index)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

struct GuiUndeliverable {
    stream: String,
    sequence: u64,
    ts: u64,
}

struct GuiStreamPoll {
    deliveries: Vec<Delivery>,
    bytes: usize,
    undeliverable: Vec<GuiUndeliverable>,
}

/// One active drain of the device's durable consumer. The token is in-process
/// attachment state, never a second identity or a caller-supplied principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuiLease {
    pub consumer: String,
    pub device: Ulid,
    token: Ulid,
}

struct GuiAttachment {
    lease: GuiLease,
    last_stream: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct GuiCursor {
    pub stream: String,
    pub admitted: u64,
    pub published: u64,
    pub read: u64,
    pub high: u64,
    pub unread: u64,
}

pub(crate) struct GuiPage {
    pub records: Vec<Delivery>,
    pub next: Option<u64>,
    pub cursor: GuiCursor,
}

/// Detached browse result: filtered events, next page, last scanned sequence.
pub(crate) type RetainedPage = (Vec<Envelope>, Option<u64>, u64);

struct Stream {
    name: String,
    log: EventLog,
    consumers: BTreeMap<String, Subscriber>,
}

impl Stream {
    fn gui_notify(&mut self) {
        for sub in self.consumers.values_mut() {
            if let Some(gui) = &mut sub.gui {
                gui.next_due = 0;
            }
        }
    }
    fn subscribe(&mut self, id: &str, config: &Consumer) -> Result<()> {
        let db = sqlite::open(&self.log.path)?;
        let old: Option<(i64, String)> = db
            .query_row(
                "SELECT cursor,policy FROM consumers WHERE name=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let cursor = if let Some((cursor, policy)) = old {
            let mut previous: Consumer = serde_json::from_str(&policy)?;
            previous.start = config.start.clone();
            if &previous != config {
                return Err(invalid("durable consumer configuration changed"));
            }
            if config.ack == Ack::None && config.start == Start::LastPerSubject {
                0
            } else {
                cursor.try_into()?
            }
        } else {
            match config.start {
                Start::New => self.log.last_sequence()?,
                Start::Sequence(n) => n.saturating_sub(1),
                _ => 0,
            }
        };
        if config.name.is_some() {
            db.execute(
                "INSERT OR IGNORE INTO consumers VALUES(?1,?2,?3,?3,?4,?5)",
                params![
                    id,
                    serde_json::to_string(&config.filter)?,
                    sqlint(cursor)?,
                    config.deliver,
                    serde_json::to_string(config)?
                ],
            )?;
        }
        let policy = if gui_consumer(config) {
            let stored: String =
                db.query_row("SELECT filter FROM consumers WHERE name=?1", [id], |row| {
                    row.get(0)
                })?;
            // Legacy GUI rows contain the plain filter array. Initialize only
            // that known representation; malformed policy metadata is refused.
            let policy = if stored.starts_with('[') {
                let filters: Vec<String> = serde_json::from_str(&stored)?;
                if filters != config.filter {
                    return Err(invalid("GUI filter metadata disagrees"));
                }
                GuiPolicy::new(config, cursor)
            } else {
                serde_json::from_str(&stored)?
            };
            db.execute(
                "UPDATE consumers SET filter=?2 WHERE name=?1",
                params![id, serde_json::to_string(&policy)?],
            )?;
            Some(policy)
        } else {
            None
        };
        self.consumers.insert(
            id.into(),
            Subscriber {
                config: config.clone(),
                queue: VecDeque::new(),
                gap: None,
                cursor,
                gui: gui_consumer(config).then_some(GuiPending {
                    policy: policy.unwrap_or_else(|| GuiPolicy::new(config, 0)),
                    after: 0,
                    replay_through: cursor,
                    next_due: 0,
                    sweep_due: None,
                }),
            },
        );
        if gui_consumer(config) {
            // Start::New seeds a new device at the retained head. A restored
            // device keeps its cursor and pending rows; bounded poll repairs
            // a crash between an append and pending admission.
            let pending: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM pending WHERE consumer=?1)",
                [id],
                |row| row.get(0),
            )?;
            if !pending && cursor == self.log.last_sequence()? {
                self.consumers
                    .get_mut(id)
                    .unwrap()
                    .gui
                    .as_mut()
                    .unwrap()
                    .next_due = u64::MAX;
            }
            return Ok(());
        }
        let mut latest = BTreeMap::new();
        let mut after = cursor;
        loop {
            let batch = self.log.after(after, 256)?;
            if batch.is_empty() {
                break;
            }
            for event in batch {
                after = event.seq;
                if matches!(config.start,Start::Time(t) if event.event.ts<t) {
                    continue;
                }
                if config.accepts(&event.event.subject) {
                    if matches!(config.start, Start::LastPerSubject) && config.ack == Ack::None {
                        latest.insert(event.event.subject.clone(), event);
                    } else {
                        self.enqueue(id, event)?;
                    }
                }
            }
        }
        let mut latest: Vec<_> = latest.into_values().collect();
        latest.sort_by_key(|e| e.seq);
        for event in latest {
            self.enqueue(id, event)?;
        }
        if config.ack == Ack::Explicit {
            self.set_cursor(id, after)?;
        }
        Ok(())
    }
    fn set_cursor(&mut self, id: &str, seq: u64) -> Result<()> {
        self.consumers.get_mut(id).unwrap().cursor = seq;
        sqlite::open(&self.log.path)?.execute(
            "UPDATE consumers SET cursor=?2 WHERE name=?1",
            params![id, sqlint(seq)?],
        )?;
        Ok(())
    }
    fn enqueue(&mut self, id: &str, event: Envelope) -> Result<()> {
        let subscriber = self.consumers.get_mut(id).unwrap();
        if let Some(gui) = &mut subscriber.gui {
            if event.seq != 0 {
                gui.next_due = 0;
            }
            // The retained row is the queue. Admission advances contiguously
            // on the next bounded GUI poll, not ahead of a replay hole.
            return Ok(());
        }
        if subscriber.config.ack == Ack::Explicit && event.seq != 0 {
            sqlite::open(&self.log.path)?.execute(
                "INSERT OR IGNORE INTO pending VALUES(?1,?2,0,0)",
                params![id, sqlint(event.seq)?],
            )?;
            return Ok(());
        }
        if coalesces(&event.event.subject)
            && let Some(i) = subscriber
                .queue
                .iter()
                .position(|old| old.event.subject == event.event.subject)
        {
            subscriber.queue.remove(i);
        }
        if subscriber.queue.len() == subscriber.config.queue_max
            && let Some(old) = subscriber.queue.pop_front()
        {
            subscriber.gap = Some(subscriber.gap.map_or((old.seq, old.seq), |(first, last)| {
                (first.min(old.seq), last.max(old.seq))
            }));
        }
        subscriber.queue.push_back(event);
        Ok(())
    }
    fn publish(&mut self, event: Envelope) -> Result<usize> {
        let ids: Vec<_> = self
            .consumers
            .iter()
            .filter(|(_, s)| s.config.accepts(&event.event.subject))
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            self.enqueue(id, event.clone())?;
            if event.seq > 0
                && self.consumers[id].config.ack == Ack::Explicit
                && self.consumers[id].gui.is_none()
            {
                self.set_cursor(id, event.seq)?;
            }
        }
        Ok(ids.len())
    }
    fn poll(&mut self, id: &str, limit: usize, now: u64) -> Result<Vec<Delivery>> {
        let Some(sub) = self.consumers.get_mut(id) else {
            return Err(invalid("unknown consumer"));
        };
        let limit = limit.min(sub.config.queue_max).min(1024);
        let mut result = Vec::new();
        if let Some((first, last)) = sub.gap.take() {
            result.push(Delivery::Gap {
                stream: self.name.clone(),
                first,
                last,
                reason: if first == 0 {
                    "fanout not retained"
                } else {
                    "subscriber queue overflow; replay from retained stream"
                }
                .into(),
            });
        }
        while result.len() < limit {
            let Some(envelope) = sub.queue.pop_front() else {
                break;
            };
            if envelope.seq > 0 && sub.config.ack == Ack::None {
                sqlite::open(&self.log.path)?.execute(
                    "UPDATE consumers SET cursor=?2 WHERE name=?1",
                    params![id, sqlint(envelope.seq)?],
                )?;
            }
            result.push(Delivery::Event {
                stream: self.name.clone(),
                envelope: Box::new(envelope),
                deliveries: 1,
            });
        }
        if sub.config.ack == Ack::Explicit && result.len() < limit {
            let db = sqlite::open(&self.log.path)?;
            let mut stmt=db.prepare("SELECT seq,deliveries FROM pending WHERE consumer=?1 AND (deliveries=0 OR delivered_at<=?2) AND deliveries<?3 ORDER BY seq LIMIT ?4")?;
            let due = stmt
                .query_map(
                    params![
                        id,
                        sqlint(now.saturating_sub(sub.config.ack_wait_ms))?,
                        sub.config.max_deliver,
                        (limit - result.len()) as i64
                    ],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, u32>(1)?)),
                )?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (seq, count) in due {
                let envelope = self
                    .log
                    .at(seq.try_into()?)?
                    .ok_or_else(|| invalid("pending event missing"))?;
                db.execute("UPDATE pending SET delivered_at=?3,deliveries=deliveries+1 WHERE consumer=?1 AND seq=?2",params![id,seq,sqlint(now)?])?;
                result.push(Delivery::Event {
                    stream: self.name.clone(),
                    envelope: Box::new(envelope),
                    deliveries: count + 1,
                });
            }
        }
        Ok(result)
    }
    fn gui_migrate(&mut self, old: &str, new: &str) -> Result<u64> {
        let previous = self.consumers[old].gui.as_ref().unwrap().policy.clone();
        let mut policy = self.consumers[new].gui.as_ref().unwrap().policy.clone();
        let config = self.consumers[new].config.clone();
        let mut db = sqlite::open(&self.log.path)?;
        let mut dropped = 0;
        {
            let mut statement=db.prepare("SELECT e.subject,COUNT(*) FROM pending p JOIN events e ON e.seq=p.seq WHERE p.consumer=?1 GROUP BY e.subject")?;
            let mut rows = statement.query([old])?;
            while let Some(row) = rows.next()? {
                if !config.accepts(&row.get::<_, String>(0)?) {
                    dropped += gui_uint(row, 1)?;
                }
            }
        }
        if policy.inherited_from.as_deref() == Some(old) {
            return Ok(dropped);
        }
        for (filter, start) in &mut policy.starts {
            if let Some(previous) = previous.starts.get(filter) {
                *start = *previous;
            }
        }
        policy.inherited_from = Some(old.into());
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (cursor, acked): (u64, u64) = tx.query_row(
            "SELECT cursor,acked FROM consumers WHERE name=?1",
            [old],
            |row| Ok((gui_uint(row, 0)?, gui_uint(row, 1)?)),
        )?;
        tx.execute("INSERT OR IGNORE INTO pending SELECT ?2,seq,delivered_at,deliveries FROM pending WHERE consumer=?1",params![old,new])?;
        tx.execute(
            "UPDATE consumers SET cursor=?2,acked=?3,filter=?4 WHERE name=?1",
            params![
                new,
                sqlint(cursor)?,
                sqlint(acked)?,
                serde_json::to_string(&policy)?
            ],
        )?;
        tx.commit()?;
        let subscriber = self.consumers.get_mut(new).unwrap();
        subscriber.cursor = cursor;
        subscriber.gui.as_mut().unwrap().policy = policy;
        Ok(dropped)
    }

    fn gui_poll(
        &mut self,
        id: &str,
        limit: usize,
        byte_limit: usize,
        whole_quantum: usize,
        now: u64,
    ) -> Result<GuiStreamPoll> {
        let sub = self
            .consumers
            .get(id)
            .ok_or_else(|| invalid("unknown GUI consumer"))?;
        if sub.gui.is_none() {
            return Err(invalid("consumer is not a durable GUI consumer"));
        }
        let cursor = sub.cursor;
        let config = sub.config.clone();
        let policy = sub.gui.as_ref().unwrap().policy.clone();
        let limit = limit.min(128).min(config.queue_max);
        if limit == 0 || byte_limit == 0 {
            return Err(invalid("empty GUI poll budget"));
        }
        let mut db = sqlite::open(&self.log.path)?;
        // Only small routing metadata is read here. The cursor and pending
        // admissions commit together, so a crash cannot advance past a hole.
        let rows = {
            let mut statement =
                db.prepare("SELECT seq,subject FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2")?;
            statement
                .query_map(params![sqlint(cursor)?, limit as i64], |row| {
                    Ok((gui_uint(row, 0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        if let Some((last, _)) = rows.last() {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            for (sequence, subject) in &rows {
                if policy.accepts(subject, *sequence) {
                    tx.execute(
                        "INSERT OR IGNORE INTO pending VALUES(?1,?2,0,0)",
                        params![id, sqlint(*sequence)?],
                    )?;
                }
            }
            tx.execute(
                "UPDATE consumers SET cursor=?2 WHERE name=?1",
                params![id, sqlint(*last)?],
            )?;
            tx.commit()?;
            self.consumers.get_mut(id).unwrap().cursor = *last;
        }
        let state = self.consumers[id].gui.as_ref().unwrap();
        let after = state.after;
        let replay_through = state.replay_through;
        let pending = {
            let mut statement = db.prepare(
                "SELECT p.seq,p.delivered_at,p.deliveries,length(e.payload),e.hash,e.ts,e.subject \
                 FROM pending p JOIN events e ON e.seq=p.seq \
                 WHERE p.consumer=?1 AND p.seq>?2 ORDER BY p.seq LIMIT ?3",
            )?;
            statement
                .query_map(params![id, sqlint(after)?, limit as i64], |row| {
                    Ok((
                        gui_uint(row, 0)?,
                        gui_uint(row, 1)?,
                        row.get::<_, u32>(2)?,
                        gui_uint(row, 3)?,
                        row.get::<_, String>(4)?,
                        gui_uint(row, 5)?,
                        row.get::<_, String>(6)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let mut result = GuiStreamPoll {
            deliveries: vec![],
            bytes: 0,
            undeliverable: vec![],
        };
        let mut scanned = after;
        let mut future_due = None;
        let mut complete = pending.len() < limit;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for (sequence, delivered_at, deliveries, length, expected_hash, ts, subject) in &pending {
            if !policy.accepts(subject, *sequence) {
                tx.execute(
                    "DELETE FROM pending WHERE consumer=?1 AND seq=?2",
                    params![id, sqlint(*sequence)?],
                )?;
                scanned = *sequence;
                continue;
            }
            let length = usize::try_from(*length)?;
            let due = delivered_at.saturating_add(config.ack_wait_ms);
            if *deliveries != 0 && *sequence > replay_through && due > now {
                future_due = Some(future_due.map_or(due, |old: u64| old.min(due)));
                scanned = *sequence;
                continue;
            }
            // Oversize is relative to a fresh quantum, never the budget left
            // by a previous stream. A row that merely does not fit waits.
            let cost = if length > whole_quantum { 128 } else { length };
            if result.bytes + cost > byte_limit {
                complete = false;
                break;
            }
            if *deliveries >= config.max_deliver {
                result.undeliverable.push(GuiUndeliverable {
                    stream: self.name.clone(),
                    sequence: *sequence,
                    ts: *ts,
                });
            }
            let attempts = deliveries.saturating_add(1).min(config.max_deliver);
            if length > whole_quantum {
                result.deliveries.push(Delivery::Gap {
                    stream: self.name.clone(),
                    first: *sequence,
                    last: *sequence,
                    reason: "event_exceeds_gui_page_byte_limit; original retained".into(),
                });
                result.bytes += 128;
            } else {
                let bytes: Vec<u8> = tx.query_row(
                    "SELECT payload FROM events WHERE seq=?1",
                    [sqlint(*sequence)?],
                    |row| row.get(0),
                )?;
                let envelope: Envelope = serde_json::from_slice(&bytes)?;
                envelope.verify()?;
                if envelope.seq != *sequence
                    || envelope.hash()? != *expected_hash
                    || !config.accepts(&envelope.event.subject)
                {
                    return Err(invalid(
                        "retained GUI event hash, sequence or subject mismatch",
                    ));
                }
                result.bytes += bytes.len();
                result.deliveries.push(Delivery::Event {
                    stream: self.name.clone(),
                    envelope: Box::new(envelope),
                    deliveries: attempts,
                });
            }
            tx.execute(
                "UPDATE pending SET delivered_at=?3,deliveries=?4 WHERE consumer=?1 AND seq=?2",
                params![id, sqlint(*sequence)?, sqlint(now)?, attempts],
            )?;
            scanned = *sequence;
            let next = now.saturating_add(config.ack_wait_ms);
            future_due = Some(future_due.map_or(next, |old: u64| old.min(next)));
        }
        tx.commit()?;
        let sub = self.consumers.get_mut(id).unwrap();
        let gui = sub.gui.as_mut().unwrap();
        gui.sweep_due = match (gui.sweep_due, future_due) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        // A full metadata quantum can leave admission work even if no
        // selected subject was present; never scan an unbounded tail here.
        if complete {
            gui.after = 0;
            gui.replay_through = 0;
            gui.next_due = if rows.len() == limit {
                0
            } else {
                gui.sweep_due.take().unwrap_or(u64::MAX)
            };
        } else {
            gui.after = scanned;
            gui.next_due = 0;
        }
        Ok(result)
    }
    fn ack(&mut self, id: &str, sequence: u64) -> Result<()> {
        let mut db = sqlite::open(&self.log.path)?;
        let tx = db.transaction()?;
        if tx.execute(
            "DELETE FROM pending WHERE consumer=?1 AND seq=?2 AND deliveries>0",
            params![id, sqlint(sequence)?],
        )? != 1
        {
            return Err(invalid("ack is not a delivered pending event"));
        }
        let cursor: i64 =
            tx.query_row("SELECT cursor FROM consumers WHERE name=?1", [id], |r| {
                r.get(0)
            })?;
        let first: Option<i64> = tx.query_row(
            "SELECT MIN(seq) FROM pending WHERE consumer=?1",
            [id],
            |r| r.get(0),
        )?;
        tx.execute(
            "UPDATE consumers SET acked=?2 WHERE name=?1",
            params![id, first.map_or(cursor, |n| n - 1)],
        )?;
        tx.commit()?;
        Ok(())
    }
}

/// Exact retained-stream endpoints covered by one ephemeral run key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldRange {
    pub stream: String,
    pub first: u64,
    pub first_hash: String,
    pub last: u64,
    pub last_hash: String,
}
#[derive(Clone)]
struct HoldSigning {
    run: Ulid,
    ranges: BTreeMap<String, HoldRange>,
}
/// All streams share this router; each object retains its own sequence and DB.
pub struct Broker {
    root: PathBuf,
    key: SigningKey,
    hold: Option<HoldSigning>,
    custody_signers: bool,
    issued_by: Option<String>,
    catalog: SqliteStore,
    streams: BTreeMap<String, Stream>,
    consumers: BTreeMap<String, Consumer>,
    gui_attachments: BTreeMap<Ulid, GuiAttachment>,
}

impl Broker {
    /// Startup supplies a label derived from its verified account root/context.
    pub(crate) fn set_issued_by(&mut self, issued_by: Option<String>) {
        self.issued_by = issued_by;
    }
    pub fn open(root: &Path, key: SigningKey) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(root)?;
        let catalog = SqliteStore::open(&object_dir(root, "bus")?)?;
        let mut broker = Self {
            root: root.into(),
            key,
            hold: None,
            custody_signers: false,
            issued_by: None,
            catalog,
            streams: BTreeMap::new(),
            consumers: BTreeMap::new(),
            gui_attachments: BTreeMap::new(),
        };
        for (name, bytes) in broker.catalog.list("consumer/")? {
            broker
                .consumers
                .insert(name[9..].into(), serde_json::from_slice(&bytes)?);
        }
        let streams: Vec<_> = std::fs::read_dir(root)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().join("events.sqlite").is_file())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        for stream in streams {
            broker.ensure_stream(&stream)?;
        }
        Ok(broker)
    }
    fn ensure_stream(&mut self, name: &str) -> Result<()> {
        if self.streams.contains_key(name) {
            return Ok(());
        }
        let log = EventLog::open(&object_dir(&self.root, name)?)?;
        let mut stream = Stream {
            name: name.into(),
            log,
            consumers: BTreeMap::new(),
        };
        for (id, consumer) in &self.consumers {
            stream.subscribe(id, consumer)?;
        }
        self.streams.insert(name.into(), stream);
        Ok(())
    }
    pub fn subscribe(&mut self, consumer: Consumer) -> Result<String> {
        consumer.validate()?;
        let id = consumer
            .name
            .clone()
            .unwrap_or_else(|| ulid::Ulid::new().to_string());
        if let Some(old) = self.consumers.get(&id) {
            if old == &consumer {
                return Ok(id);
            }
            return Err(invalid("consumer already exists with different policy"));
        }
        for stream in self.streams.values_mut() {
            stream.subscribe(&id, &consumer)?;
        }
        if consumer.name.is_some() {
            self.catalog
                .put(&format!("consumer/{id}"), &serde_json::to_vec(&consumer)?)?;
        }
        self.consumers.insert(id.clone(), consumer);
        Ok(id)
    }
    /// The single bus owner seals the old range before installing the new key.
    /// No event can fall between range sealing and the key transition.
    pub fn identity_signer(&mut self, key: SigningKey, hold: Option<Ulid>) -> Vec<HoldRange> {
        let ranges = self
            .hold
            .take()
            .map(|h| h.ranges.into_values().collect())
            .unwrap_or_default();
        self.key = key;
        self.custody_signers = true;
        self.hold = hold.map(|run| HoldSigning {
            run,
            ranges: BTreeMap::new(),
        });
        ranges
    }
    pub fn hold_ranges(&self) -> Vec<HoldRange> {
        self.hold
            .as_ref()
            .map(|h| h.ranges.values().cloned().collect())
            .unwrap_or_default()
    }
    pub fn publish(&mut self, stream: &str, event: Draft) -> Result<Envelope> {
        self.publish_using(stream, event, None)
    }
    fn publish_using(
        &mut self,
        stream: &str,
        mut event: Draft,
        signer: Option<SigningKey>,
    ) -> Result<Envelope> {
        if event.issued_by.is_some() && event.issued_by != self.issued_by {
            return Err(invalid("test_account_label_mismatch: event attribution"));
        }
        event.issued_by = self.issued_by.clone();
        let supplied = signer.is_some();
        let key = signer.unwrap_or_else(|| self.key.clone());
        validate_subject(&event.subject, false)?;
        if let Some(reply) = &event.reply_to {
            validate_subject(reply, false)?;
            if !reply.starts_with("inbox/") {
                return Err(invalid("reply must address an inbox"));
            }
        }
        self.ensure_stream(stream)?;
        let previous = self.streams[stream].log.last_sequence()?;
        if (self.custody_signers || supplied) && !fanout(&event.subject) {
            // An outbox may replay across a hold/rekey or a crashed process.
            // Retain the original signed bytes after checking the full draft;
            // a different signer must not rewrite an already-journaled event.
            let old: Option<Vec<u8>> = sqlite::open(&self.streams[stream].log.path)?
                .query_row(
                    "SELECT payload FROM events WHERE id=?1",
                    [event.id.to_string()],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(bytes) = old {
                let old: Envelope = serde_json::from_slice(&bytes)?;
                old.verify()?;
                if supplied && old.provenance.signer != key.verifying_key().to_bytes() {
                    return Err(invalid("activity signer conflicts with retained event"));
                }
                let mut draft = old.event.clone();
                if let Some(marked) = draft.source.strip_prefix("ephemeral-hold/")
                    && let Some((run, source)) = marked.split_once('/')
                    && run.parse::<Ulid>().is_ok()
                {
                    draft.source = source.into();
                }
                if draft != event {
                    return Err(invalid("event id conflicts with retained event"));
                }
                self.streams.get_mut(stream).unwrap().gui_notify();
                return Ok(old);
            }
        }
        if !supplied && let Some(hold) = &self.hold {
            event.source = format!("ephemeral-hold/{}/{}", hold.run, event.source);
        }
        let envelope = if fanout(&event.subject) {
            Envelope::signed(event, 0, String::new(), &key)?
        } else {
            self.streams[stream].log.append(event, &key)?
        };
        if envelope.seq != 0 && envelope.seq <= previous {
            self.streams.get_mut(stream).unwrap().gui_notify();
            return Ok(envelope);
        }
        if envelope.seq != 0
            && !supplied
            && let Some(hold) = &mut self.hold
        {
            let hash = envelope.hash()?;
            let range = hold
                .ranges
                .entry(stream.into())
                .or_insert_with(|| HoldRange {
                    stream: stream.into(),
                    first: envelope.seq,
                    first_hash: hash.clone(),
                    last: envelope.seq,
                    last_hash: hash.clone(),
                });
            range.last = envelope.seq;
            range.last_hash = hash;
        }
        let responders = self
            .streams
            .get_mut(stream)
            .unwrap()
            .publish(envelope.clone())?;
        if responders == 0
            && let Some(reply) = &envelope.event.reply_to
        {
            let response = Envelope::signed(
                Draft::new(
                    reply,
                    self.hold
                        .as_ref()
                        .map(|h| format!("ephemeral-hold/{}/bus", h.run))
                        .unwrap_or_else(|| "bus".into()),
                    serde_json::json!({"status":"no_responders","request":envelope.event.id}),
                ),
                0,
                String::new(),
                &self.key,
            )?;
            self.streams.get_mut(stream).unwrap().publish(response)?;
        }
        Ok(envelope)
    }
    fn gui_attach(&mut self, device: Ulid, mut filter: Vec<String>) -> Result<GuiLease> {
        if self.gui_attachments.contains_key(&device) {
            return Err(invalid(
                "GuiConsumerAttached: device already has an active drain",
            ));
        }
        filter.sort();
        filter.dedup();
        let base = format!("gui-{device}");
        let mut versions: Vec<_> = self
            .consumers
            .keys()
            .filter_map(|name| {
                if name == &base {
                    Some((0_u32, name.clone()))
                } else {
                    name.strip_prefix(&format!("{base}-v"))?
                        .parse::<u32>()
                        .ok()
                        .map(|n| (n, name.clone()))
                }
            })
            .collect();
        let previous = versions.iter().max_by_key(|(version, _)| version);
        let name = match previous {
            Some((_, name)) if self.consumers[name].filter == filter => name.clone(),
            Some((version, _)) => format!(
                "{base}-v{}",
                version
                    .checked_add(1)
                    .ok_or_else(|| invalid("GUI consumer version exhausted"))?
            ),
            None => base,
        };
        let mut config = Consumer::gui("seat/*/state");
        config.name = Some(name.clone());
        config.filter = filter;
        config.start = Start::New;
        config.ack = Ack::Explicit;
        self.subscribe(config)?;
        if !versions.iter().any(|(_, old)| old == &name) {
            versions.push((previous.map_or(0, |(version, _)| version + 1), name.clone()));
        }
        versions.sort_by_key(|(version, _)| *version);
        // A per-stream atomic migration can be resumed after a crash between
        // objects. Finish incomplete older versions before moving forward.
        for pair in versions.windows(2) {
            let old = &pair[0].1;
            let new = &pair[1].1;
            let mut dropped = 0_u64;
            for stream in self.streams.values_mut() {
                dropped = dropped.saturating_add(stream.gui_migrate(old, new)?);
            }
            let audit_key = format!("gui-migration/{new}");
            let audit = if let Some(bytes) = self.catalog.get(&audit_key)? {
                serde_json::from_slice(&bytes)?
            } else {
                let previous = &self.consumers[old].filter;
                let next = &self.consumers[new].filter;
                let mut audit = Draft::new(
                    "account/control",
                    "gui/consumer",
                    serde_json::json!({
                        "activity":"gui-filter-migration","old":old,"new":new,
                        "added":next.iter().filter(|f|!previous.contains(f)).collect::<Vec<_>>(),
                        "removed":previous.iter().filter(|f|!next.contains(f)).collect::<Vec<_>>(),
                        "pending_dropped":dropped.to_string(),
                    }),
                );
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(audit_key.as_bytes());
                audit.id = Ulid::from_bytes(digest[..16].try_into().unwrap());
                // This is the migration's own resumable metadata, in the
                // existing consumer catalogue, not another events ledger.
                self.catalog.put(&audit_key, &serde_json::to_vec(&audit)?)?;
                audit
            };
            self.publish("account", audit)?;
        }
        // New policy is live before deleting an old version's consumer state.
        // Signed history and operator read-marks are never deleted here.
        for (_, old) in &versions {
            if old != &name {
                self.unsubscribe(old)?;
            }
        }
        for (_, version) in &versions {
            self.catalog.sql(
                "DELETE FROM kv WHERE key=?1",
                &[format!("gui-migration/{version}").into()],
            )?;
        }
        for stream in self.streams.values_mut() {
            let subscriber = stream.consumers.get_mut(&name).unwrap();
            let pending: bool = sqlite::open(&stream.log.path)?.query_row(
                "SELECT EXISTS(SELECT 1 FROM pending WHERE consumer=?1)",
                [&name],
                |row| row.get(0),
            )?;
            let gui = subscriber.gui.as_mut().unwrap();
            gui.after = 0;
            gui.replay_through = subscriber.cursor;
            gui.sweep_due = None;
            gui.next_due = if pending || subscriber.cursor < stream.log.last_sequence()? {
                0
            } else {
                u64::MAX
            };
        }
        let lease = GuiLease {
            consumer: name,
            device,
            token: Ulid::new(),
        };
        self.gui_attachments.insert(
            device,
            GuiAttachment {
                lease: lease.clone(),
                last_stream: None,
            },
        );
        Ok(lease)
    }

    fn gui_attachment(&self, lease: &GuiLease) -> Result<&GuiAttachment> {
        self.gui_attachments
            .get(&lease.device)
            .filter(|active| active.lease == *lease)
            .ok_or_else(|| invalid("GuiConsumerDetached: stale GUI attachment"))
    }

    fn gui_refresh(&mut self, lease: &GuiLease, mut filters: Vec<String>) -> Result<GuiLease> {
        self.gui_attachment(lease)?;
        filters.sort();
        filters.dedup();
        if self.consumers[&lease.consumer].filter == filters {
            return Ok(lease.clone());
        }
        let old = self.gui_attachments.remove(&lease.device).unwrap();
        match self.gui_attach(lease.device, filters) {
            Ok(next) => Ok(next),
            Err(error) => {
                self.gui_attachments.insert(lease.device, old);
                Err(error)
            }
        }
    }

    fn gui_detach(&mut self, lease: &GuiLease) -> Result<()> {
        if self.gui_attachments.contains_key(&lease.device) {
            self.gui_attachment(lease)?;
            self.gui_attachments.remove(&lease.device);
        }
        Ok(())
    }

    fn gui_poll(
        &mut self,
        lease: &GuiLease,
        limit: usize,
        bytes: usize,
        now: u64,
    ) -> Result<Vec<Delivery>> {
        let after = self.gui_attachment(lease)?.last_stream.clone();
        if limit == 0 || bytes < 128 {
            return Err(invalid("invalid GUI poll budget"));
        }
        let limit = limit.min(128);
        let bytes = bytes.min(256 * 1024);
        let mut names: Vec<_> = self
            .streams
            .iter()
            .filter(|(_, stream)| {
                stream
                    .consumers
                    .get(&lease.consumer)
                    .and_then(|s| s.gui.as_ref())
                    .is_some_and(|g| g.next_due <= now)
            })
            .map(|(name, _)| name.clone())
            .collect();
        if let Some(after) = after {
            let at = names.partition_point(|name| name <= &after);
            names.rotate_left(at);
        }
        let mut result = Vec::new();
        let mut used = 0;
        let mut undeliverable = Vec::new();
        for name in names.into_iter().take(8) {
            if result.len() >= limit || used + 128 > bytes {
                break;
            }
            let batch = self.streams.get_mut(&name).unwrap().gui_poll(
                &lease.consumer,
                (limit - result.len()).min(32),
                bytes - used,
                bytes,
                now,
            )?;
            used += batch.bytes;
            result.extend(batch.deliveries);
            undeliverable.extend(batch.undeliverable);
            self.gui_attachments
                .get_mut(&lease.device)
                .unwrap()
                .last_stream = Some(name);
        }
        // Read transactions have ended before appending this audit. Stable IDs
        // make repeated redelivery or a process crash produce one audit row.
        for failure in undeliverable {
            use sha2::{Digest, Sha256};
            let bytes = serde_json::to_vec(&(&lease.consumer, &failure.stream, failure.sequence))?;
            let hash = Sha256::digest(bytes);
            let mut audit = Draft::new(
                "account/control",
                "gui/consumer",
                serde_json::json!({
                    "activity":"undeliverable", "consumer":lease.consumer,
                    "stream":failure.stream, "seq":failure.sequence.to_string(),
                }),
            );
            audit.id = Ulid::from_bytes(hash[..16].try_into().unwrap());
            audit.ts = failure.ts;
            self.publish("account", audit)?;
        }
        Ok(result)
    }

    fn gui_ack(&mut self, lease: &GuiLease, stream: &str, sequence: u64) -> Result<()> {
        self.gui_attachment(lease)?;
        let stream = self
            .streams
            .get_mut(stream)
            .ok_or_else(|| invalid("unknown GUI stream"))?;
        let sub = stream
            .consumers
            .get(&lease.consumer)
            .ok_or_else(|| invalid("GUI consumer does not cover stream"))?;
        if sequence == 0 || sequence > sub.cursor {
            return Err(invalid("GUI ACK is outside the admitted stream cursor"));
        }
        let deliveries: Option<u32> = sqlite::open(&stream.log.path)?
            .query_row(
                "SELECT deliveries FROM pending WHERE consumer=?1 AND seq=?2",
                params![lease.consumer, sqlint(sequence)?],
                |row| row.get(0),
            )
            .optional()?;
        match deliveries {
            Some(0) => Err(invalid("GUI ACK precedes delivery")),
            Some(_) => stream.ack(&lease.consumer, sequence),
            None => Ok(()), // An ACK retry never advances an unrelated cursor.
        }
    }

    fn gui_cursor(&self, lease: &GuiLease, name: &str) -> Result<GuiCursor> {
        self.gui_attachment(lease)?;
        let stream = self
            .streams
            .get(name)
            .ok_or_else(|| invalid("unknown GUI stream"))?;
        let config = &stream
            .consumers
            .get(&lease.consumer)
            .ok_or_else(|| invalid("GUI consumer does not cover stream"))?
            .config;
        let db = sqlite::open(&stream.log.path)?;
        let (admitted, published): (u64, u64) = db.query_row(
            "SELECT cursor,acked FROM consumers WHERE name=?1",
            [&lease.consumer],
            |row| Ok((gui_uint(row, 0)?, gui_uint(row, 1)?)),
        )?;
        let account = SqliteStore::open(&object_dir(&self.root, "account")?)?;
        let read: u64 = account
            .get(&format!("read/{name}"))?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?
            .unwrap_or(0);
        // The messaging service supplies exact retained message subjects. An
        // indexed aggregate derives unread without loading bodies, a second
        // counter, or a Rust allocation/dispatch for every historical row.
        let subjects: Vec<_> = config
            .filter
            .iter()
            .filter(|subject| subject.ends_with("/message") && !subject.contains(['*', '>']))
            .collect();
        let unread = if subjects.is_empty() {
            0
        } else {
            let placeholders = std::iter::repeat_n("?", subjects.len())
                .collect::<Vec<_>>()
                .join(",");
            let query =
                format!("SELECT COUNT(*) FROM events WHERE seq>? AND subject IN ({placeholders})");
            let values = std::iter::once(rusqlite::types::Value::Integer(sqlint(read)?)).chain(
                subjects
                    .into_iter()
                    .map(|subject| rusqlite::types::Value::Text(subject.clone())),
            );
            db.query_row(&query, rusqlite::params_from_iter(values), |row| {
                gui_uint(row, 0)
            })?
        };
        Ok(GuiCursor {
            stream: name.into(),
            admitted,
            published,
            read,
            high: stream.log.last_sequence()?,
            unread,
        })
    }

    fn gui_read(&mut self, lease: &GuiLease, stream: &str, through: u64) -> Result<GuiCursor> {
        let state = self.gui_cursor(lease, stream)?;
        if through > state.high {
            return Err(invalid("operator read-mark exceeds retained stream head"));
        }
        if through > state.read {
            SqliteStore::open(&object_dir(&self.root, "account")?)?
                .put(&format!("read/{stream}"), &serde_json::to_vec(&through)?)?;
        }
        self.gui_cursor(lease, stream)
    }

    fn read_page(
        &self,
        name: &str,
        filter: &[String],
        after: u64,
        limit: usize,
    ) -> Result<RetainedPage> {
        let Some(stream) = self.streams.get(name) else {
            return Ok((vec![], None, after));
        };
        let limit = limit.clamp(1, 128);
        let db = sqlite::open(&stream.log.path)?;
        let rows = {
            let mut statement = db.prepare(
                "SELECT seq,length(payload),subject FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2",
            )?;
            statement
                .query_map(params![sqlint(after)?, limit as i64], |row| {
                    Ok((
                        gui_uint(row, 0)?,
                        gui_uint(row, 1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let full = rows.len() == limit;
        let mut through = after;
        let mut events = Vec::new();
        let mut used = 0usize;
        let mut byte_limited = false;
        for (sequence, length, subject) in rows {
            // The operator's own lane filter; an event too large for one page is skipped
            // and stays retained, as on the window's pages.
            if !filter.iter().any(|p| matches(p, &subject)) || length > 64 * 1024 {
                through = sequence;
                continue;
            }
            if used + length as usize > crate::gui_events::PAGE_BYTES - 2048 {
                byte_limited = true;
                break;
            }
            let envelope = stream
                .log
                .at(sequence)?
                .ok_or_else(|| invalid("retained event missing"))?;
            envelope.verify()?;
            used += length as usize;
            through = sequence;
            events.push(envelope);
        }
        Ok((events, (full || byte_limited).then_some(through), through))
    }
    fn gui_page(
        &self,
        lease: &GuiLease,
        name: &str,
        after: u64,
        limit: usize,
        bytes: usize,
    ) -> Result<GuiPage> {
        let cursor = self.gui_cursor(lease, name)?;
        if after > cursor.high || limit == 0 || bytes < 128 {
            return Err(invalid("invalid GUI retained page cursor or budget"));
        }
        let stream = &self.streams[name];
        let config = &stream.consumers[&lease.consumer].config;
        let limit = limit.min(128);
        let bytes = bytes.min(256 * 1024);
        let db = sqlite::open(&stream.log.path)?;
        let metadata = {
            let mut statement = db.prepare(
                "SELECT seq,length(payload),hash,subject FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2",
            )?;
            statement
                .query_map(params![sqlint(after)?, limit as i64], |row| {
                    Ok((
                        gui_uint(row, 0)?,
                        gui_uint(row, 1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let mut used = 0;
        let mut through = after;
        let mut records = Vec::new();
        for (sequence, length, hash, subject) in metadata {
            if !config.accepts(&subject) {
                through = sequence;
                continue;
            }
            let length = usize::try_from(length)?;
            let cost = if length > bytes { 128 } else { length };
            if used + cost > bytes {
                break;
            }
            through = sequence;
            used += cost;
            if length > bytes {
                records.push(Delivery::Gap {
                    stream: name.into(),
                    first: sequence,
                    last: sequence,
                    reason: "event_exceeds_gui_page_byte_limit; original retained".into(),
                });
                continue;
            }
            let envelope = stream
                .log
                .at(sequence)?
                .ok_or_else(|| invalid("retained GUI event missing"))?;
            envelope.verify()?;
            if envelope.seq != sequence || envelope.hash()? != hash {
                return Err(invalid("retained GUI event hash or sequence mismatch"));
            }
            if config.accepts(&envelope.event.subject) {
                records.push(Delivery::Event {
                    stream: name.into(),
                    envelope: Box::new(envelope),
                    deliveries: 0,
                });
            }
        }
        let next = (through < cursor.high).then_some(through);
        Ok(GuiPage {
            records,
            next,
            cursor,
        })
    }

    fn gui_latest(&self, lease: &GuiLease, name: &str, subject: &str) -> Result<Option<Envelope>> {
        self.gui_attachment(lease)?;
        let Some(stream) = self.streams.get(name) else {
            return Ok(None);
        };
        if !stream.consumers[&lease.consumer].config.accepts(subject) {
            return Err(invalid("GUI subject is outside the consumer filter"));
        }
        let metadata: Option<(i64, i64, String)> = sqlite::open(&stream.log.path)?.query_row(
            "SELECT seq,length(payload),hash FROM events WHERE subject=?1 ORDER BY seq DESC LIMIT 1",
            [subject], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        let Some((sequence, length, expected_hash)) = metadata else {
            return Ok(None);
        };
        if usize::try_from(length)? > 256 * 1024 {
            return Err(invalid("latest GUI event exceeds bounded history read"));
        }
        let sequence = u64::try_from(sequence)?;
        let envelope = stream
            .log
            .at(sequence)?
            .ok_or_else(|| invalid("retained GUI event missing"))?;
        envelope.verify()?;
        if envelope.hash()? != expected_hash
            || envelope.seq != sequence
            || envelope.event.subject != subject
        {
            return Err(invalid("latest GUI event metadata mismatch"));
        }
        Ok(Some(envelope))
    }

    pub fn poll(&mut self, consumer: &str, limit: usize, now: u64) -> Result<Vec<Delivery>> {
        if !self.consumers.contains_key(consumer) {
            return Err(invalid("unknown consumer"));
        }
        let mut result = Vec::new();
        for stream in self.streams.values_mut() {
            if result.len() >= limit.min(1024) {
                break;
            }
            result.extend(stream.poll(consumer, limit - result.len(), now)?);
        }
        Ok(result)
    }
    pub fn ack(&mut self, consumer: &str, stream: &str, sequence: u64) -> Result<()> {
        self.streams
            .get_mut(stream)
            .ok_or_else(|| invalid("unknown stream"))?
            .ack(consumer, sequence)
    }
    pub fn unsubscribe(&mut self, id: &str) -> Result<()> {
        for stream in self.streams.values_mut() {
            let mut db = sqlite::open(&stream.log.path)?;
            let tx = db.transaction()?;
            tx.execute("DELETE FROM pending WHERE consumer=?1", [id])?;
            tx.execute("DELETE FROM consumers WHERE name=?1", [id])?;
            tx.commit()?;
            stream.consumers.remove(id);
        }
        self.catalog.sql(
            "DELETE FROM kv WHERE key=?1",
            &[format!("consumer/{id}").into()],
        )?;
        self.consumers.remove(id);
        Ok(())
    }
}

enum Command {
    Publish(String, Draft, oneshot::Sender<Result<Envelope>>),
    Activity(Box<(Draft, SigningKey)>, oneshot::Sender<Result<Envelope>>),
    Subscribe(Consumer, oneshot::Sender<Result<String>>),
    Poll(String, usize, oneshot::Sender<Result<Vec<Delivery>>>),
    Ack(String, String, u64, oneshot::Sender<Result<()>>),
    Unsubscribe(String, oneshot::Sender<Result<()>>),
    GuiAttach(Ulid, Vec<String>, oneshot::Sender<Result<GuiLease>>),
    GuiRefresh(GuiLease, Vec<String>, oneshot::Sender<Result<GuiLease>>),
    GuiPoll(
        GuiLease,
        usize,
        usize,
        oneshot::Sender<Result<Vec<Delivery>>>,
    ),
    GuiAck(GuiLease, String, u64, oneshot::Sender<Result<()>>),
    GuiDetach(GuiLease, oneshot::Sender<Result<()>>),
    GuiCursor(GuiLease, String, oneshot::Sender<Result<GuiCursor>>),
    GuiRead(GuiLease, String, u64, oneshot::Sender<Result<GuiCursor>>),
    ReadPage(
        String,
        Vec<String>,
        u64,
        usize,
        oneshot::Sender<Result<RetainedPage>>,
    ),
    ReadStreams(oneshot::Sender<Vec<String>>),
    GuiPage(
        GuiLease,
        String,
        u64,
        usize,
        usize,
        oneshot::Sender<Result<GuiPage>>,
    ),
    GuiLatest(
        GuiLease,
        String,
        String,
        oneshot::Sender<Result<Option<Envelope>>>,
    ),
    IdentitySigner(SigningKey, Option<Ulid>, oneshot::Sender<Vec<HoldRange>>),
    HoldRanges(oneshot::Sender<Vec<HoldRange>>),
    Shutdown(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct BusHandle {
    tx: mpsc::Sender<Command>,
    #[cfg(test)]
    pub(crate) gui_read_faults: std::sync::Arc<GuiReadFaults>,
}
#[cfg(test)]
#[derive(Default)]
pub(crate) struct GuiReadFaults {
    pub(crate) poll: std::sync::atomic::AtomicI32,
    pub(crate) cursor: std::sync::atomic::AtomicI32,
}
impl BusHandle {
    /// Uses the R1 handle. No runtime is created or blocked by this constructor.
    pub fn start(handle: &tokio::runtime::Handle, broker: Broker, capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > 4096 {
            return Err(invalid("invalid router capacity"));
        }
        let (tx, mut rx) = mpsc::channel(capacity);
        handle.spawn_blocking(move || {
            let mut broker = broker;
            while let Some(command) = rx.blocking_recv() {
                match command {
                    Command::Publish(stream, event, reply) => {
                        let _ = reply.send(broker.publish(&stream, event));
                    }
                    Command::Activity(activity, reply) => {
                        let (event, key) = *activity;
                        let result = if !event.subject.starts_with("doc/")
                            && !event.subject.starts_with("artifact/")
                            && !event.subject.starts_with("activity/")
                        {
                            Err(invalid("activity sink only accepts index subjects"))
                        } else {
                            broker.publish_using("index", event, Some(key))
                        };
                        let _ = reply.send(result);
                    }
                    Command::Subscribe(config, reply) => {
                        let _ = reply.send(broker.subscribe(config));
                    }
                    Command::Poll(name, limit, reply) => {
                        let _ = reply.send(broker.poll(&name, limit, now_ms()));
                    }
                    Command::Ack(name, stream, seq, reply) => {
                        let _ = reply.send(broker.ack(&name, &stream, seq));
                    }
                    Command::Unsubscribe(name, reply) => {
                        let _ = reply.send(broker.unsubscribe(&name));
                    }
                    Command::GuiAttach(device, filter, reply) => {
                        let _ = reply.send(broker.gui_attach(device, filter));
                    }
                    Command::GuiRefresh(lease, filter, reply) => {
                        let _ = reply.send(broker.gui_refresh(&lease, filter));
                    }
                    Command::GuiPoll(lease, limit, bytes, reply) => {
                        let _ = reply.send(broker.gui_poll(&lease, limit, bytes, now_ms()));
                    }
                    Command::GuiAck(lease, stream, sequence, reply) => {
                        let _ = reply.send(broker.gui_ack(&lease, &stream, sequence));
                    }
                    Command::GuiDetach(lease, reply) => {
                        let _ = reply.send(broker.gui_detach(&lease));
                    }
                    Command::GuiCursor(lease, stream, reply) => {
                        let _ = reply.send(broker.gui_cursor(&lease, &stream));
                    }
                    Command::GuiRead(lease, stream, sequence, reply) => {
                        let _ = reply.send(broker.gui_read(&lease, &stream, sequence));
                    }
                    Command::ReadPage(stream, filter, after, limit, reply) => {
                        let _ = reply.send(broker.read_page(&stream, &filter, after, limit));
                    }
                    Command::ReadStreams(reply) => {
                        let _ = reply.send(broker.streams.keys().cloned().collect());
                    }
                    Command::GuiPage(lease, stream, after, limit, bytes, reply) => {
                        let _ = reply.send(broker.gui_page(&lease, &stream, after, limit, bytes));
                    }
                    Command::GuiLatest(lease, stream, subject, reply) => {
                        let _ = reply.send(broker.gui_latest(&lease, &stream, &subject));
                    }
                    Command::IdentitySigner(key, hold, reply) => {
                        let _ = reply.send(broker.identity_signer(key, hold));
                    }
                    Command::HoldRanges(reply) => {
                        let _ = reply.send(broker.hold_ranges());
                    }
                    Command::Shutdown(reply) => {
                        let _ = reply.send(());
                        break;
                    }
                }
            }
        });
        Ok(Self {
            tx,
            #[cfg(test)]
            gui_read_faults: Default::default(),
        })
    }
    pub fn try_publish(
        &self,
        stream: impl Into<String>,
        event: Draft,
    ) -> Result<oneshot::Receiver<Result<Envelope>>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .try_send(Command::Publish(stream.into(), event, tx))
            .map_err(|_| invalid("router busy or stopped; retain original event id"))?;
        Ok(rx)
    }
    pub async fn publish(&self, stream: impl Into<String>, event: Draft) -> Result<Envelope> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Publish(stream.into(), event, tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await
            .map_err(|_| invalid("publish outcome unknown; query original event id"))?
    }
    /// Trusted index writer only: append and route once with the actual author
    /// key. This is not a seat socket operation or a general signing service.
    pub(crate) async fn publish_activity(&self, event: Draft, key: SigningKey) -> Result<Envelope> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Activity(Box::new((event, key)), tx))
            .await
            .map_err(|_| invalid("activity router stopped"))?;
        rx.await
            .map_err(|_| invalid("activity outcome unknown; retain the event ID"))?
    }
    pub async fn subscribe(&self, config: Consumer) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Subscribe(config, tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))?
    }
    pub async fn poll(&self, name: &str, limit: usize) -> Result<Vec<Delivery>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Poll(name.into(), limit, tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))?
    }
    pub async fn ack(&self, name: &str, stream: &str, seq: u64) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Ack(name.into(), stream.into(), seq, tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))?
    }
    pub async fn unsubscribe(&self, name: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Unsubscribe(name.into(), tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))?
    }
    pub(crate) async fn gui_attach(&self, device: Ulid, filter: Vec<String>) -> Result<GuiLease> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiAttach(device, filter, tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_refresh(
        &self,
        lease: &GuiLease,
        filter: Vec<String>,
    ) -> Result<GuiLease> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiRefresh(lease.clone(), filter, tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_poll(
        &self,
        lease: &GuiLease,
        limit: usize,
        bytes: usize,
    ) -> Result<Vec<Delivery>> {
        #[cfg(test)]
        self.gui_read_fault(false)?;
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiPoll(lease.clone(), limit, bytes, tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_ack(
        &self,
        lease: &GuiLease,
        stream: &str,
        sequence: u64,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiAck(lease.clone(), stream.into(), sequence, tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_detach(&self, lease: &GuiLease) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiDetach(lease.clone(), tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_cursor(&self, lease: &GuiLease, stream: &str) -> Result<GuiCursor> {
        #[cfg(test)]
        self.gui_read_fault(true)?;
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiCursor(lease.clone(), stream.into(), tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    #[cfg(test)]
    fn gui_read_fault(&self, cursor: bool) -> Result<()> {
        let fault = if cursor {
            &self.gui_read_faults.cursor
        } else {
            &self.gui_read_faults.poll
        };
        let code = fault.swap(0, std::sync::atomic::Ordering::SeqCst);
        if code != 0 {
            return Err(
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None).into(),
            );
        }
        Ok(())
    }
    pub(crate) async fn gui_read(
        &self,
        lease: &GuiLease,
        stream: &str,
        through: u64,
    ) -> Result<GuiCursor> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiRead(lease.clone(), stream.into(), through, tx))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    /// A read-only page of a retained stream after `after`, filtered by `filter`: no
    /// consumer, no attachment, no ack (ctl `EventsRead`, #272). Returns the verified events
    /// and the cursor to continue from, None at the head.
    pub(crate) async fn read_page(
        &self,
        stream: &str,
        filter: Vec<String>,
        after: u64,
        limit: usize,
    ) -> Result<RetainedPage> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::ReadPage(stream.into(), filter, after, limit, tx))
            .await
            .map_err(|_| invalid("event router stopped"))?;
        rx.await.map_err(|_| invalid("event router stopped"))?
    }
    /// Registered stream names in order. This only copies broker metadata:
    /// no log scan, consumer, read mark or acknowledgement.
    pub(crate) async fn read_streams(&self) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::ReadStreams(tx))
            .await
            .map_err(|_| invalid("event router stopped"))?;
        rx.await.map_err(|_| invalid("event router stopped"))
    }
    pub(crate) async fn gui_page(
        &self,
        lease: &GuiLease,
        stream: &str,
        after: u64,
        limit: usize,
        bytes: usize,
    ) -> Result<GuiPage> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiPage(
                lease.clone(),
                stream.into(),
                after,
                limit,
                bytes,
                tx,
            ))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub(crate) async fn gui_latest(
        &self,
        lease: &GuiLease,
        stream: &str,
        subject: &str,
    ) -> Result<Option<Envelope>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiLatest(
                lease.clone(),
                stream.into(),
                subject.into(),
                tx,
            ))
            .await
            .map_err(|_| invalid("GUI event router stopped"))?;
        rx.await.map_err(|_| invalid("GUI event router stopped"))?
    }
    pub async fn identity_signer(
        &self,
        key: SigningKey,
        hold: Option<Ulid>,
    ) -> Result<Vec<HoldRange>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::IdentitySigner(key, hold, tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await
            .map_err(|_| invalid("signer transition outcome unknown"))
    }
    pub async fn hold_ranges(&self) -> Result<Vec<HoldRange>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::HoldRanges(tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))
    }
    pub async fn shutdown(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Shutdown(tx))
            .await
            .map_err(|_| invalid("router stopped"))?;
        rx.await.map_err(|_| invalid("router stopped"))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
mod gui_tests {
    use super::*;

    #[test]
    fn retained_read_cursor_includes_filtered_rows_on_a_short_final_page() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        broker.publish("index", event("activity/visible")).unwrap();
        broker.publish("index", event("private/hidden")).unwrap();
        let (rows, next, through) = broker
            .read_page("index", &["activity/>".into()], 0, 128)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(next, None);
        assert_eq!(through, 2);
        assert!(
            broker.consumers.is_empty(),
            "a retained read made a consumer"
        );
        let (rows, next, through) = broker
            .read_page("index", &["activity/>".into()], through, 128)
            .unwrap();
        assert!(rows.is_empty());
        assert_eq!(next, None);
        assert_eq!(through, 2);
    }

    #[test]
    fn retained_read_byte_pages_resume_without_losing_large_events() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        for _ in 0..12 {
            let mut row = event("activity/large");
            row.payload = serde_json::json!({"text":"x".repeat(30_000)});
            broker.publish("index", row).unwrap();
        }
        let (mut after, mut seen, mut pages) = (0, Vec::new(), 0);
        loop {
            let (rows, next, through) = broker
                .read_page("index", &["activity/>".into()], after, 128)
                .unwrap();
            assert!(serde_json::to_vec(&rows).unwrap().len() < crate::gui_events::PAGE_BYTES);
            seen.extend(rows.into_iter().map(|e| e.seq));
            after = through;
            pages += 1;
            if next.is_none() {
                break;
            }
            assert_eq!(next, Some(after));
            assert!(pages < 10, "retained page did not converge");
        }
        assert!(pages > 1, "fixture did not exceed the byte budget");
        assert_eq!(seen, (1..=12).collect::<Vec<_>>());
        assert_eq!(after, 12);
    }

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("bbt-gui-bus-{}", Ulid::new())))
        }
        fn open(&self) -> Broker {
            Broker::open(&self.0, SigningKey::from_bytes(&[83; 32])).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn filters() -> Vec<String> {
        vec!["artifact/>".into(), "operator/inbox/message".into()]
    }
    fn event(subject: &str) -> Draft {
        Draft::new(subject, "fixture", serde_json::json!({"id":"fixture"}))
    }
    fn sequences(deliveries: &[Delivery]) -> Vec<(String, u64)> {
        deliveries
            .iter()
            .filter_map(|delivery| match delivery {
                Delivery::Event {
                    stream, envelope, ..
                } => Some((stream.clone(), envelope.seq)),
                Delivery::Gap { .. } => None,
            })
            .collect()
    }

    #[test]
    fn gui_filter_migration_resumes_after_a_crash_between_streams() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let device = Ulid::new();
        broker.ensure_stream("account").unwrap();
        let old = broker.gui_attach(device, filters()).unwrap();
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        broker.gui_poll(&old, 128, 256 * 1024, 1).unwrap();
        broker
            .publish("index", event("artifact/b/created"))
            .unwrap();
        let mut config = broker.consumers[&old.consumer].clone();
        let next = format!("{}-v1", old.consumer);
        config.name = Some(next.clone());
        config.filter.push("account/control".into());
        config.filter.sort();
        let filter = config.filter.clone();
        broker.subscribe(config).unwrap();
        broker
            .streams
            .get_mut("index")
            .unwrap()
            .gui_migrate(&old.consumer, &next)
            .unwrap();
        // Account has not migrated, the old catalog entry still exists, and
        // index's copied pending state must not be reset on recovery.
        drop(broker);
        let mut broker = fixture.open();
        let resumed = broker.gui_attach(device, filter).unwrap();
        assert_eq!(resumed.consumer, next);
        let rows = broker.gui_poll(&resumed, 128, 256 * 1024, 2).unwrap();
        assert_eq!(
            sequences(&rows)
                .into_iter()
                .filter(|(s, _)| s == "index")
                .collect::<Vec<_>>(),
            [("index".into(), 1), ("index".into(), 2)]
        );
        assert_eq!(broker.catalog.list("consumer/").unwrap().len(), 1);
        assert!(broker.catalog.list("gui-migration/").unwrap().is_empty());
        assert_eq!(broker.streams["account"].log.last_sequence().unwrap(), 1);
    }

    #[test]
    fn gui_filter_migration_keeps_pending_and_tail_but_new_subjects_start_at_head() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let device = Ulid::new();
        let old = broker
            .gui_attach(
                device,
                vec![
                    "artifact/a/created".into(),
                    "artifact/b/created".into(),
                    "account/control".into(),
                ],
            )
            .unwrap();
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        broker
            .publish("index", event("artifact/b/created"))
            .unwrap();
        broker.publish("index", event("activity/new")).unwrap();
        assert_eq!(
            sequences(&broker.gui_poll(&old, 128, 256 * 1024, 1).unwrap()),
            [("index".into(), 1), ("index".into(), 2)]
        );
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        let next = broker
            .gui_refresh(
                &old,
                vec![
                    "artifact/a/created".into(),
                    "activity/new".into(),
                    "account/control".into(),
                ],
            )
            .unwrap();
        assert_ne!(old.consumer, next.consumer);
        assert_eq!(broker.gui_cursor(&next, "index").unwrap().admitted, 3);
        let records = broker.gui_poll(&next, 128, 256 * 1024, 2).unwrap();
        assert_eq!(
            sequences(&records)
                .into_iter()
                .filter(|(s, _)| s == "index")
                .collect::<Vec<_>>(),
            [("index".into(), 1), ("index".into(), 4)]
        );
        let audit = broker.streams["account"].log.after(0, 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].event.payload["pending_dropped"], "1");
        broker.publish("index", event("activity/new")).unwrap();
        drop(broker);
        let mut broker = fixture.open();
        let resumed = broker
            .gui_attach(
                device,
                vec![
                    "artifact/a/created".into(),
                    "activity/new".into(),
                    "account/control".into(),
                ],
            )
            .unwrap();
        let records = broker.gui_poll(&resumed, 128, 256 * 1024, 3).unwrap();
        assert_eq!(
            sequences(&records)
                .into_iter()
                .filter(|(s, _)| s == "index")
                .collect::<Vec<_>>(),
            [
                ("index".into(), 1),
                ("index".into(), 4),
                ("index".into(), 5)
            ]
        );
        assert_eq!(broker.catalog.list("consumer/").unwrap().len(), 1);
    }

    #[test]
    fn gui_deduplicated_publish_wakes_a_retained_append_not_yet_routed() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let lease = broker.gui_attach(Ulid::new(), filters()).unwrap();
        let event = event("artifact/a/created");
        broker.ensure_stream("index").unwrap();
        broker.streams["index"]
            .log
            .append(event.clone(), &broker.key)
            .unwrap();
        broker
            .streams
            .get_mut("index")
            .unwrap()
            .consumers
            .get_mut(&lease.consumer)
            .unwrap()
            .gui
            .as_mut()
            .unwrap()
            .next_due = u64::MAX;
        broker.publish("index", event).unwrap();
        assert_eq!(
            sequences(&broker.gui_poll(&lease, 128, 256 * 1024, 1).unwrap()),
            [("index".into(), 1)]
        );
    }

    #[test]
    fn gui_defers_a_row_that_fits_a_fresh_quantum_without_emitting_a_gap() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let lease = broker.gui_attach(Ulid::new(), filters()).unwrap();
        for (stream, bytes) in [("a", 240 * 1024), ("b", 32 * 1024)] {
            broker
                .publish(
                    stream,
                    Draft::new(
                        "artifact/a/created",
                        "fixture",
                        serde_json::json!({"padding": "x".repeat(bytes)}),
                    ),
                )
                .unwrap();
        }
        let first = broker.gui_poll(&lease, 128, 256 * 1024, 1).unwrap();
        assert_eq!(sequences(&first), [("a".into(), 1)]);
        assert_eq!(first.len(), 1, "remaining budget must never produce a Gap");
        let pending = &broker.streams["b"];
        let db = sqlite::open(&pending.log.path).unwrap();
        let attempts: u32 = db
            .query_row(
                "SELECT deliveries FROM pending WHERE consumer=?1 AND seq=1",
                [&lease.consumer],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(attempts, 0, "a deferred row has not been delivered");
        let next = broker.gui_poll(&lease, 128, 256 * 1024, 2).unwrap();
        assert_eq!(sequences(&next), [("b".into(), 1)]);
        assert_eq!(next.len(), 1);
    }

    #[test]
    fn gui_starts_new_but_replays_unpublished_pending_after_logic_restart() {
        let fixture = Fixture::new();
        let device = Ulid::new();
        let mut broker = fixture.open();
        broker
            .publish("index", event("artifact/old/created"))
            .unwrap();
        let lease = broker.gui_attach(device, filters()).unwrap();
        assert!(
            broker
                .gui_poll(&lease, 128, 256 * 1024, 1)
                .unwrap()
                .is_empty()
        );
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        broker
            .publish("index", event("artifact/b/created"))
            .unwrap();
        let rows = broker.gui_poll(&lease, 128, 256 * 1024, 2).unwrap();
        assert_eq!(sequences(&rows), [("index".into(), 2), ("index".into(), 3)]);
        broker.gui_ack(&lease, "index", 2).unwrap();
        drop(broker);
        let mut broker = fixture.open();
        let resumed = broker.gui_attach(device, filters()).unwrap();
        assert_eq!(resumed.consumer, lease.consumer);
        assert_ne!(resumed.token, lease.token);
        // No real clock wait: a new attachment reads through old pending rows
        // even when their previous delivery's ACK deadline is still future.
        let rows = broker.gui_poll(&resumed, 128, 256 * 1024, 3).unwrap();
        assert_eq!(sequences(&rows), [("index".into(), 3)]);
        broker.gui_ack(&resumed, "index", 3).unwrap();
        broker.gui_ack(&resumed, "index", 3).unwrap();
        assert_eq!(broker.gui_cursor(&resumed, "index").unwrap().published, 3);
        assert_eq!(broker.catalog.list("consumer/").unwrap().len(), 1);
        assert!(broker.gui_ack(&lease, "index", 3).is_err());
    }

    #[test]
    fn gui_transport_ack_is_not_operator_read_and_read_marks_survive_restart() {
        let fixture = Fixture::new();
        let device = Ulid::new();
        let mut broker = fixture.open();
        broker
            .publish("swarm-fixture", event("operator/inbox/message"))
            .unwrap();
        let lease = broker.gui_attach(device, filters()).unwrap();
        assert_eq!(
            broker.gui_cursor(&lease, "swarm-fixture").unwrap().unread,
            1
        );
        let page = broker
            .gui_page(&lease, "swarm-fixture", 0, 128, 256 * 1024)
            .unwrap();
        assert_eq!(sequences(&page.records), [("swarm-fixture".into(), 1)]);
        assert!(page.next.is_none());
        broker
            .publish("swarm-fixture", event("operator/inbox/message"))
            .unwrap();
        let rows = broker.gui_poll(&lease, 128, 256 * 1024, 1).unwrap();
        assert_eq!(sequences(&rows), [("swarm-fixture".into(), 2)]);
        broker.gui_ack(&lease, "swarm-fixture", 2).unwrap();
        let state = broker.gui_cursor(&lease, "swarm-fixture").unwrap();
        assert_eq!(state.published, 2);
        assert_eq!(state.read, 0);
        assert_eq!(state.unread, 2);
        assert_eq!(
            broker.gui_read(&lease, "swarm-fixture", 1).unwrap().unread,
            1
        );
        assert_eq!(broker.gui_read(&lease, "swarm-fixture", 0).unwrap().read, 1);
        assert!(broker.gui_read(&lease, "swarm-fixture", 3).is_err());
        drop(broker);
        let mut broker = fixture.open();
        let lease = broker.gui_attach(device, filters()).unwrap();
        let state = broker.gui_cursor(&lease, "swarm-fixture").unwrap();
        assert_eq!((state.read, state.unread), (1, 1));
    }

    #[test]
    fn gui_one_attachment_detaches_without_deleting_and_filter_change_versions() {
        let fixture = Fixture::new();
        let device = Ulid::new();
        let mut broker = fixture.open();
        let lease = broker.gui_attach(device, filters()).unwrap();
        assert!(broker.gui_attach(device, filters()).is_err());
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        broker.gui_poll(&lease, 128, 256 * 1024, 1).unwrap();
        broker.gui_detach(&lease).unwrap();
        assert_eq!(broker.catalog.list("consumer/").unwrap().len(), 1);
        let resumed = broker.gui_attach(device, filters()).unwrap();
        assert_eq!(resumed.consumer, lease.consumer);
        assert!(broker.gui_detach(&lease).is_err());
        assert_eq!(
            sequences(&broker.gui_poll(&resumed, 128, 256 * 1024, 2).unwrap()),
            [("index".into(), 1)]
        );
        broker.gui_detach(&resumed).unwrap();
        let changed = broker
            .gui_attach(device, vec!["account/control".into()])
            .unwrap();
        assert_eq!(changed.consumer, format!("gui-{device}-v1"));
        assert!(!broker.consumers.contains_key(&lease.consumer));
        assert_eq!(broker.catalog.list("consumer/").unwrap().len(), 1);
        assert_eq!(broker.streams["index"].log.last_sequence().unwrap(), 1);
    }

    #[test]
    fn gui_delivery_limit_is_audited_without_discarding_the_original_row() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let lease = broker.gui_attach(Ulid::new(), filters()).unwrap();
        broker
            .publish("index", event("artifact/a/created"))
            .unwrap();
        for now in [1, 30_001, 60_001, 90_001, 120_001] {
            assert_eq!(
                sequences(&broker.gui_poll(&lease, 128, 256 * 1024, now).unwrap()),
                [("index".into(), 1)]
            );
        }
        let account = &broker.streams["account"].log;
        assert_eq!(account.last_sequence().unwrap(), 1);
        let audit = account.at(1).unwrap().unwrap();
        audit.verify().unwrap();
        assert_eq!(audit.event.payload["activity"], "undeliverable");
        assert_eq!(audit.event.payload["stream"], "index");
        assert_eq!(audit.event.payload["seq"], "1");
        let pending: bool = sqlite::open(&broker.streams["index"].log.path)
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pending WHERE consumer=?1 AND seq=1)",
                [&lease.consumer],
                |row| row.get(0),
            )
            .unwrap();
        assert!(pending);
        broker.gui_ack(&lease, "index", 1).unwrap();
    }

    #[test]
    fn gui_oversized_rows_have_a_visible_gap_and_retained_original() {
        let fixture = Fixture::new();
        let mut broker = fixture.open();
        let lease = broker.gui_attach(Ulid::new(), filters()).unwrap();
        let mut large = event("artifact/a/created");
        large.payload = serde_json::json!({"text":"x".repeat(4096)});
        broker.publish("index", large).unwrap();
        let rows = broker.gui_poll(&lease, 128, 1024, 1).unwrap();
        assert!(
            matches!(&rows[..], [Delivery::Gap {stream,first:1,last:1,reason}] if stream=="index" && reason.contains("original retained"))
        );
        assert!(broker.streams["index"].log.at(1).unwrap().is_some());
        broker.gui_ack(&lease, "index", 1).unwrap();
    }
}
