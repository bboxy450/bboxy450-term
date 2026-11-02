// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-only Harmony probe: metadata and timing, never document content.
//! Translated from bboxy450-term `scripts/probe-gitkb-plugin.py`. Run through
//! `make probe-gitkb-harmony PLUGIN_WORKSPACE=/explicit/project`, which
//! supplies `BBOXY_GITKB_PLUGIN`, `PLUGIN_WORKSPACE` and `BBOXY_GITKB_EVIDENCE`.
//! `BBOXY_GITKB_PROBE_RUNS` (1–10, default 3) sets fresh sessions; failures
//! are recorded, never retried away. `BBOXY_GITKB_PROBE_ONLY=host` runs only
//! the host Tasks read replays; `BBOXY_GITKB_PROBE_FETCH_ALL=1` compares every
//! graph node with its fetch instead of the first 128.

// A test process with no UI thread.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use bbt_plugin_sdk::encoded;
use bbt_plugin_sdk::testkit::{self, Client};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const WAIT: Duration = Duration::from_secs(5);
const CAP: Duration = Duration::from_secs(2);

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

struct Probe {
    evidence: Value,
    path: PathBuf,
    run: usize,
}

impl Probe {
    fn save(&self) {
        std::fs::write(
            &self.path,
            serde_json::to_string_pretty(&self.evidence).unwrap(),
        )
        .unwrap();
    }

    /// One timed request; every attempt is persisted before it is checked.
    fn timed(&mut self, client: &mut Client, label: &str, method: &str, params: Value) -> Value {
        let start = Instant::now();
        let result = if method == "tools/call" {
            client.call(
                params["name"].as_str().unwrap(),
                params["arguments"].clone(),
                WAIT,
            )
        } else {
            client.request(method, params, WAIT)
        };
        let elapsed = start.elapsed();
        let mut row = json!({"run": self.run, "method": label,
            "elapsed_ms": (elapsed.as_secs_f64() * 100_000.0).round() / 100.0});
        let failed = match &result {
            Ok(value) => {
                if let Some(items) = value
                    .get("items")
                    .or_else(|| value.get("nodes"))
                    .or_else(|| value.get("changed"))
                    .and_then(Value::as_array)
                {
                    row["items"] = items.len().into();
                    row["next"] = (!value["next"].is_null()).into();
                }
                false
            }
            Err(failure) => {
                row["error"] = failure.code().unwrap_or("no_reply").into();
                true
            }
        };
        if elapsed >= CAP {
            row["deadline_exceeded"] = true.into();
        }
        self.evidence["operations"]
            .as_array_mut()
            .unwrap()
            .push(row.clone());
        self.evidence["result"] = if failed || elapsed >= CAP {
            "failed"
        } else {
            "incomplete"
        }
        .into();
        self.save();
        assert!(!failed && elapsed < CAP, "{row}");
        result.unwrap()
    }

    fn call(&mut self, client: &mut Client, method: &str, args: Value) -> Value {
        self.timed(
            client,
            method,
            "tools/call",
            json!({"name": format!("data_source_{method}"), "arguments": args}),
        )
    }
}

