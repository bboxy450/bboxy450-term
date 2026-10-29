// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The bundled provider projects the existing Index; it has no second store,
//! number allocator, watcher, signer, delivery cursor or link resolver.
use super::{DataSource, WriteContext, data_source as d};
use crate::{
    Ulid,
    index::{Document, GraphKind, Index, Update},
    registry::Registry,
    store::{Events, digest},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::File,
    io::Read,
    os::fd::{AsRawFd, FromRawFd},
    path::{Component, Path, PathBuf},
};

const PAGE_BYTES: usize = d::MAX_PAGE_BYTES;

/// In-process default. The host owns the Index, its scan/publish/ack cycle and
/// optional registry context. The provider never creates a second catalogue.
pub struct BboxySource<'a> {
    index: &'a Index,
    registry: Option<&'a Registry>,
}
impl<'a> BboxySource<'a> {
    /// Use the existing root and index snapshot, on an IO worker.
    pub fn new(index: &'a Index) -> Self {
        bbt_rt::assert_not_ui_thread();
        Self {
            index,
            registry: None,
        }
    }
    /// Supply actual registry references for an explicit swarm graph scope.
    pub fn with_registry(mut self, registry: &'a Registry) -> Self {
        self.registry = Some(registry);
        self
    }
    /// Read a resource through the same root confinement as its returned link.
    /// The UI never opens a raw Markdown path or a provider's arbitrary file.
    pub fn read_resource(&self, uri: &str) -> d::Result<Vec<u8>> {
        bbt_rt::assert_not_ui_thread();
        if uri.len() > 16 * 1024 {
            return Err(error(
                "resource_outside_data_dir",
                "resource URI exceeds bound",
            ));
        }
        let encoded = uri.strip_prefix("file://").ok_or_else(|| {
            error(
                "resource_outside_data_dir",
                "resource must be a local file URI",
            )
        })?;
        let path = PathBuf::from(uri_decode(encoded)?);
        let relative = path
            .strip_prefix(self.index.knowledge_root())
            .map_err(|_| {
                error(
                    "resource_outside_data_dir",
                    "resource is outside the knowledge root",
                )
            })?;
        let (file, _, _) = Self::open_resource(self.index.knowledge_root(), relative)?;
        let mut bytes = Vec::new();
        file.take(d::MAX_RESOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(command_io)?;
        if bytes.len() as u64 > d::MAX_RESOURCE_BYTES {
            return Err(error("resource_too_large", "resource exceeds 64 MiB"));
        }
        Ok(bytes)
    }
    fn open_resource(root: &Path, relative: &Path) -> d::Result<(File, PathBuf, u64)> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(error(
                "resource_outside_data_dir",
                "resource path must stay inside the root",
            ));
        }
        let mut current = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)
            .map_err(command_io)?;
        let parts: Vec<_> = relative.components().collect();
        for (index, part) in parts.iter().enumerate() {
            use std::os::unix::ffi::OsStrExt;
            let name = std::ffi::CString::new(part.as_os_str().as_bytes())
                .map_err(|_| error("resource_outside_data_dir", "resource path contains NUL"))?;
            let flags = libc::O_RDONLY
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | libc::O_CLOEXEC
                | if index + 1 < parts.len() {
                    libc::O_DIRECTORY
                } else {
                    0
                };
            // SAFETY: current owns a live directory descriptor and name is a
            // NUL-terminated component. Each next descriptor is owned once.
            let fd = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 {
                return Err(error(
                    "resource_outside_data_dir",
                    std::io::Error::last_os_error().to_string(),
                ));
            }
            // SAFETY: openat returned a new valid descriptor, not aliased.
            current = unsafe { File::from_raw_fd(fd) };
        }
        let metadata = current.metadata().map_err(command_io)?;
        if !metadata.is_file() {
            return Err(error(
                "resource_unavailable",
                "resource must be a regular file",
            ));
        }
        if metadata.len() > d::MAX_RESOURCE_BYTES {
            return Err(error("resource_too_large", "resource exceeds 64 MiB"));
        }
        let canonical = root
            .join(relative)
            .canonicalize()
            .map_err(|e| error("resource_unavailable", e.to_string()))?;
        if !canonical.starts_with(root) {
            return Err(error(
                "resource_outside_data_dir",
                "resource realpath is outside the root",
            ));
        }
        let path_metadata = canonical.metadata().map_err(command_io)?;
        if metadata.dev() != path_metadata.dev() || metadata.ino() != path_metadata.ino() {
            return Err(error(
                "resource_changed",
                "resource path changed during admission",
            ));
        }
        Ok((current, canonical, metadata.len()))
    }
    pub(crate) fn validate_resource(root: &Path, link: &d::ResourceLink) -> d::Result<()> {
        if link.uri.len() > 16 * 1024 {
            return Err(error(
                "resource_outside_data_dir",
                "resource URI exceeds bound",
            ));
        }
        let encoded = link.uri.strip_prefix("file://").ok_or_else(|| {
            error(
                "resource_outside_data_dir",
                "resource must be a local file URI",
            )
        })?;
        let path = PathBuf::from(uri_decode(encoded)?);
        let relative = path.strip_prefix(root).map_err(|_| {
            error(
                "resource_outside_data_dir",
                "resource is outside the provider data directory",
            )
        })?;
        let (_, _, size) = Self::open_resource(root, relative)?;
        if size != link.size {
            return Err(error(
                "resource_changed",
                "resource size differs from the provider result",
            ));
        }
        Ok(())
    }
    fn resources(&self, doc: &Document) -> d::Result<Vec<d::ResourceLink>> {
        let source = format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&doc.metadata).map_err(command_io)?,
            doc.body
        );
        let parsed = bbt_doc::parse(&source);
        let mut targets = Vec::new();
        image_blocks(&parsed.blocks, &mut targets);
        let mut links = BTreeMap::new();
        for (target, wiki) in targets {
            if target.contains("://")
                || target.starts_with("data:")
                || Path::new(&target).is_absolute()
            {
                return Err(error(
                    "resource_outside_data_dir",
                    "image resource is not a root-local path",
                ));
            }
            let target = target.strip_prefix(".bboxy/").unwrap_or(&target);
            let joined = if target.starts_with("images/") {
                PathBuf::from(target)
            } else if wiki && !target.contains('/') {
                Path::new("images").join(target)
            } else {
                Path::new(&doc.path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(target)
            };
            let mut relative = PathBuf::new();
            for part in joined.components() {
                match part {
                    Component::Normal(part) => relative.push(part),
                    Component::CurDir => (),
                    Component::ParentDir if relative.pop() => (),
                    _ => {
                        return Err(error(
                            "resource_outside_data_dir",
                            "image path escapes the root",
                        ));
                    }
                }
            }
            let (_, path, size) = Self::open_resource(self.index.knowledge_root(), &relative)?;
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| error("resource_unavailable", "resource name is not UTF-8"))?
                .to_owned();
            let mime_type = match path
                .extension()
                .and_then(|s| s.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("svg") => "image/svg+xml",
                Some("webp") => "image/webp",
                _ => "application/octet-stream",
            }
            .to_owned();
            let path = path
                .to_str()
                .ok_or_else(|| error("resource_unavailable", "resource path is not UTF-8"))?;
            let uri = format!("file://{}", uri_encode(path));
            links.insert(
                uri.clone(),
                d::ResourceLink {
                    kind: d::ResourceKind::ResourceLink,
                    uri,
                    name,
                    mime_type,
                    size,
                },
            );
        }
        Ok(links.into_values().collect())
    }
    fn root_key(&self) -> String {
        use std::os::unix::ffi::OsStrExt;
        digest(self.index.knowledge_root().as_os_str().as_bytes())
    }
    fn snapshot(&self) -> String {
        let values: Vec<_> = self
            .index
            .documents
            .values()
            .map(|d| (&d.path, &d.digest))
            .collect();
        digest(&serde_json::to_vec(&values).expect("document references serialize"))
    }
    fn document(&self, reference: &d::RecordRef) -> d::Result<Document> {
        let doc = self
            .index
            .current(&reference.path)
            .map_err(command_io)?
            .ok_or_else(|| error("not_found", "record no longer exists"))?;
        if doc.id != reference.id || doc.digest != reference.digest {
            return Err(error(
                "stale_reference",
                "record identity or digest changed; resolve it again",
            ));
        }
        Ok(doc)
    }
    fn position(&self, cursor: Option<&str>, query: &Value, len: usize) -> d::Result<usize> {
        let Some(cursor) = cursor else { return Ok(0) };
        let value: Cursor = decode(cursor)?;
        if value.root != self.root_key() || value.query != query_key(query) {
            return Err(error(
                "invalid_cursor",
                "cursor belongs to another root or query",
            ));
        }
        if value.snapshot != self.snapshot() {
            return Err(error("stale_cursor", "the indexed snapshot changed"));
        }
        if value.offset > len {
            return Err(error("invalid_cursor", "cursor is beyond this result"));
        }
        Ok(value.offset)
    }
    fn cursor(&self, query: &Value, offset: usize) -> String {
        encode(&Cursor {
            root: self.root_key(),
            query: query_key(query),
            snapshot: self.snapshot(),
            offset,
        })
    }
    fn page<T: Clone + Serialize>(
        &self,
        rows: &[T],
        cursor: Option<&str>,
        limit: Option<usize>,
        query: Value,
    ) -> d::Result<d::Page<T>> {
        let start = self.position(cursor, &query, rows.len())?;
        let limit = page_limit(limit)?;
        let mut items = Vec::new();
        let mut bytes = 2; // JSON array brackets; each row is measured once.
        for row in rows.iter().skip(start).take(limit) {
            let addition =
                serde_json::to_vec(row).map_err(command_io)?.len() + usize::from(!items.is_empty());
            if bytes + addition > PAGE_BYTES {
                if items.is_empty() {
                    return Err(error(
                        "result_too_large",
                        "one record exceeds the page byte bound",
                    ));
                }
                break;
            }
            bytes += addition;
            items.push(row.clone());
        }
        loop {
            let end = start + items.len();
            let page = d::Page {
                items: items.clone(),
                next: (end < rows.len()).then(|| self.cursor(&query, end)),
            };
            if serde_json::to_vec(&page).map_err(command_io)?.len() <= PAGE_BYTES {
                return Ok(page);
            }
            if items.len() <= 1 {
                return Err(error(
                    "result_too_large",
                    "one record exceeds the complete page byte bound",
                ));
            }
            items.pop();
        }
    }
    fn timestamp(&self, text: &str) -> d::Result<i64> {
        let store = self.index.mirror();
        let value: Option<i64> = store
            .connection()
            .map_err(command_io)?
            .query_row("SELECT unixepoch(?1)", [text], |r| r.get(0))
            .map_err(command_io)?;
        value.ok_or_else(|| {
            error(
                "invalid_filter",
                "since/updated must be an ISO date or timestamp",
            )
        })
    }
    fn matches(&self, doc: &Document, filter: &d::Filter) -> d::Result<bool> {
        for (field, wanted) in [
            ("type", &filter.record_type),
            ("status", &filter.status),
            ("swarm", &filter.swarm),
        ] {
            if wanted
                .as_deref()
                .is_some_and(|value| doc.metadata[field].as_str() != Some(value))
            {
                return Ok(false);
            }
        }
        if let Some(wanted) = &filter.assignee {
            let actual = doc.metadata["assignee"].as_str().unwrap_or_default();
            if actual.strip_prefix("seat:").unwrap_or(actual)
                != wanted.strip_prefix("seat:").unwrap_or(wanted)
            {
                return Ok(false);
            }
        }
        if let Some(since) = &filter.since {
            let threshold = self.timestamp(since)?;
            let Some(actual) = doc.metadata["updated"]
                .as_str()
                .or_else(|| doc.metadata["created"].as_str())
            else {
                return Ok(false);
            };
            if self.timestamp(actual)? < threshold {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn degree(&self, id: &str) -> f64 {
        let Some(graph) = self.index.graphs.get(&GraphKind::Markdown) else {
            return 0.;
        };
        graph
            .edge_references()
            .filter(|e| graph[e.source()] == id || graph[e.target()] == id)
            .fold(0., |sum, e| (sum + e.weight().weight).min(f64::MAX))
    }
    fn edges(&self, kind: GraphKind) -> Vec<d::Edge> {
        let Some(graph) = self.index.graphs.get(&kind) else {
            return Vec::new();
        };
        let mut edges: Vec<_> = graph
            .edge_references()
            .map(|e| d::Edge {
                from: graph[e.source()].clone(),
                to: graph[e.target()].clone(),
                rel: e.weight().relation.clone(),
                weight: e.weight().weight,
            })
            .collect();
        edges.sort_by(|a, b| {
            (&a.from, &a.to, &a.rel)
                .cmp(&(&b.from, &b.to, &b.rel))
                .then(a.weight.total_cmp(&b.weight))
        });
        edges
    }
    fn graph_page(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        let family = match &request.scope {
            d::Scope::All { graph }
            | d::Scope::Type { graph, .. }
            | d::Scope::Swarm { graph, .. }
            | d::Scope::Neighborhood { graph, .. } => *graph,
        };
        let graph = self
            .index
            .graphs
            .get(&kind(family))
            .ok_or_else(|| error("graph_unavailable", "document graph unavailable"))?;
        let mut all_edges = self.edges(kind(family));
        let mut names: BTreeSet<String> = graph.node_weights().cloned().collect();
        match &request.scope {
            d::Scope::All { .. } => (),
            d::Scope::Type { record_type, .. } => {
                let allowed: BTreeSet<_> = self
                    .index
                    .documents
                    .values()
                    .filter(|d| d.metadata["type"].as_str() == Some(record_type))
                    .map(|d| d.id.clone())
                    .collect();
                names.retain(|id| allowed.contains(id));
            }
            d::Scope::Swarm { swarm, .. } => {
                let registry = self.registry.ok_or_else(|| {
                    error(
                        "scope_unavailable",
                        "swarm scope requires host registry context",
                    )
                })?;
                let swarm: Ulid = swarm
                    .parse()
                    .map_err(|_| error("invalid_scope", "swarm must be an ID"))?;
                let mut allowed = BTreeSet::new();
                for doc in self.index.documents.values() {
                    let assignee = crate::mcp::work_items::reference(doc, "assignee")
                        .map_err(|e| error("ambiguous_scope", e))?;
                    let delegator = crate::mcp::work_items::delegator(doc)
                        .map_err(|e| error("ambiguous_scope", e))?
                        .and_then(|r| r.seat());
                    let mut actual = None;
                    for seat in [assignee, delegator].into_iter().flatten() {
                        let scope = registry.get(seat).map_err(command_io)?.swarm;
                        if actual.is_some_and(|old| old != scope) {
                            return Err(error(
                                "ambiguous_scope",
                                "record references name different swarms",
                            ));
                        }
                        actual = Some(scope);
                    }
                    if actual == Some(swarm) {
                        allowed.insert(doc.id.clone());
                    }
                }
                names.retain(|id| allowed.contains(id));
            }
            d::Scope::Neighborhood {
                reference, depth, ..
            } => {
                if *depth > 16 {
                    return Err(error("invalid_scope", "neighborhood depth exceeds 16"));
                }
                let root = self.document(reference)?.id;
                let mut allowed = BTreeSet::from([root.clone()]);
                let mut ready = VecDeque::from([(root, 0)]);
                while let Some((id, at)) = ready.pop_front() {
                    if at == *depth {
                        continue;
                    }
                    for edge in &all_edges {
                        let next = if edge.from == id {
                            Some(&edge.to)
                        } else if edge.to == id {
                            Some(&edge.from)
                        } else {
                            None
                        };
                        if let Some(next) = next
                            && allowed.insert(next.clone())
                        {
                            ready.push_back((next.clone(), at + 1));
                        }
                    }
                }
                names.retain(|id| allowed.contains(id));
            }
        }
        all_edges.retain(|edge| names.contains(&edge.from) && names.contains(&edge.to));
        let mut by_id = BTreeMap::new();
        for doc in self.index.documents.values() {
            if let Some(previous) = by_id.insert(doc.id.as_str(), doc) {
                return Err(d::CommandError {
                    code: "ambiguous_reference".into(),
                    message: format!("duplicate record ID: {} and {}", previous.path, doc.path),
                    candidates: vec![reference(previous), reference(doc)],
                    retry_after_ms: None,
                });
            }
        }
        let rows: Vec<_> = names
            .into_iter()
            .map(|id| {
                let doc = by_id.get(id.as_str());
                d::Node {
                    reference: doc.map(|doc| reference(doc)),
                    title: doc
                        .and_then(|d| d.metadata["title"].as_str())
                        .map(str::to_owned),
                    record_type: doc
                        .and_then(|d| d.metadata["type"].as_str())
                        .map(str::to_owned),
                    frontmatter: doc.map_or_else(|| json!({}), |d| d.metadata.clone()),
                    id,
                }
            })
            .collect();
        let query = json!({"method":"graph","scope":request.scope});
        let start = self.position(request.cursor.as_deref(), &query, rows.len())?;
        let count = page_limit(request.limit)?;
        let total = rows.len();
        let mut nodes: Vec<_> = rows.into_iter().skip(start).take(count).collect();
        loop {
            let selected: BTreeSet<_> = nodes.iter().map(|n| n.id.as_str()).collect();
            let mut edges: Vec<_> = all_edges
                .iter()
                .filter(|e| selected.contains(e.from.as_str()))
                .cloned()
                .collect();
            let truncated = edges.len() > 512;
            edges.truncate(512);
            let end = start + nodes.len();
            let next = (end < total).then(|| self.cursor(&query, end));
            let result = d::GraphPage {
                nodes: nodes.clone(),
                edges,
                next,
                edges_truncated: truncated,
            };
            if serde_json::to_vec(&result).map_err(command_io)?.len() <= PAGE_BYTES {
                return Ok(result);
            }
            if nodes.len() <= 1 {
                return Err(error(
                    "result_too_large",
                    "one graph node exceeds the page byte bound",
                ));
            }
            nodes.pop();
        }
    }
}

fn image_blocks(blocks: &[bbt_doc::Block], targets: &mut Vec<(String, bool)>) {
    use bbt_doc::BlockKind;
    for block in blocks {
        match &block.kind {
            BlockKind::Paragraph(values)
            | BlockKind::Heading {
                content: values, ..
            } => image_inlines(values, targets),
            BlockKind::List { items, .. } => {
                for blocks in items {
                    image_blocks(blocks, targets)
                }
            }
            BlockKind::Quote(blocks) | BlockKind::Footnote { blocks, .. } => {
                image_blocks(blocks, targets)
            }
            BlockKind::Table { header, rows, .. } => {
                for values in header {
                    image_inlines(values, targets)
                }
                for row in rows {
                    for values in row {
                        image_inlines(values, targets)
                    }
                }
            }
            _ => (),
        }
    }
}
fn image_inlines(values: &[bbt_doc::Inline], targets: &mut Vec<(String, bool)>) {
    use bbt_doc::Inline;
    for (index, value) in values.iter().enumerate() {
        match value {
            Inline::Image { target, .. } => targets.push((target.clone(), false)),
            Inline::WikiLink { target, .. }
                if index > 0
                    && matches!(&values[index-1],Inline::Text(text) if text.ends_with('!')) =>
            {
                targets.push((target.clone(), true))
            }
            Inline::Emphasis(values)
            | Inline::Strong(values)
            | Inline::Strike(values)
            | Inline::Link {
                content: values, ..
            } => image_inlines(values, targets),
            _ => (),
        }
    }
}
fn uri_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(&byte) {
            out.push(byte as char)
        } else {
            out.push_str(&format!("%{byte:02X}"))
        }
    }
    out
}
fn uri_decode(value: &str) -> d::Result<String> {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes
                .get(index + 1..index + 3)
                .ok_or_else(|| error("resource_outside_data_dir", "invalid URI escape"))?;
            let hex = std::str::from_utf8(hex).map_err(command_io)?;
            out.push(
                u8::from_str_radix(hex, 16)
                    .map_err(|_| error("resource_outside_data_dir", "invalid URI escape"))?,
            );
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out)
        .map_err(|_| error("resource_outside_data_dir", "resource URI is not UTF-8"))
}

