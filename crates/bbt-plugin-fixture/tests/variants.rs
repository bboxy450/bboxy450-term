// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "owned fixture processes and files"
)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use bbt_plugin_fixture::{Kind, Knobs};
use bbt_plugin_sdk::testkit::{Client, allowed_env, shape};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(10);

#[test]
fn variants_are_compiled_in_cached_and_have_distinct_bytes() {
    let plain = Knobs::reference().build();
    assert_eq!(Knobs::reference().build(), plain, "one build per variant");
    let major = Knobs::reference().fault("major").build();
    assert_ne!(
        std::fs::read(&plain).unwrap(),
        std::fs::read(&major).unwrap()
    );
    assert_eq!(shape(&plain).unwrap()["shape_version"], "1.0");
    assert_eq!(shape(&major).unwrap()["shape_version"], "2.0");
    let graph = shape(&Knobs::reference().system("repo_graph").build()).unwrap();
    assert_eq!(graph["systems"][0]["contract"], "repo_graph");
}

#[test]
fn root_fixture_serves_its_markdown_root_through_the_sdk() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir_all(root.join("tasks")).unwrap();
    std::fs::write(
        root.join("tasks/one.md"),
        "---\nid: one\nnumber: 1\ntype: task\ntitle: First\n---\nBody [[two]]\n",
    )
    .unwrap();
    let trace = temp.path().join("trace.jsonl");
    let plugin = Knobs {
        trace: Some(trace.to_string_lossy().into_owned()),
        ..Knobs::root()
    }
    .install(&temp.path().join("bboxy450-term-data_source-root-fixture"));
    let described = shape(&plugin).unwrap();
    assert_eq!(described["skills"][0]["name"], "root-fixture");
    let mut client = Client::spawn(&plugin, &temp.path().join("data"), &allowed_env()).unwrap();
    client
        .initialize(json!({"root": root.to_string_lossy()}), WAIT)
        .unwrap();
    let found = client
        .call("data_source_resolve", json!({"locator": "#1"}), WAIT)
        .unwrap();
    assert_eq!(found["slug"], "one");
    let record = client
        .call("data_source_fetch", json!({"ref": found}), WAIT)
        .unwrap();
    assert_eq!(record["body"], "Body [[two]]\n");
    let written = client
        .call(
            "data_source_write",
            json!({"path": "notes/two.md", "frontmatter": {"title": "Two é"}, "body": "x\n"}),
            WAIT,
        )
        .unwrap();
    assert_eq!(written["record"]["id"], "fixture:notes/two.md");
    assert_eq!(
        std::fs::read_to_string(root.join("notes/two.md")).unwrap(),
        "---\n{\"title\": \"Two é\"}\n---\nx\n"
    );
    let page = client
        .call(
            "data_source_search",
            json!({"query": "type:task first"}),
            WAIT,
        )
        .unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let links = client
        .call("data_source_links", json!({"ref": found}), WAIT)
        .unwrap();
    assert_eq!(links["outgoing"][0]["to"], written["record"]["id"]);
    let changes = client
        .call("data_source_changes_since", json!({"cursor": "0"}), WAIT)
        .unwrap();
    assert_eq!(changes["changed"].as_array().unwrap().len(), 2);
    assert_eq!(client.close(WAIT), Some(0));
    let kinds: Vec<String> = std::fs::read_to_string(&trace)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["kind"].to_string())
        .collect();
    assert_eq!(kinds[0], "\"shape\"");
    assert_eq!(kinds[1], "\"initialize\"");
}

fn shape_latency(binary: &Path) -> Duration {
    let started = Instant::now();
    let status = Command::new(binary)
        .arg("--shape")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    started.elapsed()
}

/// First exec of freshly copied fixture executables (new inode each time),
/// the case that exceeded the 2 s `--shape` budget under load. Run with
/// `--ignored --nocapture`, ideally beside a loaded machine.
#[test]
#[ignore = "measurement: first-exec latency of fresh fixture copies"]
fn first_exec_latency_of_fresh_copies() {
    let temp = tempfile::tempdir().unwrap();
    for kind in [Kind::Reference, Kind::Root] {
        let built = Knobs::of(kind).build();
        let mut samples = Vec::new();
        for index in 0..20 {
            let copy = Knobs::of(kind).install(&temp.path().join(format!("{kind:?}-{index}")));
            assert_eq!(
                std::fs::read(&copy).unwrap(),
                std::fs::read(&built).unwrap()
            );
            samples.push(shape_latency(&copy));
        }
        let warm = shape_latency(&temp.path().join(format!("{kind:?}-0")));
        samples.sort();
        let ms = |d: &Duration| d.as_secs_f64() * 1000.0;
        println!(
            "first-exec {kind:?}: n=20 min={:.1}ms median={:.1}ms p90={:.1}ms max={:.1}ms warm={:.1}ms",
            ms(&samples[0]),
            ms(&samples[10]),
            ms(&samples[18]),
            ms(&samples[19]),
            ms(&warm)
        );
    }
}
