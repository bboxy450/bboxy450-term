// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-only paging probe against the SDK Harmony variant, never an account.
#![allow(clippy::disallowed_methods)]
use bbt_plugin_fixture::Knobs;
use bbt_plugin_sdk::testkit::Client;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{Duration, Instant},
};
const CAP: Duration = Duration::from_secs(2);

fn bytes(root: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let value = fs::read(&path).unwrap();
            (path, value)
        })
        .collect()
}

fn pages(client: &mut Client, method: &str, request: Value, expected: usize) -> BTreeSet<String> {
    let mut request = request;
    let limit = request["limit"].as_u64().unwrap() as usize;
    let mut seen = BTreeSet::new();
    let mut count = 0;
    let mut slowest = Duration::ZERO;
    loop {
        let start = Instant::now();
        let page = client
            .call(&format!("data_source_{method}"), request.clone(), CAP)
            .unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed < CAP, "{method} exceeded 2 s: {elapsed:?}");
        slowest = slowest.max(elapsed);
        count += 1;
        let items = page["items"].as_array().unwrap();
        assert!(
            !items.is_empty(),
            "a populated query returned an empty page"
        );
        assert!(items.len() <= limit);
        if !page["next"].is_null() {
            assert_eq!(items.len(), limit, "nonterminal {method} page underfilled");
        }
        for item in items {
            let reference = if method == "search" {
                &item["ref"]
            } else {
                item
            };
            assert!(
                seen.insert(reference["id"].as_str().unwrap().into()),
                "duplicate record across pages"
            );
        }
        assert!(
            count <= expected.div_ceil(limit),
            "cursor did not terminate"
        );
        if page["next"].is_null() {
            break;
        }
        request["cursor"] = page["next"].clone();
    }
    assert_eq!(seen.len(), expected);
    eprintln!(
        "SDK fixture {method}: {count} pages at limit {limit}, {expected} unique records, slowest {slowest:?}, every call < 2 s"
    );
    seen
}

#[test]
fn harmony_readonly_pages_fill_limits_and_both_status_spellings_find_active_tasks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo/.bboxy");
    let backing = temp.path().join("repo/external/tasks");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&backing).unwrap();
    let active: BTreeSet<String> = (0..129).map(|i| format!("{:026}", i + 1)).collect();
    let all_tasks: BTreeSet<String> = (0..133).map(|i| format!("{:026}", i + 1)).collect();
    for i in 0..133 {
        let (id, status) = if i < 129 {
            (format!("{:026}", i + 1), "active")
        } else {
            (format!("{:026}", i + 1), "completed")
        };
        fs::write(backing.join(format!("{i:03}.md")), format!("---\nid: \"{id}\"\nnumber: {}\ntype: task\nstatus: {status}\ntitle: Knowledge fixture {i}\n---\nKnowledge private paging witness\n",i+1)).unwrap();
    }
    for i in 0..2 {
        fs::write(backing.join(format!("note-{i}.md")), format!("---\nid: \"{:026}\"\nnumber: {}\ntype: note\nstatus: active\ntitle: Knowledge note\n---\nKnowledge must stay outside task filters\n",1000+i,200+i)).unwrap();
    }
    let before = bytes(&backing);
    let plugin = Knobs {
        harmony: true,
        ..Knobs::root()
    }
    .build();
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let env = vec![
        ("HOME".into(), home.to_string_lossy().into_owned()),
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("LANG".into(), "en_US.UTF-8".into()),
    ];
    let mut client = Client::spawn(&plugin, &temp.path().join("data"), &env).unwrap();
    let start = Instant::now();
    client.initialize(json!({"root":root}), CAP).unwrap();
    assert!(start.elapsed() < CAP);
    for limit in [7, 32, 128] {
        let all = pages(
            &mut client,
            "list",
            json!({"filter":{"type":"task"},"limit":limit}),
            133,
        );
        assert_eq!(all, all_tasks, "list must return the exact task inventory");
        assert_eq!(
            pages(
                &mut client,
                "search",
                json!({"query":"knowledge type:task","limit":limit}),
                133
            ),
            all
        );
    }
    for spelling in ["active", "running"] {
        assert_eq!(
            pages(
                &mut client,
                "search",
                json!({"query":format!("type:task status:{spelling}"),"limit":32}),
                129
            ),
            active
        );
        assert_eq!(
            pages(
                &mut client,
                "list",
                json!({"filter":{"type":"task","status":spelling},"limit":32}),
                129
            ),
            active
        );
    }
    assert_eq!(client.close(CAP), Some(0));
    assert_eq!(
        bytes(&backing),
        before,
        "read-only probe changed the backing source"
    );
}

#[test]
fn ordinary_root_keeps_status_spellings_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir_all(root.join("tasks")).unwrap();
    for (number, status) in [(1, "active"), (2, "running")] {
        fs::write(root.join(format!("tasks/{status}.md")), format!(
            "---\nid: \"{number:026}\"\nnumber: {number}\ntype: task\nstatus: {status}\ntitle: Status control\n---\nPrivate control\n"
        )).unwrap();
    }
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let env = vec![
        ("HOME".into(), home.to_string_lossy().into_owned()),
        ("PATH".into(), "/usr/bin:/bin".into()),
    ];
    let mut client =
        Client::spawn(&Knobs::root().build(), &temp.path().join("data"), &env).unwrap();
    client.initialize(json!({"root": root}), CAP).unwrap();
    for (number, status) in [(1, "active"), (2, "running")] {
        let expected = BTreeSet::from([format!("{number:026}")]);
        assert_eq!(
            pages(
                &mut client,
                "list",
                json!({"filter":{"type":"task","status":status},"limit":1}),
                1
            ),
            expected
        );
        assert_eq!(
            pages(
                &mut client,
                "search",
                json!({"query":format!("type:task status:{status}"),"limit":1}),
                1
            ),
            expected
        );
    }
    assert_eq!(client.close(CAP), Some(0));
}
