// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The in-memory catalog index: answers the adapter's own filtered
//! `git-kb list` flags without a backend call per filter. New in this crate.
//!
//! Two layers, both keyed by the cheap store signal:
//! - `Committed`: the `--where` fields of every committed document, from one
//!   `git-kb export --format jsonl`, built off the request thread.
//! - `Listed`: one unfiltered native `git-kb list` (the exact catalog rows),
//!   also keyed by the workspace file stats.
//!
//! Selection reproduces native 0.2.19 prefilter semantics measured on a
//! private fixture: `--type`/`--status` lowercase their input and compare the
//! listed row exactly; `--where field = value` compares the committed
//! frontmatter for committed documents (even when a workspace file differs)
//! and the workspace source for workspace-only documents. A document whose
//! fields are unknown stays a candidate; the caller's exact or batched summary
//! applies the authoritative filter, so the index only ever narrows.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bbt_plugin_sdk::{Ctx, encoded};
use serde_json::{Map, Value, json};

use crate::backend::Backend;
use crate::native::digest;
use crate::store::native_frontmatter;

/// Largest accepted export output (Harmony's 3691 documents: 61 MB).
pub const MAX_EXPORT: u64 = 1 << 30;
const MAX_LINE: usize = 64 << 20;
const MAX_DOCS: usize = 1_000_000;
const MAX_OVERLAY: u64 = 1 << 20;
const BUILD_BUDGET: Duration = Duration::from_secs(120);
const ATTEMPTS: usize = 3;
/// Call budget kept back when a filtered call waits for a build: the row
/// refresh, a backend fallback and the page fill must still fit.
pub const WAIT_RESERVE: Duration = Duration::from_millis(800);
/// Longest wait of one call. The host's per-call deadline is at most 2 s and
/// shrinks to whatever its operation (an 8 s task list) has left, which the
/// plugin cannot see; a short wait keeps each call, wait plus fallback, near
/// one second, so a cold list stays within the operation's own deadline.
pub const WAIT_CAP: Duration = Duration::from_millis(700);
/// Longest single condition wait, so cancellation is noticed promptly.
const WAIT_SLICE: Duration = Duration::from_millis(50);
/// The frontmatter fields the adapter filters with `--where`.
pub const WHERE_FIELDS: [&str; 3] = ["assignee", "swarm", "bboxy_status"];

type Fields = [Value; 3];

/// Store identity: (dev, ino, size, mtime, ctime) of the store directory,
/// its manifest, document and commit directories, refs directory and
/// document tips. A native commit replaces the manifest and tips by rename;
/// workspace writes and reads change none of them (measured on 0.2.19).
/// GitKB's `.cache` database is excluded: every read rewrites it.
pub fn store_signal(root: &Path) -> Option<String> {
    let store = root.join(".kb/store");
    let mut parts = Vec::new();
    for (relative, required) in [
        ("", true),
        ("manifest.json", true),
        ("documents", false),
        ("commits", false),
        ("refs", false),
        ("refs/document-tips.json", false),
    ] {
        let path = if relative.is_empty() {
            store.clone()
        } else {
            store.join(relative)
        };
        match path.symlink_metadata() {
            Ok(m) if !m.file_type().is_symlink() => parts.push(json!([
                m.dev(),
                m.ino(),
                m.size(),
                m.mtime(),
                m.mtime_nsec(),
                m.ctime(),
                m.ctime_nsec()
            ])),
            Err(_) if !required => parts.push(Value::Null),
            _ => return None,
        }
    }
    Some(digest(&encoded(&Value::Array(parts))))
}

/// Committed `--where` fields by slug, with the native id they belong to.
#[derive(Debug)]
pub struct Committed {
    pub signal: String,
    docs: HashMap<String, (String, Fields)>,
    /// Lines refused, by reason; their documents stay candidates.
    pub skipped: Map<String, Value>,
}

