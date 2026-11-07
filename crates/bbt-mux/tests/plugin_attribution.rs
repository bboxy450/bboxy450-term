// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Seat attribution end to end through the host: a seat's ATC session
//! attribution reaches the selected data_source plugin's call context in
//! request `_meta`; the operator's reads and an unattributed seat carry none,
//! and one seat's attribution never reaches another seat's call.
#![allow(
    clippy::disallowed_methods,
    reason = "owned file-account, process and plugin fixtures; no operator account or desktop"
)]
use crate::common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::TestAccount,
    logic::{Logic, LogicClient, LogicConfig},
    mcp::{Call, Door, Origin},
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::{Class, SeatRecord},
};
use bbt_plugin_fixture::Knobs;
use bbt_proto::gui::{KnowledgeRequest, PluginRequest};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

fn origin() -> Origin {
    Origin {
        door: Door::Ctl,
        issued_by: None,
    }
}

fn lent(seat: &str) -> BTreeMap<String, String> {
    [
        ("ATC_SESSION_ID", format!("xses-{seat}")),
        ("SESSION_ACTIVITY_RESOURCE_ID", format!("xses-{seat}")),
        ("SESSION_ACTIVITY_PROVIDER", "claude".into()),
        ("ATC_TOKEN", "secret-token".into()),
    ]
    .map(|(k, v)| (k.to_owned(), v))
    .into()
}

/// The plugin call lines traced since `from`, and the new line count.
fn calls_since(trace: &Path, from: usize) -> (Vec<Value>, usize) {
    let lines: Vec<Value> = fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let calls = lines[from..]
        .iter()
        .filter(|line| line["kind"] == "call" && line["method"] != "health")
        .cloned()
        .collect();
    (calls, lines.len())
}

