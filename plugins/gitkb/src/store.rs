// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Root-bound GitKB state: catalog, exact source-aware reads, batched
//! committed reads, the per-process record cache and cursor tokens.
//! Translated from bboxy450-term `plugins/gitkb/adapter.py` (`catalog`,
//! `workspace_stats`, `read`, `resources`, `token`); the batch path and cache
//! are new. Uses (does not copy) GitKB's document.read.v1 CLI contract,
//! https://github.com/gitkb/gitkb-core crates/gitkb-cli/src/commands/show.rs.
use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bbt_plugin_sdk::{Ctx, MAX_RESULT, Refusal, encoded};
use serde_json::{Map, Value, json};

use crate::backend::Backend;
use crate::index::{Committed, Doc, Index, Listed, Query, WAIT_CAP, WAIT_RESERVE, store_signal};
use crate::native::{
    committed_digest, committed_updated_at, content_digest, digest, embeds, file_updated_at,
    file_uri, mime_type, number, portable, prefix, random_hex, sha256_hex, slug_path, task_status,
};

/// Time left for the reply when a page stops filling.
pub const RESERVE: Duration = Duration::from_millis(400);
/// Rough cost of one batched record, used to size a batch to the time left.
pub const PER_DOC_MS: u128 = 15;
/// Serialized bytes a page may fill; the rest covers its envelope.
pub const PAGE_BYTES: usize = MAX_RESULT - 8 * 1024;
const MAX_CURSORS: usize = 128;
/// Concurrent exact reads of workspace-backed records, and the measured
/// cost of one (Harmony: about 55 ms, mostly process start).
const PARALLEL_READS: usize = 6;
const EXACT_MS: u128 = 60;
const MAX_RESOURCE: u64 = 64 * 1024 * 1024;

/// One `git-kb list` inventory with exact workspace file stats.
pub struct Catalog {
    pub rows: Map<String, Value>,
    pub snapshot: String,
    /// Native order (title), for locator candidates.
    pub order: Vec<String>,
}

impl Catalog {
    fn seal(&mut self) {
        self.snapshot = digest(&encoded(&Value::Object(self.rows.clone())));
    }

    pub fn workspace(&self, slug: &str) -> bool {
        self.rows
            .get(slug)
            .is_some_and(|row| !row["workspace_stat"].is_null())
    }

    /// Cache key for one record: its full catalog row, including the
    /// committed `modified_at` or the exact workspace file stat.
    fn fingerprint(&self, slug: &str) -> String {
        encoded(&self.rows[slug])
    }
}

/// What list, search and changes need of a record. Built from an exact read
/// or from a batched committed read; both carry the same `RecordRef`.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub reference: Value,
    pub title: Value,
    pub kind: Value,
    pub status: Value,
    pub native_status: Value,
    pub assignee: Value,
    pub swarm: Value,
    pub snippet: String,
    /// Frontmatter and body text, for workspace sources that native FTS skips.
    pub haystack: Option<String>,
}

impl Summary {
    pub fn from_record(record: &Value) -> Self {
        let fm = &record["frontmatter"];
        let body = record["body"].as_str().unwrap_or("");
        Self {
            reference: reference(record),
            title: fm["title"].clone(),
            kind: fm["type"].clone(),
            status: fm["status"].clone(),
            native_status: record["gitkb"]["native_status"].clone(),
            assignee: fm["assignee"].clone(),
            swarm: fm["swarm"].clone(),
            snippet: prefix(body, 240),
            haystack: Some(format!("{} {body}", encoded(fm)).to_lowercase()),
        }
    }

    /// A committed-source summary from one `show --raw-content --json`
    /// document. GitKB returns the committed document when the workspace has
    /// no file, so its id and version give an exact read's committed digest.
    /// `None` asks for an exact read.
    pub fn from_batch(root: &str, slug: &str, doc: &Value) -> Result<Option<Self>, Refusal> {
        let (Some(id), Some(_)) = (doc["id"].as_str(), doc["rawContent"].as_str()) else {
            return Ok(None);
        };
        if doc["slug"] != slug || !doc["version"].is_u64() {
            return Ok(None);
        }
        let extras = doc["frontmatter"].as_object();
        let kind = doc["type"].clone();
        let native_status = doc["status"].clone();
        let status = if kind == "task" {
            task_status(&native_status)
        } else {
            native_status.clone()
        };
        let extra = |key: &str| extras.and_then(|fm| fm.get(key)).cloned();
        let assignee = match &doc["assignee"] {
            Value::Null => extra("assignee").unwrap_or(Value::Null),
            value => value.clone(),
        };
        Ok(Some(Self {
            reference: reference(&json!({
                "id": portable(id)?,
                "external_id": id,
                "number": number(extras, slug)?,
                "slug": slug,
                "path": format!("{slug}.md"),
                "digest": committed_digest(root, id, &doc["version"]),
                "updated_at": committed_updated_at(&doc["modified_at"]),
            })),
            title: doc["title"].clone(),
            kind,
            status,
            native_status,
            assignee,
            swarm: extra("swarm").unwrap_or(Value::Null),
            snippet: prefix(doc["content"].as_str().unwrap_or(""), 240),
            haystack: None,
        }))
    }

    /// A committed-source summary and node frontmatter from the index: the
    /// same projection an exact committed read applies to the same parsed
    /// frontmatter, and the same committed digest.
    pub fn from_index(root: &str, slug: &str, doc: &Doc) -> Result<(Self, Value), Refusal> {
        let id = portable(&doc.id)?;
        let projected = project(
            &id,
            slug,
            &json!({"slug": slug, "title": doc.title}),
            &doc.doc_type,
            doc.frontmatter.clone(),
        )?;
        let fm = &projected.frontmatter;
        let summary = Self {
            reference: reference(&json!({
                "id": id,
                "external_id": doc.id,
                "number": projected.number,
                "slug": slug,
                "path": format!("{slug}.md"),
                "digest": committed_digest(root, &doc.id, &doc.version),
                "updated_at": committed_updated_at(&doc.modified_at),
            })),
            title: fm["title"].clone(),
            kind: fm["type"].clone(),
            status: fm.get("status").cloned().unwrap_or(Value::Null),
            native_status: projected.native_status.clone(),
            assignee: fm.get("assignee").cloned().unwrap_or(Value::Null),
            swarm: fm.get("swarm").cloned().unwrap_or(Value::Null),
            snippet: doc.snippet.clone(),
            haystack: None,
        };
        Ok((summary, Value::Object(projected.frontmatter)))
    }