impl Committed {
    /// Parse `git-kb export --format jsonl`: one `{slug, content}` line per
    /// committed document, `content` itself JSON with `id`, `slug` and
    /// `frontmatter`. Malformed and duplicate lines are counted and skipped.
    pub fn parse(signal: String, reader: impl Read, max_line: usize) -> Result<Self, String> {
        let mut reader = BufReader::new(reader);
        let mut docs = HashMap::new();
        let mut duplicates = Vec::new();
        let mut skipped = Map::new();
        let mut count = |reason: &str| {
            let n = skipped.get(reason).and_then(Value::as_u64).unwrap_or(0);
            skipped.insert(reason.into(), (n + 1).into());
        };
        let mut line = Vec::new();
        loop {
            line.clear();
            let n = (&mut reader)
                .take(max_line as u64 + 1)
                .read_until(b'\n', &mut line)
                .map_err(|e| format!("export read: {e}"))?;
            if n == 0 {
                break;
            }
            if line.len() > max_line {
                return Err("export line exceeds its size bound".into());
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match entry(&line) {
                Some((slug, value)) => {
                    if docs.insert(slug.clone(), value).is_some() {
                        duplicates.push(slug);
                        count("duplicate");
                    }
                }
                None => count("malformed"),
            }
            if docs.len() > MAX_DOCS {
                return Err("export exceeds 1000000 documents".into());
            }
        }
        // A repeated slug has no single committed value: keep it a candidate.
        for slug in duplicates {
            docs.remove(&slug);
        }
        Ok(Self {
            signal,
            docs,
            skipped,
        })
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    fn fields(&self, slug: &str, id: &Value) -> Option<&Fields> {
        self.docs
            .get(slug)
            .filter(|(native, _)| id == native.as_str())
            .map(|(_, fields)| fields)
    }
}

fn entry(line: &[u8]) -> Option<(String, (String, Fields))> {
    let outer: Value = serde_json::from_slice(line).ok()?;
    let slug = outer["slug"].as_str()?;
    let content: Value = serde_json::from_str(outer["content"].as_str()?).ok()?;
    if content["slug"] != slug {
        return None;
    }
    let id = content["id"].as_str()?;
    let fm = content["frontmatter"].as_object()?;
    let fields = WHERE_FIELDS.map(|f| fm.get(f).cloned().unwrap_or(Value::Null));
    Some((slug.into(), (id.into(), fields)))
}

/// The `--where` fields of a workspace-only document, from its workspace
/// source as GitKB parses it; `None` keeps the document a candidate.
pub fn workspace_fields(path: &Path) -> Option<Fields> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let info = file.metadata().ok()?;
    if !info.is_file() || info.len() > MAX_OVERLAY {
        return None;
    }
    let mut raw = String::new();
    (&mut file)
        .take(MAX_OVERLAY + 1)
        .read_to_string(&mut raw)
        .ok()?;
    let fm = native_frontmatter(&raw)?;
    Some(WHERE_FIELDS.map(|f| fm.get(f).cloned().unwrap_or(Value::Null)))
}

/// A filter the index can answer, parsed back from the adapter's own
/// `git-kb list` flags. Anything else (`--since`, other fields, values the
/// native parser might type or split differently) uses the backend.
#[derive(Debug, Default, PartialEq)]
pub struct Query {
    types: Option<Vec<String>>,
    statuses: Option<Vec<String>>,
    wheres: Vec<(usize, String)>,
}

/// A value both parsers read as the same plain string.
fn plain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:@/+-".contains(&b))
        && value.parse::<f64>().is_err()
        && !matches!(value, "true" | "false" | "null")
}

/// Comma-separated `--type`/`--status` values, lowercased as GitKB does.
fn tokens(value: &str) -> Option<Vec<String>> {
    value
        .split(',')
        .map(|t| plain(t).then(|| t.to_ascii_lowercase()))
        .collect()
}

impl Query {
    pub fn parse(flags: &[String]) -> Option<Self> {
        let mut query = Self::default();
        let mut flags = flags.iter();
        while let Some(flag) = flags.next() {
            let value = flags.next()?;
            match flag.as_str() {
                "--type" if query.types.is_none() => query.types = Some(tokens(value)?),
                "--status" if query.statuses.is_none() => query.statuses = Some(tokens(value)?),
                "--where" => {
                    let (field, value) = value.split_once(" = ")?;
                    let field = WHERE_FIELDS.iter().position(|f| *f == field)?;
                    if !plain(value) {
                        return None;
                    }
                    query.wheres.push((field, value.into()));
                }
                _ => return None,
            }
        }
        Some(query)
    }

    /// Whether committed `--where` fields are needed.
    pub fn needs_fields(&self) -> bool {
        !self.wheres.is_empty()
    }

    /// The native prefilter for one listed row; `fields` is `None` when the
    /// row's `--where` values are unknown, which admits it.
    pub fn admits(&self, row: &Value, fields: Option<&Fields>) -> bool {
        let one_of = |set: &Option<Vec<String>>, key: &str| {
            set.as_ref().is_none_or(|set| {
                row[key]
                    .as_str()
                    .is_some_and(|v| set.iter().any(|t| t == v))
            })
        };
        one_of(&self.types, "type")
            && one_of(&self.statuses, "status")
            && self
                .wheres
                .iter()
                .all(|(i, value)| fields.is_none_or(|f| f[*i] == value.as_str()))
    }
}

