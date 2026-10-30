// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `host/messages.send` end to end (G7): a private fixture plugin, asked
//! through its seat tool, sends through the host API, the logic signs as the
//! host on the plugin's behalf, and the granted seat's inbox holds a message
//! attributed to `plugin:<candidate>`; an ungranted destination is refused.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private accounts, plugin fixtures and child processes; no operator account"
)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::TestAccount,
    logic::{Logic, LogicClient, LogicConfig},
    mcp::{Call, Door, Origin},
    messaging::api::Request,
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::SeatRecord,
};
use bbt_proto::gui::{PluginGrants, PluginRequest, PluginTier, SwarmScope};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

fn config() -> RegistryConfig {
    RegistryConfig {
        descriptor_timeout: Duration::from_secs(30),
        conformance_timeout: Duration::from_secs(30),
    }
}

async fn call(client: &LogicClient, seat: Ulid, tool: &str, args: Value) -> Value {
    let request = Call {
        op_id: Ulid::new(),
        tool: tool.into(),
        args,
    };
    for _ in 0..100 {
        let result = client.control_call(seat, 1, request.clone()).await.unwrap();
        if result["structuredContent"]["error"]["code"] != "workspace_pending" {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{tool}: workspace preparation did not finish");
}

// keeps: G7 host API messages.send / plugins §3c — host-signed plugin sender.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_plugin_message_reaches_its_granted_seat_signed_by_the_host_as_the_plugin() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference")
        .replacen("SYSTEM = \"data_source\"", "SYSTEM = \"repo_graph\"", 1)
        .replacen(
            "HOST_API = []",
            r#"HOST_API = ["messages.send", "records.search"]"#,
            1,
        );
    let path = repo.join(".bboxy/plugins/bboxy450-term-repo_graph-reference");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let (caller, recipient, swarm) = (Ulid::new(), Ulid::new(), Ulid::new());
    let granted = format!("seats/{recipient}/inbox");
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
            registry: config(),
            provider: ProviderConfig::default(),
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // The operator's product path: trust with grants (the tier is implied:
    // records.search needs export), check, enable.
    let scope = Some(SwarmScope { swarm, revision: 0 });
    let origin = || Origin {
        door: Door::Ctl,
        issued_by: None,
    };
    let trusted = logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Trust {
                path: path.to_str().unwrap().into(),
                sha256: None,
                grants: Some(PluginGrants {
                    host_api: ["messages.send".to_owned(), "records.search".to_owned()].into(),
                    messages_to: [granted.clone()].into(),
                    ..Default::default()
                }),
                tier: None,
            },
            scope,
            origin(),
        )
        .await
        .unwrap();
    let candidate = trusted[0].candidate.id.clone();
    let record = trusted[0].trust.as_ref().unwrap();
    assert_eq!(record.approved_tier, PluginTier::Export);
    assert_eq!(record.grants.messages_to, [granted.clone()].into());
    for request in [
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    ] {
        logic
            .client
            .gui_plugin_scoped(Ulid::new(), request, scope, origin())
            .await
            .unwrap();
    }
    for id in [caller, recipient] {
        let record = SeatRecord::new(id, swarm, "plugin message fixture", "zsh");
        let mut spec = LaunchSpec::new("/bin/cat", temp.path());
        spec.seat = id;
        spec.generation = 1;
        logic.client.spawn(record, spec).await.unwrap();
    }
    let ask = |destination: String| json!({"method": "messages.send", "params": {"destination": destination, "text": "build finished"}});
    let tool = "plugin_tool_reference_host";
    // The workspace mount installs the root's record reader for the host API.
    let searched = call(
        &logic.client,
        caller,
        tool,
        json!({"method": "records.search", "params": {"query": "fixture"}}),
    )
    .await;
    let found = &searched["structuredContent"]["result"]["answer"]["result"];
    assert!(found["items"].is_array(), "{searched:#}");
    let sent = call(&logic.client, caller, tool, ask(granted.clone())).await;
    let answer = &sent["structuredContent"]["result"]["answer"];
    assert_eq!(answer["result"]["destination"], granted, "{sent:#}");
    let refused = call(
        &logic.client,
        caller,
        tool,
        ask(format!("seats/{caller}/inbox")),
    )
    .await;
    assert_eq!(
        refused["structuredContent"]["result"]["answer"]["error"]["data"]["code"],
        "destination_not_granted",
        "{refused:#}"
    );

    let listed = logic
        .client
        .messaging(
            recipient,
            1,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        )
        .await
        .unwrap();
    let listed = serde_json::to_value(listed).unwrap();
    let text = listed.to_string();
    assert!(text.contains("build finished"), "{listed:#}");
    assert!(text.contains(&format!("plugin:{candidate}")), "{listed:#}");
    assert!(text.contains("\"issuer_kind\":\"host\""), "{listed:#}");
    let caller_inbox = logic
        .client
        .messaging(
            caller,
            1,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        )
        .await
        .unwrap();
    assert!(
        !serde_json::to_string(&caller_inbox)
            .unwrap()
            .contains("build finished"),
        "an ungranted destination receives nothing"
    );
    logic.shutdown().unwrap();
}