async fn fetch(client: &LogicClient, seat: Ulid, attribution: BTreeMap<String, String>) -> Value {
    let call = Call {
        op_id: Ulid::new(),
        tool: "knowledge_fetch".into(),
        args: json!({"locator":"#1"}),
    };
    for _ in 0..100 {
        let reply = client
            .control_call_attributed(seat, 1, call.clone(), attribution.clone())
            .await
            .unwrap();
        if reply["structuredContent"]["error"]["code"] != "workspace_pending" {
            return reply;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("workspace preparation did not finish");
}

// keeps: seat attribution — the seat's validated ATC session attribution
// rides its plugin calls in `_meta`; operator calls carry none; no cross-seat leak.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seat_plugin_calls_carry_their_own_attribution_and_operator_calls_none() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let root = repo.join(".bboxy");
    fs::create_dir_all(root.join("plugins")).unwrap();
    fs::create_dir_all(root.join("notes")).unwrap();
    fs::write(
        root.join("notes/one.md"),
        format!(
            "---\n{}\n---\nPrivate note body.\n",
            json!({"id":Ulid::from(1_u128).to_string(),"number":1,"type":"note","title":"Private note"})
        ),
    )
    .unwrap();
    let trace = temp.path().join("protocol.jsonl");
    let source = root.join("plugins/bboxy450-term-data_source-root-fixture");
    Knobs {
        trace: Some(trace.to_string_lossy().into_owned()),
        ..Knobs::root()
    }
    .install(&source);
    let account = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: repo,
            ring: Default::default(),
        },
        account.key_store(),
        HostConfig {
            registry: RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
            provider: ProviderConfig {
                timeout: Duration::from_secs(30),
            },
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let client = &logic.client;
    let trusted = client
        .gui_plugin(
            Ulid::new(),
            PluginRequest::Trust {
                path: source.to_str().unwrap().into(),
                sha256: None,
                grants: None,
                tier: None,
            },
            origin(),
        )
        .await
        .unwrap();
    let candidate = trusted[0].candidate.id.clone();
    for request in [
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    ] {
        client
            .gui_plugin(Ulid::new(), request, origin())
            .await
            .unwrap();
    }
    let status = client
        .gui_knowledge_control(Ulid::new(), KnowledgeRequest::Status, origin())
        .await
        .unwrap();
    let choice = status.offered.iter().find(|p| p.id == candidate).unwrap();
    client
        .gui_knowledge_control(
            Ulid::new(),
            KnowledgeRequest::Select {
                root: status.roots[0].id.clone(),
                provider: choice.id.clone(),
                revision: status.roots[0].revision,
                sha256: choice.sha256.clone(),
            },
            origin(),
        )
        .await
        .unwrap();
    let swarm = Ulid::new();
    let (a, b) = (Ulid::new(), Ulid::new());
    for id in [a, b] {
        let mut record = SeatRecord::new(id, swarm, "attribution fixture", "zsh");
        // Oracle: document commits need it.
        record.role.class = Class::Oracle;
        record.role.name = "oracle".into();
        let mut spec = LaunchSpec::new("/bin/cat", temp.path());
        spec.seat = id;
        spec.generation = 1;
        client.spawn(record, spec).await.unwrap();
    }
    let expect = |seat: &str| {
        let mut env = lent(seat);
        env.remove("ATC_TOKEN");
        json!(env)
    };

    let (_, mut seen) = calls_since(&trace, 0);
    let mut step = async |seat: Ulid, attribution: BTreeMap<String, String>| {
        let reply = fetch(client, seat, attribution).await;
        assert_eq!(reply["isError"], false, "{reply:#}");
        assert_eq!(reply["structuredContent"]["body"], "Private note body.\n");
        let (calls, now) = calls_since(&trace, seen);
        seen = now;
        assert!(!calls.is_empty(), "the read reached the plugin");
        calls
    };
    for call in step(a, lent("a")).await {
        assert_eq!(call["attribution"], expect("a"), "{call}");
    }
    for call in step(b, BTreeMap::new()).await {
        assert!(call.get("attribution").is_none(), "{call}");
    }
    for call in step(b, lent("b")).await {
        assert_eq!(call["attribution"], expect("b"), "{call}");
    }
    for call in step(a, lent("a")).await {
        assert_eq!(call["attribution"], expect("a"), "{call}");
    }
    drop(step);

    // The operator's read of the same root, after both seats lent theirs.
    let reference = client
        .gui_knowledge("resolve".into(), json!({"locator":"#1"}))
        .await
        .unwrap()
        .unwrap();
    client
        .gui_knowledge("fetch".into(), json!({"ref":reference}))
        .await
        .unwrap()
        .unwrap();
    let (calls, now) = calls_since(&trace, seen);
    assert!(!calls.is_empty(), "the operator read reached the plugin");
    for call in calls {
        assert!(call.get("attribution").is_none(), "{call}");
    }

    // A document effect: the seat's reads while preparing it carry its
    // attribution too (this fixture then refuses the commit itself).
    let commit = client
        .control_call_attributed(
            a,
            1,
            Call {
                op_id: Ulid::new(),
                tool: "knowledge_commit".into(),
                args: json!({"locators":["#1"],"message":"attributed commit"}),
            },
            lent("a"),
        )
        .await
        .unwrap();
    assert_eq!(
        commit["structuredContent"]["error"]["code"], "unsupported_operation",
        "{commit:#}"
    );
    let (calls, _) = calls_since(&trace, now);
    assert!(!calls.is_empty(), "the effect read reached the plugin");
    for call in calls {
        assert_eq!(call["attribution"], expect("a"), "{call}");
    }
    let text = fs::read_to_string(&trace).unwrap();
    assert!(
        !text.contains("secret-token"),
        "a secret reached the plugin"
    );
    logic.shutdown().unwrap();
}

// keeps: seat attribution — a declared plugin seat tool receives the calling
// seat's validated attribution in `_meta`; an unattributed seat sends none.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seat_tool_calls_carry_only_the_calling_seats_attribution() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let path = Knobs {
        tools: true,
        ..Knobs::reference().system("repo_graph")
    }
    .install(&repo.join(".bboxy/plugins/bboxy450-term-repo_graph-reference"));
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: repo.clone(),
            ring: Default::default(),
        },
        custody.key_store(),
        HostConfig {
            registry: RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
            provider: ProviderConfig::default(),
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let client = &logic.client;
    let trusted = client
        .gui_plugin(
            Ulid::new(),
            PluginRequest::Trust {
                path: path.to_str().unwrap().into(),
                sha256: None,
                grants: None,
                tier: None,
            },
            origin(),
        )
        .await
        .unwrap();
    let candidate = trusted[0].candidate.id.clone();
    for request in [
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    ] {
        client
            .gui_plugin(Ulid::new(), request, origin())
            .await
            .unwrap();
    }
    let swarm = Ulid::new();
    let (a, b) = (Ulid::new(), Ulid::new());
    for id in [a, b] {
        let record = SeatRecord::new(id, swarm, "attribution tool fixture", "zsh");
        let mut spec = LaunchSpec::new("/bin/cat", temp.path());
        spec.seat = id;
        spec.generation = 1;
        client.spawn(record, spec).await.unwrap();
    }
    let echo = async |seat: Ulid, attribution: BTreeMap<String, String>| {
        let call = Call {
            op_id: Ulid::new(),
            tool: "plugin_tool_reference_echo".into(),
            args: json!({"text":"hi"}),
        };
        for _ in 0..100 {
            let reply = client
                .control_call_attributed(seat, 1, call.clone(), attribution.clone())
                .await
                .unwrap();
            if reply["structuredContent"]["error"]["code"] != "workspace_pending" {
                assert_eq!(reply["isError"], false, "{reply:#}");
                return reply["structuredContent"]["result"].clone();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("workspace preparation did not finish");
    };
    let mut expected = lent("a");
    expected.remove("ATC_TOKEN");
    let attributed = echo(a, lent("a")).await;
    assert_eq!(
        attributed["meta"],
        json!({"bboxy450/attribution":{"env":expected}}),
        "{attributed:#}"
    );
    let plain = echo(b, BTreeMap::new()).await;
    assert_eq!(plain["tool"], "tool_echo", "{plain:#}");
    assert!(plain.get("meta").is_none(), "{plain:#}");
    logic.shutdown().unwrap();
}