    /// A filter value matches; status matches the portable or native value.
    pub fn matches(&self, filters: &Map<String, Value>) -> bool {
        let wanted = |field: &str| filters.get(field).filter(|v| truthy(v));
        wanted("type").is_none_or(|v| &self.kind == v)
            && wanted("status").is_none_or(|v| &self.status == v || &self.native_status == v)
            && wanted("assignee").is_none_or(|v| &self.assignee == v)
            && wanted("swarm").is_none_or(|v| &self.swarm == v)
    }
}

/// Python truthiness of a JSON value.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// The portable `RecordRef` of a record.
pub fn reference(record: &Value) -> Value {
    let mut out = Map::new();
    for key in ["id", "external_id", "number", "slug", "path", "digest"] {
        out.insert(key.into(), record[key].clone());
    }
    if !record["updated_at"].is_null() {
        out.insert("updated_at".into(), record["updated_at"].clone());
    }
    Value::Object(out)
}

/// A native document's portable frontmatter and the values lifted from it.
pub struct Projected {
    pub frontmatter: Map<String, Value>,
    pub native_status: Value,
    pub number: Value,
}

/// The one frontmatter projection for exact and batched reads: graph-derived
/// keys dropped, identity, slug, title and type from the document, the task
/// status read from the native status alone, the number lifted. The legacy
/// in-document `bboxy_status` and `bboxy_signed_record` are dropped unread.
/// `doc` supplies `slug` and `title`; `slug` is the requested slug used for
/// the number fallback.
pub fn project(
    id: &str,
    slug: &str,
    doc: &Value,
    kind: &Value,
    mut fm: Map<String, Value>,
) -> Result<Projected, Refusal> {
    // GitKB declares these projections graph-derived, never source YAML.
    fm.retain(|k, _| !matches!(k.as_str(), "references" | "blocks" | "children"));
    fm.insert("id".into(), id.into());
    fm.insert("slug".into(), doc["slug"].clone());
    fm.insert("title".into(), doc["title"].clone());
    fm.insert("type".into(), kind.clone());
    let native_status = fm.get("status").cloned().unwrap_or(Value::Null);
    fm.remove("bboxy_status");
    fm.remove("bboxy_signed_record");
    if fm["type"] == "task" {
        fm.insert("status".into(), task_status(&native_status));
    }
    let number = number(Some(&fm), slug)?;
    if !number.is_null() {
        fm.insert("number".into(), number.clone());
    }
    Ok(Projected {
        frontmatter: fm,
        native_status,
        number,
    })
}

/// The private file holding the host proof for one document version: the
/// plugin data dir, keyed by native identity and version digest. The proof
/// never enters the shared KB, and another editor's change (a new digest)
/// leaves it unread rather than stale.
fn signed_path(data: &Path, id: &str, digest: &str) -> PathBuf {
    data.join("signed")
        .join(crate::native::digest(&format!("{id}\n{digest}")))
}

/// The host proof recorded for this document version, if any.
pub fn load_signed(data: &Path, id: &str, digest: &str) -> Option<String> {
    let mut text = String::new();
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(signed_path(data, id, digest))
        .ok()?
        .take(MAX_RESULT as u64)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

/// Record the host proof for this document version (owner-only files).
pub fn store_signed(data: &Path, id: &str, digest: &str, proof: &str) -> Result<(), Refusal> {
    let failed = || {
        Refusal::new(
            "signed_record_unstored",
            "the host proof could not be kept in the plugin data directory",
        )
    };
    let directory = data.join("signed");
    let _ = std::fs::DirBuilder::new().mode(0o700).create(&directory);
    if directory
        .symlink_metadata()
        .map_or(true, |m| m.file_type().is_symlink() || !m.is_dir())
    {
        return Err(failed());
    }
    let path = signed_path(data, id, digest);
    let partial = directory.join(format!(".{}", random_hex(12)));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&partial)
        .and_then(|mut file| file.write_all(proof.as_bytes()))
        .and_then(|()| std::fs::rename(&partial, &path))
        .map_err(|_| {
            let _ = std::fs::remove_file(&partial);
            failed()
        })
}

/// `Option<String>` fields of GitKB's typed `Frontmatter`; null is omitted.
const NATIVE_STRINGS: [&str; 12] = [
    "$schema",
    "status",
    "priority",
    "assignee",
    "agent_id",
    "worktree",
    "component",
    "repo",
    "created_by",
    "estimated_effort",
    "actual_effort",
    "rank",
];

/// The frontmatter GitKB's exact `show` returns for these committed source
/// bytes: the YAML block through GitKB's typed `Frontmatter` round trip
/// (serde_yaml 0.9, as GitKB parses it), without document-level keys and
/// without the relationship keys GitKB stores as edges. `None` when a field
/// needs GitKB's own normalization (dates, external refs, typed mismatches);
/// the caller then reads exactly.
/// Mirrors (does not copy) GitKB `crates/gitkb-types/src/document.rs`
/// `Frontmatter` and `crates/gitkb-parser/src/parser/markdown.rs`.
pub fn native_frontmatter(raw: &str) -> Option<Map<String, Value>> {
    let rest = raw.strip_prefix("---\n")?;
    let header = rest
        .split_once("\n---\n")
        .map(|(header, _)| header)
        .or_else(|| rest.strip_suffix("\n---"))?;
    if header.contains('\r') {
        return None;
    }
    let mut fm: Map<String, Value> = if header.trim().is_empty() {
        Map::new()
    } else {
        serde_yaml::from_str(header).ok()?
    };
    for key in [
        "id",
        "slug",
        "title",
        "type",
        "parent",
        "in",
        "depends_on",
        "blocked_by",
        "implements",
    ] {
        fm.remove(key);
    }
    for key in NATIVE_STRINGS {
        match fm.get(key) {
            Some(Value::Null) => {
                fm.remove(key);
            }
            None | Some(Value::String(_)) => {}
            Some(_) => return None,
        }
    }
    match fm.get("tags") {
        None => {}
        Some(Value::Array(tags)) if tags.is_empty() => {
            fm.remove("tags");
        }
        Some(Value::Array(tags)) if tags.iter().all(Value::is_string) => {}
        Some(_) => return None,
    }
    match fm.get("due_date") {
        None => {}
        Some(Value::Null) => {
            fm.remove("due_date");
        }
        Some(_) => return None,
    }
    if fm.contains_key("external_refs") {
        return None;
    }
    Some(fm)
}

struct Cached {
    fingerprint: String,
    summary: Summary,
    record: Option<Value>,
    /// Portable frontmatter, equal to the exact record's, when known.
    frontmatter: Option<Value>,
}

