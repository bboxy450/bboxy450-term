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
    probe.evidence["result"] = "passed; fresh sessions, exact native source parity, status filters, full 32-node graph pages equal to fetch, all calls read-only and below 2 seconds".into();
    probe.save();
    println!("{}", serde_json::to_string_pretty(&probe.evidence).unwrap());
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