#[test]
#[ignore = "reads the operator-selected project; run through make probe-gitkb-harmony"]
fn harmony_probe() {
    let plugin = PathBuf::from(std::env::var_os("BBOXY_GITKB_PLUGIN").expect("BBOXY_GITKB_PLUGIN"))
        .canonicalize()
        .unwrap();
    let root =
        PathBuf::from(std::env::var_os("PLUGIN_WORKSPACE").expect("PLUGIN_WORKSPACE is required"))
            .canonicalize()
            .unwrap();
    let path = PathBuf::from(std::env::var_os("BBOXY_GITKB_EVIDENCE").expect("evidence path"));
    let runs: usize = std::env::var("BBOXY_GITKB_PROBE_RUNS")
        .ok()
        .and_then(|r| r.parse().ok())
        .filter(|r| (1..=10).contains(r))
        .unwrap_or(3);
    let shape = testkit::shape(&plugin).unwrap();
    let backend = shape["identity"]["backend"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut probe = Probe {
        evidence: json!({"root": root.display().to_string(),
            "plugin_sha256": sha256(&std::fs::read(&plugin).unwrap()), "operations": []}),
        path,
        run: 0,
    };
    let env = testkit::allowed_env();
    let base = std::env::temp_dir().canonicalize().unwrap();
    // `BBOXY_GITKB_PROBE_ONLY=host` replays only the host task reads.
    if std::env::var("BBOXY_GITKB_PROBE_ONLY").as_deref() == Ok("host") {
        host_replays(&mut probe, &plugin, &backend, &root, &env, &base);
        probe.evidence["result"] = "passed; host task reads only".into();
        probe.save();
        println!("{}", serde_json::to_string_pretty(&probe.evidence).unwrap());
        return;
    }
    for run in 1..=runs {
        probe.run = run;
        let data = base.join(format!(
            "bboxy-harmony-readonly-{}-{run}",
            std::process::id()
        ));
        let mut client = Client::spawn(&plugin, &data, &env).unwrap();
        probe.timed(
            &mut client,
            "initialize",
            "initialize",
            json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()}),
        );
        let _ = client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        let reference = probe.call(
            &mut client,
            "resolve",
            json!({"locator": "context/immutable/project-brief"}),
        );
        probe.evidence["resolved"] = reference.clone();
        let record = probe.call(&mut client, "fetch", json!({ "ref": reference }));
        let source = record["gitkb"]["source"].clone();
        probe.evidence["source"] = source.clone();
        probe.evidence["title"] = record["frontmatter"]["title"].clone();
        let parity = native_parity(&backend, &root, &data, &env, &reference, &record);
        let ok = ["raw", "body", "id", "etag"]
            .iter()
            .all(|k| parity[*k] == true);
        let mut parity = parity;
        parity["run"] = run.into();
        if probe.evidence.get("source_parity").is_none() {
            probe.evidence["source_parity"] = json!([]);
        }
        probe.evidence["source_parity"]
            .as_array_mut()
            .unwrap()
            .push(parity);
        if !ok {
            probe.evidence["result"] = "failed; native source parity".into();
        }
        probe.save();
        assert!(ok, "native source parity failed");
        // Full pages below need the background index and workspace records;
        // cold-process pages are measured by the host task read replays.
        let began = Instant::now();
        loop {
            let facts = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
            if facts["ready"] == true && facts["workspace"]["ms"].is_u64() {
                break;
            }
            assert!(began.elapsed() < Duration::from_secs(60), "index not ready");
            std::thread::sleep(Duration::from_millis(50));
        }
        for (method, request) in [
            ("list", json!({"filter": {"type": "task"}, "limit": 32})),
            (
                "search",
                json!({"query": "knowledge type:task", "limit": 32}),
            ),
            (
                "graph",
                json!({"scope": {"kind": "type", "graph": "markdown", "type": "task"}, "limit": 32}),
            ),
        ] {
            let first = probe.call(&mut client, method, request.clone());
            let mut pages = vec![first.clone()];
            if !first["next"].is_null() {
                let mut next = request;
                next["cursor"] = first["next"].clone();
                pages.push(probe.call(&mut client, method, next));
            }
            if method == "graph" {
                graph_parity(&mut probe, &mut client, &pages);
            }
        }
        // Status filters: the portable and the native value both select tasks.
        for (method, request) in [
            (
                "list",
                json!({"filter": {"type": "task", "status": "active"}, "limit": 32}),
            ),
            (
                "search",
                json!({"query": "type:task status:running", "limit": 32}),
            ),
        ] {
            let page = probe.call(&mut client, method, request.clone());
            assert!(
                !page["items"].as_array().unwrap().is_empty(),
                "{method} {request} returned no items"
            );
        }
        probe.call(&mut client, "links", json!({ "ref": reference }));
        let changes = probe.call(&mut client, "changes_since", json!({"cursor": "0"}));
        let unchanged = probe.call(
            &mut client,
            "changes_since",
            json!({"cursor": changes["next"]}),
        );
        assert_eq!(unchanged["changed"], json!([]));
        probe.call(
            &mut client,
            "resolve",
            json!({"locator": "context/immutable/project-brief"}),
        );
        let exit = client.close(Duration::from_secs(5));
        assert_eq!(exit, Some(0));
        if probe.evidence.get("sessions").is_none() {
            probe.evidence["sessions"] = json!([]);
        }
        probe.evidence["sessions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"run": run, "process_exit": exit}));
        let _ = std::fs::remove_dir_all(&data);
    }
    let seats = task_list_simulation(&mut probe, &plugin, &backend, &root, &env, &base);
    cold_replays(&mut probe, &plugin, &root, &env, &base, &seats);
    host_replays(&mut probe, &plugin, &backend, &root, &env, &base);
    probe.evidence["result"] = "passed; fresh sessions, exact native source parity, status filters, full 32-node graph pages equal to fetch, all calls read-only and below 2 seconds, 126-call task list equal to the backend path, its repeated pass from the index below 2 seconds, cold-process task list below 6.5 seconds from spawn, host Tasks reads (wide graph inventory and 126 lists) cold and respawned with no call over the host deadline, nodes and lists equal to the backend path, edges equal to per-document native reads".into();
    probe.save();
    println!("{}", serde_json::to_string_pretty(&probe.evidence).unwrap());
}