/// One unfiltered native listing, valid for one store signal and one
/// workspace stat digest, with lazily parsed workspace-only fields.
pub struct Listed {
    pub signal: String,
    pub workspace: String,
    pub rows: Vec<Value>,
    overlay: HashMap<String, Option<Fields>>,
}

impl Listed {
    pub fn new(signal: String, workspace: String, rows: Vec<Value>) -> Self {
        Self {
            signal,
            workspace,
            rows,
            overlay: HashMap::new(),
        }
    }

    /// Rows `query` admits, in native order. Committed documents use their
    /// committed fields (native semantics, even under a workspace edit);
    /// workspace-only documents use their workspace source.
    pub fn select(
        &mut self,
        query: &Query,
        committed: Option<&Committed>,
        workspace_root: &Path,
        stats: &Map<String, Value>,
    ) -> Vec<Value> {
        let mut out = Vec::new();
        for row in &self.rows {
            let slug = row["slug"].as_str().unwrap_or("");
            let fields = if !query.needs_fields() {
                None
            } else if let Some(fields) = committed.and_then(|c| c.fields(slug, &row["id"])) {
                Some(fields)
            } else if committed.is_some_and(|c| c.docs.contains_key(slug))
                || !stats.contains_key(slug)
            {
                None
            } else {
                self.overlay
                    .entry(slug.to_owned())
                    .or_insert_with(|| workspace_fields(&workspace_root.join(format!("{slug}.md"))))
                    .as_ref()
            };
            if query.admits(row, fields) {
                out.push(row.clone());
            }
        }
        out
    }
}

/// Build state shared with the background thread.
#[derive(Default)]
struct Shared {
    committed: Option<Arc<Committed>>,
    building: bool,
    builds: u64,
    failures: u64,
    discarded: u64,
    last_error: Option<String>,
    build_ms: Option<u64>,
    ready_ms: Option<u64>,
    /// Snapshot loaded at initialize, its load time, saves and last failure.
    loaded_us: Option<u64>,
    saves: u64,
    save_error: Option<String>,
}

/// The build state and the condition a finished build signals.
#[derive(Default)]
struct Build {
    state: Mutex<Shared>,
    done: Condvar,
}

fn lock(shared: &Build) -> MutexGuard<'_, Shared> {
    shared.state.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Index {
    enabled: bool,
    started: Instant,
    shared: Arc<Build>,
    /// The plugin's private data directory, where the snapshot lives.
    dir: Option<PathBuf>,
    root_text: String,
    pub listed: Option<Listed>,
    pub served: u64,
    pub fallbacks: u64,
    /// Filtered calls that waited for a build in progress, and for how long.
    pub waits: u64,
    pub waited_ms: u64,
}

