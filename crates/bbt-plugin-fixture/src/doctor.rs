// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Red/green controls for the SDK's plugin doctor: an SDK-served,
//! in-memory `data_source` that is correct with no fault, and breaks exactly
//! one host rule when `fault` names a doctor check id (`shape.digest`,
//! `page.cursor`, …). The doctor must flag that id and pass the clean one.
//!
//! One build serves every variant: the fault is the `fault` knob or, when
//! unset, read from the installed file name ([`installed`]), so the red/green
//! test copies one binary under many names instead of building each.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use bbt_plugin_sdk::{Contract, Ctx, Init, Manifest, Plugin, Refusal, Skill};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::Knobs;

/// The installed name of every doctor variant.
pub const NAME: &str = "bboxy450-term-data_source-doctor";
/// The child the env fault starts through PATH.
pub const CHILD: &str = "bboxy450-doctor-fixture-child";
const SKILL: &str = "# Doctor fixture\n\nThe served skill.\n";

static FAULT: OnceLock<String> = OnceLock::new();
/// Requests read minus replies written (the concurrency fault).
static OUTSTANDING: AtomicI64 = AtomicI64::new(0);
/// Every `_meta` attribution env seen so far (the env fault keeps it).
static LEAKED: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
static CALLS: AtomicU64 = AtomicU64::new(0);

/// Every fault, by the doctor check id it breaks.
pub const FAULTS: &[&str] = &[
    "shape.descriptor",
    "shape.digest",
    "shape.catalogue",
    "shape.skills",
    "contract.dto",
    "call.refusal",
    "call.cold_start",
    "call.deadline",
    "result.size",
    "page.limit",
    "page.cursor",
    "page.order",
    "identity.stable",
    "identity.id",
    "write.roundtrip",
    "write.status",
    "pending.retry_hint",
    "concurrency.busy",
    "env.scrubbed",
    "env.children",
];

/// The file name that selects `fault` (none: the clean plugin).
pub fn installed(fault: Option<&str>) -> String {
    match fault {
        None => NAME.to_owned(),
        Some(fault) => format!("{NAME}-{}", fault.replace(['.', '_'], "-")),
    }
}

fn fault(name: &str) -> bool {
    FAULT.get().is_some_and(|f| f == name)
}

pub fn main(knobs: &Knobs) {
    let named = std::env::args()
        .next()
        .and_then(|arg0| {
            let file = std::path::Path::new(&arg0)
                .file_name()?
                .to_str()?
                .to_owned();
            FAULTS
                .iter()
                .find(|f| installed(Some(f)) == file)
                .map(|f| (*f).to_owned())
        })
        .unwrap_or_default();
    FAULT
        .set(knobs.fault.clone().unwrap_or(named))
        .expect("one fixture configuration");
    if std::env::args().nth(1).as_deref() == Some("--shape") {
        let mut shape = Doctor::manifest().shape();
        if fault("shape.descriptor") {
            // A data_source without `links` is refused by the validator.
            if let Some(commands) = shape["systems"][0]["commands"].as_array_mut() {
                commands.retain(|c| c["name"] != "links");
            }
        }
        if fault("shape.digest") {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            shape["description"] = json!(format!("built at {nanos}"));
        }
        println!("{shape}");
        return;
    }
    let input = Input {
        inner: BufReader::new(std::io::stdin()),
        line: Vec::new(),
        at: 0,
    };
    bbt_plugin_sdk::serve::<Doctor>(input, Output(Vec::new()));
}

