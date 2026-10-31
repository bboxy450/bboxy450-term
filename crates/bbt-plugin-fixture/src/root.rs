// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A credential-free data_source over the Markdown files of its root, served
//! by the SDK runtime.
//!
//! Translated from bboxy450-term,
//! crates/bbt-mux/tests/fixtures/plugins/root_provider.py (the former Python
//! fixture) and the Harmony host variant in crates/bbt-mux/tests/harmony_host.rs:
//! https://github.com/bboxy450/bboxy450-term
//! Changes: Rust on bboxy450-term-plugin-sdk; knobs compiled in.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use bbt_plugin_fixture::Knobs;
use bbt_plugin_sdk::{Contract, Ctx, Init, Manifest, Plugin, Refusal, Skill};
use regex::Regex;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const SKILL_CONTENT: &str = "---\nname: root-fixture\ndescription: Root fixture knowledge instructions.\n---\n\n# Root fixture\n\nResolve before fetch; preserve é and exact source bytes.\n";
const MAX_RECORD: usize = 2 * 1024 * 1024;
const MAX_RESOURCE: u64 = 64 * 1024 * 1024;

static KNOBS: OnceLock<Knobs> = OnceLock::new();

fn knobs() -> &'static Knobs {
    KNOBS.get().expect("knobs set in main")
}

fn trace(kind: &str, fields: Value) {
    if let Some(path) = &knobs().trace {
        let mut value = json!({"kind": kind, "pid": std::process::id()});
        if let Value::Object(fields) = fields {
            value.as_object_mut().unwrap().extend(fields);
        }
        crate::reference::append(path, &value);
    }
}

pub fn main(k: Knobs) {
    KNOBS.set(k).expect("knobs set once");
    if std::env::args().skip(1).collect::<Vec<_>>() == ["--shape"] {
        trace("shape", json!({}));
    }
    bbt_plugin_sdk::run::<RootFixture>();
}

// Compiled once per process; a debug-build compile per record cost ~250 ms a call.
fn compiled(slot: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    slot.get_or_init(|| Regex::new(pattern).expect("fixture pattern"))
}

fn refused(code: &str, message: &str) -> Refusal {
    Refusal::new(code, message)
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Python's `str()` of a JSON value, which the original fixture compared.
fn pystr(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "None".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(true)) => "True".into(),
        Some(Value::Bool(false)) => "False".into(),
        Some(other) => other.to_string(),
    }
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        _ => true,
    }
}

