//! Repository-local Markdown mirror and weighted graph projections.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use notify::{RecursiveMode, Watcher};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use crate::store::{Result, Store, digest, events::Draft, invalid, sqlite::SqliteStore};

const MAX_DOCUMENT: u64 = 2 * 1024 * 1024;

/// A decision name refused before any intent, file or number is committed.
#[derive(Debug)]
pub(crate) struct DecisionFilenameRequired {
    expected: String,
}

impl std::fmt::Display for DecisionFilenameRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "decision filename required: {}; create with decisions/<slug>.md to let the Index assign its number",
            self.expected
        )
    }
}

impl std::error::Error for DecisionFilenameRequired {}

fn decision_birth_path(path: &str, number: u64) -> Result<String> {
    let path = Path::new(path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let prefixed = stem
        .strip_prefix('d')
        .and_then(|rest| rest.split_once('-'))
        .filter(|(digits, _)| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    let slug = prefixed.map_or(stem, |(_, slug)| slug);
    let expected = format!(
        "decisions/d{number}-{}.md",
        if slug.is_empty() { "<slug>" } else { slug }
    );
    if path.parent() != Some(Path::new("decisions"))
        || slug.is_empty()
        || prefixed.is_some_and(|(digits, _)| digits.parse::<u64>() != Ok(number))
    {
        return Err(Box::new(DecisionFilenameRequired { expected }));
    }
    Ok(expected)
}

fn record_number(metadata: &Value) -> Option<u64> {
    metadata
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileState {
    len: u64,
    modified: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub to: String,
    pub relation: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub path: String,
    pub metadata: Value,
    pub body: String,
    pub digest: String,
    pub error: Option<String>,
    pub links: Vec<Link>,
}

pub fn wikilinks(text: &str) -> Vec<Link> {
    let mut result = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        let label = &rest[..end];
        let target = label
            .split_once('|')
            .map_or(label, |(target, _)| {
                target.strip_suffix('\\').unwrap_or(target)
            })
            .trim();
        rest = &rest[end + 2..];
        if target.is_empty() {
            continue;
        }
        let (mut relation, mut weight) = ("ref".to_string(), 1.);
        if let Some(annotation) = rest.strip_prefix('{').and_then(|s| s.split_once('}')) {
            let (rel, w) = annotation.0.split_once(':').unwrap_or((annotation.0, "1"));
            if !rel.is_empty()
                && let Ok(value) = w.parse::<f64>()
                && value.is_finite()
                && value > 0.
            {
                relation = rel.to_string();
                weight = value;
            }
            rest = annotation.1;
        }
        result.push(Link {
            to: target.to_string(),
            relation,
            weight,
        });
    }
    result
}

impl Document {
    pub fn parse(path: &str, text: &str) -> Self {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(path.as_bytes());
        let fallback = Ulid::from_bytes(hash[..16].try_into().unwrap()).to_string();
        let mut doc = Self {
            id: fallback,
            path: path.into(),
            metadata: json!({}),
            body: text.into(),
            digest: digest(text.as_bytes()),
            error: None,
            links: Vec::new(),
        };
        let source = text.strip_prefix('\u{feff}').unwrap_or(text);
        if let Some(after) = source
            .strip_prefix("---\n")
            .or_else(|| source.strip_prefix("---\r\n"))
        {
            let mut offset = 0;
            let closing = after.split_inclusive('\n').find_map(|line| {
                let start = offset;
                offset += line.len();
                matches!(line.trim_end_matches(['\r', '\n']), "---" | "...")
                    .then_some((start, offset))
            });
            if let Some((end, body_start)) = closing {
                let (front, body) = (&after[..end], &after[body_start..]);
                match serde_yaml::from_str::<Value>(front) {
                    Ok(value) if value.is_object() => {
                        doc.metadata = value;
                        doc.body = body.into();
                    }
                    Ok(_) => doc.error = Some("frontmatter must be a mapping".into()),
                    Err(error) => doc.error = Some(format!("broken frontmatter: {error}")),
                }
            } else {
                doc.error = Some("frontmatter has no closing delimiter".into());
            }
        }
        if let Some(id) = doc.metadata.get("id").and_then(Value::as_str) {
            if id.parse::<Ulid>().is_ok() {
                doc.id = id.into();
            } else {
                doc.error = Some("frontmatter id is not a ULID".into());
            }
        }
        if doc.error.is_none() && record_number(&doc.metadata).is_none() {
            doc.error = Some("frontmatter number must be a positive integer".into());
        }
        doc.links = wikilinks(text);
        if let Some(links) = doc.metadata.get("links").and_then(Value::as_array) {
            for link in links {
                if let Some(to) = link.get("to").and_then(Value::as_str) {
                    let target = wikilinks(to)
                        .first()
                        .map(|l| l.to.clone())
                        .unwrap_or_else(|| to.into());
                    doc.links.push(Link {
                        to: target,
                        relation: link
                            .get("rel")
                            .and_then(Value::as_str)
                            .unwrap_or("ref")
                            .into(),
                        weight: link
                            .get("weight")
                            .and_then(Value::as_f64)
                            .filter(|w| w.is_finite() && *w > 0.)
                            .unwrap_or(1.),
                    });
                }
            }
        }
        for (field, rel) in [
            ("parent", "parent"),
            ("depends_on", "depends"),
            ("children", "child"),
            ("assignee", "assignee"),
            ("delegated_by", "delegated_by"),
            ("produced_by", "produced_by"),
            ("for", "for"),
            ("derived_from", "derived_from"),
            ("created_by", "wasAttributedTo"),
            ("updated_by", "wasAttributedTo"),
        ] {
            let Some(value) = doc.metadata.get(field) else {
                continue;
            };
            let values: Vec<&Value> = value
                .as_array()
                .map(|a| a.iter().collect())
                .unwrap_or_else(|| vec![value]);
            for value in values {
                if let Some(to) = value.as_str() {
                    doc.links.push(Link {
                        to: to.into(),
                        relation: rel.into(),
                        weight: if rel == "parent" { 3. } else { 2. },
                    });
                }
            }
        }
        let mut seen = BTreeSet::new();
        doc.links.retain(|link| {
            seen.insert((
                link.to.clone(),
                link.relation.clone(),
                link.weight.to_bits(),
            ))
        });
        doc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphKind {
    Markdown,
    Task,
    Execution,
    Artifact,
    Provenance,
}

#[derive(Clone)]
pub struct Index {
    root: PathBuf,
    store: SqliteStore,
    activity: crate::store::events::EventLog,
    activity_sink: Option<(crate::bus::BusHandle, tokio::runtime::Handle)>,
    issued_by: Option<String>,
    pub documents: BTreeMap<String, Document>,
    pub graphs: BTreeMap<GraphKind, DiGraph<String, Link>>,
    // A reopened index cold-reconciles its root. Persisted documents alone do
    // not prove that an on-disk file still has the same contents.
    file_states: BTreeMap<String, FileState>,
    #[cfg(test)]
    document_reads: usize,
}

/// A checked write expects either these exact source bytes or a new path.
pub(crate) struct Update {
    pub path: String,
    pub metadata: Value,
    pub body: String,
    pub expected: Option<String>,
}

impl Index {
    /// Called only with attribution derived by the admitted identity provider.
    pub(crate) fn set_issued_by(&mut self, issued_by: Option<String>) {
        self.issued_by = issued_by;
    }
    /// The bundled knowledge provider's data directory is this existing root.
    pub(crate) fn knowledge_root(&self) -> &Path {
        &self.root
    }

    pub fn open(root: &Path, store: SqliteStore) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(root)?;
        let mut index = Self {
            root: root.canonicalize()?,
            activity: crate::store::events::EventLog::open(
                store
                    .path
                    .parent()
                    .ok_or_else(|| invalid("index store has no object directory"))?,
            )?,
            activity_sink: None,
            issued_by: None,
            store,
            documents: BTreeMap::new(),
            graphs: BTreeMap::new(),
            file_states: BTreeMap::new(),
            #[cfg(test)]
            document_reads: 0,
        };
        index.store.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS document_outbox(seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT NOT NULL UNIQUE,payload BLOB NOT NULL)")?;
        for (_, bytes) in index.store.list("doc/")? {
            let doc: Document = serde_json::from_slice(&bytes)?;
            index.documents.insert(doc.path.clone(), doc);
        }
        index.rebuild_graphs();
        Ok(index)
    }
    pub fn scan(&mut self) -> Result<Vec<Draft>> {
        bbt_rt::assert_not_ui_thread();
        let mut paths = Vec::new();
        collect(&self.root, &mut paths)?;
        let mut seen = BTreeSet::new();
        let mut changed = false;
        let mut events = {
            let db = self.store.connection()?;
            let mut statement = db.prepare("SELECT payload FROM document_outbox ORDER BY seq")?;
            let pending = statement
                .query_map([], |r| r.get::<_, Vec<u8>>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            pending
                .into_iter()
                .map(|bytes| Ok(serde_json::from_slice(&bytes)?))
                .collect::<Result<Vec<Draft>>>()?
        };
        for (path, metadata) in paths {
            let relative = path.strip_prefix(&self.root)?.to_string_lossy().to_string();
            let state = FileState {
                len: metadata.len(),
                modified: metadata.modified()?,
            };
            if self.file_states.get(&relative) == Some(&state) {
                seen.insert(relative);
                continue;
            }
            #[cfg(test)]
            {
                self.document_reads += 1;
            }
            let mut bytes = Vec::new();
            std::fs::File::open(&path)?
                .take(MAX_DOCUMENT + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_DOCUMENT {
                oversized(&path, bytes.len() as u64);
                continue;
            }
            seen.insert(relative.clone());
            let text = String::from_utf8(bytes)?;
            if self
                .documents
                .get(&relative)
                .is_some_and(|old| old.digest == digest(text.as_bytes()))
            {
                self.file_states.insert(relative, state);
                continue;
            }
            let doc = Document::parse(&relative, &text);
            let event_start = events.len();
            let kind = if self.documents.contains_key(&relative) {
                "modified"
            } else {
                "created"
            };
            let previous = self
                .documents
                .get(&relative)
                .map(|d| d.metadata["status"].clone())
                .unwrap_or(Value::Null);
            if matches!(doc.metadata["type"].as_str(), Some("task" | "epic" | "bug"))
                && doc.metadata["status"].is_string()
                && previous != doc.metadata["status"]
            {
                events.push(Draft::new(format!("task/{}/status",doc.id),"index",json!({"from":previous,"to":doc.metadata["status"],"path":relative,"swarm_id":doc.metadata["swarm_id"]})));
            }
            let payload = json!({"path":relative,"digest":doc.digest,"frontmatter":doc.metadata,"error":doc.error});
            events.push(Draft::new(
                format!("doc/{}/{kind}", doc.id),
                "index",
                payload.clone(),
            ));
            events.push(Draft::new(
                format!("doc/path/{relative}/{kind}"),
                "index",
                payload,
            ));
            let mut db = self.store.connection()?;
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                rusqlite::params![format!("doc/{relative}"), serde_json::to_vec(&doc)?],
            )?;
            for event in &events[event_start..] {
                tx.execute(
                    "INSERT INTO document_outbox(id,payload) VALUES(?1,?2)",
                    rusqlite::params![event.id.to_string(), serde_json::to_vec(event)?],
                )?;
            }
            tx.commit()?;
            self.file_states.insert(relative.clone(), state);
            self.documents.insert(relative, doc);
            changed = true;
        }
        let removed: Vec<_> = self
            .documents
            .keys()
            .filter(|p| !seen.contains(*p))
            .cloned()
            .collect();
        for path in removed {
            if let Some(doc) = self.documents.remove(&path) {
                let event_start = events.len();
                events.push(Draft::new(
                    format!("doc/{}/deleted", doc.id),
                    "index",
                    json!({"path":path,"digest":doc.digest}),
                ));
                events.push(Draft::new(
                    format!("doc/path/{path}/deleted"),
                    "index",
                    json!({"path":path}),
                ));
                let mut db = self.store.connection()?;
                let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                tx.execute("DELETE FROM kv WHERE key=?1", [format!("doc/{path}")])?;
                for event in &events[event_start..] {
                    tx.execute(
                        "INSERT INTO document_outbox(id,payload) VALUES(?1,?2)",
                        rusqlite::params![event.id.to_string(), serde_json::to_vec(event)?],
                    )?;
                }
                tx.commit()?;
                changed = true;
            }
        }
        self.file_states.retain(|path, _| seen.contains(path));
        if changed {
            self.rebuild_graphs();
        }
        Ok(events)
    }
    /// Called only after the bus confirms append; retry uses the same event ID.
    pub fn acknowledge(&self, event: Ulid) -> Result<()> {
        self.store.connection()?.execute(
            "DELETE FROM document_outbox WHERE id=?1",
            [event.to_string()],
        )?;
        Ok(())
    }
    pub fn rebuild_graphs(&mut self) {
        self.graphs = [
            GraphKind::Markdown,
            GraphKind::Task,
            GraphKind::Execution,
            GraphKind::Artifact,
            GraphKind::Provenance,
        ]
        .into_iter()
        .map(|kind| (kind, DiGraph::new()))
        .collect();
        let mut aliases: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for doc in self.documents.values() {
            for alias in [
                doc.id.clone(),
                doc.path.trim_end_matches(".md").to_string(),
                Path::new(&doc.path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ] {
                aliases.entry(alias).or_default().push(doc.id.clone());
            }
            if let Some(number) = doc.metadata.get("number").and_then(Value::as_u64)
                && number > 0
            {
                aliases
                    .entry(format!("#{number}"))
                    .or_default()
                    .push(doc.id.clone());
            }
        }
        let mut nodes: BTreeMap<GraphKind, BTreeMap<String, NodeIndex>> = BTreeMap::new();
        for doc in self.documents.values() {
            for graph in self.graphs.values_mut() {
                graph.add_node(doc.id.clone());
            }
        }
        for (kind, graph) in &self.graphs {
            nodes.insert(
                *kind,
                graph
                    .node_indices()
                    .map(|n| (graph[n].clone(), n))
                    .collect(),
            );
        }
        for doc in self.documents.values() {
            for link in &doc.links {
                let mut target = wikilinks(&link.to)
                    .first()
                    .map(|l| l.to.clone())
                    .unwrap_or_else(|| link.to.clone());
                if let Some(ids) = aliases.get(&target) {
                    let unique: BTreeSet<_> = ids.iter().collect();
                    if unique.len() == 1 {
                        target = ids[0].clone();
                    }
                }
                let mut kinds = vec![GraphKind::Markdown];
                match link.relation.as_str() {
                    "parent" | "child" | "depends" | "blocks" => kinds.push(GraphKind::Task),
                    "assignee" | "delegated_by" => kinds.push(GraphKind::Execution),
                    "produced_by" | "for" => kinds.push(GraphKind::Artifact),
                    "revision" | "evidence" | "derived_from" | "generated" | "used"
                    | "wasAttributedTo" => kinds.push(GraphKind::Provenance),
                    _ => (),
                }
                for kind in kinds {
                    let graph = self.graphs.get_mut(&kind).unwrap();
                    let map = nodes.get_mut(&kind).unwrap();
                    let from = map[&doc.id];
                    let to = *map
                        .entry(target.clone())
                        .or_insert_with(|| graph.add_node(target.clone()));
                    graph.add_edge(from, to, link.clone());
                }
            }
        }
    }
    /// Only mux-authored writes are stamped; watching an external edit does not
    /// attribute it to this signer. The caller supplies its actual seat leaf.
    pub fn write(
        &self,
        path: &str,
        metadata: Value,
        body: &str,
        seat: Ulid,
        key: &SigningKey,
    ) -> Result<()> {
        self.write_inner(
            Update {
                path: path.into(),
                metadata,
                body: body.into(),
                expected: None,
            },
            &crate::provenance::author::Author::seat(seat, None, key),
            false,
        )
        .map(|_| ())
    }
    /// Production activity has one append/sign/route owner; fixtures may append directly.
    pub(crate) fn route_activity(
        &mut self,
        bus: crate::bus::BusHandle,
        handle: tokio::runtime::Handle,
    ) {
        self.activity_sink = Some((bus, handle));
    }
    pub(crate) fn append_activity(
        &self,
        mut event: Draft,
        key: &SigningKey,
    ) -> Result<crate::store::events::Envelope> {
        use crate::store::Events;
        if event.issued_by.is_some() && event.issued_by != self.issued_by {
            return Err(invalid("test_account_label_mismatch: activity attribution"));
        }
        event.issued_by = self.issued_by.clone();
        if let Some((bus, handle)) = &self.activity_sink {
            handle.block_on(bus.publish_activity(event, key.clone()))
        } else {
            self.activity.append(event, key)
        }
    }
    pub(crate) fn project_activity(&self) -> Result<crate::provenance::projection::Projection> {
        crate::provenance::projection::step(&self.root, &self.store, &self.activity)
    }
    pub(crate) fn mirror(&self) -> SqliteStore {
        self.store.clone()
    }
    pub(crate) fn activity(&self) -> &crate::store::events::EventLog {
        &self.activity
    }
    /// Read current bounded bytes without trusting the watcher's cached copy.
    pub(crate) fn current(&self, path: &str) -> Result<Option<Document>> {
        bbt_rt::assert_not_ui_thread();
        use std::os::unix::fs::OpenOptionsExt;
        let destination = self.destination(path)?;
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(destination)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !file.metadata()?.is_file() {
            return Err(invalid("document must be a regular file"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_DOCUMENT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_DOCUMENT {
            return Err(invalid("document exceeds write bound"));
        }
        Ok(Some(Document::parse(path, &String::from_utf8(bytes)?)))
    }
    /// Numbers come from the current files, not the watcher's cached mirror.
    /// Refuse an incomplete/ambiguous store instead of guessing a free handle.
    fn record_numbers(&self) -> Result<BTreeMap<u64, String>> {
        let mut paths = Vec::new();
        collect(&self.root, &mut paths)?;
        paths.sort_by(|a, b| a.0.cmp(&b.0));
        let mut numbers = BTreeMap::new();
        for (path, _) in paths {
            let relative = path
                .strip_prefix(&self.root)
                .ok()
                .and_then(Path::to_str)
                .ok_or_else(|| invalid("record path is not relative UTF-8"))?;
            let doc = self
                .current(relative)?
                .ok_or_else(|| invalid("record disappeared during number scan"))?;
            let number = record_number(&doc.metadata)
                .ok_or_else(|| invalid(format!("{relative}: invalid or missing record number")))?;
            if let Some(previous) = numbers.insert(number, relative.to_owned()) {
                return Err(invalid(format!(
                    "duplicate record number {number}: {previous} and {relative}"
                )));
            }
        }
        Ok(numbers)
    }
    pub(crate) fn by_number(&self, number: u64) -> Result<Option<Document>> {
        let numbers = self.record_numbers()?;
        let Some(path) = numbers.get(&number) else {
            return Ok(None);
        };
        let doc = self
            .current(path)?
            .ok_or_else(|| invalid("numbered record disappeared"))?;
        if record_number(&doc.metadata) != Some(number) {
            return Err(invalid("record number changed during lookup"));
        }
        Ok(Some(doc))
    }
    fn assign_number(&self, update: &mut Update, current: Option<&Document>) -> Result<()> {
        let map = update
            .metadata
            .as_object_mut()
            .ok_or_else(|| invalid("frontmatter must be a mapping"))?;
        let numbers = self.record_numbers()?;
        let number = if let Some(current) = current {
            record_number(&current.metadata)
                .ok_or_else(|| invalid("existing record has no valid number; repair the record"))?
        } else {
            numbers
                .last_key_value()
                .map_or(0, |(number, _)| *number)
                .checked_add(1)
                .ok_or_else(|| invalid("record number exhausted"))?
        };
        if let Some(supplied) = map.get("number") {
            if supplied.as_u64() != Some(number) {
                return Err(invalid(if current.is_some() {
                    "existing record number is immutable".into()
                } else {
                    format!("new record number must be next free number {number}")
                }));
            }
        } else {
            map.insert("number".into(), json!(number));
        }
        Ok(())
    }
    /// Reject a detected editor conflict; creation never replaces an existing file.
    #[cfg(test)]
    pub(crate) fn apply(&self, update: Update, seat: Ulid, key: &SigningKey) -> Result<()> {
        self.apply_author(
            update,
            &crate::provenance::author::Author::seat(seat, None, key),
        )
    }
    pub(crate) fn apply_author(
        &self,
        update: Update,
        author: &crate::provenance::author::Author<'_>,
    ) -> Result<()> {
        self.apply_author_path(update, author).map(|_| ())
    }
    /// Return the path actually published after assigning a new record's number.
    pub(crate) fn apply_author_path(
        &self,
        mut update: Update,
        author: &crate::provenance::author::Author<'_>,
    ) -> Result<String> {
        if update.metadata["id"]
            .as_str()
            .and_then(|id| id.parse::<Ulid>().ok())
            .is_none()
            || update.metadata["type"].as_str().is_none_or(str::is_empty)
        {
            return Err(invalid("frontmatter requires id and type"));
        }
        let timestamp: String = self.store.connection()?.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            [],
            |row| row.get(0),
        )?;
        update.metadata["updated"] = json!(timestamp);
        if update.expected.is_none() {
            update.metadata["created"] = json!(timestamp);
        }
        self.write_inner(update, author, true)
    }
    fn destination(&self, path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || path.extension().is_none_or(|e| e != "md")
        {
            return Err(invalid("write must be a relative Markdown path"));
        }
        let mut destination = self.root.clone();
        for component in path.components() {
            destination.push(component);
            match destination.symlink_metadata() {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(invalid("symlink in document path"));
                }
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(error.into());
                }
                _ => (),
            }
        }
        Ok(destination)
    }
    fn unchanged(&self, update: &Update) -> Result<()> {
        let current = self.current(&update.path)?;
        if current.as_ref().map(|doc| doc.digest.as_str()) != update.expected.as_deref() {
            return Err(invalid("document changed before write"));
        }
        Ok(())
    }
    fn write_inner(
        &self,
        mut update: Update,
        author: &crate::provenance::author::Author<'_>,
        guarded: bool,
    ) -> Result<String> {
        bbt_rt::assert_not_ui_thread();
        // One OS lock per store root spans the fresh scan, signature and
        // atomic publication. Closing this descriptor also releases it after
        // an error or process exit; no counter/reservation state is needed.
        let number_lock = std::fs::File::open(&self.root)?;
        number_lock.lock()?;
        let key = author.key;
        self.destination(&update.path)?;
        if guarded {
            self.unchanged(&update)?;
        }
        let current = self.current(&update.path)?;
        self.assign_number(&mut update, current.as_ref())?;
        let decision_birth = current.is_none() && update.metadata["type"] == "decision";
        if decision_birth {
            update.path = decision_birth_path(
                &update.path,
                record_number(&update.metadata).ok_or_else(|| invalid("record number missing"))?,
            )?;
            self.unchanged(&update)?;
        }
        let destination = self.destination(&update.path)?;
        let metadata = &mut update.metadata;
        let body = &update.body;
        let path = Path::new(&update.path);
        let map = metadata
            .as_object_mut()
            .ok_or_else(|| invalid("frontmatter must be a mapping"))?;
        map.remove("provenance");
        map.entry("created_by").or_insert(json!(author.id));
        map.insert("updated_by".into(), json!(author.id));
        let mut authorship = author.public.clone();
        if let Some(label) = &self.issued_by {
            if map
                .get("issued_by")
                .is_some_and(|value| {
                    value.as_str() != Some(label)
                        && current.as_ref().and_then(|doc| doc.metadata.get("issued_by"))
                            != Some(value)
                })
            {
                return Err(invalid("test_account_label_mismatch: record attribution"));
            }
            // A fetched record may retain its previous run's label. Its new
            // signature always uses this receiver's verified account context.
            map.insert("issued_by".into(), json!(label));
            authorship["issued_by"] = json!(label);
        }
        map.insert("authorship".into(), authorship);
        let front = serde_yaml::to_string(&metadata)?;
        let body_digest = digest(body.as_bytes());
        let front_digest = digest(front.as_bytes());
        let payload = format!("{front_digest}{body_digest}");
        let domain = b"bboxy/item/v1";
        let mut pae = format!("DSSEv1 {} ", domain.len()).into_bytes();
        pae.extend(domain);
        pae.extend(format!(" {} ", payload.len()).as_bytes());
        pae.extend(payload.as_bytes());
        metadata["provenance"] = json!({"digest":format!("sha256:{body_digest}"),"frontmatter_digest":format!("sha256:{front_digest}"),"signer":author.id,"sig":base64::engine::general_purpose::STANDARD.encode(key.sign(&pae).to_bytes())});
        let parent = destination.parent().ok_or_else(|| invalid("no parent"))?;
        std::fs::create_dir_all(parent)?;
        if !parent.canonicalize()?.starts_with(&self.root)
            || destination
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(invalid("write escapes index root"));
        }
        // The durable signed intent comes first; JSONL remains a projection.
        let mut intent = Draft::new(
            format!(
                "doc/{}/write_prepared",
                metadata["id"].as_str().unwrap_or("legacy")
            ),
            "mux/document",
            json!({"activity":"write","path":path,"body_digest":body_digest,"frontmatter_digest":front_digest,"agent":author.id,"used":metadata["derived_from"]}),
        );
        let artifact = crate::provenance::artifacts::summary(metadata);
        if let Some(artifact) = &artifact {
            intent.payload["artifact"] = artifact.clone();
        }
        intent.seat = author.seat;
        intent.generation = author.generation;
        let prepared = self.append_activity(intent, key)?;
        let mut projection = self.project_activity()?;
        for _ in 0..64 {
            if projection.cursor() >= prepared.seq {
                break;
            }
            projection = self.project_activity()?;
        }
        metadata["provenance"]["log"] = json!(
            projection
                .reference(&prepared, &self.store)?
                .ok_or_else(|| invalid("activity projection backlog; write intent retained"))?
        );
        let document_id = metadata["id"].as_str().unwrap_or("legacy").to_owned();
        let temp = parent.join(format!(".{}.tmp", Ulid::new()));
        struct Temporary(PathBuf);
        impl Drop for Temporary {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        let _temporary = Temporary(temp.clone());
        let rendered = format!("---\n{}---\n{body}", serde_yaml::to_string(metadata)?);
        if rendered.len() as u64 > MAX_DOCUMENT {
            return Err(invalid("document exceeds write bound"));
        }
        file.write_all(rendered.as_bytes())?;
        file.sync_all()?;
        // This catches edits during preparation. External editors do not share
        // a lock, so their rename after this check remains a documented race.
        self.destination(&update.path)?;
        if guarded {
            self.unchanged(&update)?;
        }
        if (guarded || decision_birth) && update.expected.is_none() {
            std::fs::hard_link(&temp, &destination)?;
        } else {
            std::fs::rename(&temp, &destination)?;
        }
        std::fs::File::open(parent)?.sync_all()?;
        let mut completed = Draft::new(
            format!("doc/{document_id}/written"),
            "mux/document",
            json!({"activity":"write_applied","prepared":prepared.event.id,"generated":[format!("sha256:{body_digest}")]}),
        );
        if let Some(artifact) = artifact {
            completed.payload["artifact"] = artifact;
        }
        completed.seat = author.seat;
        completed.generation = author.generation;
        self.append_activity(completed, key)?;
        self.project_activity()?;
        Ok(update.path)
    }
}

fn oversized(path: &Path, bytes: u64) {
    eprintln!(
        "IndexDocumentTooLarge: skipped {} ({bytes} bytes, limit {MAX_DOCUMENT})",
        path.display()
    );
}

fn collect(directory: &Path, paths: &mut Vec<(PathBuf, std::fs::Metadata)>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect(&entry.path(), paths)?;
        } else if entry.path().extension().is_some_and(|e| e == "md") {
            let metadata = entry.metadata()?;
            if metadata.len() > MAX_DOCUMENT {
                oversized(&entry.path(), metadata.len());
                continue;
            }
            paths.push((entry.path(), metadata));
        }
    }
    Ok(())
}

/// FSEvents/inotify only dirties one atomic bit. Bursts cannot overflow a path
/// queue and lose edits; the logic worker reconciles the final filesystem view.
pub struct FileWatch {
    _watcher: notify::RecommendedWatcher,
    dirty: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}
impl FileWatch {
    pub fn start(root: &Path) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let root = root.canonicalize()?;
        let watched_root = root.clone();
        let dirty = Arc::new(AtomicBool::new(true));
        let failed = Arc::new(AtomicBool::new(false));
        let d = dirty.clone();
        let f = failed.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                match event {
                    Ok(event) => {
                        // Reconciliation reads must not trigger another scan.
                        if matches!(event.kind, notify::EventKind::Access(_)) {
                            return;
                        }
                        if event.paths.is_empty()
                            || event
                                .paths
                                .iter()
                                .any(|path| path.starts_with(&watched_root))
                        {
                            d.store(true, Ordering::Release);
                        }
                    }
                    Err(_) => {
                        f.store(true, Ordering::Release);
                        d.store(true, Ordering::Release);
                    }
                }
            })?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            dirty,
            failed,
        })
    }
    pub fn changed(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "Disposable filesystem index/watcher fixtures."
)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant, UNIX_EPOCH};

    #[test]
    fn checked_writer_keeps_editor_changes_and_refuses_symlink_escapes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let index = Index::open(
            &root,
            SqliteStore::open(&temp.path().join("index")).unwrap(),
        )
        .unwrap();
        let seat = Ulid::new();
        let key = SigningKey::from_bytes(&[73; 32]);
        let path = "tasks/fixture.md";
        let metadata = json!({"id":Ulid::new(),"type":"task","status":"running"});
        index
            .apply(
                Update {
                    path: path.into(),
                    metadata: metadata.clone(),
                    body: "first\n".into(),
                    expected: None,
                },
                seat,
                &key,
            )
            .unwrap();
        let before = index.current(path).unwrap().unwrap();
        std::fs::write(root.join(path), "operator edit\n").unwrap();
        assert!(
            index
                .apply(
                    Update {
                        path: path.into(),
                        metadata: metadata.clone(),
                        body: "must not replace edit".into(),
                        expected: Some(before.digest)
                    },
                    seat,
                    &key
                )
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        assert_eq!(
            std::fs::read_to_string(root.join(path)).unwrap(),
            "operator edit\n"
        );
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
        assert!(
            index
                .apply(
                    Update {
                        path: "escape/never.md".into(),
                        metadata,
                        body: "never".into(),
                        expected: None
                    },
                    seat,
                    &key
                )
                .is_err()
        );
        assert!(std::fs::read_dir(outside).unwrap().next().is_none());
    }

    #[test]
    fn checked_creation_has_one_winner_and_does_not_overwrite_it() {
        let temp = tempfile::tempdir().unwrap();
        let index = Arc::new(
            Index::open(
                &temp.path().join(".bboxy"),
                SqliteStore::open(&temp.path().join("index")).unwrap(),
            )
            .unwrap(),
        );
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let id = Ulid::new();
        let workers: Vec<_> = (0..2)
            .map(|n| {
                let index = index.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    index.apply(
                        Update {
                            path: "artifact.md".into(),
                            metadata: json!({"id":id,"type":"artifact"}),
                            body: format!("creator {n}\n"),
                            expected: None,
                        },
                        Ulid::new(),
                        &SigningKey::from_bytes(&[n + 30; 32]),
                    )
                })
            })
            .collect();
        let successes = workers
            .into_iter()
            .map(|w| usize::from(w.join().unwrap().is_ok()))
            .sum::<usize>();
        assert_eq!(successes, 1);
        assert!(
            index
                .current("artifact.md")
                .unwrap()
                .unwrap()
                .body
                .starts_with("creator ")
        );
    }

    fn scan_and_ack(index: &mut Index) -> Vec<Draft> {
        let events = index.scan().unwrap();
        for event in &events {
            index.acknowledge(event.id).unwrap();
        }
        events
    }

    fn stamp(path: &Path, seconds: u64) {
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    }

    #[test]
    fn unchanged_documents_are_not_reread_and_same_length_edits_are_indexed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let state = temp.path().join("state");
        let mut index = Index::open(&root, SqliteStore::open(&state).unwrap()).unwrap();
        let path = root.join("note.md");
        std::fs::write(&path, "first [[peer]]").unwrap();
        stamp(&path, 10);
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert_eq!(index.document_reads, 1);
        for _ in 0..20 {
            assert!(scan_and_ack(&mut index).is_empty());
        }
        assert_eq!(
            index.document_reads, 1,
            "unchanged metadata must bypass file open/read"
        );
        std::fs::write(&path, "other [[peer]]").unwrap();
        stamp(&path, 20);
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert_eq!(index.documents["note.md"].body, "other [[peer]]");
        assert_eq!(index.document_reads, 2);
        // A metadata-only change reads once, without reparsing or publishing.
        stamp(&path, 30);
        assert!(scan_and_ack(&mut index).is_empty());
        assert_eq!(index.document_reads, 3);
        assert!(scan_and_ack(&mut index).is_empty());
        assert_eq!(index.document_reads, 3);
        drop(index);
        let mut index = Index::open(&root, SqliteStore::open(&state).unwrap()).unwrap();
        assert!(scan_and_ack(&mut index).is_empty());
        assert_eq!(
            index.document_reads, 1,
            "restart must cold-reconcile persisted documents"
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert!(index.documents.is_empty());
        assert!(index.file_states.is_empty());
    }

    #[test]
    fn oversized_documents_are_skipped_without_blocking_valid_edits() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let mut index = Index::open(
            &root,
            SqliteStore::open(&temp.path().join("state")).unwrap(),
        )
        .unwrap();
        let large = root.join("large.md");
        std::fs::File::create(&large)
            .unwrap()
            .set_len(MAX_DOCUMENT + 1)
            .unwrap();
        std::fs::write(root.join("good.md"), "valid").unwrap();
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert_eq!(index.documents.len(), 1);
        assert_eq!(index.documents["good.md"].body, "valid");
        assert_eq!(
            index.document_reads, 1,
            "oversized metadata is rejected before reading"
        );
        std::fs::write(&large, "small again").unwrap();
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert_eq!(index.documents["large.md"].body, "small again");
        std::fs::File::options()
            .write(true)
            .open(&large)
            .unwrap()
            .set_len(MAX_DOCUMENT + 1)
            .unwrap();
        std::fs::write(root.join("good.md"), "edited valid").unwrap();
        assert_eq!(scan_and_ack(&mut index).len(), 4);
        assert!(!index.documents.contains_key("large.md"));
        assert_eq!(index.documents["good.md"].body, "edited valid");
    }

    fn wait_for_change(watch: &FileWatch) {
        let until = Instant::now() + Duration::from_secs(5);
        while !watch.changed() {
            assert!(!watch.failed(), "filesystem watcher failed");
            assert!(Instant::now() < until, "in-root edit was not observed");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn settle(watch: &FileWatch) {
        let until = Instant::now() + Duration::from_secs(5);
        let mut quiet_since = Instant::now();
        while quiet_since.elapsed() < Duration::from_millis(500) {
            assert!(!watch.failed(), "filesystem watcher failed");
            assert!(Instant::now() < until, "filesystem watcher did not settle");
            if watch.changed() {
                quiet_since = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let mut index = Index::open(
            &root,
            SqliteStore::open(&temp.path().join("state")).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(temp.path().join("target/deep")).unwrap();
        std::fs::create_dir_all(temp.path().join(".git/objects")).unwrap();
        let watch = FileWatch::start(&root).unwrap();
        assert!(watch.changed());
        let path = root.join("note.md");
        std::fs::write(&path, "initial").unwrap();
        wait_for_change(&watch);
        scan_and_ack(&mut index);
        settle(&watch);
        let reads = index.document_reads;
        for i in 0..32 {
            std::fs::write(
                temp.path().join(format!("target/deep/{i}.md")),
                "build output",
            )
            .unwrap();
            std::fs::write(
                temp.path().join(format!(".git/objects/{i}.md")),
                "git output",
            )
            .unwrap();
        }
        std::fs::write(temp.path().join("README.md"), "outside the index").unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        let mut scans = 0;
        while Instant::now() < until {
            assert!(!watch.failed());
            if watch.changed() {
                scans += 1;
                scan_and_ack(&mut index);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(scans, 0, "sibling writes must not trigger a scan");
        assert_eq!(index.document_reads, reads);
        assert_eq!(index.documents.len(), 1);
        std::fs::write(&path, "edited inside .bboxy").unwrap();
        wait_for_change(&watch);
        assert_eq!(scan_and_ack(&mut index).len(), 2);
        assert_eq!(index.documents["note.md"].body, "edited inside .bboxy");
        assert_eq!(index.document_reads, reads + 1);
    }
}

#[cfg(test)]
mod test_account_attribution {
    use super::*;
    use crate::{
        identity::{AccountHead, Signed, TestAccount},
        store::Events,
    };
    #[test]
    fn verified_test_context_is_covered_by_document_and_activity_signatures() {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        let generated = TestAccount::open(&root.path().join("keys"), &run).unwrap();
        let account =
            SqliteStore::open(&crate::store::object_dir(&run, "account").unwrap()).unwrap();
        let head: Signed<AccountHead> =
            serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
        let device = head.payload.active_devices[0].device_id;
        let key = generated.key_store().load(device).unwrap().unwrap();
        let mut index = Index::open(
            &root.path().join(".bboxy"),
            SqliteStore::open(&root.path().join("index")).unwrap(),
        )
        .unwrap();
        index.set_issued_by(Some(generated.issued_by().into()));
        let author = crate::provenance::author::Author::operator(
            device,
            crate::messaging::wire::IssuerKind::GpgRoot,
            &key,
        );
        index.apply_author(Update {
            path: "notes/fixture.md".into(),
            metadata: json!({"id":Ulid::new().to_string(),"type":"note","title":"Test context",
                "created":"2026-10-24T00:00:00Z","updated":"2026-10-24T00:00:00Z"}),
            body: "private generated fixture\n".into(), expected: None,
        }, &author).unwrap();
        let record = index.current("notes/fixture.md").unwrap().unwrap();
        assert_eq!(record.metadata["issued_by"], generated.issued_by());
        assert_eq!(
            record.metadata["authorship"]["issued_by"],
            generated.issued_by()
        );
        let mut front = record.metadata.clone();
        let signature = front.as_object_mut().unwrap().remove("provenance").unwrap();
        let signed_digest = signature["frontmatter_digest"].as_str().unwrap();
        let signed_digest = signed_digest
            .strip_prefix("sha256:")
            .unwrap_or(signed_digest);
        assert_eq!(
            digest(serde_yaml::to_string(&front).unwrap().as_bytes()),
            signed_digest
        );
        front["issued_by"] = json!("test-account:another-run");
        assert_ne!(
            digest(serde_yaml::to_string(&front).unwrap().as_bytes()),
            signed_digest
        );
        let refused = index
            .apply_author(
                Update {
                    path: record.path.clone(),
                    metadata: front,
                    body: record.body.clone(),
                    expected: Some(record.digest.clone()),
                },
                &author,
            )
            .unwrap_err();
        assert!(refused.to_string().contains("test_account_label_mismatch"));
        assert_eq!(
            index.current(&record.path).unwrap().unwrap().digest,
            record.digest
        );
        let activity = index.activity.after(0, 16).unwrap();
        assert!(!activity.is_empty());
        for row in activity {
            assert_eq!(row.event.issued_by.as_deref(), Some(generated.issued_by()));
            row.verify().unwrap();
        }
        let next_run = root.path().join("next-run");
        let next = TestAccount::open(&root.path().join("next-keys"), &next_run).unwrap();
        let account =
            SqliteStore::open(&crate::store::object_dir(&next_run, "account").unwrap()).unwrap();
        let head: Signed<AccountHead> =
            serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
        let next_device = head.payload.active_devices[0].device_id;
        let next_key = next.key_store().load(next_device).unwrap().unwrap();
        let next_author = crate::provenance::author::Author::operator(
            next_device,
            crate::messaging::wire::IssuerKind::GpgRoot,
            &next_key,
        );
        index.set_issued_by(Some(next.issued_by().into()));
        index
            .apply_author(
                Update {
                    path: record.path.clone(),
                    metadata: record.metadata,
                    body: record.body,
                    expected: Some(record.digest),
                },
                &next_author,
            )
            .unwrap();
        let updated = index.current(&record.path).unwrap().unwrap();
        assert_eq!(updated.metadata["issued_by"], next.issued_by());
        assert_eq!(
            updated.metadata["authorship"]["issued_by"],
            next.issued_by()
        );
        assert_eq!(updated.metadata["created_by"], author.id);
        assert_eq!(updated.metadata["updated_by"], next_author.id);
    }
}