/// Opaque page tokens; the oldest is dropped beyond 128.
#[derive(Default)]
pub struct Cursors {
    order: VecDeque<String>,
    values: HashMap<String, Value>,
}

impl Cursors {
    pub fn token(&mut self, value: Value) -> String {
        let token = random_hex(24);
        self.order.push_back(token.clone());
        self.values.insert(token.clone(), value);
        while self.order.len() > MAX_CURSORS {
            if let Some(old) = self.order.pop_front() {
                self.values.remove(&old);
            }
        }
        token
    }

    pub fn get(&self, token: &str) -> Option<&Value> {
        self.values.get(token)
    }
}

pub struct GitKb {
    pub backend: Backend,
    pub ctx: Ctx,
    pub root: PathBuf,
    pub root_text: String,
    pub workspace: String,
    pub workspace_path: PathBuf,
    pub fixture: bool,
    pub cursors: Cursors,
    /// Exact reads within one call.
    pub call_records: HashMap<String, Value>,
    cache: HashMap<String, Cached>,
    /// The catalog index; disabled until `bind` starts it.
    pub index: Index,
}

impl GitKb {
    pub fn new(backend: Backend, ctx: Ctx, root: PathBuf, fixture: bool) -> Self {
        Self {
            backend,
            ctx,
            root_text: root.display().to_string(),
            root,
            workspace: String::new(),
            workspace_path: PathBuf::new(),
            fixture,
            cursors: Cursors::default(),
            call_records: HashMap::new(),
            cache: HashMap::new(),
            index: Index::new(),
        }
    }

    pub fn cli(&mut self, args: &[&str]) -> Result<Value, Refusal> {
        self.backend.cli(&self.ctx, args, None, false)
    }

    pub fn cli_stdin(&mut self, args: &[&str], stdin: &str) -> Result<Value, Refusal> {
        self.backend.cli(&self.ctx, args, Some(stdin), false)
    }

    pub fn forget(&mut self, slug: &str) {
        self.call_records.remove(slug);
        self.cache.remove(slug);
    }

    /// Markdown files in this workspace: slug → [size, mtime_ns, ctime_ns, ino].
    pub fn workspace_stats(&self) -> Result<Map<String, Value>, Refusal> {
        walk_workspace(&self.workspace_path, || self.ctx.remaining().is_zero())
    }

    /// The catalog for these `git-kb list` flags: from the index when it can
    /// answer them for the current store and workspace, else from the backend.
    pub fn catalog(&mut self, flags: &[String]) -> Result<Catalog, Refusal> {
        if let Some(catalog) = self.indexed(flags)? {
            self.index.served += 1;
            return Ok(catalog);
        }
        if self.index.enabled() {
            self.index.fallbacks += 1;
        }
        let rows = self.native_rows(flags)?;
        let stats = self.workspace_stats()?;
        assemble(rows, &stats)
    }

    fn native_rows(&mut self, flags: &[String]) -> Result<Vec<Value>, Refusal> {
        let mut args = vec!["list", "--json", "--sort", "title", "--asc"];
        args.extend(flags.iter().map(String::as_str));
        match self.cli(&args)? {
            Value::Array(rows) => Ok(rows),
            _ => Err(Refusal::new(
                "backend_protocol_error",
                "GitKB list is not an array",
            )),
        }
    }

    /// The index answer, or `None` for the backend path: unsupported flags,
    /// no store signal, committed fields not yet fresh, or a store or
    /// workspace that changed while the unfiltered listing was refreshed.
    fn indexed(&mut self, flags: &[String]) -> Result<Option<Catalog>, Refusal> {
        if !self.index.enabled() {
            return Ok(None);
        }
        let Some(query) = Query::parse(flags) else {
            return Ok(None);
        };
        let Some(signal) = store_signal(&self.root) else {
            return Ok(None);
        };
        // Always keep the committed layer warm; only `--where` needs it.
        let mut committed = self.index.committed(&signal, &self.backend, &self.root);
        if query.needs_fields() && committed.is_none() {
            // A build in progress (a cold process's first reads) usually
            // finishes sooner than the per-call backend path would add up:
            // wait for it within this call's budget, less a reserve.
            let budget = self
                .ctx
                .remaining()
                .saturating_sub(WAIT_RESERVE)
                .min(WAIT_CAP);
            let began = Instant::now();
            let ctx = self.ctx.clone();
            committed = self.index.wait_for(&signal, budget, || ctx.is_cancelled());
            self.index.waits += 1;
            self.index.waited_ms += began.elapsed().as_millis() as u64;
            if committed.is_none() {
                return Ok(None);
            }
        }
        let stats = self.workspace_stats()?;
        let workspace = digest(&encoded(&Value::Object(stats.clone())));
        let fresh = |listed: &Listed| listed.signal == signal && listed.workspace == workspace;
        if !self.index.listed.as_ref().is_some_and(fresh) {
            self.index.listed = None;
            let rows = self.native_rows(&[])?;
            if self.workspace_stats()? != stats
                || store_signal(&self.root).as_deref() != Some(signal.as_str())
            {
                return Ok(None);
            }
            self.index.listed = Some(Listed::new(signal, workspace, rows));
        }
        let Some(listed) = self.index.listed.as_mut() else {
            return Ok(None);
        };
        let rows = listed.select(&query, committed.as_deref(), &self.workspace_path, &stats);
        assemble(rows, &stats).map(Some)
    }
}

/// The exact source-aware record: the workspace source, else committed when
/// the workspace does not participate. Explicit workspace reads never fall
/// back. Free of plugin state, so independent reads can run in parallel.
pub fn read_source(
    backend: &mut Backend,
    ctx: &Ctx,
    root_text: &str,
    workspace: &Path,
    slug: &str,
    requested: Option<&str>,
) -> Result<Value, Refusal> {
    read_source_state(backend, ctx, root_text, workspace, slug, requested).map(|(record, _)| record)
}