/// Python `json.dumps(value, ensure_ascii=False)`: `", "` and `": "`
/// separators, so written files keep the original fixture's bytes.
fn pydumps(value: &Value) -> String {
    match value {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(pydumps).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(k, v)| format!("{}: {}", Value::String(k.clone()), pydumps(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => other.to_string(),
    }
}

// Path.as_uri(): percent-encode everything but unreserved bytes and '/'.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
            uri.push(*byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn mime(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        _ => "application/octet-stream",
    }
}

fn plain_relative(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().next().is_some()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

struct RootFixture {
    root: PathBuf,
    history: HashMap<String, Vec<(String, Value)>>,
}

impl RootFixture {
    fn confined(&self, relative: &str) -> Result<PathBuf, Refusal> {
        let path = Path::new(relative);
        if !plain_relative(path) || path.extension().and_then(|e| e.to_str()) != Some("md") {
            return Err(refused(
                "path_outside_root",
                "a root-relative Markdown path is required",
            ));
        }
        let target = self.root.join(path);
        for part in target.ancestors() {
            if part == self.root {
                break;
            }
            if fs::symlink_metadata(part).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(refused("path_outside_root", "symlinks are refused"));
            }
        }
        let resolved = lexical_resolve(&target);
        if !resolved.starts_with(&self.root) {
            return Err(refused("path_outside_root", "path leaves root"));
        }
        Ok(target)
    }

    fn record(&self, path: &Path, resources: bool) -> Result<Value, Refusal> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| refused("path_outside_root", "path leaves root"))?
            .to_string_lossy()
            .into_owned();
        self.confined(&relative)?;
        let raw = fs::read(path).map_err(|e| refused("fixture_error", &e.to_string()))?;
        if raw.len() > MAX_RECORD || raw.contains(&b'\r') {
            return Err(refused(
                "representation_unavailable",
                "fixture record size/newlines are unsupported",
            ));
        }
        let text = std::str::from_utf8(&raw)
            .map_err(|_| refused("representation_unavailable", "record is not UTF-8"))?;
        let (metadata, body) = frontmatter(text)?;
        let digest = hex(&raw);
        let id = match metadata.get("id") {
            value if truthy(value) => pystr(value),
            _ => format!("fixture:{relative}"),
        };
        let slug = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut links = Vec::new();
        if resources {
            static IMAGE: OnceLock<Regex> = OnceLock::new();
            let image = compiled(&IMAGE, r"!\[\[([^\]|]+)(?:\|[^\]]*)?\]\]");
            for name in image.captures_iter(&body).map(|c| c[1].to_owned()) {
                links.push(self.resource(&name)?);
            }
        }
        Ok(
            json!({"id":id,"number":metadata.get("number").cloned().unwrap_or(Value::Null),
            "slug":slug,"path":relative,"digest":digest,"frontmatter":metadata,"body":body,
            "etag":digest,"resources":links}),
        )
    }

    fn resource(&self, name: &str) -> Result<Value, Refusal> {
        let resource = if name.starts_with("images/") {
            self.root.join(name)
        } else {
            self.root.join("images").join(name)
        };
        let outside = || {
            refused(
                "resource_outside_data_dir",
                "image leaves the selected root",
            )
        };
        if !plain_relative(Path::new(name))
            || fs::symlink_metadata(&resource).is_ok_and(|m| m.file_type().is_symlink())
            || !fs::canonicalize(&resource).is_ok_and(|p| p.starts_with(&self.root))
        {
            return Err(outside());
        }
        let mut file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&resource)
            .map_err(|e| refused("fixture_error", &e.to_string()))?;
        let mut data = Vec::new();
        file.by_ref()
            .take(MAX_RESOURCE + 1)
            .read_to_end(&mut data)
            .map_err(|e| refused("fixture_error", &e.to_string()))?;
        if data.len() as u64 > MAX_RESOURCE {
            return Err(refused(
                "resource_too_large",
                "image exceeds the resource bound",
            ));
        }
        let directory = std::env::current_dir().unwrap().join("resources");
        if !directory.is_dir() {
            fs::create_dir(&directory).map_err(|e| refused("fixture_error", &e.to_string()))?;
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&directory, fs::Permissions::from_mode(0o700));
        }
        let suffix = resource
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let target = directory.join(format!("{}{suffix}", hex(&data)));
        if !target.exists() {
            fs::write(&target, &data).map_err(|e| refused("fixture_error", &e.to_string()))?;
        }
        Ok(
            json!({"type":"resource_link","uri":file_uri(&target),"name":name,
            "mimeType":mime(name),"size":data.len()}),
        )
    }

    fn records(&self) -> Result<Vec<Value>, Refusal> {
        let mut paths = Vec::new();
        walk(&self.root, &mut paths);
        paths.sort_by(|a, b| a.components().cmp(b.components()));
        paths.iter().map(|path| self.record(path, false)).collect()
    }

    fn call_base(&mut self, method: &str, args: &Value) -> Result<Value, Refusal> {
        if method == "write" {
            let path = if truthy(args.get("ref")) {
                let rows = self.records()?;
                let current = exact(&args["ref"], &rows)?;
                let path = self.confined(current["path"].as_str().unwrap_or(""))?;
                if truthy(args.get("etag")) && args["etag"] != current["etag"] {
                    return Err(refused("etag_mismatch", "etag does not match"));
                }
                path
            } else {
                let path = self.confined(args["path"].as_str().unwrap_or(""))?;
                if path.exists() {
                    return Err(refused("path_exists", "create path already exists"));
                }
                path
            };
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| refused("fixture_error", &e.to_string()))?;
            }
            let text = format!(
                "---\n{}\n---\n{}",
                pydumps(&args["frontmatter"]),
                args["body"].as_str().unwrap_or("")
            );
            fs::write(&path, text).map_err(|e| refused("fixture_error", &e.to_string()))?;
            return Ok(json!({"record": self.record(&path, true)?, "lossy": false}));
        }
        let rows = self.records()?;
        let references: Vec<Value> = rows.iter().map(reference).collect();
        let snapshot = hex(serde_json::to_string(&references).unwrap().as_bytes());
        match method {
            "resolve" => Ok(reference(resolve(
                args["locator"].as_str().unwrap_or(""),
                &rows,
            )?)),
            "fetch" => {
                let found = exact(&args["ref"], &rows)?;
                let path = self.confined(found["path"].as_str().unwrap_or(""))?;
                self.record(&path, true)
            }
            "list" => {
                let filter = args.get("filter").cloned().unwrap_or_else(|| json!({}));
                let selected: Vec<Value> = filtered(&rows, &filter).map(reference).collect();
                page(
                    selected,
                    args,
                    &hex(format!("{snapshot}{filter}").as_bytes()),
                )
            }
            "search" => {
                let query = args["query"].as_str().unwrap_or("");
                let mut filters = Map::new();
                let mut words = Vec::new();
                for term in query.split_whitespace() {
                    match term.split_once(':') {
                        Some((key, value)) if ["type", "status", "since"].contains(&key) => {
                            filters.insert(key.into(), value.into());
                        }
                        _ => words.push(term.to_lowercase()),
                    }
                }
                let filters = Value::Object(filters);
                let items: Vec<Value> = filtered(&rows, &filters)
                    .filter(|r| {
                        let text = format!(
                            "{} {}",
                            pystr_or(&r["frontmatter"], "title", ""),
                            r["body"].as_str().unwrap_or("")
                        )
                        .to_lowercase();
                        words.iter().all(|w| text.contains(w.as_str()))
                    })
                    .map(|r| {
                        let snippet: String =
                            r["body"].as_str().unwrap_or("").chars().take(512).collect();
                        json!({"ref":reference(r),
                            "title":pystr_or(&r["frontmatter"],"title",r["slug"].as_str().unwrap_or("")),
                            "type":pystr_or(&r["frontmatter"],"type","note"),"number":r["number"],
                            "snippet":snippet,"score":1.0})
                    })
                    .collect();
                page(items, args, &hex(format!("{snapshot}{query}").as_bytes()))
            }
            "links" => {
                let found = exact(&args["ref"], &rows)?;
                let all = edges(&rows);
                let id = &found["id"];
                Ok(
                    json!({"outgoing": all.iter().filter(|e| e["from"] == *id).collect::<Vec<_>>(),
                    "incoming": all.iter().filter(|e| e["to"] == *id).collect::<Vec<_>>()}),
                )
            }
            "graph" => self.graph(args, &rows, &snapshot),
            "changes_since" => self.changes(args, &references),
            _ => Err(refused(
                "unknown_method",
                "method is not in the data_source contract",
            )),
        }
    }

    fn graph(&self, args: &Value, rows: &[Value], snapshot: &str) -> Result<Value, Refusal> {
        let scope = &args["scope"];
        let mut selected: Vec<&Value> = rows.iter().collect();
        if scope["graph"] != "markdown" {
            selected.retain(|r| r["frontmatter"]["type"] == scope["graph"]);
        }
        let only = |selected: Vec<&Value>, key: &str| -> Vec<Value> {
            let filter = json!({ key: scope[key] });
            let owned: Vec<Value> = selected.into_iter().cloned().collect();
            filtered(&owned, &filter).cloned().collect()
        };
        let selected: Vec<Value> = match scope["kind"].as_str() {
            Some("type") => only(selected, "type"),
            Some("swarm") => only(selected, "swarm"),
            Some("neighborhood") => {
                let center = exact(&scope["ref"], rows)?["id"].clone();
                let mut ids = vec![center];
                let all = edges(rows);
                for _ in 0..scope["depth"].as_u64().unwrap_or(0) {
                    let mut next = ids.clone();
                    for e in &all {
                        if ids.contains(&e["from"]) && !next.contains(&e["to"]) {
                            next.push(e["to"].clone());
                        }
                        if ids.contains(&e["to"]) && !next.contains(&e["from"]) {
                            next.push(e["from"].clone());
                        }
                    }
                    ids = next;
                }
                selected
                    .into_iter()
                    .filter(|r| ids.contains(&r["id"]))
                    .cloned()
                    .collect()
            }
            _ => selected.into_iter().cloned().collect(),
        };
        let nodes: Vec<Value> = selected
            .iter()
            .map(|r| {
                json!({"id":r["id"],"ref":reference(r),"title":r["frontmatter"].get("title"),
                    "type":r["frontmatter"].get("type"),"frontmatter":r["frontmatter"]})
            })
            .collect();
        let paged = page(nodes, args, &hex(format!("{snapshot}{scope}").as_bytes()))?;
        let ids: Vec<&Value> = paged["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| &n["id"])
            .collect();
        let outgoing: Vec<Value> = edges(rows)
            .into_iter()
            .filter(|e| ids.contains(&&e["from"]))
            .collect();
        Ok(
            json!({"nodes":paged["items"],"edges":outgoing.iter().take(512).collect::<Vec<_>>(),
            "next":paged["next"],"edges_truncated":outgoing.len() > 512}),
        )
    }

    fn changes(&mut self, args: &Value, references: &[Value]) -> Result<Value, Refusal> {
        let cursor = args["cursor"].as_str().unwrap_or("");
        let previous = if cursor == "0" {
            Vec::new()
        } else {
            self.history.get(cursor).cloned().ok_or_else(|| {
                refused(
                    "invalid_cursor",
                    "changes cursor is unknown to this instance",
                )
            })?
        };
        let limit = limit(args)?;
        let current: Vec<(String, Value)> = references
            .iter()
            .map(|r| (pystr(r.get("id")), r.clone()))
            .collect();
        let get = |list: &[(String, Value)], key: &str| {
            list.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
        };
        let mut changed: Vec<Value> = current
            .iter()
            .filter(|(key, r)| get(&previous, key).as_ref() != Some(r))
            .map(|(_, r)| r.clone())
            .collect();
        changed.extend(
            previous
                .iter()
                .filter(|(key, _)| get(&current, key).is_none())
                .map(|(_, r)| {
                    let mut r = r.clone();
                    r["number"] = Value::Null;
                    r
                }),
        );
        changed.truncate(limit);
        let mut progress = previous;
        for row in &changed {
            let key = pystr(row.get("id"));
            if let Some(now) = get(&current, &key) {
                match progress.iter_mut().find(|(k, _)| *k == key) {
                    Some(entry) => entry.1 = now,
                    None => progress.push((key, now)),
                }
            } else {
                progress.retain(|(k, _)| *k != key);
            }
        }
        let mut sorted: Vec<&(String, Value)> = progress.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let next = hex(serde_json::to_string(&sorted).unwrap().as_bytes());
        self.history.insert(next.clone(), progress);
        Ok(json!({"changed": changed, "next": next}))
    }

    // The Harmony host variant: commit, held and slow calls, signed sidecar.
    fn call_harmony(&mut self, method: &str, args: &Value) -> Result<Value, Refusal> {
        let flag = |name: &str| self.root.join(name).exists();
        let touch = |name: &str| {
            let _ = fs::write(self.root.join(name), b"");
        };
        if method == "commit" {
            let rows = self.records()?;
            for reference in args["refs"].as_array().into_iter().flatten() {
                exact(reference, &rows)?;
            }
            touch("commit-entered");
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.root.join("commit-attempts"))
                .map_err(|e| refused("fixture_error", &e.to_string()))?;
            use std::io::Write;
            let _ = log.write_all(b"commit\n");
            while flag("hold-commit") && !flag("release-commit") {
                std::thread::sleep(Duration::from_millis(10));
            }
            return Ok(json!({"commit":"private-budget-fixture","committed":args["refs"]}));
        }
        if method == "fetch" && flag("slow-fetch") {
            touch("fetch-entered");
            std::thread::sleep(Duration::from_secs(3));
        }
        if method == "fetch" && flag("slow-preflight") {
            touch("preflight-entered");
            std::thread::sleep(Duration::from_millis(1600));
        }
        if method == "write" && flag("hold-write") {
            touch("write-entered");
            while !flag("release-write") {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if method == "graph" && flag("forbid-graph") {
            return Err(refused(
                "global_scan_refused",
                "task operations must use scoped lists/direct references",
            ));
        }
        let signed = args
            .get("signed_record")
            .filter(|s| method == "write" && truthy(Some(s)))
            .cloned();
        let mut args = args.clone();
        if let Some(signed) = &signed {
            args["frontmatter"]["bboxy_signed_record"] = signed.clone();
        }
        let mut result = self.call_base(method, &args)?;
        if let Some(signed) = signed {
            result["signed_record"] = signed;
        }
        if method == "fetch"
            && let Some(signed) = result["frontmatter"].get("bboxy_signed_record").cloned()
            && truthy(Some(&signed))
        {
            result["signed_record"] = signed;
        }
        Ok(result)
    }

    fn wait_gate(&self) {
        let k = knobs();
        let (Some(gate), Some(root)) = (&k.gate, &k.gate_root) else {
            return;
        };
        let armed = k
            .gate_armed
            .as_ref()
            .is_none_or(|path| Path::new(path).exists());
        if self.root.to_string_lossy() == *root && armed {
            crate::raw::wait_gate(gate);
        }
    }
}

fn pystr_or(fields: &Value, key: &str, default: &str) -> String {
    fields
        .get(key)
        .map_or_else(|| default.to_owned(), |v| pystr(Some(v)))
}

// Path.resolve() for a path that may not exist yet: resolve the longest
// existing prefix and append the rest.
fn lexical_resolve(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name().map(ToOwned::to_owned) else {
            break;
        };
        rest.push(name);
        if !existing.pop() {
            break;
        }
    }
    let mut resolved = fs::canonicalize(&existing).unwrap_or(existing);
    for name in rest.into_iter().rev() {
        resolved.push(name);
    }
    resolved
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn frontmatter(text: &str) -> Result<(Map<String, Value>, String), Refusal> {
    let unavailable = |message: &str| refused("representation_unavailable", message);
    let Some(rest) = text.strip_prefix("---\n") else {
        return Err(unavailable(
            "fixture requires JSON or flat YAML frontmatter",
        ));
    };
    let Some((raw, body)) = rest.split_once("\n---\n") else {
        return Err(unavailable(
            "fixture requires JSON or flat YAML frontmatter",
        ));
    };
    let value = match serde_json::from_str::<Value>(raw) {
        Ok(value) => value,
        Err(_) => {
            static LINE: OnceLock<Regex> = OnceLock::new();
            let line_re = compiled(&LINE, r"^([a-zA-Z_][a-zA-Z0-9_-]*):\s*(.*)$");
            let mut fields = Map::new();
            for line in raw.lines() {
                if line.trim().is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some(captures) = line_re.captures(line) else {
                    return Err(unavailable("nested YAML is outside this fixture"));
                };
                let (key, scalar) = (captures[1].to_owned(), captures[2].to_owned());
                let parsed = if scalar.starts_with('"') {
                    serde_json::from_str::<Value>(&scalar)
                        .map_err(|_| unavailable("invalid JSON scalar"))?
                } else if scalar.len() >= 2 && scalar.starts_with('\'') && scalar.ends_with('\'') {
                    scalar[1..scalar.len() - 1].replace("''", "'").into()
                } else if scalar == "null" || scalar == "~" {
                    Value::Null
                } else if scalar == "true" || scalar == "false" {
                    (scalar == "true").into()
                } else if key == "number"
                    && !scalar.is_empty()
                    && scalar.chars().all(|c| c.is_ascii_digit())
                {
                    scalar
                        .parse::<u64>()
                        .map_or(scalar.clone().into(), Value::from)
                } else if scalar.starts_with(['|', '>', '[', '{', '&', '*', '!']) {
                    return Err(unavailable("complex YAML is outside this fixture"));
                } else {
                    scalar.into()
                };
                fields.insert(key, parsed);
            }
            Value::Object(fields)
        }
    };
    match value {
        Value::Object(fields) => Ok((fields, body.to_owned())),
        _ => Err(unavailable("frontmatter must be an object")),
    }
}

fn reference(record: &Value) -> Value {
    let mut out = Map::new();
    for key in ["id", "number", "slug", "path", "digest"] {
        out.insert(key.into(), record[key].clone());
    }
    Value::Object(out)
}

fn resolve<'a>(locator: &str, rows: &'a [Value]) -> Result<&'a Value, Refusal> {
    let found: Vec<&Value> = rows
        .iter()
        .filter(|r| {
            [&r["id"], &r["slug"], &r["path"]]
                .iter()
                .any(|v| v.as_str() == Some(locator))
                || locator
                    .strip_prefix('#')
                    .is_some_and(|n| pystr(r.get("number")) == n)
        })
        .collect();
    if found.len() == 1 {
        return Ok(found[0]);
    }
    let mut refusal = if found.is_empty() {
        refused("not_found", "record was not found")
    } else {
        refused("ambiguous_reference", "record is ambiguous")
    };
    if !found.is_empty() {
        refusal = refusal.with(
            "candidates",
            found.iter().map(|r| reference(r)).collect::<Vec<_>>(),
        );
    }
    Err(refusal)
}