impl Index {
    pub fn new() -> Self {
        Self {
            enabled: false,
            started: Instant::now(),
            shared: Arc::default(),
            dir: None,
            root_text: String::new(),
            listed: None,
            served: 0,
            fallbacks: 0,
            waits: 0,
            waited_ms: 0,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the index and start its first build. `BBOXY_GITKB_INDEX=off`
    /// in the plugin's own environment keeps every call on the backend path.
    /// A snapshot in the plugin's private data directory (its cwd) for the
    /// current store signal is loaded synchronously, so a respawned process
    /// answers filtered calls at once; otherwise the build starts.
    pub fn start(&mut self, backend: &Backend, root: &Path) {
        self.enabled = std::env::var("BBOXY_GITKB_INDEX").as_deref() != Ok("off");
        self.dir = Some(backend.cwd.clone());
        self.root_text = root.display().to_string();
        let Some(signal) = self.enabled.then(|| store_signal(root)).flatten() else {
            return;
        };
        let began = Instant::now();
        if let Some(committed) = load(&backend.cwd, &self.root_text, &signal) {
            let mut state = lock(&self.shared);
            state.loaded_us = Some(began.elapsed().as_micros() as u64);
            state.ready_ms = Some(self.started.elapsed().as_millis() as u64);
            state.committed = Some(Arc::new(committed));
            return;
        }
        self.committed(&signal, backend, root);
    }

    /// Committed fields fresh for `signal`; a stale or missing index starts
    /// one background rebuild and answers `None` meanwhile.
    pub fn committed(
        &self,
        signal: &str,
        backend: &Backend,
        root: &Path,
    ) -> Option<Arc<Committed>> {
        let mut shared = lock(&self.shared);
        if let Some(committed) = shared.committed.as_ref().filter(|c| c.signal == signal) {
            return Some(committed.clone());
        }
        if !shared.building {
            shared.building = true;
            drop(shared);
            self.spawn(backend.clone(), root.to_owned());
        }
        None
    }

    fn spawn(&self, backend: Backend, root: PathBuf) {
        let shared = self.shared.clone();
        let started = self.started;
        let dir = self.dir.clone();
        let root_text = self.root_text.clone();
        let spawned = std::thread::Builder::new()
            .name("gitkb-index".into())
            .spawn(move || {
                let began = Instant::now();
                let result = build(backend, &root);
                // Save before publishing, so a process torn down right after
                // its build still leaves the snapshot for its successor.
                let saved = match (&result, &dir) {
                    (Ok((committed, _)), Some(dir)) => Some(save(dir, &root_text, committed)),
                    _ => None,
                };
                let mut state = lock(&shared);
                match saved {
                    Some(Ok(())) => state.saves += 1,
                    Some(Err(error)) => state.save_error = Some(error.to_string()),
                    None => {}
                }
                state.building = false;
                state.builds += 1;
                match result {
                    Ok((committed, discarded)) => {
                        state.discarded += discarded;
                        state.build_ms = Some(began.elapsed().as_millis() as u64);
                        state
                            .ready_ms
                            .get_or_insert(started.elapsed().as_millis() as u64);
                        state.committed = Some(Arc::new(committed));
                    }
                    Err((error, discarded)) => {
                        state.discarded += discarded;
                        state.failures += 1;
                        state.last_error = Some(error);
                    }
                }
                drop(state);
                shared.done.notify_all();
            });
        if spawned.is_err() {
            let mut state = lock(&self.shared);
            state.building = false;
            state.failures += 1;
            state.last_error = Some("index thread unavailable".into());
        }
    }

    /// Wait up to `budget` for the build in progress to produce committed
    /// fields fresh for `signal`. `None` when no build is running, it failed,
    /// it finished for another signal, the budget ran out or `cancelled`.
    pub fn wait_for(
        &self,
        signal: &str,
        budget: Duration,
        cancelled: impl Fn() -> bool,
    ) -> Option<Arc<Committed>> {
        let deadline = Instant::now() + budget;
        let mut state = lock(&self.shared);
        loop {
            if let Some(committed) = state.committed.as_ref().filter(|c| c.signal == signal) {
                return Some(committed.clone());
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if !state.building || left.is_zero() || cancelled() {
                return None;
            }
            state = self
                .shared
                .done
                .wait_timeout(state, left.min(WAIT_SLICE))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Health facts; `signal` is the current store signal.
    pub fn facts(&self, signal: Option<&str>) -> Value {
        let state = lock(&self.shared);
        let committed = state.committed.as_ref();
        json!({
            "enabled": self.enabled,
            "ready": committed.is_some_and(|c| Some(c.signal.as_str()) == signal),
            "building": state.building,
            "documents": committed.map(|c| c.len()),
            "skipped": committed.map(|c| Value::Object(c.skipped.clone())),
            "builds": state.builds,
            "failures": state.failures,
            "discarded": state.discarded,
            "last_error": state.last_error,
            "build_ms": state.build_ms,
            "ready_ms": state.ready_ms,
            "snapshot": {"loaded_us": state.loaded_us, "saves": state.saves,
                "save_error": state.save_error},
            "served": self.served,
            "fallbacks": self.fallbacks,
            "waits": self.waits,
            "waited_ms": self.waited_ms,
            "store_signal": signal,
            "indexed_signal": committed.map(|c| c.signal.clone()),
        })
    }
}

/// Snapshot format version; a different version is ignored and rebuilt.
const SNAPSHOT_VERSION: u64 = 1;
const MAX_SNAPSHOT: u64 = 256 << 20;

/// One snapshot file per root in the plugin's private data directory.
fn snapshot_path(dir: &Path, root: &str) -> PathBuf {
    dir.join(format!("gitkb-index-{}.json", &digest(root)[..24]))
}

/// Write the committed layer atomically: a private new file renamed over
/// the old one, so a reader sees the old or the new snapshot, never a part.
pub fn save(dir: &Path, root: &str, committed: &Committed) -> std::io::Result<()> {
    use std::io::Write;
    let docs: Vec<Value> = committed
        .docs
        .iter()
        .map(|(slug, (id, fields))| json!([slug, id, fields[0], fields[1], fields[2]]))
        .collect();
    let body = encoded(&json!({"version": SNAPSHOT_VERSION, "root": root,
        "signal": committed.signal, "skipped": committed.skipped, "docs": docs}));
    let path = snapshot_path(dir, root);
    let temporary = dir.join(format!(
        ".gitkb-index-{}.tmp",
        crate::native::random_hex(12)
    ));
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, &path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// The snapshot for this root and exactly this store signal and version;
/// anything else (absent, other signal, malformed, oversized, symlinked)
/// is `None` and the index builds.
pub fn load(dir: &Path, root: &str, signal: &str) -> Option<Committed> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(snapshot_path(dir, root))
        .ok()?;
    let info = file.metadata().ok()?;
    if !info.is_file() || info.len() > MAX_SNAPSHOT {
        return None;
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_SNAPSHOT + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    if value["version"] != SNAPSHOT_VERSION || value["root"] != root || value["signal"] != signal {
        return None;
    }
    let rows = value["docs"].as_array()?;
    if rows.len() > MAX_DOCS {
        return None;
    }
    let mut docs = HashMap::with_capacity(rows.len());
    for row in rows {
        let [slug, id, a, s, b] = row.as_array()?.as_slice() else {
            return None;
        };
        let fields = [a.clone(), s.clone(), b.clone()];
        docs.insert(slug.as_str()?.to_owned(), (id.as_str()?.to_owned(), fields));
    }
    Some(Committed {
        signal: signal.to_owned(),
        docs,
        skipped: value["skipped"].as_object().cloned().unwrap_or_default(),
    })
}

/// Export committed state between two equal store signals; a commit during
/// the export discards that attempt.
fn build(mut backend: Backend, root: &Path) -> Result<(Committed, u64), (String, u64)> {
    let ctx = Ctx::detached(BUILD_BUDGET);
    let mut discarded = 0;
    for _ in 0..ATTEMPTS {
        let unavailable = |d| ("store signal unavailable".to_owned(), d);
        let before = store_signal(root).ok_or_else(|| unavailable(discarded))?;
        let output = backend
            .cli_output(&ctx, &["export", "--format", "jsonl"], MAX_EXPORT)
            .map_err(|r| (format!("{}: {}", r.code, r.message), discarded))?;
        if store_signal(root).as_deref() != Some(before.as_str()) {
            discarded += 1;
            continue;
        }
        return Committed::parse(before, output, MAX_LINE)
            .map(|c| (c, discarded))
            .map_err(|e| (e, discarded));
    }
    Err((
        "the store changed during every export attempt".into(),
        discarded,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from a private git-kb 0.2.19 fixture: the unfiltered
    /// `list --json --sort title --asc` rows (identity, type and status),
    /// the export's `--where` fields, and native filtered listings. `tasks/a`
    /// is committed with assignee alice and edited to carol in its workspace;
    /// `tasks/f2` (a workspace `mv` of `tasks/f`) and `tasks/w` exist only in
    /// the workspace; `tasks/d` stores status `Active`.
    fn fixture() -> (Vec<Value>, String, Map<String, Value>) {
        let rows = json!([
            {"id": "01a19974-ec4d-7391-b80e-6f6a24880cb5", "slug": "tasks/c", "type": "task", "status": "active"},
            {"id": "01a19974-88ad-7bf3-845a-b8dba67544e3", "slug": "tasks/a", "type": "task", "status": "active"},
            {"id": "01a19974-88e5-7ec3-9f03-17fef1c11233", "slug": "tasks/b", "type": "task", "status": "draft"},
            {"id": "01a19976-3076-7230-8192-ff01866394e3", "slug": "tasks/d", "type": "task", "status": "Active"},
            {"id": "01a19976-30ad-71e0-a16e-a3c14ce291fc", "slug": "tasks/e", "type": "task", "status": "active"},
            {"id": "01a19976-30e6-7701-953f-00ea8af14473", "slug": "tasks/f", "type": "task", "status": "backlog"},
            {"id": "01a19976-30e6-7701-953f-00ea8af14473", "slug": "tasks/f2", "type": "task", "status": "backlog"},
            {"id": "01a1997b-4c91-7a12-9593-59543558aa31", "slug": "tasks/w", "type": "task", "status": "active"}
        ]);
        let exported = [
            (
                "tasks/f",
                "01a19976-30e6-7701-953f-00ea8af14473",
                json!({"status": "backlog", "assignee": "Bob@x-y", "bboxy_status": "claimed"}),
            ),
            (
                "tasks/e",
                "01a19976-30ad-71e0-a16e-a3c14ce291fc",
                json!({"status": "active", "assignee": "123", "swarm": "x y"}),
            ),
            (
                "tasks/d",
                "01a19976-3076-7230-8192-ff01866394e3",
                json!({"status": "Active", "assignee": "seat:01JABCDEFGHJKMNPQRSTVWXYZ0"}),
            ),
            (
                "tasks/c",
                "01a19974-ec4d-7391-b80e-6f6a24880cb5",
                json!({"status": "active", "assignee": "carol"}),
            ),
            (
                "tasks/b",
                "01a19974-88e5-7ec3-9f03-17fef1c11233",
                json!({"status": "draft", "assignee": "bob", "bboxy_status": "claimed"}),
            ),
            (
                "tasks/a",
                "01a19974-88ad-7bf3-845a-b8dba67544e3",
                json!({"status": "active", "assignee": "alice", "references": []}),
            ),
        ];
        let export: String = exported
            .iter()
            .map(|(slug, id, fm)| {
                let content = json!({"id": id, "slug": slug, "version": 1, "doc_type": "task",
                    "title": slug, "content": "body\n", "frontmatter": fm,
                    "created_at": "2026-11-01T23:31:53.777641Z", "modified_at": "2026-11-01T23:31:53.777641Z"});
                format!("{}\n", encoded(&json!({"slug": slug, "content": encoded(&content)})))
            })
            .collect();
        let stats: Map<String, Value> = [
            "tasks/a", "tasks/b", "tasks/c", "tasks/d", "tasks/f2", "tasks/w",
        ]
        .iter()
        .map(|s| ((*s).to_owned(), json!([1, 2, 3, 4])))
        .collect();
        (rows.as_array().unwrap().clone(), export, stats)
    }

    fn workspace() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bboxy-gitkb-index-{}",
            crate::native::random_hex(8)
        ));
        std::fs::create_dir_all(dir.join("tasks")).unwrap();
        std::fs::write(
            dir.join("tasks/f2.md"),
            "---\nid: 01a19976-30e6-7701-953f-00ea8af14473\nslug: tasks/f2\ntitle: \"tasks/f\"\ntype: task\nstatus: backlog\nassignee: Bob@x-y\nbboxy_status: claimed\n---\n\nbody",
        )
        .unwrap();
        std::fs::write(
            dir.join("tasks/w.md"),
            "---\nid: 01a1997b-4c91-7a12-9593-59543558aa31\nslug: tasks/w\ntitle: \"W\"\ntype: task\nstatus: active\nassignee: carol\nswarm: s9\n---\n\nbody w\n",
        )
        .unwrap();
        // The workspace edit of a committed document: native ignores it.
        std::fs::write(
            dir.join("tasks/a.md"),
            "---\nid: 01a19974-88ad-7bf3-845a-b8dba67544e3\nslug: tasks/a\ntitle: A edited\ntype: task\nstatus: completed\nassignee: carol\nswarm: s9\n---\nbody a2\n",
        )
        .unwrap();
        dir
    }

    fn flags(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn selection_equals_native_listings() {
        let (rows, export, stats) = fixture();
        let committed = Committed::parse("s".into(), export.as_bytes(), MAX_LINE).unwrap();
        assert_eq!(committed.len(), 6);
        assert!(committed.skipped.is_empty());
        let root = workspace();
        let mut listed = Listed::new("s".into(), "w".into(), rows);
        // Native `git-kb list` results for these flags, recorded on the fixture.
        for (flags_text, native) in [
            (
                &["--type", "task"][..],
                &[
                    "tasks/a", "tasks/b", "tasks/c", "tasks/d", "tasks/e", "tasks/f", "tasks/f2",
                    "tasks/w",
                ][..],
            ),
            (
                &["--type", "Task"],
                &[
                    "tasks/a", "tasks/b", "tasks/c", "tasks/d", "tasks/e", "tasks/f", "tasks/f2",
                    "tasks/w",
                ],
            ),
            (&["--type", "bug"], &[]),
            (
                &["--status", "active"],
                &["tasks/a", "tasks/c", "tasks/e", "tasks/w"],
            ),
            (
                &["--status", "ACTIVE"],
                &["tasks/a", "tasks/c", "tasks/e", "tasks/w"],
            ),
            (
                &["--status", "active,running"],
                &["tasks/a", "tasks/c", "tasks/e", "tasks/w"],
            ),
            (
                &["--status", "draft,backlog,open"],
                &["tasks/b", "tasks/f", "tasks/f2"],
            ),
            (&["--where", "assignee = carol"], &["tasks/c", "tasks/w"]),
            (&["--where", "assignee = alice"], &["tasks/a"]),
            (&["--where", "assignee = bob"], &["tasks/b"]),
            (&["--where", "assignee = Bob@x-y"], &["tasks/f", "tasks/f2"]),
            (
                &["--where", "assignee = seat:01JABCDEFGHJKMNPQRSTVWXYZ0"],
                &["tasks/d"],
            ),
            (&["--where", "swarm = s9"], &["tasks/w"]),
            (
                &["--where", "bboxy_status = claimed"],
                &["tasks/b", "tasks/f", "tasks/f2"],
            ),
            (
                &["--type", "task", "--where", "assignee = carol"],
                &["tasks/c", "tasks/w"],
            ),
            (
                &["--status", "active", "--where", "assignee = carol"],
                &["tasks/c", "tasks/w"],
            ),
            (
                &[
                    "--type",
                    "task",
                    "--where",
                    "assignee = seat:01JZZZZZZZZZZZZZZZZZZZZZZZ",
                ],
                &[],
            ),
        ] {
            let query = Query::parse(&flags(flags_text)).unwrap();
            let fields = query.needs_fields().then_some(&committed);
            let mut got: Vec<String> = listed
                .select(&query, fields, &root, &stats)
                .iter()
                .map(|r| r["slug"].as_str().unwrap().to_owned())
                .collect();
            got.sort();
            assert_eq!(got, native, "{flags_text:?}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unknown_fields_stay_candidates() {
        let (rows, export, stats) = fixture();
        let committed = Committed::parse("s".into(), export.as_bytes(), MAX_LINE).unwrap();
        // No workspace files on disk: workspace-only rows are not excluded.
        let missing = std::env::temp_dir().join("bboxy-gitkb-index-missing-workspace");
        let mut listed = Listed::new("s".into(), "w".into(), rows);
        let query = Query::parse(&flags(&["--where", "assignee = nobody"])).unwrap();
        let got: Vec<Value> = listed.select(&query, Some(&committed), &missing, &stats);
        let slugs: Vec<&str> = got.iter().map(|r| r["slug"].as_str().unwrap()).collect();
        assert_eq!(slugs, ["tasks/f2", "tasks/w"]);
        // An id mismatch with the committed document also stays a candidate.
        let row = json!({"id": "00000000-0000-0000-0000-000000000001", "slug": "tasks/a",
            "type": "task", "status": "active"});
        let mut other = Listed::new("s".into(), "w".into(), vec![row]);
        assert_eq!(
            other
                .select(&query, Some(&committed), &missing, &stats)
                .len(),
            1
        );
    }

    #[test]
    fn export_parse_counts_refused_lines() {
        let good = |slug: &str, id: &str| {
            let content = json!({"id": id, "slug": slug, "frontmatter": {"assignee": "x"}});
            format!(
                "{}\n",
                encoded(&json!({"slug": slug, "content": encoded(&content)}))
            )
        };
        let mismatched = encoded(
            &json!({"slug": "a", "content": encoded(&json!({"id": "1", "slug": "b", "frontmatter": {}}))}),
        );
        let text = format!(
            "{}not json\n{mismatched}\n{}{}\n{}",
            good("a", "1"),
            good("d", "2"),
            good("d", "3"),
            good("e", "4").trim_end()
        );
        let committed = Committed::parse("s".into(), text.as_bytes(), MAX_LINE).unwrap();
        assert_eq!(committed.len(), 2, "{committed:?}");
        assert_eq!(
            committed.skipped,
            *json!({"malformed": 2, "duplicate": 1}).as_object().unwrap()
        );
        assert_eq!(committed.fields("a", &"1".into()).unwrap()[0], "x");
        assert!(committed.fields("a", &"9".into()).is_none());
        assert!(
            committed.fields("d", &"2".into()).is_none(),
            "duplicates are unknown"
        );
        assert!(
            committed.fields("e", &"4".into()).is_some(),
            "final line without newline"
        );
        let long = format!("{}\n", "x".repeat(64));
        assert!(Committed::parse("s".into(), long.as_bytes(), 32).is_err());
    }

    #[test]
    fn only_plain_known_flags_are_indexed() {
        let parsed = Query::parse(&flags(&[
            "--type",
            "Task",
            "--status",
            "active,Running",
            "--where",
            "assignee = seat:01JABC",
            "--where",
            "swarm = s-1",
        ]))
        .unwrap();
        assert_eq!(
            parsed,
            Query {
                types: Some(vec!["task".into()]),
                statuses: Some(vec!["active".into(), "running".into()]),
                wheres: vec![(0, "seat:01JABC".into()), (1, "s-1".into())],
            }
        );
        assert_eq!(Query::parse(&[]), Some(Query::default()));
        for refused in [
            &["--since", "7d"][..],
            &["--where", "priority = high"],
            &["--where", "assignee = 123"],
            &["--where", "assignee = true"],
            &["--where", "assignee = two words"],
            &["--where", "assignee = 'quoted'"],
            &["--where", "assignee=x"],
            &["--status", "active,,done"],
            &["--type", "task", "--type", "bug"],
            &["--type"],
        ] {
            assert_eq!(Query::parse(&flags(refused)), None, "{refused:?}");
        }
    }

    /// Finish the pretend build after `after`, as the build thread does.
    fn finish(index: &Index, after: Duration, committed: Option<Committed>) {
        let shared = index.shared.clone();
        std::thread::spawn(move || {
            std::thread::sleep(after);
            let mut state = lock(&shared);
            state.building = false;
            state.committed = committed.map(Arc::new);
            drop(state);
            shared.done.notify_all();
        });
    }

    #[test]
    fn filtered_calls_wait_for_a_build_in_progress() {
        let ms = Duration::from_millis;
        let built = |signal: &str| Committed::parse(signal.into(), &b""[..], MAX_LINE).unwrap();
        let index = Index::new();
        // No build running: answer at once.
        let began = Instant::now();
        assert!(index.wait_for("s1", ms(5000), || false).is_none());
        assert!(began.elapsed() < ms(100));
        // A build that finishes within the budget is awaited.
        lock(&index.shared).building = true;
        finish(&index, ms(150), Some(built("s1")));
        let began = Instant::now();
        let got = index.wait_for("s1", ms(5000), || false).unwrap();
        assert_eq!(got.signal, "s1");
        assert!((ms(140)..ms(2000)).contains(&began.elapsed()));
        // Finished for another signal: no wait.
        let began = Instant::now();
        assert!(index.wait_for("s2", ms(5000), || false).is_none());
        assert!(began.elapsed() < ms(100));
        // A build that cannot finish within the budget: wait the budget only.
        lock(&index.shared).building = true;
        let began = Instant::now();
        assert!(index.wait_for("s2", ms(120), || false).is_none());
        assert!((ms(120)..ms(1000)).contains(&began.elapsed()));
        // Cancellation stops the wait.
        let began = Instant::now();
        assert!(index.wait_for("s2", ms(5000), || true).is_none());
        assert!(began.elapsed() < ms(100));
        // A failed build ends the wait when it finishes.
        finish(&index, ms(100), None);
        let began = Instant::now();
        assert!(index.wait_for("s2", ms(5000), || false).is_none());
        assert!((ms(90)..ms(2000)).contains(&began.elapsed()));
    }

    #[test]
    fn snapshots_round_trip_only_for_their_root_signal_and_version() {
        let (_, export, _) = fixture();
        let committed = Committed::parse("s1".into(), export.as_bytes(), MAX_LINE).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "bboxy-gitkb-snapshot-{}",
            crate::native::random_hex(8)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load(&dir, "/kb", "s1").is_none(), "no snapshot yet");
        save(&dir, "/kb", &committed).unwrap();
        let loaded = load(&dir, "/kb", "s1").unwrap();
        assert_eq!(loaded.signal, "s1");
        assert_eq!(loaded.docs, committed.docs);
        assert_eq!(loaded.skipped, committed.skipped);
        assert!(load(&dir, "/kb", "s2").is_none(), "another store state");
        assert!(load(&dir, "/other", "s1").is_none(), "another root");
        // Replacing is atomic and leaves no temporary file behind.
        let newer = Committed::parse("s2".into(), export.as_bytes(), MAX_LINE).unwrap();
        save(&dir, "/kb", &newer).unwrap();
        assert!(load(&dir, "/kb", "s1").is_none());
        assert!(load(&dir, "/kb", "s2").is_some());
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        let path = snapshot_path(&dir, "/kb");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("\"version\":1", "\"version\":2")).unwrap();
        assert!(load(&dir, "/kb", "s2").is_none(), "another format version");
        std::fs::write(&path, "{not json").unwrap();
        assert!(load(&dir, "/kb", "s2").is_none(), "malformed");
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(dir.join("elsewhere"), &path).unwrap();
        assert!(load(&dir, "/kb", "s2").is_none(), "symlink refused");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn store_signal_follows_manifest_replacement() {
        let root = std::env::temp_dir().join(format!(
            "bboxy-gitkb-signal-{}",
            crate::native::random_hex(8)
        ));
        let store = root.join(".kb/store");
        assert_eq!(store_signal(&root), None, "no store");
        std::fs::create_dir_all(store.join("refs")).unwrap();
        std::fs::write(store.join("manifest.json"), "{}").unwrap();
        let first = store_signal(&root).unwrap();
        assert_eq!(store_signal(&root).unwrap(), first, "reads are stable");
        // A native commit renames a new manifest into place.
        std::fs::write(store.join("manifest.next"), "{\"head\": 1}").unwrap();
        std::fs::rename(store.join("manifest.next"), store.join("manifest.json")).unwrap();
        let second = store_signal(&root).unwrap();
        assert_ne!(second, first);
        std::fs::write(store.join("refs/document-tips.json"), "{}").unwrap();
        assert_ne!(store_signal(&root).unwrap(), second);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