/// Distinct plain assignee values, most frequent first, from one read-only
/// native export. Only counts and hashes reach the evidence.
fn real_assignees(
    backend: &str,
    root: &Path,
    data: &Path,
    env: &[(String, String)],
) -> Vec<String> {
    let output = Command::new(backend)
        .args([
            "--no-pager",
            "--color",
            "never",
            "export",
            "--format",
            "jsonl",
        ])
        .current_dir(data)
        .env_clear()
        .envs(env.iter().cloned())
        .env("GITKB_ROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "native export failed");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for line in output
        .stdout
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
    {
        let outer: Value = serde_json::from_slice(line).unwrap();
        let content: Value = serde_json::from_str(outer["content"].as_str().unwrap()).unwrap();
        if let Some(assignee) = content["frontmatter"]["assignee"].as_str()
            && assignee
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:@/+-".contains(&b))
        {
            *counts.entry(assignee.to_owned()).or_default() += 1;
        }
    }
    let mut ranked: Vec<(usize, String)> = counts.into_iter().map(|(a, n)| (n, a)).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, a)| a).collect()
}

/// A random portable ULID (first character at most 7).
fn random_ulid() -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut bytes = [0u8; 26];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes))
        .unwrap();
    bytes
        .iter()
        .enumerate()
        .map(|(i, b)| ALPHABET[(*b as usize) % if i == 0 { 8 } else { 32 }] as char)
        .collect()
}

/// Every page of one list request.
fn all_pages(client: &mut Client, request: &Value) -> Vec<Value> {
    let mut items = Vec::new();
    let mut request = request.clone();
    for _ in 0..64 {
        let page = client
            .call("data_source_list", request.clone(), WAIT)
            .unwrap();
        items.extend(page["items"].as_array().unwrap().iter().cloned());
        if page["next"].is_null() {
            return items;
        }
        request["cursor"] = page["next"].clone();
    }
    panic!("task list paging did not finish");
}

/// The host's narrow-path requests: each seat × (`seat:<id>`, bare id) ×
/// (task, bug, epic), at the host's page size.
fn gui_requests(seats: &[String]) -> Vec<Value> {
    let requests: Vec<Value> = seats
        .iter()
        .flat_map(|id| [format!("seat:{id}"), id.clone()])
        .flat_map(|selector| {
            ["task", "bug", "epic"].map(|kind| {
                json!({"filter": {"type": kind, "assignee": selector.clone()}, "limit": 128})
            })
        })
        .collect();
    assert_eq!(requests.len(), 126);
    requests
}

/// The GUI's first task list from a COLD process: wall time from spawn
/// through initialize and all 126 calls, issued at once with no readiness
/// wait, as the Tasks panel's startup read does.
fn cold_task_list(
    plugin: &Path,
    root: &Path,
    env: &[(String, String)],
    dir: &Path,
    seats: &[String],
    index: bool,
) -> (Vec<Vec<Value>>, Value) {
    let mut env = env.to_vec();
    if !index {
        env.push(("BBOXY_GITKB_INDEX".into(), "off".into()));
    }
    let requests = gui_requests(seats);
    let started = Instant::now();
    let mut client = Client::spawn(plugin, dir, &env).unwrap();
    client
        .request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()}),
            WAIT,
        )
        .unwrap();
    let initialize_ms = started.elapsed().as_millis() as u64;
    let _ = client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    // The host's per-call deadline (DataSourceProvider::call): the declared
    // 2 s command timeout, capped by what the 8 s task-list operation has
    // left. Every page is its own host call.
    let mut slowest = Duration::ZERO;
    let mut overruns = 0;
    let mut results = Vec::new();
    for request in &requests {
        let mut items = Vec::new();
        let mut request = request.clone();
        loop {
            let budget = HOST_CALL.min(OPERATION.saturating_sub(started.elapsed()));
            let one = Instant::now();
            let page = client
                .call("data_source_list", request.clone(), WAIT)
                .unwrap();
            let elapsed = one.elapsed();
            slowest = slowest.max(elapsed);
            if elapsed >= budget {
                overruns += 1;
            }
            items.extend(page["items"].as_array().unwrap().iter().cloned());
            if page["next"].is_null() {
                break;
            }
            request["cursor"] = page["next"].clone();
        }
        results.push(items);
    }
    let total = started.elapsed();
    let facts = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
    assert_eq!(client.close(Duration::from_secs(5)), Some(0));
    let row = json!({"index": index, "initialize_ms": initialize_ms,
        "total_from_spawn_ms": total.as_millis() as u64,
        "slowest_call_ms": slowest.as_millis() as u64, "host_deadline_overruns": overruns,
        "items": results.iter().map(Vec::len).sum::<usize>(),
        "index_ready_ms": facts["ready_ms"], "snapshot": facts["snapshot"],
        "served": facts["served"], "fallbacks": facts["fallbacks"], "waits": facts["waits"],
        "waited_ms": facts["waited_ms"]});
    (results, row)
}