fn exact<'a>(reference: &Value, rows: &'a [Value]) -> Result<&'a Value, Refusal> {
    let found: Vec<&Value> = rows
        .iter()
        .filter(|r| r["id"] == reference["id"] && r["path"] == reference["path"])
        .collect();
    if found.len() != 1 {
        return Err(refused("not_found", "record was not found"));
    }
    if found[0]["digest"] != reference["digest"] {
        return Err(refused("stale_reference", "record digest changed"));
    }
    Ok(found[0])
}

fn filtered<'a>(rows: &'a [Value], filters: &'a Value) -> impl Iterator<Item = &'a Value> + 'a {
    let empty = Map::new();
    let filters = filters.as_object().unwrap_or(&empty).clone();
    rows.iter().filter(move |r| {
        let fm = &r["frontmatter"];
        let plain = filters.iter().filter(|(k, _)| *k != "since").all(|(k, v)| {
            if !truthy(Some(v)) {
                return true;
            }
            let have = pystr(fm.get(k));
            let want = pystr(Some(v));
            if k == "assignee" {
                have.strip_prefix("seat:").unwrap_or(&have)
                    == want.strip_prefix("seat:").unwrap_or(&want)
            } else {
                have == want
            }
        });
        let since = filters.get("since");
        plain
            && (!truthy(since) || {
                let stamp = fm
                    .get("updated")
                    .or_else(|| fm.get("created"))
                    .map_or_else(String::new, |v| pystr(Some(v)));
                stamp >= pystr(since)
            })
    })
}