/// Stdin, observed per line: requests are counted, and `_meta` attribution
/// is kept (the env fault leaks it to every later child).
struct Input<R> {
    inner: BufReader<R>,
    line: Vec<u8>,
    at: usize,
}
impl<R: Read> Read for Input<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.at == self.line.len() {
            self.line.clear();
            self.at = 0;
            if self.inner.read_until(b'\n', &mut self.line)? == 0 {
                return Ok(0);
            }
            observe(&self.line);
        }
        let n = buf.len().min(self.line.len() - self.at);
        buf[..n].copy_from_slice(&self.line[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

fn observe(line: &[u8]) {
    let Ok(request) = serde_json::from_slice::<Value>(line) else {
        return;
    };
    if request.get("method").is_none() || request.get("id").is_none() {
        return;
    }
    if OUTSTANDING.fetch_add(1, Ordering::SeqCst) >= 1 && fault("concurrency.busy") {
        // A plugin that assumes one request at a time.
        std::process::exit(70);
    }
    if let Some(env) = request["params"]["_meta"]["bboxy450/attribution"]["env"].as_object() {
        let mut leaked = LEAKED.lock().unwrap();
        for (k, v) in env {
            leaked.insert(k.clone(), v.as_str().unwrap_or("").to_owned());
        }
    }
}

/// Stdout, observed per line: replies are counted, and the catalogue and
/// skill faults rewrite theirs.
struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        while let Some(end) = self.0.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.0.drain(..=end).collect();
            let mut out = std::io::stdout().lock();
            match serde_json::from_slice::<Value>(&line) {
                Ok(mut reply) if reply.get("method").is_none() => {
                    if fault("shape.catalogue")
                        && let Some(tools) = reply
                            .pointer_mut("/result/tools")
                            .and_then(Value::as_array_mut)
                    {
                        tools.retain(|t| t["name"] != "data_source_links");
                    }
                    if fault("shape.skills")
                        && let Some(text) = reply.pointer_mut("/result/contents/0/text")
                    {
                        *text = json!("# Something else\n");
                    }
                    OUTSTANDING.fetch_sub(1, Ordering::SeqCst);
                    writeln!(out, "{reply}")?;
                }
                _ => out.write_all(&line)?,
            }
            out.flush()?;
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

#[derive(Clone)]
struct Doc {
    id: String,
    external_id: String,
    slug: String,
    frontmatter: Value,
    body: String,
}

struct Doctor {
    docs: BTreeMap<String, Doc>,
    changes: Vec<String>,
}

fn digest(doc: &Doc) -> String {
    let mut hash = Sha256::new();
    hash.update(doc.frontmatter.to_string());
    hash.update(&doc.body);
    let base: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if fault("identity.stable") {
        // A version digest that changes with no content change.
        let n = CALLS.fetch_add(1, Ordering::Relaxed);
        return format!("{}{n:04x}", &base[..60]);
    }
    base
}

impl Doctor {
    fn reference(&self, doc: &Doc) -> Value {
        let mut value = json!({"id": doc.id, "number": null, "slug": doc.slug,
            "path": format!("{}.md", doc.slug), "digest": digest(doc), "external_id": doc.external_id});
        if fault("contract.dto") {
            value.as_object_mut().unwrap().remove("digest");
        }
        value
    }

    fn record(&self, doc: &Doc) -> Value {
        let mut record = self.reference(doc);
        record["frontmatter"] = doc.frontmatter.clone();
        record["body"] = json!(if fault("result.size") {
            "x".repeat(300 * 1024)
        } else {
            doc.body.clone()
        });
        record["etag"] = json!(digest(doc));
        record["resources"] = json!([]);
        record
    }

    fn find(&self, locator: &str) -> Result<&Doc, Refusal> {
        self.docs
            .values()
            .find(|d| d.id == locator || d.slug == locator || format!("{}.md", d.slug) == locator)
            .ok_or_else(|| Refusal::new("not_found", format!("no record {locator}")))
    }

    /// One keyset page over `docs` (by slug): `(items, next)`.
    fn page(
        &self,
        args: &Value,
        matching: impl Fn(&Doc) -> bool,
    ) -> Result<(Vec<Doc>, Value), Refusal> {
        let limit = match &args["limit"] {
            Value::Null => 32,
            Value::Number(n) => n.as_u64().map_or(32, |n| n.clamp(1, 128)) as usize,
            _ => return Err(Refusal::new("invalid_limit", "limit must be a number")),
        };
        let docs: Vec<Doc> = self
            .docs
            .values()
            .filter(|d| matching(d))
            .cloned()
            .collect();
        if fault("page.limit") {
            return Ok((docs, Value::Null));
        }
        let cursor = match &args["cursor"] {
            Value::Null => None,
            Value::String(c) => Some(c.clone()),
            _ => return Err(Refusal::new("invalid_cursor", "cursor must be a string")),
        };
        if fault("page.cursor") {
            let next = if docs.len() > limit {
                json!("again")
            } else {
                Value::Null
            };
            return Ok((docs.into_iter().take(limit).collect(), next));
        }
        if fault("page.order") {
            // Off by one: each page starts at the previous page's last item.
            let n: usize = cursor.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0);
            let start = n.saturating_sub(1);
            let items: Vec<Doc> = docs.iter().skip(start).take(limit).cloned().collect();
            let next = if n + limit < docs.len() {
                json!((n + limit).to_string())
            } else {
                Value::Null
            };
            return Ok((items, next));
        }
        let rest: Vec<Doc> = docs
            .into_iter()
            .filter(|d| cursor.as_ref().is_none_or(|c| d.slug.as_str() > c.as_str()))
            .collect();
        let next = if rest.len() > limit {
            json!(rest[limit - 1].slug)
        } else {
            Value::Null
        };
        Ok((rest.into_iter().take(limit).collect(), next))
    }

    fn write(&mut self, args: &Value) -> Result<Value, Refusal> {
        let mut frontmatter = args["frontmatter"].clone();
        let body = args["body"]
            .as_str()
            .ok_or_else(|| Refusal::new("invalid_arguments", "body"))?
            .to_owned();
        if !frontmatter.is_object() {
            return Err(Refusal::new(
                "invalid_arguments",
                "frontmatter must be an object",
            ));
        }
        if fault("write.roundtrip") {
            // A shadow status and a copied host record: pollution.
            if let Some(status) = frontmatter.as_object_mut().unwrap().remove("status") {
                frontmatter["bboxy_status"] = status;
            }
            frontmatter["bboxy_signed_record"] = json!("copied host record");
        }
        if fault("write.status")
            && let Some(status) = frontmatter.get_mut("status")
            && !matches!(status.as_str(), Some("open" | "done"))
        {
            // Silently mapping a status it does not know.
            *status = json!("open");
        }
        let existing = args["ref"]["id"]
            .as_str()
            .and_then(|id| self.docs.values().find(|d| d.id == id).cloned());
        let doc = match existing {
            Some(doc) => {
                if args["etag"].as_str() != Some(digest(&doc).as_str()) && !fault("identity.stable")
                {
                    return Err(Refusal::new("etag_mismatch", "the record changed"));
                }
                Doc {
                    frontmatter,
                    body,
                    ..doc
                }
            }
            None => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_arguments", "path"))?;
                let slug = path.strip_suffix(".md").unwrap_or(path).to_owned();
                if self.docs.contains_key(&slug) {
                    return Err(Refusal::new("path_exists", path.to_owned()));
                }
                let id = frontmatter["id"]
                    .as_str()
                    .map_or_else(|| format!("doc-{slug}"), str::to_owned);
                Doc {
                    id,
                    external_id: format!("ext-{slug}"),
                    slug,
                    frontmatter,
                    body,
                }
            }
        };
        self.changes.push(doc.id.clone());
        self.docs.insert(doc.slug.clone(), doc.clone());
        Ok(json!({"record": self.record(&doc), "lossy": false}))
    }

    /// Start the observed child: the env fault hands it every attribution
    /// ever seen; the clean fixture hands it nothing inherited.
    fn child(&self) {
        let mut command = Command::new(CHILD);
        command
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(path) = std::env::var("PATH") {
            command.env("PATH", path);
        }
        if fault("env.children") {
            command.envs(LEAKED.lock().unwrap().clone());
        }
        if let Ok(mut child) = command.spawn() {
            let _ = child.wait();
        }
    }
}