/// The host's fixed per-call budget for data_source list (ProviderConfig
/// timeout and the declared command timeout, both 2 s).
const HOST_CALL: Duration = Duration::from_secs(2);
/// The host task list's operation deadline, which also bounds each call.
const OPERATION: Duration = Duration::from_secs(8);

/// Calls timed against the host's budgets: each call gets
/// min(HOST_CALL, OPERATION - time since the operation began).
struct HostOperation {
    began: Instant,
    calls: usize,
    overruns: usize,
    slowest: Duration,
}

impl HostOperation {
    fn new(began: Instant) -> Self {
        Self {
            began,
            calls: 0,
            overruns: 0,
            slowest: Duration::ZERO,
        }
    }

    fn call(&mut self, client: &mut Client, tool: &str, args: Value) -> Value {
        let budget = HOST_CALL.min(OPERATION.saturating_sub(self.began.elapsed()));
        let one = Instant::now();
        let value = client.call(tool, args, WAIT).unwrap();
        let elapsed = one.elapsed();
        self.calls += 1;
        self.slowest = self.slowest.max(elapsed);
        if elapsed >= budget {
            self.overruns += 1;
        }
        value
    }

    fn row(&self) -> Value {
        json!({"total_ms": self.began.elapsed().as_millis() as u64, "calls": self.calls,
            "host_deadline_overruns": self.overruns,
            "slowest_call_ms": self.slowest.as_millis() as u64})
    }
}

/// The host Tasks panel's two reads, as `ProviderTasks::documents` issues
/// them: the wide unassigned inventory (graph Type task, bug and epic, limit
/// 128, every page until `next` is null) and the narrow per-seat list (126
/// calls). Each is one host operation with its own 8 s deadline; the
/// process spawn and initialize count against the first.
fn host_task_reads(
    plugin: &Path,
    root: &Path,
    env: &[(String, String)],
    dir: &Path,
    seats: &[String],
    index: bool,
) -> (Value, Value) {
    let mut env = env.to_vec();
    if !index {
        env.push(("BBOXY_GITKB_INDEX".into(), "off".into()));
    }
    let began = Instant::now();
    let mut client = Client::spawn(plugin, dir, &env).unwrap();
    client
        .request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()}),
            WAIT,
        )
        .unwrap();
    let initialize_ms = began.elapsed().as_millis() as u64;
    let _ = client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let mut wide = HostOperation::new(began);
    let mut pages = 0;
    let mut nodes = Vec::new();
    let mut edges = std::collections::BTreeSet::new();
    let mut truncated = false;
    let mut timeline = Vec::new();
    for kind in ["task", "bug", "epic"] {
        let mut request = json!({"scope": {"kind": "type", "graph": "markdown", "type": kind},
            "limit": 128});
        for _ in 0..64 {
            let at = began.elapsed().as_millis() as u64;
            let page = wide.call(&mut client, "data_source_graph", request.clone());
            timeline.push(json!([kind, at, page["nodes"].as_array().unwrap().len()]));
            pages += 1;
            nodes.extend(page["nodes"].as_array().unwrap().iter().cloned());
            edges.extend(page["edges"].as_array().unwrap().iter().map(encoded));
            truncated |= page["edges_truncated"] == true;
            let next = page["next"].clone();
            if next.is_null() {
                break;
            }
            request["cursor"] = next;
        }
    }
    let wide_row = wide.row();
    let mut narrow = HostOperation::new(Instant::now());
    let mut lists = Vec::new();
    for request in gui_requests(seats) {
        let mut request = request;
        let mut items = Vec::new();
        loop {
            let page = narrow.call(&mut client, "data_source_list", request.clone());
            items.extend(page["items"].as_array().unwrap().iter().cloned());
            if page["next"].is_null() {
                break;
            }
            request["cursor"] = page["next"].clone();
        }
        lists.push(Value::Array(items));
    }
    let facts = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
    assert_eq!(client.close(Duration::from_secs(5)), Some(0));
    let row = json!({"index": index, "initialize_ms": initialize_ms, "nodes": nodes.len(),
        "edges": edges.len(), "edges_truncated": truncated, "pages": pages, "wide": wide_row,
        "timeline": timeline, "narrow": narrow.row(),
        "index_ready_ms": facts["ready_ms"], "snapshot": facts["snapshot"],
        "workspace": facts["workspace"], "served": facts["served"], "fallbacks": facts["fallbacks"],
        "waits": facts["waits"], "waited_ms": facts["waited_ms"]});
    (json!({"nodes": nodes, "edges": edges, "lists": lists}), row)
}

/// Portable ULID text of a native UUID.
fn portable(uuid: &str) -> Option<String> {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    let n = u128::from_str_radix(&hex, 16).ok()?;
    (hex.len() == 32).then(|| {
        (0..26)
            .rev()
            .map(|i| ALPHABET[((n >> (5 * i)) & 31) as usize] as char)
            .collect()
    })
}

