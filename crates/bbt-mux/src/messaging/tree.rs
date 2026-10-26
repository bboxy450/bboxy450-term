//! A swarm's persistent filesystem of folders, inbox sinks and links.
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ulid::Ulid;

use super::wire::MessageKind;
use crate::registry::Class;
use crate::store::{Result, digest, events::Draft, invalid, sqlite::SqliteStore};
pub use bbt_proto::messaging::{Arrival, EntryKind, Filter, Node, Rate, Recipient, Right};

const MAX_NODES: usize = 16_384;
const MAX_STEPS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Operator,
    Seat { seat: Ulid, class: Class },
}
pub(crate) struct Destination {
    pub path: String,
    pub read: bool,
    pub post: bool,
}
impl Actor {
    pub fn seat(self) -> Option<Ulid> {
        match self {
            Self::Operator => None,
            Self::Seat { seat, .. } => Some(seat),
        }
    }
    pub fn controls(self) -> bool {
        matches!(
            self,
            Self::Operator
                | Self::Seat {
                    class: Class::Oracle,
                    ..
                }
        )
    }
}

trait RightExt {
    fn name(self) -> &'static str;
}
impl RightExt for Right {
    fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Link => "link",
            Self::Admin => "admin",
        }
    }
}

trait FilterExt {
    fn accepts(&self, kind: MessageKind, class: Class, priority: u8) -> bool;
}
impl FilterExt for Filter {
    fn accepts(&self, kind: MessageKind, class: Class, priority: u8) -> bool {
        (self.kinds.is_empty() || self.kinds.contains(&kind))
            && (self.classes.is_empty() || self.classes.contains(&class))
            && (self.priorities.is_empty() || self.priorities.contains(&priority))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    #[serde(rename = "seat")]
    pub recipient: Recipient,
    pub agent: Option<String>,
    pub arrivals: Vec<Arrival>,
}

pub struct Tree {
    store: SqliteStore,
    swarm: Ulid,
    nodes: BTreeMap<String, Node>,
    operator_recipient: Option<Recipient>,
}
pub(crate) fn schema(store: &SqliteStore) -> Result<()> {
    store.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS msg_tree(path TEXT PRIMARY KEY,kind TEXT NOT NULL,target TEXT,filter BLOB NOT NULL,rate BLOB NOT NULL,paused INTEGER NOT NULL,grants BLOB NOT NULL);
        CREATE TABLE IF NOT EXISTS msg_bus_outbox(id TEXT PRIMARY KEY,draft BLOB NOT NULL,sent INTEGER NOT NULL DEFAULT 0);")?;
    Ok(())
}
pub(crate) fn queue_event(tx: &rusqlite::Transaction<'_>, draft: &Draft) -> Result<()> {
    tx.execute(
        "INSERT INTO msg_bus_outbox(id,draft) VALUES(?1,?2)",
        params![draft.id.to_string(), serde_json::to_vec(draft)?],
    )?;
    Ok(())
}
impl Tree {
    pub fn open(store: SqliteStore, swarm: Ulid) -> Result<Self> {
        if swarm == Ulid::nil() {
            return Err(invalid("invalid swarm"));
        }
        schema(&store)?;
        let nodes = {
            let db = store.connection()?;
            let mut query = db.prepare(
                "SELECT path,kind,target,filter,rate,paused,grants FROM msg_tree ORDER BY path",
            )?;
            let mut rows = query.query([])?;
            let mut nodes = BTreeMap::new();
            while let Some(row) = rows.next()? {
                if nodes.len() >= MAX_NODES {
                    return Err(invalid("channel tree node budget exceeded"));
                }
                let node = Node {
                    path: row.get(0)?,
                    kind: serde_json::from_str(&row.get::<_, String>(1)?)?,
                    target: row.get(2)?,
                    filter: serde_json::from_slice(&row.get::<_, Vec<u8>>(3)?)?,
                    rate: serde_json::from_slice(&row.get::<_, Vec<u8>>(4)?)?,
                    paused: row.get(5)?,
                    grants: serde_json::from_slice(&row.get::<_, Vec<u8>>(6)?)?,
                };
                nodes.insert(node.path.clone(), node);
            }
            nodes
        };
        let mut tree = Self {
            store,
            swarm,
            nodes,
            operator_recipient: None,
        };
        for path in [
            "", "seats", "channels", "groups", "swarms", "ext", "operator",
        ] {
            if !tree.nodes.contains_key(path) {
                tree.put(Actor::Operator, Node::folder(path))?;
            }
        }
        let swarm_path = format!("swarms/{swarm}");
        if !tree.nodes.contains_key(&swarm_path) {
            tree.put(Actor::Operator, Node::folder(swarm_path))?;
        }
        if !tree.nodes.contains_key("operator/inbox") {
            tree.put(
                Actor::Operator,
                Node {
                    kind: EntryKind::Sink,
                    ..Node::folder("operator/inbox")
                },
            )?;
        }
        Ok(tree)
    }
    pub(crate) fn set_operator(&mut self, device_id: Ulid) {
        self.operator_recipient = Some(Recipient::Operator(device_id));
    }
    pub(crate) fn set_operator_reference(
        &mut self,
        reference: bbt_proto::messaging::OperatorReference,
    ) {
        match reference.kind {
            bbt_proto::messaging::OperatorReferenceKind::DeviceDerived => {
                self.set_operator(reference.id)
            }
            bbt_proto::messaging::OperatorReferenceKind::OperatorLocal => {
                self.operator_recipient = Some(reference.recipient())
            }
        }
    }
    /// Whether `path` names a node of this swarm's tree.
    pub fn contains(&self, path: &str) -> bool {
        self.nodes.contains_key(path)
    }
    /// Bounded operator settings read over only the channel prefix, never the seat tree.
    pub(crate) fn attention_channels(&self, limit: usize) -> (Vec<String>, bool) {
        let mut paths: Vec<_> = self.nodes.range("channels/".to_owned()..)
            .map(|(path, _)| path).take_while(|path| path.starts_with("channels/"))
            .take(limit.saturating_add(1)).cloned().collect();
        let complete = paths.len() <= limit;
        paths.truncate(limit);
        (paths, complete)
    }
    /// The paths `actor` may write, in order, at most `limit` (decision #264: a refusal
    /// lists them).
    pub fn writable(&self, actor: Actor, limit: usize) -> Vec<String> {
        self.nodes
            .keys()
            .filter(|path| self.check(actor, path, Right::Write).is_ok())
            .take(limit)
            .cloned()
            .collect()
    }
    pub(crate) fn destinations(&self, actor: Actor) -> Vec<Destination> {
        self.nodes
            .keys()
            .filter(|path| !path.is_empty())
            .filter_map(|path| {
                let read = self.check(actor, path, Right::Read).is_ok();
                let post = self.check(actor, path, Right::Write).is_ok();
                (read || post).then(|| Destination {
                    path: path.clone(),
                    read,
                    post,
                })
            })
            .collect()
    }
    pub fn check(&self, actor: Actor, path: &str, right: Right) -> Result<()> {
        // The operator's private inbox is write-only for every seat, including
        // Oracle-class seats. The trusted operator retains its own read access.
        if right == Right::Read
            && matches!(actor, Actor::Seat { .. })
            && (path == "operator" || path.starts_with("operator/"))
        {
            return Err(invalid(if self.check(actor, path, Right::Write).is_ok() {
                "write-only; it is in your destinations"
            } else {
                "operator inbox is private; no posting grant"
            }));
        }
        if actor.controls() {
            return Ok(());
        }
        let Some(seat) = actor.seat() else {
            return Err(invalid("missing actor"));
        };
        let mut current = path;
        loop {
            if let Some(rights) = self.nodes.get(current).and_then(|n| n.grants.get(&seat)) {
                if rights.contains(&right) || rights.contains(&Right::Admin) {
                    return Ok(());
                }
                // The nearest explicit entry replaces inheritance. In
                // particular [] revokes an inherited path grant.
                break;
            }
            if current.is_empty() {
                break;
            }
            current = parent(current);
        }
        if right == Right::Read && self.check(actor, path, Right::Write).is_ok() {
            return Err(invalid("write-only; it is in your destinations"));
        }
        Err(invalid(format!(
            "folder {} missing {} right",
            if path.is_empty() { "bboxy450://" } else { path },
            right.name()
        )))
    }
    pub fn get(&self, path: &str) -> Option<&Node> {
        self.nodes.get(path)
    }
    pub fn ls(&self, actor: Actor, path: &str) -> Result<Vec<Node>> {
        let path = normalize(path, false)?;
        self.check(actor, &path, Right::Read)?;
        let node = self
            .nodes
            .get(&path)
            .ok_or_else(|| invalid("tree path not found"))?;
        if node.kind != EntryKind::Folder {
            return Ok(vec![node.clone()]);
        }
        Ok(self.children(&path).cloned().collect())
    }
    fn children<'a>(&'a self, path: &'a str) -> impl Iterator<Item = &'a Node> {
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        self.nodes
            .range(prefix.clone()..)
            .take_while(move |(p, _)| p.starts_with(&prefix))
            .filter(move |(p, _)| p.as_str() != path && parent(p) == path)
            .map(|(_, n)| n)
    }
    pub fn put(&mut self, actor: Actor, mut node: Node) -> Result<()> {
        node.path = normalize(&node.path, false)?;
        if let Some(target) = &mut node.target {
            *target = normalize(target, false)?;
        }
        if node.rate.batch_ms > 86_400_000
            || node.filter.kinds.len() > 5
            || node.filter.classes.len() > 2
            || node.filter.priorities.len() > 10
            || node.filter.priorities.iter().any(|p| *p > 9)
            || node.grants.len() > 4096
            || node
                .grants
                .iter()
                .any(|(id, rights)| *id == Ulid::nil() || rights.len() > 4)
            || serde_json::to_vec(&node)?.len() > 64 * 1024
        {
            return Err(invalid("invalid tree policy bounds"));
        }
        if node.kind == EntryKind::Link {
            let target = node
                .target
                .as_deref()
                .ok_or_else(|| invalid("link target missing"))?;
            if !self.nodes.contains_key(target) {
                return Err(invalid("link target missing"));
            }
            self.check(actor, parent(&node.path), Right::Link)?;
            if !actor.controls() {
                let own = format!("seats/{}/inbox", actor.seat().unwrap());
                if target != own
                    || !node.grants.is_empty()
                    || self.nodes.get(&node.path).is_some_and(|old| {
                        old.kind != EntryKind::Link || old.target.as_deref() != Some(&own)
                    })
                {
                    return Err(invalid("worker may link only its own inbox"));
                }
            }
        } else {
            self.check(actor, &node.path, Right::Admin)?;
        }
        if node.kind == EntryKind::Sink && node.path != "operator/inbox" {
            sink(&node.path)?;
        }
        if node.kind != EntryKind::Link && node.target.is_some() {
            return Err(invalid("only links carry a target"));
        }
        if !node.path.is_empty()
            && self
                .nodes
                .get(parent(&node.path))
                .is_none_or(|n| n.kind != EntryKind::Folder)
        {
            return Err(invalid("parent is not a folder"));
        }
        if self.nodes.get(&node.path) == Some(&node) {
            return Ok(());
        }
        if !self.nodes.contains_key(&node.path) && self.nodes.len() >= MAX_NODES {
            return Err(invalid("channel tree full"));
        }
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO msg_tree VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(path) DO UPDATE SET kind=excluded.kind,target=excluded.target,filter=excluded.filter,rate=excluded.rate,paused=excluded.paused,grants=excluded.grants",params![node.path,serde_json::to_string(&node.kind)?,node.target,serde_json::to_vec(&node.filter)?,serde_json::to_vec(&node.rate)?,node.paused,serde_json::to_vec(&node.grants)?])?;
        queue_event(
            &tx,
            &Draft::new(
                format!(
                    "swarm/{}/tree/{}/changed",
                    self.swarm,
                    &digest(node.path.as_bytes())[..16]
                ),
                "messaging",
                serde_json::json!({"actor":actor.seat(),"node":node}),
            ),
        )?;
        tx.commit()?;
        self.nodes.insert(node.path.clone(), node);
        Ok(())
    }
    pub fn remove(&mut self, actor: Actor, path: &str) -> Result<()> {
        let path = normalize(path, false)?;
        let Some(node) = self.nodes.get(&path) else {
            return Ok(());
        };
        if path.is_empty() || self.children(&path).next().is_some() {
            return Err(invalid("folder is nonempty or protected root"));
        }
        if !actor.controls() {
            self.check(actor, parent(&path), Right::Link)?;
            if node.kind != EntryKind::Link
                || node.target != actor.seat().map(|s| format!("seats/{s}/inbox"))
            {
                return Err(invalid("worker may remove only its own inbox link"));
            }
        }
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM msg_tree WHERE path=?1", [&path])?;
        queue_event(
            &tx,
            &Draft::new(
                format!(
                    "swarm/{}/tree/{}/removed",
                    self.swarm,
                    &digest(path.as_bytes())[..16]
                ),
                "messaging",
                serde_json::json!({"actor":actor.seat(),"path":path}),
            ),
        )?;
        tx.commit()?;
        self.nodes.remove(&path);
        Ok(())
    }
    pub fn pause(&mut self, actor: Actor, path: &str, paused: bool) -> Result<()> {
        let path = normalize(path, false)?;
        let mut node = self
            .nodes
            .get(&path)
            .cloned()
            .ok_or_else(|| invalid("tree path missing"))?;
        if !actor.controls()
            && (node.kind != EntryKind::Link
                || node.target != actor.seat().map(|s| format!("seats/{s}/inbox")))
        {
            return Err(invalid("only a controller may pause another sink"));
        }
        node.paused = paused;
        self.put(actor, node)
    }
    pub fn grant(
        &mut self,
        actor: Actor,
        path: &str,
        seat: Ulid,
        rights: Vec<Right>,
    ) -> Result<()> {
        let path = normalize(path, false)?;
        self.check(actor, &path, Right::Admin)?;
        if !actor.controls() {
            return Err(invalid("only a controller may change tree grants"));
        }
        let mut node = self
            .nodes
            .get(&path)
            .cloned()
            .ok_or_else(|| invalid("tree path missing"))?;
        if seat == Ulid::nil() || rights.len() > 4 {
            return Err(invalid("invalid tree grant"));
        }
        node.grants.insert(seat, rights);
        self.put(actor, node)
    }
    pub fn resolve(
        &self,
        actor: Actor,
        pattern: &str,
        kind: MessageKind,
        class: Class,
        priority: u8,
    ) -> Result<Vec<Route>> {
        let pattern = normalize(pattern, true)?;
        let (base, mode) = if let Some(p) = pattern.strip_suffix("/**") {
            (p, 2)
        } else if let Some(p) = pattern.strip_suffix("/*") {
            (p, 1)
        } else {
            (pattern.as_str(), 0)
        };
        self.check(actor, base, Right::Write)?;
        let mut walk = Walk {
            tree: self,
            kind,
            class,
            priority,
            seen: BTreeSet::new(),
            steps: 0,
            bytes: 0,
            routes: BTreeMap::new(),
        };
        walk.visit(
            base,
            base,
            mode,
            Arrival {
                path: base.into(),
                links: Vec::new(),
                paused: false,
                batch_ms: 0,
                coalesce: false,
            },
        )?;
        Ok(walk.routes.into_values().collect())
    }
}
struct Walk<'a> {
    tree: &'a Tree,
    kind: MessageKind,
    class: Class,
    priority: u8,
    seen: BTreeSet<String>,
    steps: usize,
    bytes: usize,
    routes: BTreeMap<(Recipient, Option<String>), Route>,
}
impl Walk<'_> {
    fn visit(&mut self, path: &str, alias: &str, mode: u8, mut arrival: Arrival) -> Result<()> {
        self.steps += 1;
        if self.steps > MAX_STEPS || self.seen.len() >= 64 {
            return Err(invalid("channel resolution step budget exceeded"));
        }
        if !self.seen.insert(path.into()) {
            return Ok(());
        }
        let node = self
            .tree
            .nodes
            .get(path)
            .ok_or_else(|| invalid(format!("dangling tree path {path}")))?;
        if !node.filter.accepts(self.kind, self.class, self.priority) {
            self.seen.remove(path);
            return Ok(());
        }
        arrival.paused |= node.paused;
        arrival.batch_ms = arrival.batch_ms.max(node.rate.batch_ms);
        arrival.coalesce |= node.rate.coalesce;
        match node.kind {
            EntryKind::Sink => {
                if mode != 0 {
                    return Err(invalid("cannot publish wildcards below a sink"));
                }
                let (recipient, agent) = if path == "operator/inbox" {
                    (
                        self.tree.operator_recipient.ok_or_else(|| {
                            invalid("held: operator device custody is unavailable")
                        })?,
                        None,
                    )
                } else {
                    let (seat, agent) = sink(path)?;
                    (Recipient::Seat(seat), agent)
                };
                arrival.path = alias.into();
                let route = self
                    .routes
                    .entry((recipient, agent.clone()))
                    .or_insert_with(|| Route {
                        recipient,
                        agent,
                        arrivals: Vec::new(),
                    });
                if !route.arrivals.contains(&arrival) {
                    if route.arrivals.len() >= 64 {
                        return Err(invalid("too many distinct arrival paths for one sink"));
                    }
                    let size = serde_json::to_vec(&arrival)?.len();
                    if serde_json::to_vec(&route.arrivals)?.len() + size > 48 * 1024
                        || self.bytes + size > 8 * 1024 * 1024
                    {
                        return Err(invalid("channel resolution metadata byte budget exceeded"));
                    }
                    self.bytes += size;
                    route.arrivals.push(arrival);
                }
                if self.routes.len() > 4096 {
                    return Err(invalid("too many message recipients"));
                }
            }
            EntryKind::Link => {
                arrival.links.push(path.into());
                self.visit(node.target.as_deref().unwrap(), alias, mode, arrival)?;
            }
            EntryKind::Folder => {
                if mode == 0 {
                    return Err(invalid("publish to folder members with /* or /**"));
                }
                for child in self.tree.children(path) {
                    let child_alias = format!("{alias}/{}", child.path.rsplit('/').next().unwrap());
                    let mut target = Some(child);
                    let mut links = BTreeSet::new();
                    while let Some(n) = target.filter(|n| n.kind == EntryKind::Link) {
                        if !links.insert(n.path.as_str()) || links.len() > 64 {
                            target = None;
                            break;
                        }
                        target = self.tree.nodes.get(n.target.as_deref().unwrap());
                    }
                    let target_kind = target.map(|n| n.kind);
                    if target_kind.is_none() {
                        continue;
                    }
                    if mode == 1 && matches!(target_kind, Some(EntryKind::Folder)) {
                        continue;
                    }
                    let child_mode = if matches!(target_kind, Some(EntryKind::Folder)) {
                        2
                    } else {
                        0
                    };
                    self.visit(&child.path, &child_alias, child_mode, arrival.clone())?;
                }
            }
        }
        self.seen.remove(path);
        Ok(())
    }
}
pub fn normalize(path: &str, pattern: bool) -> Result<String> {
    let path = if let Some(channel) = path.strip_prefix('#') {
        format!("channels/{channel}")
    } else {
        path.strip_prefix("bboxy450://").unwrap_or(path).to_string()
    };
    let path = path.trim_end_matches('/');
    let tokens: Vec<_> = path.split('/').collect();
    if path.len() > 230
        || tokens.len() > 14
        || (!path.is_empty()
            && tokens.iter().enumerate().any(|(i, t)| {
                t.is_empty()
                    || matches!(*t, "." | "..")
                    || t.chars().any(|c| {
                        c.is_control()
                            || c.is_whitespace()
                            || matches!(c, '\\' | '>' | '?' | '%' | '#')
                    })
                    || (t.contains('*')
                        && !(pattern && i + 1 == tokens.len() && matches!(*t, "*" | "**")))
            }))
    {
        return Err(invalid("invalid or overlong channel path"));
    }
    Ok(path.into())
}
pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}
pub fn sink(path: &str) -> Result<(Ulid, Option<String>)> {
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() < 3 || parts[0] != "seats" {
        return Err(invalid("invalid seat sink"));
    }
    let seat = parts[1].parse::<Ulid>()?;
    if seat == Ulid::nil() {
        return Err(invalid("invalid seat sink"));
    }
    match parts.as_slice() {
        [_, _, "inbox"] => Ok((seat, None)),
        [_, _, "agents", agent] if !agent.is_empty() && agent.len() <= 128 => {
            Ok((seat, Some(agent.to_string())))
        }
        _ => Err(invalid("invalid seat sink")),
    }
}
