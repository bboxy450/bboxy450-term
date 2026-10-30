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
use std::time::Duration;

use bbt_plugin_sdk::{Ctx, MAX_RESULT, Refusal, encoded};
use serde_json::{Map, Value, json};

use crate::backend::Backend;
use crate::native::{
    content_digest, digest, embeds, file_uri, mime_type, number, portable, prefix, random_hex,
    sha256_hex, slug_path, task_status,
};

/// Time left for the reply when a page stops filling.
pub const RESERVE: Duration = Duration::from_millis(400);
/// Rough cost of one batched record, used to size a batch to the time left.
pub const PER_DOC_MS: u128 = 15;
/// Serialized bytes a page may fill; the rest covers its envelope.
pub const PAGE_BYTES: usize = MAX_RESULT - 8 * 1024;
const MAX_CURSORS: usize = 128;
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

    pub fn merge(&mut self, other: Catalog) {
        for slug in other.order {
            if !self.rows.contains_key(&slug) {
                self.order.push(slug.clone());
            }
            self.rows.insert(slug.clone(), other.rows[&slug].clone());
        }
        self.seal();
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
    /// document. GitKB returns the committed bytes when the workspace has no
    /// file, so the digest equals an exact read's. `None` asks for an exact read.
    pub fn from_batch(root: &str, slug: &str, doc: &Value) -> Result<Option<Self>, Refusal> {
        let (Some(id), Some(raw)) = (doc["id"].as_str(), doc["rawContent"].as_str()) else {
            return Ok(None);
        };
        if doc["slug"] != slug {
            return Ok(None);
        }
        let extras = doc["frontmatter"].as_object();
        let kind = doc["type"].clone();
        let native_status = doc["status"].clone();
        let status = if kind == "task" {
            task_status(extras.and_then(|fm| fm.get("bboxy_status")), &native_status)
        } else {
            native_status.clone()
        };
        let extra = |key: &str| extras.and_then(|fm| fm.get(key)).cloned();
        let assignee = match &doc["assignee"] {
            Value::Null => extra("assignee").unwrap_or(Value::Null),
            value => value.clone(),
        };
        Ok(Some(Self {
            reference: json!({
                "id": portable(id)?,
                "external_id": id,
                "number": number(extras, slug)?,
                "slug": slug,
                "path": format!("{slug}.md"),
                "digest": content_digest(root, &json!({"kind": "committed"}), raw),
            }),
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
    Value::Object(out)
}

struct Cached {
    fingerprint: String,
    summary: Summary,
    record: Option<Value>,
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
        if self
            .workspace_path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(Refusal::new(
                "workspace_outside_root",
                "symlinked workspace refused",
            ));
        }
        let mut result = Map::new();
        let mut pending = vec![self.workspace_path.clone()];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                if self.ctx.remaining().is_zero() {
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
                        .strip_prefix(&self.workspace_path)
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

    pub fn catalog(&mut self, flags: &[String]) -> Result<Catalog, Refusal> {
        let mut args = vec!["list", "--json", "--sort", "title", "--asc"];
        args.extend(flags.iter().map(String::as_str));
        let rows = self.cli(&args)?;
        let Value::Array(rows) = rows else {
            return Err(Refusal::new(
                "backend_protocol_error",
                "GitKB list is not an array",
            ));
        };
        let stats = self.workspace_stats()?;
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
        let mut workspace_generation = Value::Null;
        let source = requested.unwrap_or("workspace");
        let mut result = self.cli(&[
            "show",
            "--source",
            source,
            "--raw-content",
            "--json",
            "--",
            &slug,
        ])?;
        if result["exists"] != true {
            if requested == Some("workspace") || result["participates"] == true {
                return Err(Refusal::new(
                    "not_found",
                    "the selected workspace source is deleted or absent",
                ));
            }
            workspace_generation = result["generation"].clone();
            result = self.cli(&[
                "show",
                "--source",
                "committed",
                "--raw-content",
                "--json",
                "--",
                &slug,
            ])?;
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
        // GitKB declares these projections graph-derived, never source YAML.
        let mut fm: Map<String, Value> = doc["frontmatter"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, _)| !matches!(k.as_str(), "references" | "blocks" | "children"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        fm.insert("id".into(), id.clone().into());
        fm.insert("slug".into(), doc["slug"].clone());
        fm.insert("title".into(), doc["title"].clone());
        fm.insert("type".into(), doc["doc_type"].clone());
        let native_status = fm.get("status").cloned().unwrap_or(Value::Null);
        if fm["type"] == "task" {
            let bboxy = fm.remove("bboxy_status");
            fm.insert("status".into(), task_status(bboxy.as_ref(), &native_status));
        }
        let number = number(Some(&fm), &slug)?;
        let source = result["documentSource"].clone();
        let mut record = json!({
            "id": id, "external_id": native_id, "number": number, "slug": slug,
            "path": format!("{slug}.md"),
            "digest": content_digest(&self.root_text, &source, raw),
            "body": doc["content"], "etag": result["generation"], "resources": [],
            "gitkb": {"source": source, "native_status": native_status,
                      "committed_revision": result.get("committedRevision").cloned().unwrap_or(Value::Null),
                      "version": doc["version"], "created_at": doc["created_at"],
                      "modified_at": doc["modified_at"], "raw_content": raw},
        });
        if let Some(signed) = fm.remove("bboxy_signed_record") {
            record["signed_record"] = signed;
        }
        if truthy(&workspace_generation) {
            record["gitkb"]["workspace_generation"] = workspace_generation;
        }
        if !number.is_null() {
            fm.insert("number".into(), number);
        }
        record["frontmatter"] = Value::Object(fm);
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
            },
        );
        Ok(Some((summary, record)))
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
                self.cache.insert(
                    slug.into(),
                    Cached {
                        fingerprint: catalog.fingerprint(slug),
                        summary,
                        record: None,
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
            "type": "task", "title": "A", "status": "active", "assignee": null, "content": "# A\n",
            "frontmatter": {"bboxy_status": "claimed", "number": 77, "swarm": "s1"},
            "rawContent": raw});
        (exact, batch)
    }

    /// The record `read` builds from an exact committed show.
    fn record_from(root: &str, show: &Value) -> Value {
        let doc = &show["document"];
        let mut fm = doc["frontmatter"].as_object().unwrap().clone();
        let native_status = fm["status"].clone();
        let bboxy = fm.remove("bboxy_status");
        fm.insert("status".into(), task_status(bboxy.as_ref(), &native_status));
        fm.insert("title".into(), doc["title"].clone());
        fm.insert("type".into(), doc["doc_type"].clone());
        let slug = doc["slug"].as_str().unwrap();
        json!({
            "id": portable(doc["id"].as_str().unwrap()).unwrap(),
            "external_id": doc["id"], "number": number(Some(&fm), slug).unwrap(),
            "slug": slug, "path": format!("{slug}.md"),
            "digest": content_digest(root, &show["documentSource"], show["rawContent"].as_str().unwrap()),
            "frontmatter": fm, "body": doc["content"], "gitkb": {"native_status": native_status},
        })
    }

    #[test]
    fn batch_reference_digest_equals_exact_read() {
        let (exact, batch) = native_pair();
        let record = record_from("/kb", &exact);
        let summary = Summary::from_batch("/kb", "notes/a", &batch)
            .unwrap()
            .unwrap();
        assert_eq!(summary.reference, reference(&record));
        assert_eq!(summary.reference["number"], 77);
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
    fn status_filter_accepts_portable_or_native_value() {
        let (_, batch) = native_pair();
        let task = Summary::from_batch("/kb", "notes/a", &batch)
            .unwrap()
            .unwrap();
        let filter = |v: Value| v.as_object().unwrap().clone();
        assert!(task.matches(&filter(json!({"type": "task", "status": "claimed"}))));
        assert!(task.matches(&filter(json!({"status": "active"}))));
        assert!(task.matches(&filter(
            json!({"type": "task", "status": "", "swarm": "s1"})
        )));
        assert!(!task.matches(&filter(json!({"status": "running"}))));
        assert!(!task.matches(&filter(json!({"swarm": "s2"}))));
        assert!(!task.matches(&filter(json!({"type": "note"}))));
        let mut running = task.clone();
        running.status = "running".into();
        assert!(running.matches(&filter(json!({"type": "task", "status": "running"}))));
        assert!(running.matches(&filter(json!({"type": "task", "status": "active"}))));
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
