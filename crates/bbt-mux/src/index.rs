//! Repository-local Markdown mirror and weighted graph projections.

use std::collections::{BTreeMap, BTreeSet};
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
        let target = rest[..end].split('|').next().unwrap_or_default().trim();
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
        if let Some(after) = text.strip_prefix("---\n") {
            if let Some((front, body)) = after.split_once("\n---\n") {
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

pub struct Index {
    root: PathBuf,
    store: SqliteStore,
    pub documents: BTreeMap<String, Document>,
    pub graphs: BTreeMap<GraphKind, DiGraph<String, Link>>,
}

impl Index {
    pub fn open(root: &Path, store: SqliteStore) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(root)?;
        let mut index = Self {
            root: root.canonicalize()?,
            store,
            documents: BTreeMap::new(),
            graphs: BTreeMap::new(),
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
        for path in paths {
            let relative = path.strip_prefix(&self.root)?.to_string_lossy().to_string();
            seen.insert(relative.clone());
            let text = std::fs::read_to_string(&path)?;
            let doc = Document::parse(&relative, &text);
            if self
                .documents
                .get(&relative)
                .is_some_and(|old| old.digest == doc.digest)
            {
                continue;
            }
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
            let tx = db.transaction()?;
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
            self.documents.insert(relative, doc);
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
                let tx = db.transaction()?;
                tx.execute("DELETE FROM kv WHERE key=?1", [format!("doc/{path}")])?;
                for event in &events[event_start..] {
                    tx.execute(
                        "INSERT INTO document_outbox(id,payload) VALUES(?1,?2)",
                        rusqlite::params![event.id.to_string(), serde_json::to_vec(event)?],
                    )?;
                }
                tx.commit()?;
            }
        }
        self.rebuild_graphs();
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
        mut metadata: Value,
        body: &str,
        seat: Ulid,
        key: &SigningKey,
    ) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        let path = Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || path.extension().is_none_or(|e| e != "md")
        {
            return Err(invalid("write must be a relative Markdown path"));
        }
        let map = metadata
            .as_object_mut()
            .ok_or_else(|| invalid("frontmatter must be a mapping"))?;
        map.remove("provenance");
        map.entry("created_by")
            .or_insert(json!(format!("seat:{seat}")));
        map.insert("updated_by".into(), json!(format!("seat:{seat}")));
        let front = serde_yaml::to_string(&metadata)?;
        let body_digest = digest(body.as_bytes());
        let front_digest = digest(front.as_bytes());
        let payload = format!("{front_digest}{body_digest}");
        let domain = b"bboxy/item/v1";
        let mut pae = format!("DSSEv1 {} ", domain.len()).into_bytes();
        pae.extend(domain);
        pae.extend(format!(" {} ", payload.len()).as_bytes());
        pae.extend(payload.as_bytes());
        metadata["provenance"] = json!({"digest":format!("sha256:{body_digest}"),"frontmatter_digest":format!("sha256:{front_digest}"),"signer":format!("seat:{seat}"),"sig":base64::engine::general_purpose::STANDARD.encode(key.sign(&pae).to_bytes())});
        let destination = self.root.join(path);
        let parent = destination.parent().ok_or_else(|| invalid("no parent"))?;
        std::fs::create_dir_all(parent)?;
        if !parent.canonicalize()?.starts_with(&self.root)
            || destination
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(invalid("write escapes index root"));
        }
        // The durable signed intent comes first. A later completion distinguishes
        // an applied write from a crash/unknown outcome; the JSONL is a projection.
        use crate::store::Events;
        let events = crate::store::events::EventLog::open(
            self.store
                .path
                .parent()
                .ok_or_else(|| invalid("index store has no object directory"))?,
        )?;
        let day: String = self
            .store
            .connection()?
            .query_row("SELECT date('now')", [], |r| r.get(0))?;
        let log_dir = self.root.join("log");
        std::fs::create_dir_all(&log_dir)?;
        if !log_dir.canonicalize()?.starts_with(&self.root) {
            return Err(invalid("activity log escapes index root"));
        }
        let mut intent = Draft::new(
            format!(
                "doc/{}/write_prepared",
                metadata["id"].as_str().unwrap_or("legacy")
            ),
            "mux/document",
            json!({"activity":"write","path":path,"body_digest":body_digest,"frontmatter_digest":front_digest,"agent":format!("seat:{seat}"),"used":metadata["derived_from"]}),
        );
        intent.seat = Some(seat);
        let prepared = events.append(intent, key)?;
        metadata["provenance"]["log"] = json!(format!(".bboxy/log/{day}.jsonl#{}", prepared.seq));
        let temp = parent.join(format!(".{}.tmp", Ulid::new()));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        write!(
            file,
            "---\n{}---\n{body}",
            serde_yaml::to_string(&metadata)?
        )?;
        file.sync_all()?;
        std::fs::rename(temp, &destination)?;
        std::fs::File::open(parent)?.sync_all()?;
        let mut completed = Draft::new(
            format!(
                "doc/{}/written",
                metadata["id"].as_str().unwrap_or("legacy")
            ),
            "mux/document",
            json!({"activity":"write_applied","prepared":prepared.event.id,"generated":[format!("sha256:{body_digest}")]}),
        );
        completed.seat = Some(seat);
        let completed = events.append(completed, key)?;
        let mut projection = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_dir.join(format!("{day}.jsonl")))?;
        for envelope in [prepared, completed] {
            serde_json::to_writer(&mut projection, &envelope)?;
            projection.write_all(b"\n")?;
        }
        projection.sync_all()?;
        Ok(())
    }
}

fn collect(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect(&entry.path(), paths)?;
        } else if entry.path().extension().is_some_and(|e| e == "md") {
            if entry.metadata()?.len() > MAX_DOCUMENT {
                return Err(invalid(format!(
                    "document exceeds 2 MiB: {}",
                    entry.path().display()
                )));
            }
            paths.push(entry.path());
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
        let dirty = Arc::new(AtomicBool::new(true));
        let failed = Arc::new(AtomicBool::new(false));
        let d = dirty.clone();
        let f = failed.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                d.store(true, Ordering::Release);
                if event.is_err() {
                    f.store(true, Ordering::Release);
                }
            })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
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