/// `read_source`, plus whether GitKB reports the workspace source as a
/// clean, current checkout (its bytes equal the committed revision).
pub fn read_source_state(
    backend: &mut Backend,
    ctx: &Ctx,
    root_text: &str,
    workspace: &Path,
    slug: &str,
    requested: Option<&str>,
) -> Result<(Value, bool), Refusal> {
    let mut workspace_generation = Value::Null;
    let source = requested.unwrap_or("workspace");
    let mut result = backend.cli(
        ctx,
        &[
            "show",
            "--source",
            source,
            "--raw-content",
            "--json",
            "--",
            slug,
        ],
        None,
        false,
    )?;
    if result["exists"] != true {
        if requested == Some("workspace") || result["participates"] == true {
            return Err(Refusal::new(
                "not_found",
                "the selected workspace source is deleted or absent",
            ));
        }
        workspace_generation = result["generation"].clone();
        result = backend.cli(
            ctx,
            &[
                "show",
                "--source",
                "committed",
                "--raw-content",
                "--json",
                "--",
                slug,
            ],
            None,
            false,
        )?;
    }
    if result["exists"] != true || !result["document"].is_object() {
        return Err(Refusal::new(
            "not_found",
            "no document in the selected GitKB source",
        ));
    }
    let doc = &result["document"];
    let Some(raw) = result["rawContent"].as_str() else {
        return Err(Refusal::new(
            "raw_content_unavailable",
            "GitKB omitted source bytes",
        ));
    };
    let native_id = doc["id"].as_str().unwrap_or("");
    let id = portable(native_id)?;
    let Projected {
        frontmatter: fm,
        native_status,
        number,
    } = project(
        &id,
        slug,
        doc,
        &doc["doc_type"],
        doc["frontmatter"].as_object().cloned().unwrap_or_default(),
    )?;
    let source = result["documentSource"].clone();
    // Committed sources digest their native id and version, so batched
    // and indexed references need no bytes; workspace sources digest bytes.
    let (digest, updated_at) = if source == json!({"kind": "committed"}) {
        (
            committed_digest(root_text, native_id, &doc["version"]),
            committed_updated_at(&doc["modified_at"]),
        )
    } else {
        (
            content_digest(root_text, &source, raw),
            file_updated_at(&workspace.join(format!("{slug}.md"))),
        )
    };
    let mut record = json!({
        "id": id, "external_id": native_id, "number": number, "slug": slug,
        "path": format!("{slug}.md"),
        "digest": digest,
        "body": doc["content"], "etag": result["generation"], "resources": [],
        "gitkb": {"source": source, "native_status": native_status,
                  "committed_revision": result.get("committedRevision").cloned().unwrap_or(Value::Null),
                  "version": doc["version"], "created_at": doc["created_at"],
                  "modified_at": doc["modified_at"], "raw_content": raw},
    });
    if let Some(signed) = load_signed(&backend.cwd, native_id, &digest) {
        record["signed_record"] = signed.into();
    }
    if !updated_at.is_null() {
        record["updated_at"] = updated_at;
    }
    if truthy(&workspace_generation) {
        record["gitkb"]["workspace_generation"] = workspace_generation;
    }
    record["frontmatter"] = Value::Object(fm);
    let clean = result["availableDocumentSources"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|s| {
            s["documentSource"]["kind"] == "workspace"
                && s["checkoutState"] == "clean"
                && s["freshness"] == "current"
        });
    Ok((record, clean))
}

/// Markdown files in a workspace: slug → [size, mtime_ns, ctime_ns, ino];
/// `expired` stops the walk with `deadline_exceeded`.
pub fn walk_workspace(
    workspace: &Path,
    expired: impl Fn() -> bool,
) -> Result<Map<String, Value>, Refusal> {
    if workspace
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(Refusal::new(
            "workspace_outside_root",
            "symlinked workspace refused",
        ));
    }
    let mut result = Map::new();
    let mut pending = vec![workspace.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if expired() {
                return Err(Refusal::new(
                    "deadline_exceeded",
                    "workspace inventory exceeded the method budget",
                ));
            }
            let path = entry.path();
            let Ok(st) = path.symlink_metadata() else {
                continue;
            };
            if st.file_type().is_symlink() {
                return Err(Refusal::new(
                    "unsafe_workspace_path",
                    "symlinked workspace entry refused",
                ));
            }
            if st.is_dir() {
                pending.push(path.clone());
            }
            let name = entry.file_name();
            if name.to_string_lossy().ends_with(".md") {
                let relative = path
                    .strip_prefix(workspace)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                let slug = relative[..relative.len() - 3].to_owned();
                let mtime = st.mtime() as i128 * 1_000_000_000 + st.mtime_nsec() as i128;
                let ctime = st.ctime() as i128 * 1_000_000_000 + st.ctime_nsec() as i128;
                result.insert(
                    slug,
                    json!([st.size(), mtime as i64, ctime as i64, st.ino()]),
                );
            }
            if result.len() > 20_000 {
                return Err(Refusal::new(
                    "workspace_too_large",
                    "workspace exceeds 20000 files",
                ));
            }
        }
    }
    Ok(result)
}

/// Catalog rows with exact workspace file stats; a workspace-backed row
/// drops GitKB's read-time `modified_at`.
fn assemble(rows: Vec<Value>, stats: &Map<String, Value>) -> Result<Catalog, Refusal> {
    let mut catalog = Catalog {
        rows: Map::new(),
        snapshot: String::new(),
        order: Vec::new(),
    };
    for mut row in rows {
        let Some(slug) = row["slug"].as_str().map(str::to_owned) else {
            return Err(Refusal::new(
                "backend_protocol_error",
                "GitKB list row has no slug",
            ));
        };
        let stat = stats.get(&slug).cloned().unwrap_or(Value::Null);
        if let Value::Object(fields) = &mut row {
            if !stat.is_null() {
                // GitKB 0.2.19 reparses workspace modified_at as read time.
                // Exact file stats fence that source; committed timestamps stay.
                fields.remove("modified_at");
            }
            fields.insert("workspace_stat".into(), stat);
        }
        if catalog.rows.insert(slug.clone(), row).is_none() {
            catalog.order.push(slug);
        }
    }
    catalog.seal();
    Ok(catalog)
}

impl GitKb {
    /// The exact source-aware record: the workspace source, else committed
    /// when the workspace does not participate. Explicit workspace reads
    /// never fall back.
    pub fn read(&mut self, slug: &Value, requested: Option<&str>) -> Result<Value, Refusal> {
        let slug = slug_path(slug)?;
        if requested.is_none()
            && let Some(record) = self.call_records.get(&slug)
        {
            return Ok(record.clone());
        }
        let record = read_source(
            &mut self.backend,
            &self.ctx,
            &self.root_text,
            &self.workspace_path,
            &slug,
            requested,
        )?;
        // Refresh: a cached summary with another reference is stale.
        if self
            .cache
            .get(&slug)
            .is_some_and(|c| c.summary.reference != reference(&record))
        {
            self.cache.remove(&slug);
        }
        if requested.is_none() {
            self.call_records.insert(slug, record.clone());
        }
        Ok(record)
    }