/// One document's outgoing document edges from a native per-document read
/// (`graph --direction out` of that slug alone), in portable form.
fn native_outgoing(
    backend: &str,
    root: &Path,
    dir: &Path,
    env: &[(String, String)],
    slug: &str,
    id: &str,
) -> Vec<String> {
    let output = Command::new(backend)
        .args([
            "--no-pager",
            "--color",
            "never",
            "graph",
            "--json",
            "--depth",
            "1",
        ])
        .args(["--direction", "out", "--", slug])
        .current_dir(dir)
        .env_clear()
        .envs(env.iter().cloned())
        .env("GITKB_ROOT", root)
        .env("GITKB_WORKSPACE", "main")
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["from"] == id)
        .filter_map(|e| {
            Some(encoded(&json!({"from": portable(e["from"].as_str()?)?,
                "rel": e["rel_type"], "to": portable(e["to"].as_str()?)?, "weight": 1.0})))
        })
        .collect()
}

/// Graph Type pages of 16 nodes (the first 12 task pages, every bug and epic
/// page): the index's nodes equal the backend path's, and its edges equal
/// the union of per-document native reads of the page's nodes. The backend
/// path's own edges come from one merged multi-document native read, which
/// GitKB 0.2.19 answers with fewer edges (a document already reached from
/// another is not expanded); those are counted, not required equal.
fn page_parity(
    plugin: &Path,
    backend: &str,
    root: &Path,
    env: &[(String, String)],
    indexed: &Path,
    native: &Path,
) -> Value {
    let mut off = env.to_vec();
    off.push(("BBOXY_GITKB_INDEX".into(), "off".into()));
    let open = |dir: &Path, env: &[(String, String)]| {
        let mut client = Client::spawn(plugin, dir, env).unwrap();
        client
            .request(
                "initialize",
                json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()}),
                WAIT,
            )
            .unwrap();
        client
    };
    let (mut a, mut b) = (open(indexed, env), open(native, &off));
    let (mut compared, mut nodes_equal, mut edges_true, mut backend_short) = (0, 0, 0, 0);
    let mut first_difference = Value::Null;
    for (kind, pages) in [("task", 12), ("bug", 64), ("epic", 64)] {
        let request = json!({"scope": {"kind": "type", "graph": "markdown", "type": kind},
            "limit": 16});
        let (mut ra, mut rb) = (request.clone(), request);
        for _ in 0..pages {
            let pa = a.call("data_source_graph", ra.clone(), WAIT).unwrap();
            let pb = b.call("data_source_graph", rb.clone(), WAIT).unwrap();
            compared += 1;
            nodes_equal += usize::from(pa["nodes"] == pb["nodes"]);
            let set = |p: &Value| -> std::collections::BTreeSet<String> {
                p["edges"].as_array().unwrap().iter().map(encoded).collect()
            };
            let truth: std::collections::BTreeSet<String> = pa["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|n| {
                    native_outgoing(
                        backend,
                        root,
                        indexed,
                        env,
                        n["ref"]["slug"].as_str().unwrap(),
                        n["ref"]["external_id"].as_str().unwrap(),
                    )
                })
                .collect();
            let index_edges = set(&pa);
            let same = index_edges == truth && pa["edges_truncated"] == false;
            edges_true += usize::from(same);
            backend_short += usize::from(set(&pb) != truth);
            if !same && first_difference.is_null() {
                first_difference = json!({"kind": kind, "page": compared,
                    "only_index": index_edges.difference(&truth).take(3).collect::<Vec<_>>(),
                    "only_native": truth.difference(&index_edges).take(3).collect::<Vec<_>>()});
            }
            if pa["next"].is_null() || pb["next"].is_null() {
                break;
            }
            ra["cursor"] = pa["next"].clone();
            rb["cursor"] = pb["next"].clone();
        }
    }
    assert_eq!(a.close(Duration::from_secs(5)), Some(0));
    assert_eq!(b.close(Duration::from_secs(5)), Some(0));
    json!({"compared": compared, "nodes_equal": nodes_equal,
        "edges_equal_per_document_native": edges_true,
        "backend_pages_differing_from_per_document_native": backend_short,
        "first_difference": first_difference})
}