impl Plugin for Doctor {
    fn manifest() -> Manifest {
        Manifest::new(
            NAME,
            "Doctor fixture",
            "Red/green controls for the plugin doctor",
        )
        .contract(Contract::DataSource)
        .skill(Skill {
            name: "doctor-fixture".into(),
            description: "The doctor fixture's skill".into(),
            content: SKILL.into(),
        })
    }

    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        if fault("call.cold_start") {
            std::thread::sleep(Duration::from_millis(2_300));
        }
        if fault("env.scrubbed") && std::env::var_os("CARGO_MANIFEST_DIR").is_none() {
            return Err(Refusal::new("missing_env", "CARGO_MANIFEST_DIR is not set"));
        }
        let mut docs = BTreeMap::new();
        for i in 0..6 {
            let slug = format!("doctor-seed-{i}");
            docs.insert(slug.clone(), Doc {
                id: format!("01M6DSEED{i:017}"),
                external_id: format!("ext-{i}"),
                slug,
                frontmatter: json!({"id": format!("01M6DSEED{i:017}"), "type": "note", "title": format!("Seed {i}")}),
                body: format!("Seed {i}.\n"),
            });
        }
        let changes = docs.values().map(|d: &Doc| d.id.clone()).collect();
        Ok(Self { docs, changes })
    }

    fn call(
        &mut self,
        _contract: Contract,
        method: &str,
        args: Value,
        _ctx: &Ctx,
    ) -> Result<Value, Refusal> {
        match method {
            "resolve" => {
                let locator = args["locator"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_locator", "locator must be a string"))?;
                let doc = self.find(locator)?;
                let mut reference = self.reference(doc);
                if fault("identity.id") {
                    // The backend's own handle reported as the portable id.
                    reference["id"] = json!(doc.external_id);
                    reference["external_id"] = json!(doc.id);
                }
                Ok(reference)
            }
            "fetch" => {
                let id = args["ref"]["id"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_arguments", "ref.id"))?;
                match self.docs.values().find(|d| d.id == id) {
                    Some(doc) => Ok(self.record(doc)),
                    None if fault("call.refusal") => panic!("unknown record {id}"),
                    None => Err(Refusal::new("not_found", format!("no record {id}"))),
                }
            }
            "links" => {
                let id = args["ref"]["id"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_arguments", "ref.id"))?;
                self.docs
                    .values()
                    .find(|d| d.id == id)
                    .ok_or_else(|| Refusal::new("not_found", id.to_owned()))?;
                Ok(json!({"outgoing": [], "incoming": []}))
            }
            "list" => {
                self.child();
                let (items, next) = self.page(&args, |_| true)?;
                Ok(
                    json!({"items": items.iter().map(|d| self.reference(d)).collect::<Vec<_>>(), "next": next}),
                )
            }
            "search" => {
                if fault("call.deadline") && CALLS.fetch_add(1, Ordering::Relaxed) == 0 {
                    std::thread::sleep(Duration::from_millis(2_300));
                }
                if fault("pending.retry_hint") {
                    return Err(Refusal::new(
                        "workspace_pending",
                        "the index is still building",
                    ));
                }
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_arguments", "query must be a string"))?
                    .to_lowercase();
                let words: Vec<&str> = query.split_whitespace().collect();
                let (items, next) = self.page(&args, |d| {
                    words
                        .iter()
                        .all(|w| d.slug.contains(w) || d.body.to_lowercase().contains(w))
                })?;
                Ok(
                    json!({"items": items.iter().map(|d| json!({"ref": self.reference(d), "title": d.frontmatter["title"].as_str().unwrap_or(""),
                    "type": d.frontmatter["type"].as_str().unwrap_or("note"), "number": null, "snippet": "", "score": 1.0})).collect::<Vec<_>>(), "next": next}),
                )
            }
            "graph" => {
                if args["scope"]["kind"] != "all" {
                    return Err(Refusal::new("unsupported_scope", "only kind all"));
                }
                let (items, next) = self.page(&args, |_| true)?;
                Ok(
                    json!({"nodes": items.iter().map(|d| json!({"id": d.id, "ref": self.reference(d), "title": d.frontmatter["title"],
                    "type": d.frontmatter["type"], "frontmatter": d.frontmatter})).collect::<Vec<_>>(), "edges": [], "next": next, "edges_truncated": false}),
                )
            }
            "write" => self.write(&args),
            "changes_since" => {
                let cursor: usize = args["cursor"]
                    .as_str()
                    .ok_or_else(|| Refusal::new("invalid_arguments", "cursor must be a string"))?
                    .parse()
                    .unwrap_or(0);
                let limit = args["limit"].as_u64().map_or(32, |l| l.clamp(1, 128)) as usize;
                let changed: Vec<Value> = self
                    .changes
                    .iter()
                    .skip(cursor)
                    .take(limit)
                    .filter_map(|id| self.docs.values().find(|d| &d.id == id))
                    .map(|d| self.reference(d))
                    .collect();
                let next = (cursor + changed.len()).min(self.changes.len());
                Ok(json!({"changed": changed, "next": next.to_string()}))
            }
            _ => Err(Refusal::new("unknown_method", method.to_owned())),
        }
    }
}
