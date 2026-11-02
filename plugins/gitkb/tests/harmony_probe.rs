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
//! are recorded, never retried away.

// A test process with no UI thread.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

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
    probe.evidence["result"] = "passed; fresh sessions, exact native source parity, status filters, full 32-node graph pages equal to fetch, all calls read-only and below 2 seconds, 126-call task list equal to the backend path, its repeated pass from the index below 2 seconds, cold-process task list below 6.5 seconds from spawn".into();
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
    let mut slowest = Duration::ZERO;
    let mut results = Vec::new();
    for request in &requests {
        let one = Instant::now();
        results.push(all_pages(&mut client, request));
        slowest = slowest.max(one.elapsed());
    }
    let total = started.elapsed();
    let facts = client.call("health", json!({}), WAIT).unwrap()["index"].clone();
    assert_eq!(client.close(Duration::from_secs(5)), Some(0));
    let row = json!({"index": index, "initialize_ms": initialize_ms,
        "total_from_spawn_ms": total.as_millis() as u64,
        "slowest_call_ms": slowest.as_millis() as u64,
        "items": results.iter().map(Vec::len).sum::<usize>(),
        "index_ready_ms": facts["ready_ms"], "served": facts["served"],
        "fallbacks": facts["fallbacks"], "waits": facts["waits"], "waited_ms": facts["waited_ms"]});
    (results, row)
}

/// Cold-start replays: seat-like ids (`BBOXY_GITKB_PROBE_SEATS`, comma
/// separated, else 21 random ULIDs: registry seat ids that no Harmony record
/// names) must finish within 6.5 s from spawn and equal the backend path; the
/// replay's seats (real Harmony assignees included) are recorded only, as
/// their first read includes each matching workspace record's exact read.
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
    let (indexed, on) = cold_task_list(plugin, root, env, &data.join("seats-on"), &seats, true);
    let (native, off) = cold_task_list(plugin, root, env, &data.join("seats-off"), &seats, false);
    let (_, real) = cold_task_list(plugin, root, env, &data.join("replay-on"), replay, true);
    let identical = indexed.iter().zip(&native).filter(|(a, b)| a == b).count();
    let total = on["total_from_spawn_ms"].as_u64().unwrap();
    probe.evidence["cold_task_list"] = json!({
        "seat_ids": if registry { "BBOXY_GITKB_PROBE_SEATS" } else { "random ULIDs" },
        "identical": identical, "compared": indexed.len(),
        "indexed": on, "backend": off, "replay_seats_indexed": real,
    });
    let passed = identical == indexed.len() && total < 6_500;
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