/// Every node of every `graph all` page against its own fetch: frontmatter,
/// identity, title, type and reference digest. Run with
/// `BBOXY_GITKB_PROBE_FETCH_ALL=1`; otherwise the first 128 nodes.
fn fetch_parity(plugin: &Path, root: &Path, env: &[(String, String)], dir: &Path) -> Value {
    let all = std::env::var_os("BBOXY_GITKB_PROBE_FETCH_ALL").is_some();
    let mut client = Client::spawn(plugin, dir, env).unwrap();
    client
        .request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()}),
            WAIT,
        )
        .unwrap();
    let mut request = json!({"scope": {"kind": "all", "graph": "markdown"}, "limit": 128});
    let (mut compared, mut equal, mut committed, mut too_large) = (0, 0, 0, 0);
    let mut differing = Vec::new();
    loop {
        let page = client
            .call("data_source_graph", request.clone(), WAIT)
            .unwrap();
        for node in page["nodes"].as_array().unwrap() {
            let record = match client.call("data_source_fetch", json!({"ref": node["ref"]}), WAIT) {
                Ok(record) => record,
                // A record above the 256 KiB result cap, or one whose image
                // link is missing, cannot be fetched; count it by refusal.
                Err(failure)
                    if matches!(
                        failure.code(),
                        Some("response_too_large" | "resource_not_found")
                    ) =>
                {
                    too_large += 1;
                    continue;
                }
                Err(failure) => panic!("fetch {}: {failure:?}", node["ref"]),
            };
            compared += 1;
            committed += usize::from(record["gitkb"]["source"]["kind"] == "committed");
            if record["frontmatter"] == node["frontmatter"]
                && record["id"] == node["id"]
                && record["digest"] == node["ref"]["digest"]
                && record["frontmatter"]["title"] == node["title"]
                && record["frontmatter"]["type"] == node["type"]
            {
                equal += 1;
            } else if differing.len() < 8 {
                differing.push(sha256(node["ref"]["slug"].as_str().unwrap().as_bytes()));
            }
        }
        if page["next"].is_null() || (!all && compared >= 128) {
            break;
        }
        request["cursor"] = page["next"].clone();
    }
    assert_eq!(client.close(Duration::from_secs(5)), Some(0));
    json!({"all": all, "compared": compared, "equal": equal, "committed_source": committed,
        "unfetchable": too_large,
        "differing_slug_sha256": differing})
}

/// The host reads replayed from a cold process with no snapshot, again
/// after a respawn that loads it, and with the index off. All three must
/// return identical pages; the indexed runs must have no call over the host
/// deadline, and graph nodes must equal fetch.
fn host_replays(
    probe: &mut Probe,
    plugin: &Path,
    backend: &str,
    root: &Path,
    env: &[(String, String)],
    base: &Path,
) {
    let data = base.join(format!("bboxy-harmony-host-{}", std::process::id()));
    let seats: Vec<String> = (0..21).map(|_| random_ulid()).collect();
    let dir = data.join("indexed");
    let (cold, cold_row) = host_task_reads(plugin, root, env, &dir, &seats, true);
    let (warm, warm_row) = host_task_reads(plugin, root, env, &dir, &seats, true);
    let (native, native_row) = host_task_reads(plugin, root, env, &data.join("off"), &seats, false);
    let parity = fetch_parity(plugin, root, env, &dir);
    let pages = page_parity(plugin, backend, root, env, &dir, &data.join("off"));
    // Node inventories and seat lists equal across all three runs; page
    // boundaries (and so the 512-edge truncation) follow each run's timing,
    // so edges are compared page by page at a fixed small page size.
    let identical = cold["nodes"] == native["nodes"]
        && warm["nodes"] == native["nodes"]
        && cold["lists"] == native["lists"]
        && warm["lists"] == native["lists"]
        && pages["nodes_equal"] == pages["compared"]
        && pages["edges_equal_per_document_native"] == pages["compared"];
    probe.evidence["host_task_reads"] = json!({"identical": identical, "cold": cold_row,
        "respawn_with_snapshot": warm_row, "backend": native_row, "fetch_parity": parity,
        "page_parity": pages});
    let passed = identical
        && cold_row["wide"]["host_deadline_overruns"] == 0
        && cold_row["narrow"]["host_deadline_overruns"] == 0
        && warm_row["wide"]["host_deadline_overruns"] == 0
        && warm_row["narrow"]["host_deadline_overruns"] == 0
        && parity["equal"] == parity["compared"];
    if !passed {
        probe.evidence["result"] = "failed; host task reads".into();
    }
    probe.save();
    let _ = std::fs::remove_dir_all(&data);
    assert!(passed, "{}", probe.evidence["host_task_reads"]);
}

