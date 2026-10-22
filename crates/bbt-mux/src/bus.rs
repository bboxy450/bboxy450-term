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
}

struct Stream {
    name: String,
    log: EventLog,
    consumers: BTreeMap<String, Subscriber>,
}

impl Stream {
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
        self.consumers.insert(
            id.into(),
            Subscriber {
                config: config.clone(),
                queue: VecDeque::new(),
                gap: None,
                cursor,
            },
        );
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
            if event.seq > 0 && self.consumers[id].config.ack == Ack::Explicit {
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
    catalog: SqliteStore,
    streams: BTreeMap<String, Stream>,
    consumers: BTreeMap<String, Consumer>,
}

impl Broker {
    pub fn open(root: &Path, key: SigningKey) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(root)?;
        let catalog = SqliteStore::open(&object_dir(root, "bus")?)?;
        let mut broker = Self {
            root: root.into(),
            key,
            hold: None,
            custody_signers: false,
            catalog,
            streams: BTreeMap::new(),
            consumers: BTreeMap::new(),
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
    pub fn publish(&mut self, stream: &str, mut event: Draft) -> Result<Envelope> {
        validate_subject(&event.subject, false)?;
        if let Some(reply) = &event.reply_to {
            validate_subject(reply, false)?;
            if !reply.starts_with("inbox/") {
                return Err(invalid("reply must address an inbox"));
            }
        }
        self.ensure_stream(stream)?;
        let previous = self.streams[stream].log.last_sequence()?;
        if self.custody_signers && !fanout(&event.subject) {
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
                return Ok(old);
            }
        }
        if let Some(hold) = &self.hold {
            event.source = format!("ephemeral-hold/{}/{}", hold.run, event.source);
        }
        let envelope = if fanout(&event.subject) {
            Envelope::signed(event, 0, String::new(), &self.key)?
        } else {
            self.streams[stream].log.append(event, &self.key)?
        };
        if envelope.seq != 0 && envelope.seq <= previous {
            return Ok(envelope);
        }
        if envelope.seq != 0
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
    Subscribe(Consumer, oneshot::Sender<Result<String>>),
    Poll(String, usize, oneshot::Sender<Result<Vec<Delivery>>>),
    Ack(String, String, u64, oneshot::Sender<Result<()>>),
    Unsubscribe(String, oneshot::Sender<Result<()>>),
    IdentitySigner(SigningKey, Option<Ulid>, oneshot::Sender<Vec<HoldRange>>),
    HoldRanges(oneshot::Sender<Vec<HoldRange>>),
    Shutdown(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct BusHandle {
    tx: mpsc::Sender<Command>,
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
        Ok(Self { tx })
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