fn limit(args: &Value) -> Result<usize, Refusal> {
    let limit = match args.get("limit") {
        None => 32,
        Some(Value::Number(n)) if n.is_u64() || n.is_i64() => n.as_i64().unwrap_or(0),
        Some(_) => 0,
    };
    if !(1..=128).contains(&limit) {
        return Err(refused("invalid_page", "limit must be 1..128"));
    }
    Ok(limit as usize)
}

fn page(items: Vec<Value>, args: &Value, identity: &str) -> Result<Value, Refusal> {
    let limit = limit(args)?;
    let mut offset = 0;
    if let Some(cursor) = args
        .get("cursor")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty())
    {
        let invalid = || refused("invalid_cursor", "invalid page cursor");
        let (token, at) = cursor.rsplit_once(':').ok_or_else(invalid)?;
        offset = at.parse::<usize>().map_err(|_| invalid())?;
        if token != identity || offset > items.len() {
            return Err(refused(
                "invalid_cursor",
                "cursor belongs to another query/root snapshot",
            ));
        }
    }
    let end = offset + limit;
    let next = if end < items.len() {
        Value::from(format!("{identity}:{end}"))
    } else {
        Value::Null
    };
    let items: Vec<Value> = items.into_iter().skip(offset).take(limit).collect();
    Ok(json!({"items": items, "next": next}))
}