    /// Summaries for `slugs` in order, from the cache, one batched read of
    /// committed-only records, or exact reads for workspace records. Returns
    /// a prefix (at least one entry) when time runs short; `None` marks a
    /// listed record whose source read is absent.
    ///
    /// Known gap: a document deleted in the workspace but not committed has no
    /// workspace file, so it takes the batched path and lists with its
    /// committed reference; an exact fetch then answers `not_found`. Detecting
    /// it needs `git-kb status`, which costs about a second on large KBs.
    pub fn summaries(
        &mut self,
        catalog: &Catalog,
        slugs: &[String],
    ) -> Result<Vec<Option<Summary>>, Refusal> {
        self.indexed_records(catalog, slugs)?;
        let hit = |cache: &HashMap<String, Cached>, slug: &str| {
            cache
                .get(slug)
                .filter(|c| c.fingerprint == catalog.fingerprint(slug))
                .map(|c| c.summary.clone())
        };
        let batch: Vec<&str> = slugs
            .iter()
            .filter(|s| !catalog.workspace(s) && hit(&self.cache, s).is_none())
            .map(String::as_str)
            .collect();
        if !batch.is_empty() {
            self.batch(catalog, &batch)?;
        }
        self.prefetch(catalog, slugs, RESERVE / 2);
        let mut out = Vec::new();
        for slug in slugs {
            if let Some(summary) = hit(&self.cache, slug) {
                out.push(Some(summary));
                continue;
            }
            if !out.is_empty() && self.ctx.remaining() < RESERVE / 2 {
                break;
            }
            out.push(self.exact(catalog, slug)?.map(|(s, _)| s));
        }
        Ok(out)
    }

    /// The committed layer fresh for the current store; while a build runs
    /// (a cold process), wait for it within this call's budget, at most
    /// `WAIT_CAP` and never into the last `WAIT_RESERVE`.
    pub fn wait_committed(&mut self) -> Option<Arc<Committed>> {
        if !self.index.enabled() {
            return None;
        }
        let signal = store_signal(&self.root)?;
        if let Some(committed) = self.index.committed(&signal, &self.backend, &self.root) {
            return Some(committed);
        }
        let budget = self
            .ctx
            .remaining()
            .saturating_sub(WAIT_RESERVE)
            .min(WAIT_CAP);
        let began = Instant::now();
        let ctx = self.ctx.clone();
        let committed = self.index.wait_for(&signal, budget, || ctx.is_cancelled());
        self.index.waits += 1;
        self.index.waited_ms += began.elapsed().as_millis() as u64;
        committed
    }

    /// Whether background workspace preparation has finished, waiting for it
    /// like `wait_committed` does for the committed layer.
    pub fn wait_prepared(&mut self) -> bool {
        if !self.index.enabled() {
            return true;
        }
        let budget = self
            .ctx
            .remaining()
            .saturating_sub(WAIT_RESERVE)
            .min(WAIT_CAP);
        let ctx = self.ctx.clone();
        self.index.wait_prepared(budget, || ctx.is_cancelled())
    }

    /// The committed layer fresh for the current store, without waiting.
    pub fn index_committed(&self) -> Option<Arc<Committed>> {
        if !self.index.enabled() {
            return None;
        }
        let signal = store_signal(&self.root)?;
        self.index.committed(&signal, &self.backend, &self.root)
    }

    /// Records the index answers, cached like batched ones: committed-only
    /// records (no workspace file) from the committed layer, and
    /// workspace-backed records from exact reads made in the background at
    /// the same file stat. Anything else keeps its batched or exact read.
    fn indexed_records(&mut self, catalog: &Catalog, slugs: &[String]) -> Result<(), Refusal> {
        let missing: Vec<&String> = slugs
            .iter()
            .filter(|s| {
                !self.cache.get(s.as_str()).is_some_and(|c| {
                    c.fingerprint == catalog.fingerprint(s) && c.frontmatter.is_some()
                })
            })
            .collect();
        if missing.is_empty() || !self.index.enabled() {
            return Ok(());
        }
        let committed = self.index_committed();
        for slug in missing {
            let row = &catalog.rows[slug];
            let (summary, frontmatter) = if catalog.workspace(slug) {
                // An exact record read in the background at this file stat.
                let Some(record) = self.index.prepared(slug, &row["workspace_stat"]) else {
                    continue;
                };
                (Summary::from_record(&record), record["frontmatter"].clone())
            } else {
                let Some(doc) = committed.as_ref().and_then(|c| c.doc(slug, &row["id"])) else {
                    continue;
                };
                Summary::from_index(&self.root_text, slug, doc)?
            };
            self.cache.insert(
                slug.clone(),
                Cached {
                    fingerprint: catalog.fingerprint(slug),
                    summary,
                    record: None,
                    frontmatter: Some(frontmatter),
                },
            );
        }
        Ok(())
    }