#[derive(Serialize, Deserialize)]
struct Cursor {
    root: String,
    query: String,
    snapshot: String,
    offset: usize,
}
#[derive(Serialize, Deserialize)]
struct ChangeCursor {
    root: String,
    sequence: u64,
}
fn encode(value: &impl Serialize) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).expect("cursor serializes"))
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> d::Result<T> {
    if value.len() > 4096 {
        return Err(error("invalid_cursor", "cursor exceeds bound"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| error("invalid_cursor", "invalid cursor encoding"))?;
    serde_json::from_slice(&bytes).map_err(|_| error("invalid_cursor", "invalid cursor value"))
}
fn query_key(query: &Value) -> String {
    digest(&serde_json::to_vec(query).expect("query serializes"))
}
fn error(code: &str, message: impl Into<String>) -> d::CommandError {
    d::CommandError::new(code, message)
}
fn command_io(value: impl std::fmt::Display) -> d::CommandError {
    error("index_error", value.to_string())
}
fn page_limit(value: Option<usize>) -> d::Result<usize> {
    let value = value.unwrap_or(d::DEFAULT_PAGE);
    if !(1..=d::MAX_PAGE).contains(&value) {
        return Err(error("invalid_limit", "limit must be 1..128"));
    }
    Ok(value)
}
fn reference(doc: &Document) -> d::RecordRef {
    d::RecordRef {
        id: doc.id.clone(),
        number: doc.metadata["number"].as_u64().filter(|n| *n > 0),
        slug: Path::new(&doc.path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        path: doc.path.clone(),
        digest: doc.digest.clone(),
        external_id: None,
    }
}
fn title(doc: &Document) -> String {
    doc.metadata["title"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}
fn kind(value: d::DocumentGraph) -> GraphKind {
    match value {
        d::DocumentGraph::Markdown => GraphKind::Markdown,
        d::DocumentGraph::Task => GraphKind::Task,
        d::DocumentGraph::Artifact => GraphKind::Artifact,
    }
}

impl DataSource for BboxySource<'_> {
    fn import_root(&self) -> d::Result<PathBuf> {
        Ok(self.index.knowledge_root().to_owned())
    }
    fn commit(
        &self,
        request: d::CommitRequest,
        _context: &WriteContext<'_>,
    ) -> d::Result<d::CommitResult> {
        super::commit::validate(&request)?;
        for reference in &request.refs {
            self.document(reference)?;
        }
        super::commit::local(self.index.knowledge_root(), request)
    }
    fn resolve(&self, request: d::ResolveRequest) -> d::Result<d::RecordRef> {
        bbt_rt::assert_not_ui_thread();
        if request.locator.len() > 4096 {
            return Err(error("invalid_locator", "locator exceeds bound"));
        }
        // The resolver only needs frontmatter identity fields. Reuse the
        // parsed Index metadata without reparsing every Markdown body.
        let catalogue: Vec<_> = self
            .index
            .documents
            .values()
            .map(|doc| {
                let fields = doc
                    .metadata
                    .as_object()
                    .map(|map| {
                        map.iter()
                            .map(|(key, value)| {
                                (
                                    key.clone(),
                                    serde_yaml::to_value(value).expect("JSON metadata is YAML"),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                (
                    doc,
                    bbt_doc::Markdown {
                        frontmatter: bbt_doc::Frontmatter {
                            fields,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
            })
            .collect();
        let records = || {
            catalogue
                .iter()
                .map(|(doc, markdown)| bbt_doc::links::Record {
                    path: &doc.path,
                    markdown,
                })
        };
        let resolved = match bbt_doc::links::resolve(records(), &request.locator) {
            Ok(found) => found,
            Err(bbt_doc::links::ResolveError::Missing) => {
                return Err(error("not_found", "record does not resolve in this root"));
            }
            Err(bbt_doc::links::ResolveError::Ambiguous) => {
                let candidates: Vec<_> = catalogue
                    .iter()
                    .filter(|(doc, markdown)| {
                        bbt_doc::links::resolve(
                            [bbt_doc::links::Record {
                                path: &doc.path,
                                markdown,
                            }],
                            &request.locator,
                        )
                        .is_ok()
                    })
                    .map(|(doc, _)| reference(doc))
                    .collect();
                return Err(d::CommandError {
                    code: "ambiguous_reference".into(),
                    message: format!(
                        "record resolves to multiple paths: {}",
                        candidates
                            .iter()
                            .map(|r| r.path.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    candidates,
                    retry_after_ms: None,
                });
            }
        };
        let cached = self
            .index
            .documents
            .get(resolved.record.path)
            .expect("resolver uses this catalogue");
        // NUM2 remains the fresh source of number uniqueness. There is no
        // second #N parser, matching algorithm or allocation state here.
        let current = if let Some(number) = resolved.record.markdown.frontmatter.number() {
            self.index.by_number(number).map_err(command_io)?
        } else {
            self.index.current(&cached.path).map_err(command_io)?
        }
        .ok_or_else(|| error("not_found", "record disappeared during resolution"))?;
        if current.path != cached.path || current.id != cached.id {
            return Err(error(
                "stale_reference",
                "record identity changed; rescan the root",
            ));
        }
        Ok(reference(&current))
    }

    fn fetch(&self, request: d::FetchRequest) -> d::Result<d::Record> {
        bbt_rt::assert_not_ui_thread();
        let doc = self.document(&request.reference)?;
        let resources = self.resources(&doc)?;
        Ok(d::Record {
            reference: reference(&doc),
            frontmatter: doc.metadata,
            body: doc.body,
            etag: doc.digest,
            resources,
            parse_error: doc.error,
        })
    }

    fn search(&self, request: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        bbt_rt::assert_not_ui_thread();
        if request.query.len() > 4096 {
            return Err(error("invalid_query", "query exceeds bound"));
        }
        let mut filter = d::Filter::default();
        let mut terms = Vec::new();
        for term in request.query.split_whitespace() {
            let option = term.split_once(':');
            let target = match option.map(|(key, _)| key) {
                Some("type") => Some(&mut filter.record_type),
                Some("status") => Some(&mut filter.status),
                Some("since") => Some(&mut filter.since),
                _ => None,
            };
            if let Some(target) = target {
                let value = option.expect("recognized filter").1;
                if value.is_empty() || target.as_ref().is_some_and(|old| old != value) {
                    return Err(error("invalid_query", "empty or conflicting query filter"));
                }
                *target = Some(value.to_owned());
            } else {
                terms.push(term.to_lowercase());
            }
        }
        let mut hits = Vec::new();
        for doc in self.index.documents.values() {
            if !self.matches(doc, &filter)? {
                continue;
            }
            let front = doc.metadata.to_string().to_lowercase();
            let body = doc.body.to_lowercase();
            if !terms
                .iter()
                .all(|term| front.contains(term) || body.contains(term))
            {
                continue;
            }
            let front_hits = terms.iter().filter(|term| front.contains(*term)).count();
            let body_hits = terms.iter().filter(|term| body.contains(*term)).count();
            let score =
                front_hits as f64 * 1e12 + body_hits as f64 * 1e6 + self.degree(&doc.id).ln_1p();
            let reference = reference(doc);
            hits.push(d::SearchHit {
                number: reference.number,
                reference,
                title: title(doc),
                record_type: doc.metadata["type"].as_str().unwrap_or_default().to_owned(),
                snippet: doc.body.chars().take(256).collect(),
                score,
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.reference.path.cmp(&b.reference.path))
        });
        self.page(
            &hits,
            request.cursor.as_deref(),
            request.limit,
            json!({"method":"search","query":request.query}),
        )
    }

    fn list(&self, request: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        bbt_rt::assert_not_ui_thread();
        let mut rows = Vec::new();
        for doc in self.index.documents.values() {
            if self.matches(doc, &request.filter)? {
                rows.push(reference(doc));
            }
        }
        self.page(
            &rows,
            request.cursor.as_deref(),
            request.limit,
            json!({"method":"list","filter":request.filter}),
        )
    }

    fn links(&self, request: d::FetchRequest) -> d::Result<d::Links> {
        bbt_rt::assert_not_ui_thread();
        let doc = self.document(&request.reference)?;
        let edges = self.edges(GraphKind::Markdown);
        let links = d::Links {
            outgoing: edges.iter().filter(|e| e.from == doc.id).cloned().collect(),
            incoming: edges.into_iter().filter(|e| e.to == doc.id).collect(),
        };
        if serde_json::to_vec(&links).map_err(command_io)?.len() > PAGE_BYTES {
            return Err(error(
                "result_too_large",
                "links exceed the bounded result; use graph pages",
            ));
        }
        Ok(links)
    }

    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        bbt_rt::assert_not_ui_thread();
        self.graph_page(request)
    }

    fn write(
        &self,
        request: d::WriteRequest,
        context: &WriteContext<'_>,
    ) -> d::Result<d::WriteResult> {
        bbt_rt::assert_not_ui_thread();
        let (path, expected) = match request.reference {
            Some(reference) => {
                if request.path.is_some() {
                    return Err(error(
                        "invalid_path",
                        "path is only supplied on create; update uses the resolved record",
                    ));
                }
                let current = self.document(&reference)?;
                if request
                    .etag
                    .as_deref()
                    .is_some_and(|etag| etag != current.digest)
                {
                    return Err(error(
                        "etag_mismatch",
                        "etag does not match the current record",
                    ));
                }
                (current.path, Some(current.digest))
            }
            None => {
                if request.etag.is_some() {
                    return Err(error(
                        "etag_mismatch",
                        "create cannot supply an existing etag",
                    ));
                }
                (
                    request.path.ok_or_else(|| {
                        error("path_required", "create requires a relative Markdown path")
                    })?,
                    None,
                )
            }
        };
        // Resource admission precedes the effect, so a missing/escaping image
        // cannot make an applied write look like an ordinary rejected write.
        self.resources(&Document {
            id: String::new(),
            path: path.clone(),
            metadata: request.frontmatter.clone(),
            body: request.body.clone(),
            digest: String::new(),
            error: None,
            links: Vec::new(),
        })?;
        let path = self
            .index
            .apply_author_path(
                Update {
                    path: path.clone(),
                    metadata: request.frontmatter,
                    body: request.body,
                    expected,
                },
                &context.author,
            )
            .map_err(|e| {
                if e.downcast_ref::<crate::index::DecisionFilenameRequired>()
                    .is_some()
                {
                    return error("decision_filename_required", e.to_string());
                }
                error(
                    "write_not_confirmed",
                    format!("Index write not confirmed; inspect the record before retry: {e}"),
                )
            })?;
        let current = self
            .index
            .current(&path)
            .map_err(|e| {
                error(
                    "write_applied_result_unavailable",
                    format!("write applied to {path}; current read failed: {e}"),
                )
            })?
            .ok_or_else(|| {
                error(
                    "write_applied_result_unavailable",
                    "write applied; document then disappeared",
                )
            })?;
        let record = self
            .fetch(d::FetchRequest {
                reference: reference(&current),
            })
            .map_err(|failure| {
                error(
                    "write_applied_result_unavailable",
                    format!("write applied to {path}; result unavailable: {failure}"),
                )
            })?;
        Ok(d::WriteResult {
            record,
            lossy: false,
        })
    }

    fn changes_since(&self, request: d::ChangesRequest) -> d::Result<d::Changes> {
        bbt_rt::assert_not_ui_thread();
        let mut sequence = if request.cursor == "0" {
            0
        } else {
            let cursor: ChangeCursor = decode(&request.cursor)?;
            if cursor.root != self.root_key() {
                return Err(error(
                    "invalid_cursor",
                    "change cursor belongs to another root",
                ));
            }
            cursor.sequence
        };
        let high = self.index.activity().last_sequence().map_err(command_io)?;
        if sequence > high {
            return Err(error(
                "stale_cursor",
                "change cursor is ahead of this activity log",
            ));
        }
        let mut changed = Vec::new();
        // The quantum bounds scanned events, not just matching documents;
        // prepared intents or unrelated activity cannot create an unbounded poll.
        for row in self
            .index
            .activity()
            .after(sequence, page_limit(request.limit)?)
            .map_err(command_io)?
        {
            let fields: Vec<_> = row.event.subject.split('/').collect();
            if fields.len() == 3
                && fields[0] == "doc"
                && fields[1].parse::<Ulid>().is_ok()
                && matches!(fields[2], "created" | "modified" | "deleted")
            {
                let path = row.event.payload["path"]
                    .as_str()
                    .ok_or_else(|| error("invalid_change_log", "document change has no path"))?;
                let hash = row.event.payload["digest"]
                    .as_str()
                    .ok_or_else(|| error("invalid_change_log", "document change has no digest"))?;
                let reference = d::RecordRef {
                    id: fields[1].into(),
                    path: path.into(),
                    slug: Path::new(path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    digest: hash.into(),
                    number: row.event.payload["frontmatter"]["number"].as_u64(),
                    external_id: None,
                };
                if serde_json::to_vec(&changed).map_err(command_io)?.len()
                    + serde_json::to_vec(&reference).map_err(command_io)?.len()
                    + 1024
                    > PAGE_BYTES
                {
                    break;
                }
                changed.push(reference);
            }
            sequence = row.seq;
        }
        Ok(d::Changes {
            changed,
            next: encode(&ChangeCursor {
                root: self.root_key(),
                sequence,
            }),
        })
    }
}