fn edges(rows: &[Value]) -> Vec<Value> {
    static LINK: OnceLock<Regex> = OnceLock::new();
    let link = compiled(&LINK, r"\[\[([^\]]+)\]\]");
    let mut result = Vec::new();
    for row in rows {
        let body = row["body"].as_str().unwrap_or("");
        for found in link.captures_iter(body) {
            let whole = found.get(0).unwrap();
            // (?<!!): an image embed is not a link.
            if body[..whole.start()].ends_with('!') {
                continue;
            }
            let parts: Vec<&str> = found[1].split('|').collect();
            let target =
                resolve(parts[0], rows).map_or_else(|_| Value::from(parts[0]), |r| r["id"].clone());
            let (rel, weight) = if parts.len() == 3 {
                (parts[1].to_owned(), parts[2].parse::<f64>().unwrap_or(1.0))
            } else {
                ("references".to_owned(), 1.0)
            };
            result.push(json!({"from":row["id"],"to":target,"rel":rel,"weight":weight}));
        }
    }
    result
}

impl Plugin for RootFixture {
    fn manifest() -> Manifest {
        let k = knobs();
        let mut manifest = Manifest::new(
            "bboxy450-term-data_source-root-fixture",
            "root fixture",
            "Credential-free Markdown root fixture",
        )
        .version("1.0")
        .contract(Contract::DataSource)
        .skill(Skill {
            name: "root-fixture".into(),
            description: "Root fixture knowledge instructions.".into(),
            content: SKILL_CONTENT.into(),
        });
        if k.harmony {
            manifest = manifest
                .method(Contract::DataSource, "commit")
                .timeout(Contract::DataSource, "commit", 15_000)
                .signed_writer()
                .fixture_bootstrap();
        }
        for method in Contract::DataSource.methods() {
            manifest = manifest.timeout(Contract::DataSource, method, 30_000);
        }
        manifest
    }