    /// Workspace-backed records not yet cached for this catalog, read
    /// exactly several at a time: the same reads `exact` makes, run
    /// concurrently, only as many as this call's time allows. Failed reads
    /// are left to the sequential path, which reports them.
    fn prefetch(&mut self, catalog: &Catalog, slugs: &[String], reserve: Duration) {
        let spare = self.ctx.remaining().saturating_sub(reserve).as_millis() / EXACT_MS;
        let room = (spare as usize).saturating_mul(PARALLEL_READS);
        let wanted: Vec<String> = slugs
            .iter()
            .filter(|s| {
                catalog.workspace(s)
                    && !self.cache.get(s.as_str()).is_some_and(|c| {
                        c.fingerprint == catalog.fingerprint(s) && c.frontmatter.is_some()
                    })
            })
            .take(room)
            .cloned()
            .collect();
        if wanted.len() < 2 {
            return;
        }
        let per = wanted.len().div_ceil(PARALLEL_READS);
        let (ctx, root, workspace) = (&self.ctx, &self.root_text, &self.workspace_path);
        let results: Vec<(String, Result<Value, Refusal>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = wanted
                .chunks(per)
                .map(|part| {
                    let mut backend = self.backend.clone();
                    scope.spawn(move || {
                        part.iter()
                            .map(|slug| {
                                let record =
                                    read_source(&mut backend, ctx, root, workspace, slug, None);
                                (slug.clone(), record)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        for (slug, result) in results {
            if let Ok(record) = result {
                self.cache.insert(
                    slug.clone(),
                    Cached {
                        fingerprint: catalog.fingerprint(&slug),
                        summary: Summary::from_record(&record),
                        frontmatter: Some(record["frontmatter"].clone()),
                        record: Some(record),
                    },
                );
            }
        }
    }

    /// An exact record through the cache; `None` when its source is absent.
    pub fn exact(
        &mut self,
        catalog: &Catalog,
        slug: &str,
    ) -> Result<Option<(Summary, Value)>, Refusal> {
        let fingerprint = catalog.fingerprint(slug);
        if let Some(cached) = self.cache.get(slug)
            && cached.fingerprint == fingerprint
            && let Some(record) = &cached.record
        {
            return Ok(Some((cached.summary.clone(), record.clone())));
        }
        let record = match self.read(&slug.into(), None) {
            Ok(record) => record,
            Err(refusal) if refusal.code == "not_found" => return Ok(None),
            Err(refusal) => return Err(refusal),
        };
        let summary = Summary::from_record(&record);
        self.cache.insert(
            slug.into(),
            Cached {
                fingerprint,
                summary: summary.clone(),
                record: Some(record.clone()),
                frontmatter: Some(record["frontmatter"].clone()),
            },
        );
        Ok(Some((summary, record)))
    }

    /// Graph nodes for `slugs` in order: each summary with the frontmatter an
    /// exact read returns. Committed-only records take one batched read and
    /// build their frontmatter from its source bytes; workspace records and
    /// any batch entry that needs GitKB's own normalization are read exactly.
    /// Returns a prefix (at least one entry) when time runs short; `None`
    /// marks a listed record whose source read is absent.
    pub fn nodes(
        &mut self,
        catalog: &Catalog,
        slugs: &[String],
        reserve: Duration,
    ) -> Result<Vec<Option<(Summary, Value)>>, Refusal> {
        self.indexed_records(catalog, slugs)?;
        let hit = |cache: &HashMap<String, Cached>, slug: &str| {
            cache
                .get(slug)
                .filter(|c| c.fingerprint == catalog.fingerprint(slug))
                .and_then(|c| Some((c.summary.clone(), c.frontmatter.clone()?)))
        };
        let batch: Vec<&str> = slugs
            .iter()
            .filter(|s| !catalog.workspace(s) && hit(&self.cache, s).is_none())
            .map(String::as_str)
            .collect();
        if !batch.is_empty() {
            self.batch(catalog, &batch)?;
        }
        self.prefetch(catalog, slugs, reserve);
        let mut out = Vec::new();
        for slug in slugs {
            if let Some(node) = hit(&self.cache, slug) {
                out.push(Some(node));
                continue;
            }
            if !out.is_empty() && self.ctx.remaining() < reserve {
                break;
            }
            out.push(
                self.exact(catalog, slug)?
                    .map(|(summary, record)| (summary, record["frontmatter"].clone())),
            );
        }
        Ok(out)
    }

    /// One `git-kb show --raw-content --json -- <slug>...` for committed-only
    /// records; anything it does not return cleanly is read exactly later.
    fn batch(&mut self, catalog: &Catalog, slugs: &[&str]) -> Result<(), Refusal> {
        let mut args = vec!["show", "--raw-content", "--json", "--"];
        args.extend(slugs);
        let result = match self.cli(&args) {
            Ok(result) => result,
            Err(refusal)
                if matches!(
                    refusal.code.as_str(),
                    "cancelled" | "deadline_exceeded" | "backend_changed"
                ) =>
            {
                return Err(refusal);
            }
            Err(_) => return Ok(()),
        };
        let Some(documents) = result["documents"].as_array() else {
            return Ok(());
        };
        for doc in documents {
            let Some(slug) = doc["slug"].as_str() else {
                continue;
            };
            if !slugs.contains(&slug) {
                continue;
            }
            if let Some(summary) = Summary::from_batch(&self.root_text, slug, doc)? {
                let frontmatter = doc["rawContent"]
                    .as_str()
                    .and_then(native_frontmatter)
                    .map(|fm| {
                        project(
                            summary.reference["id"].as_str().unwrap_or(""),
                            slug,
                            doc,
                            &doc["type"],
                            fm,
                        )
                    })
                    .transpose()?
                    .map(|projected| Value::Object(projected.frontmatter));
                self.cache.insert(
                    slug.into(),
                    Cached {
                        fingerprint: catalog.fingerprint(slug),
                        summary,
                        record: None,
                        frontmatter,
                    },
                );
            }
        }
        Ok(())
    }

    /// Copy confined embedded images into the private resource directory.
    pub fn resources(&mut self, mut record: Value) -> Result<Value, Refusal> {
        let outside = |m: &str| Refusal::new("resource_outside_data_dir", m.to_owned());
        let body = record["body"].as_str().unwrap_or("").to_owned();
        for target in embeds(&body) {
            let relative = Path::new(&target);
            let components: Vec<Component> = relative.components().collect();
            if components.is_empty()
                || components
                    .iter()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(outside("image path escapes the GitKB root"));
            }
            let path = [
                self.root.join(relative),
                self.root.join(".kb").join(relative),
            ]
            .into_iter()
            .find(|p| p.symlink_metadata().is_ok())
            .ok_or_else(|| Refusal::new("resource_not_found", "GitKB image link is missing"))?;
            for parent in path.ancestors() {
                if parent == self.root {
                    break;
                }
                if parent
                    .symlink_metadata()
                    .is_ok_and(|m| m.file_type().is_symlink())
                {
                    return Err(outside("symlinked image path refused"));
                }
            }
            let unsafe_resource = || {
                Refusal::new(
                    "unsafe_resource",
                    "image must be a regular unlinked file, at most 64 MiB",
                )
            };
            let mut source = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .map_err(|_| unsafe_resource())?;
            let info = source.metadata().map_err(|_| unsafe_resource())?;
            if !info.is_file() || info.nlink() != 1 || info.len() > MAX_RESOURCE {
                return Err(unsafe_resource());
            }
            let mut data = Vec::new();
            (&mut source)
                .take(MAX_RESOURCE + 1)
                .read_to_end(&mut data)
                .map_err(|_| unsafe_resource())?;
            if data.len() as u64 > MAX_RESOURCE {
                return Err(Refusal::new("resource_too_large", "image exceeds 64 MiB"));
            }
            let directory = self.backend.cwd.join("resources");
            let _ = std::fs::DirBuilder::new().mode(0o700).create(&directory);
            if directory
                .symlink_metadata()
                .map_or(true, |m| m.file_type().is_symlink() || !m.is_dir())
            {
                return Err(outside("private resource directory is a symlink"));
            }
            let dest = directory.join(sha256_hex(&data));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&dest)
            {
                Ok(mut file) => file.write_all(&data).map_err(|_| unsafe_resource())?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let mut existing = Vec::new();
                    OpenOptions::new()
                        .read(true)
                        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                        .open(&dest)
                        .and_then(|f| f.take(data.len() as u64 + 1).read_to_end(&mut existing))
                        .map_err(|_| unsafe_resource())?;
                    if existing != data {
                        return Err(Refusal::new(
                            "resource_changed",
                            "private cached image changed",
                        ));
                    }
                }
                Err(_) => return Err(unsafe_resource()),
            }
            let name = relative
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(resources) = record["resources"].as_array_mut() {
                resources.push(json!({"type": "resource_link", "uri": file_uri(&dest),
                    "name": name, "mimeType": mime_type(&target), "size": data.len()}));
            }
        }
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shapes recorded from git-kb 0.2.19 (`show --source committed` and the
    /// batched `show --raw-content --json`) for one committed task.
    fn native_pair() -> (Value, Value) {
        let raw = "---\nid: 01a18d68-6fe0-7d03-92fd-8b75477fbe85\nslug: notes/a\ntitle: \"A\"\ntype: task\nstatus: active\nbboxy_status: claimed\nnumber: 77\nswarm: s1\n---\n\n# A\n";
        let exact = json!({
            "exists": true, "participates": true, "generation": "g1:c",
            "documentSource": {"kind": "committed"}, "committedRevision": null, "rawContent": raw,
            "document": {"id": "01a18d68-6fe0-7d03-92fd-8b75477fbe85", "slug": "notes/a",
                "version": 1, "doc_type": "task", "title": "A", "content": "# A\n",
                "frontmatter": {"status": "active", "swarm": "s1", "bboxy_status": "claimed", "number": 77},
                "created_at": "2026-10-30T15:23:14Z", "modified_at": "2026-10-30T15:23:14Z"}});
        let batch = json!({"id": "01a18d68-6fe0-7d03-92fd-8b75477fbe85", "slug": "notes/a",
            "type": "task", "title": "A", "status": "active", "assignee": null, "content": "# A\n", "version": 1,
            "modified_at": "2026-10-30T15:23:14+00:00",
            "frontmatter": {"bboxy_status": "claimed", "number": 77, "swarm": "s1"},
            "rawContent": raw});
        (exact, batch)
    }

    /// The record `read` builds from an exact committed show.
    fn record_from(root: &str, show: &Value) -> Value {
        let doc = &show["document"];
        let mut fm = doc["frontmatter"].as_object().unwrap().clone();
        let native_status = fm["status"].clone();
        fm.remove("bboxy_status");
        fm.insert("status".into(), task_status(&native_status));
        fm.insert("title".into(), doc["title"].clone());
        fm.insert("type".into(), doc["doc_type"].clone());
        let slug = doc["slug"].as_str().unwrap();
        json!({
            "id": portable(doc["id"].as_str().unwrap()).unwrap(),
            "external_id": doc["id"], "number": number(Some(&fm), slug).unwrap(),
            "slug": slug, "path": format!("{slug}.md"),
            "digest": committed_digest(root, doc["id"].as_str().unwrap(), &doc["version"]),
            "updated_at": committed_updated_at(&doc["modified_at"]),
            "frontmatter": fm, "body": doc["content"], "gitkb": {"native_status": native_status},
        })
    }
    /// `2026-10-30T15:23:14Z`, the fixture's committed `modified_at`.
    const MODIFIED_MS: u64 = 1_793_373_794_000;

    #[test]
    fn batch_reference_digest_equals_exact_read() {
        let (exact, batch) = native_pair();
        let record = record_from("/kb", &exact);
        let summary = Summary::from_batch("/kb", "notes/a", &batch)
            .unwrap()
            .unwrap();
        assert_eq!(summary.reference, reference(&record));
        assert_eq!(summary.reference["number"], 77);
        // `show` says `…Z`, the batch `…+00:00`: one instant, one updated_at.
        assert_eq!(summary.reference["updated_at"], MODIFIED_MS);
        let from_exact = Summary::from_record(&record);
        assert_eq!(summary.status, from_exact.status);
        assert_eq!(summary.native_status, from_exact.native_status);
        assert_eq!(summary.swarm, from_exact.swarm);
        assert_eq!(summary.snippet, from_exact.snippet);
        // Another root or source never shares a digest.
        let other = Summary::from_batch("/elsewhere", "notes/a", &batch)
            .unwrap()
            .unwrap();
        assert_ne!(other.reference["digest"], summary.reference["digest"]);
        assert!(
            Summary::from_batch("/kb", "notes/b", &batch)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn indexed_summary_and_node_equal_exact_read() {
        let (exact, _) = native_pair();
        let record = record_from("/kb", &exact);
        let doc = &exact["document"];
        // The export line for the same committed document: GitKB's parsed
        // frontmatter plus graph-only relationship keys, which the index drops.
        let mut fm = doc["frontmatter"].clone();
        fm["references"] = json!(["notes/b"]);
        fm["parent"] = json!("notes/p");
        let content = json!({"id": doc["id"], "slug": "notes/a", "version": doc["version"],
            "doc_type": doc["doc_type"], "title": doc["title"], "content": doc["content"],
            "frontmatter": fm, "modified_at": doc["modified_at"]});
        let line = format!(
            "{}\n",
            encoded(&json!({"slug": "notes/a", "content": encoded(&content)}))
        );
        let committed = Committed::parse("s".into(), line.as_bytes(), 1 << 20).unwrap();
        let entry = committed.doc("notes/a", &doc["id"]).unwrap();
        let (summary, frontmatter) = Summary::from_index("/kb", "notes/a", entry).unwrap();
        assert_eq!(summary.reference, reference(&record));
        assert_eq!(summary.reference["updated_at"], MODIFIED_MS);
        let from_exact = Summary::from_record(&record);
        for (a, b) in [
            (&summary.status, &from_exact.status),
            (&summary.native_status, &from_exact.native_status),
            (&summary.swarm, &from_exact.swarm),
            (&summary.title, &from_exact.title),
            (&summary.kind, &from_exact.kind),
        ] {
            assert_eq!(a, b);
        }
        assert_eq!(summary.snippet, from_exact.snippet);
        let id = portable(doc["id"].as_str().unwrap()).unwrap();
        let exact_fm = project(
            &id,
            "notes/a",
            doc,
            &doc["doc_type"],
            doc["frontmatter"].as_object().unwrap().clone(),
        )
        .unwrap()
        .frontmatter;
        assert_eq!(frontmatter, Value::Object(exact_fm));
        // A later commit bumps the version and so the digest, and its time.
        let mut next = content.clone();
        next["version"] = json!(2);
        next["modified_at"] = json!("2026-10-31T00:00:00.000001Z");
        let line = format!(
            "{}\n",
            encoded(&json!({"slug": "notes/a", "content": encoded(&next)}))
        );
        let bumped = Committed::parse("s2".into(), line.as_bytes(), 1 << 20).unwrap();
        let entry = bumped.doc("notes/a", &doc["id"]).unwrap();
        let (moved, _) = Summary::from_index("/kb", "notes/a", entry).unwrap();
        assert_ne!(moved.reference["digest"], summary.reference["digest"]);
        assert_eq!(moved.reference["updated_at"], 1_793_404_800_000_u64);
    }

    #[test]
    fn batch_node_frontmatter_equals_exact_read() {
        let (exact, batch) = native_pair();
        let doc = &exact["document"];
        let id = portable(doc["id"].as_str().unwrap()).unwrap();
        let from_exact = project(
            &id,
            "notes/a",
            doc,
            &doc["doc_type"],
            doc["frontmatter"].as_object().unwrap().clone(),
        )
        .unwrap();
        let native = native_frontmatter(batch["rawContent"].as_str().unwrap()).unwrap();
        let from_batch = project(&id, "notes/a", &batch, &batch["type"], native).unwrap();
        assert_eq!(from_batch.frontmatter, from_exact.frontmatter);
        // A legacy bboxy_status shadow reads through the native status alone.
        assert_eq!(from_exact.frontmatter["status"], "running");
        assert_eq!(from_exact.frontmatter["number"], 77);
        assert!(!from_exact.frontmatter.contains_key("bboxy_status"));
    }

    #[test]
    fn native_frontmatter_follows_gitkb_typed_round_trip() {
        // What GitKB 0.2.19 `show --source committed` returns for this YAML:
        // document-level and relationship keys removed, null typed fields and
        // empty typed lists omitted, extra fields (null and dates) kept as YAML gives them.
        let raw = "---\nid: 01a18d68-6fe0-7d03-92fd-8b75477fbe85\nslug: notes/a\ntitle: A\ntype: note\n\
            status: draft\nassignee: null\ntags: []\nparent: notes/p\nin:\n  notes/p: a0\n\
            depends_on: [notes/b]\nblocked_by: [notes/c]\nimplements: [specs/s]\n\
            references: [notes/d]\ncustom: null\ncreated: 2026-10-30T15:09:46Z\nnested: {k: [1, two]}\n---\n\nBody\n";
        assert_eq!(
            Value::Object(native_frontmatter(raw).unwrap()),
            json!({"status": "draft", "references": ["notes/d"], "custom": null,
                "created": "2026-10-30T15:09:46Z", "nested": {"k": [1, "two"]}})
        );
        let doc = json!({"slug": "notes/a", "title": "A"});
        let projected = project(
            "01JBAAAAAAAAAAAAAAAAAAAAAA",
            "notes/a",
            &doc,
            &"note".into(),
            native_frontmatter(raw).unwrap(),
        )
        .unwrap();
        assert!(!projected.frontmatter.contains_key("references"));
        assert_eq!(projected.frontmatter["status"], "draft");
        // Fields GitKB normalizes itself, and malformed blocks, read exactly.
        for fallback in [
            "---\ndue_date: 2026-11-01\n---\n",
            "---\nexternal_refs: []\n---\n",
            "---\npriority: 1\n---\n",
            "---\ntags: [1]\n---\n",
            "---\nstatus: [open\n---\n",
            "---\r\nstatus: open\r\n---\r\n",
            "no frontmatter\n",
        ] {
            assert!(native_frontmatter(fallback).is_none(), "{fallback:?}");
        }
        assert_eq!(native_frontmatter("---\n\n---\nBody\n"), Some(Map::new()));
        assert_eq!(
            native_frontmatter("---\nstatus: open\n---"),
            Some(json!({"status": "open"}).as_object().unwrap().clone())
        );
    }

    #[test]
    fn status_filter_accepts_portable_or_native_value() {
        let (_, batch) = native_pair();
        let task = Summary::from_batch("/kb", "notes/a", &batch)
            .unwrap()
            .unwrap();
        let filter = |v: Value| v.as_object().unwrap().clone();
        // The legacy bboxy_status shadow is not read: native active is running.
        assert!(!task.matches(&filter(json!({"type": "task", "status": "claimed"}))));
        assert!(task.matches(&filter(json!({"status": "active"}))));
        assert!(task.matches(&filter(json!({"type": "task", "status": "running"}))));
        assert!(task.matches(&filter(
            json!({"type": "task", "status": "", "swarm": "s1"})
        )));
        assert!(!task.matches(&filter(json!({"status": "open"}))));
        assert!(!task.matches(&filter(json!({"swarm": "s2"}))));
        assert!(!task.matches(&filter(json!({"type": "note"}))));
    }

    #[test]
    fn signed_proofs_are_kept_per_document_version_outside_the_kb() {
        use std::os::unix::fs::PermissionsExt;
        let data = std::env::temp_dir().join(format!("bboxy-gitkb-signed-{}", random_hex(8)));
        std::fs::create_dir(&data).unwrap();
        let id = "01a18d68-6fe0-7d03-92fd-8b75477fbe85";
        assert_eq!(load_signed(&data, id, "d1"), None);
        store_signed(&data, id, "d1", "proof one").unwrap();
        assert_eq!(load_signed(&data, id, "d1").as_deref(), Some("proof one"));
        // Another version or another document never sees it.
        assert_eq!(load_signed(&data, id, "d2"), None);
        assert_eq!(load_signed(&data, "other", "d1"), None);
        let mode = std::fs::metadata(signed_path(&data, id, "d1"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
        std::fs::remove_dir_all(&data).unwrap();
    }

    #[test]
    fn cursors_keep_the_newest_128() {
        let mut cursors = Cursors::default();
        let first = cursors.token(json!(0));
        let tokens: Vec<String> = (1..=128).map(|i| cursors.token(json!(i))).collect();
        assert!(cursors.get(&first).is_none());
        assert_eq!(cursors.get(&tokens[0]), Some(&json!(1)));
        assert_eq!(cursors.get(&tokens[127]), Some(&json!(128)));
        assert_ne!(tokens[0], tokens[1]);
    }
}
