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
    logic::{Logic, LogicConfig},
    mcp::{Call, Door, Origin},
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::{SeatRecord},
};
use bbt_plugin_fixture::Knobs;
use bbt_proto::gui::{PluginRequest};
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};

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

// A seat tool receives the calling
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