/// Cold-start replays: seat-like ids (`BBOXY_GITKB_PROBE_SEATS`, comma
/// separated, else 21 random ULIDs: registry seat ids that no Harmony record
/// names) must finish within 6.5 s from spawn and equal the backend path,
/// with no call over the host's per-call deadline, both from an empty data
/// directory and after a respawn that loads the saved snapshot (which must
/// serve every filtered call without a fallback). The replay's seats (real
/// Harmony assignees included) are recorded only, as their first read
/// includes each matching workspace record's exact read.
fn cold_replays(
    probe: &mut Probe,
    plugin: &Path,
    root: &Path,
    env: &[(String, String)],
    base: &Path,
    replay: &[String],
) {
    let data = base.join(format!("bboxy-harmony-cold-{}", std::process::id()));
    let supplied: Vec<String> = std::env::var("BBOXY_GITKB_PROBE_SEATS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let registry = !supplied.is_empty();
    let mut seats = supplied;
    seats.truncate(21);
    while seats.len() < 21 {
        seats.push(random_ulid());
    }
    // No snapshot: a fresh private data directory builds and saves one.
    let seats_dir = data.join("seats-on");
    let (indexed, on) = cold_task_list(plugin, root, env, &seats_dir, &seats, true);
    // A respawn in the same data directory: the snapshot answers at once.
    let (respawned, warm) = cold_task_list(plugin, root, env, &seats_dir, &seats, true);
    let (native, off) = cold_task_list(plugin, root, env, &data.join("seats-off"), &seats, false);
    let (_, real) = cold_task_list(plugin, root, env, &data.join("replay-on"), replay, true);
    let identical = indexed
        .iter()
        .zip(&native)
        .zip(&respawned)
        .filter(|((a, b), c)| a == b && a == c)
        .count();
    let total = on["total_from_spawn_ms"].as_u64().unwrap();
    probe.evidence["cold_task_list"] = json!({
        "seat_ids": if registry { "BBOXY_GITKB_PROBE_SEATS" } else { "random ULIDs" },
        "identical": identical, "compared": indexed.len(),
        "indexed": on, "respawn_with_snapshot": warm, "backend": off,
        "replay_seats_indexed": real,
    });
    let passed = identical == indexed.len()
        && total < 6_500
        && on["host_deadline_overruns"] == 0
        && warm["host_deadline_overruns"] == 0
        && warm["snapshot"]["loaded_us"].is_u64()
        && warm["fallbacks"] == 0;
    if !passed {
        probe.evidence["result"] = "failed; cold task list".into();
    }
    probe.save();
    let _ = std::fs::remove_dir_all(&data);
    assert!(passed, "{}", probe.evidence["cold_task_list"]);
}

/// The GUI task list's narrow path: 21 seats × (`seat:<id>`, bare id) ×
/// (task, bug, epic) filtered lists, twice per session: once the index is
/// ready, then with the backend path (`BBOXY_GITKB_INDEX=off`). Every result
/// must be identical. The first pass also performs each matching record's
/// first summary read (an exact read per workspace-backed record, unchanged
/// by the index); the repeated pass, which reuses those, must finish within
/// two seconds from the index.
fn task_list_simulation(
    probe: &mut Probe,
    plugin: &Path,
    backend: &str,
    root: &Path,
    env: &[(String, String)],
    base: &Path,
) -> Vec<String> {
    let data = base.join(format!("bboxy-harmony-tasklist-{}", std::process::id()));
    std::fs::create_dir_all(&data).unwrap();
    let real = real_assignees(backend, root, &data, env);
    let mut seats: Vec<String> = real.iter().take(10).cloned().collect();
    while seats.len() < 21 {
        seats.push(random_ulid());
    }
    let requests = gui_requests(&seats);
    let initialize = json!({"protocolVersion": "2025-06-18", "root": root.display().to_string()});
    let session = |index: bool| {
        let mut env = env.to_vec();
        if !index {
            env.push(("BBOXY_GITKB_INDEX".into(), "off".into()));
        }
        let dir = data.join(if index { "on" } else { "off" });
        let mut client = Client::spawn(plugin, &dir, &env).unwrap();
        let started = Instant::now();
        client
            .request("initialize", initialize.clone(), WAIT)
            .unwrap();
        let initialize_ms = started.elapsed().as_millis() as u64;
        let _ = client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        let mut facts = Value::Null;
        while index && facts["ready"] != true {
            facts = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
            if facts["ready"] == true {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "index not ready: {facts}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        let ready_wall_ms = started.elapsed().as_millis() as u64;
        // Two passes: the first also reads each matching record's summary
        // (exact reads for workspace-backed records); the second reuses them.
        let mut passes = Vec::new();
        let mut results = Vec::new();
        let mut first: Vec<Vec<Value>> = Vec::new();
        for _ in 0..2 {
            first = std::mem::take(&mut results);
            let (mut slowest, mut empty_ms, mut matched_ms) = (Duration::ZERO, 0u128, 0u128);
            let begin = Instant::now();
            for request in &requests {
                let one = Instant::now();
                let items = all_pages(&mut client, request);
                let elapsed = one.elapsed();
                slowest = slowest.max(elapsed);
                if items.is_empty() {
                    empty_ms += elapsed.as_millis();
                } else {
                    matched_ms += elapsed.as_millis();
                }
                results.push(items);
            }
            passes.push(json!({"total_ms": begin.elapsed().as_millis() as u64,
                "slowest_call_ms": slowest.as_millis() as u64,
                "empty_calls": results.iter().filter(|r| r.is_empty()).count(),
                "empty_calls_ms": empty_ms as u64, "matched_calls_ms": matched_ms as u64}));
        }
        assert_eq!(first, results, "a repeated task list changed");
        let after = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
        assert_eq!(client.close(Duration::from_secs(5)), Some(0));
        let row = json!({
            "index": index, "initialize_ms": initialize_ms, "ready_wall_ms": ready_wall_ms,
            "index_ready_ms": facts["ready_ms"], "index_build_ms": facts["build_ms"],
            "indexed_documents": facts["documents"], "skipped": facts["skipped"],
            "calls": requests.len(), "passes": passes,
            "items": results.iter().map(Vec::len).sum::<usize>(),
            "served": after["served"], "fallbacks": after["fallbacks"],
        });
        (results, row)
    };
    let (indexed, on_row) = session(true);
    let (native, off_row) = session(false);
    let indexed_total = Duration::from_millis(on_row["passes"][1]["total_ms"].as_u64().unwrap());
    let identical = indexed.iter().zip(&native).filter(|(a, b)| a == b).count();
    let selectors: Vec<String> = seats
        .iter()
        .map(|s| sha256(s.as_bytes())[..12].into())
        .collect();
    probe.evidence["task_list_simulation"] = json!({
        "seats": seats.len(), "real_assignees": seats.iter().filter(|s| real.contains(s)).count(),
        "seat_sha256_prefixes": selectors, "identical": identical, "compared": requests.len(),
        "indexed": on_row, "backend": off_row,
    });
    let passed = identical == requests.len()
        && indexed_total < Duration::from_secs(2)
        && on_row["served"].as_u64().unwrap_or(0) >= 2 * requests.len() as u64;
    if !passed {
        probe.evidence["result"] = "failed; task list simulation".into();
    }
    probe.save();
    let _ = std::fs::remove_dir_all(&data);
    assert!(passed, "{}", probe.evidence["task_list_simulation"]);
    seats
}

/// Graph pages with a continuation hold `limit` (32) nodes within the call
/// budget, and every node of the first page carries exactly what fetch returns.
fn graph_parity(probe: &mut Probe, client: &mut Client, pages: &[Value]) {
    for page in pages.iter().filter(|page| !page["next"].is_null()) {
        let held = page["nodes"].as_array().unwrap().len();
        if held != 32 {
            probe.evidence["result"] = format!("failed; graph page held {held} of 32").into();
            probe.save();
        }
        assert_eq!(held, 32, "graph page did not fill to its limit");
    }
    let nodes = pages[0]["nodes"].as_array().unwrap().clone();
    let mut equal = 0;
    for node in &nodes {
        let record = probe.call(client, "fetch", json!({"ref": node["ref"]}));
        if record["frontmatter"] == node["frontmatter"]
            && record["id"] == node["id"]
            && record["frontmatter"]["title"] == node["title"]
            && record["frontmatter"]["type"] == node["type"]
        {
            equal += 1;
        }
    }
    let row = json!({"run": probe.run, "compared": nodes.len(), "equal": equal});
    if probe.evidence.get("graph_fetch_parity").is_none() {
        probe.evidence["graph_fetch_parity"] = json!([]);
    }
    probe.evidence["graph_fetch_parity"]
        .as_array_mut()
        .unwrap()
        .push(row);
    if equal != nodes.len() {
        probe.evidence["result"] = "failed; graph node frontmatter differs from fetch".into();
    }
    probe.save();
    assert!(nodes.len() >= 10, "too few graph nodes to compare");
    assert_eq!(
        equal,
        nodes.len(),
        "graph node frontmatter differs from fetch"
    );
}

/// Compare the plugin record with a direct native read of the same source.
fn native_parity(
    backend: &str,
    root: &Path,
    data: &Path,
    env: &[(String, String)],
    reference: &Value,
    record: &Value,
) -> Value {
    let source = &record["gitkb"]["source"];
    let mut command = Command::new(backend);
    command
        .args(["--no-pager", "--color", "never", "show", "--source"])
        .arg(source["kind"].as_str().unwrap())
        .args(["--raw-content", "--json", "--"])
        .arg(reference["slug"].as_str().unwrap())
        .current_dir(data)
        .env_clear()
        .envs(env.iter().cloned())
        .env("GITKB_ROOT", root);
    if source["kind"] == "workspace" {
        command.env("GITKB_WORKSPACE", source["workspace"].as_str().unwrap());
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "native show failed");
    let native: Value = serde_json::from_slice(&output.stdout).unwrap();
    json!({
        "raw": record["gitkb"]["raw_content"] == native["rawContent"],
        "body": record["body"] == native["document"]["content"],
        "id": record["external_id"] == native["document"]["id"],
        "etag": record["etag"] == native["generation"],
        "raw_sha256": sha256(native["rawContent"].as_str().unwrap_or("").as_bytes()),
    })
}