    fn initialize(init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        let root = init
            .root
            .as_deref()
            .ok_or_else(|| refused("invalid_root", "root is required"))?;
        let mut root =
            fs::canonicalize(root).map_err(|e| refused("invalid_root", &e.to_string()))?;
        let cwd = std::env::current_dir().unwrap();
        assert_ne!(
            root,
            fs::canonicalize(&cwd).unwrap(),
            "root is not the data dir"
        );
        if knobs().harmony && !init.fixture {
            // A distinct backing directory detects a bundled fallback.
            root = root.parent().unwrap_or(&root).join("external");
            let _ = fs::create_dir(&root);
        }
        trace(
            "initialize",
            json!({"root": root.to_string_lossy(), "cwd": cwd.to_string_lossy()}),
        );
        if !root.is_dir() {
            return Err(refused("invalid_root", "root is not a directory"));
        }
        Ok(Self {
            root,
            history: HashMap::new(),
        })
    }

    fn call(
        &mut self,
        _contract: Contract,
        method: &str,
        args: Value,
        _ctx: &Ctx,
    ) -> Result<Value, Refusal> {
        trace("call", json!({"method": method}));
        if method == "fetch" {
            self.wait_gate();
        }
        if knobs().harmony {
            self.call_harmony(method, &args)
        } else {
            self.call_base(method, &args)
        }
    }

    fn health(&mut self, _ctx: &Ctx) -> Result<Value, Refusal> {
        trace("call", json!({"method": "health"}));
        let cwd = std::env::current_dir().unwrap();
        Ok(json!({"ok": true, "pid": std::process::id(), "cwd": cwd.to_string_lossy()}))
    }
}
