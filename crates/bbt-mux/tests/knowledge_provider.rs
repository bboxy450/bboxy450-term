// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "owned file-account, process and socket fixtures; no operator account or desktop"
)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::TestAccount,
    logic::{Logic, LogicClient, LogicConfig},
    mcp::{Call, Door, Origin},
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::{Class, Registry, SeatRecord},
    store::sqlite::SqliteStore,
};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest,
    gui::{KnowledgeRequest, KnowledgeStatus, PluginRequest},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};

async fn roots(client: &LogicClient, request: KnowledgeRequest) -> KnowledgeStatus {
    client
        .gui_knowledge_control(
            Ulid::new(),
            request,
            Origin {
                door: Door::Ctl,
                issued_by: None,
            },
        )
        .await
        .unwrap()
}
async fn plugin(client: &LogicClient, request: PluginRequest) -> Vec<bbt_plugin::registry::Entry> {
    client
        .gui_plugin(
            Ulid::new(),
            request,
            Origin {
                door: Door::Ctl,
                issued_by: None,
            },
        )
        .await
        .unwrap()
}
async fn tool(client: &LogicClient, seat: Ulid, tool: &str, args: Value) -> Value {
    client
        .control_call(
            seat,
            1,
            Call {
                op_id: Ulid::new(),
                tool: tool.into(),
                args,
            },
        )
        .await
        .unwrap()
}

// keeps: knowledge-base §6(3), #350 stdio root binding, ui-never-waits and current seat authority.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_root_swap_serves_gui_mcp_and_graph_reads_and_fences_a_changed_role() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let root = repo.join(".bboxy");
    fs::create_dir_all(root.join("plugins")).unwrap();
    fs::create_dir_all(root.join("notes")).unwrap();
    fs::create_dir_all(root.join("tasks")).unwrap();
    let seat_id = Ulid::new();
    let swarm = Ulid::new();
    let note = format!(
        "---\n{}\n---\nPrivate note body.\n",
        json!({"id":Ulid::from(1_u128).to_string(),"number":1,"type":"note","title":"Private note"})
    );
    let task = format!(
        "---\n{}\n---\nPrivate task body.\n",
        json!({"id":Ulid::from(2_u128).to_string(),"number":2,"type":"task","title":"Private task","assignee":seat_id.to_string(),"status":"open"})
    );
    fs::write(root.join("notes/one.md"), &note).unwrap();
    fs::write(root.join("tasks/two.md"), &task).unwrap();
    let gate_dir = tempfile::tempdir().unwrap();
    let gate = gate_dir.path().join("gate.sock");
    let armed = gate_dir.path().join("armed");
    let listener = tokio::net::UnixListener::bind(&gate).unwrap();
    let trace = temp.path().join("protocol.jsonl");
    let source = root.join("plugins/bboxy450-term-data_source-root-fixture");
    let template = include_str!("fixtures/plugins/root_provider.py")
        .replace(
            "TRACE = None",
            &format!("TRACE = {}", json!(trace.to_str().unwrap())),
        )
        .replace(
            "GATE = None",
            &format!("GATE = {}", json!(gate.to_str().unwrap())),
        )
        .replace(
            "GATE_ROOT = None",
            &format!(
                "GATE_ROOT = {}",
                json!(root.canonicalize().unwrap().to_str().unwrap())
            ),
        )
        .replace(
            "GATE_ARMED = None",
            &format!("GATE_ARMED = {}", json!(armed.to_str().unwrap())),
        );
    fs::write(
        &source,
        format!(
            "#!/usr/bin/env python3\nimport json\nSCHEMA=json.loads({})\n{template}",
            serde_json::to_string(bbt_plugin::data_source::SCHEMA).unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
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
    let untrusted = plugin(&logic.client, PluginRequest::List).await;
    assert_eq!(untrusted[0].state(), bbt_plugin::registry::State::Untrusted);
    assert!(!trace.exists());
    let mut seat = SeatRecord::new(seat_id, swarm, "private provider reader", "zsh");
    seat.role.class = Class::Oracle;
    seat.role.name = "oracle".into();
    let mut launch = LaunchSpec::new("/bin/cat", temp.path());
    launch.seat = seat.id;
    launch.generation = seat.generation;
    logic.client.spawn(seat, launch).await.unwrap();
    let fixture_names = |reply: &Value| {
        reply["structuredContent"]["skills"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|skill| skill["name"] == "root-fixture")
            .count()
    };
    let untrusted_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(fixture_names(&untrusted_skills), 0);
    assert!(
        !trace.exists(),
        "skill_list started an untrusted plugin process"
    );
    let trusted = plugin(
        &logic.client,
        PluginRequest::Trust {
            path: source.to_str().unwrap().into(),
            sha256: None,
            grants: None,
            tier: None,
        },
    )
    .await;
    let candidate = trusted[0].candidate.id.clone();
    let checked = plugin(
        &logic.client,
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
    )
    .await;
    assert!(checked[0].conformance.as_ref().unwrap().ok);
    let disabled_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(
        fixture_names(&disabled_skills),
        0,
        "a checked but disabled plugin supplied instructions"
    );
    plugin(
        &logic.client,
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    )
    .await;
    let before = roots(&logic.client, KnowledgeRequest::Status).await;
    let before_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(before_skills["isError"], false);
    assert!(before_skills["structuredContent"]["provider_error"].is_null());
    let unselected_skill = before_skills["structuredContent"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|skill| skill["name"] == "root-fixture")
        .cloned()
        .expect("an enabled, checked plugin supplies its instructions without selection");
    assert_eq!(unselected_skill["origin"], candidate);
    assert_eq!(
        unselected_skill["plugin_sha256"],
        json!(checked[0].conformance.as_ref().unwrap().sha256)
    );
    let choice = before.offered.iter().find(|p| p.id == candidate).unwrap();
    // Exercise the new operation-correlated GUI socket port, not only a library call.
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let (read, mut write) = UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    let op = Ulid::new();
    let mut bytes = serde_json::to_vec(&GuiRequest {
        version: bbt_proto::gui::VERSION,
        capability: endpoint.capability,
        action: GuiAction::KnowledgeControl {
            scope: None,
            id: op,
            request: KnowledgeRequest::Select {
                root: before.roots[0].id.clone(),
                provider: choice.id.clone(),
                revision: before.roots[0].revision,
                sha256: choice.sha256.clone(),
            },
        },
    })
    .unwrap();
    bytes.push(b'\n');
    write.write_all(&bytes).await.unwrap();
    let swapped = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let line = bbt_mux::seat_socket::read_line(&mut read)
                .await
                .unwrap()
                .unwrap();
            match serde_json::from_slice::<GuiReply>(&line).unwrap() {
                GuiReply::KnowledgeControl { op: id, status } if id == op => break status,
                GuiReply::Error { op: id, reason } if id == op => panic!("{reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    let pid = swapped.roots[0].process.unwrap();
    assert_eq!(swapped.roots[0].records, None);
    let reference = logic
        .client
        .gui_knowledge("resolve".into(), json!({"locator":"#1"}))
        .await
        .unwrap()
        .unwrap();
    let fetched = logic
        .client
        .gui_knowledge("fetch".into(), json!({"ref":reference}))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched["body"], "Private note body.\n");
    let selected_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(selected_skills["isError"], false);
    assert!(selected_skills["structuredContent"]["provider_error"].is_null());
    let selected_skills = selected_skills["structuredContent"]["skills"]
        .as_array()
        .unwrap();
    assert_eq!(selected_skills.len(), 3);
    let fixture_skill = selected_skills
        .iter()
        .find(|skill| skill["name"] == "root-fixture")
        .unwrap();
    assert_eq!(
        fixture_skill, &unselected_skill,
        "selection does not change a plugin skill's identity"
    );
    let expected_skill = "---\nname: root-fixture\ndescription: Root fixture knowledge instructions.\n---\n\n# Root fixture\n\nResolve before fetch; preserve é and exact source bytes.\n";
    assert_eq!(fixture_skill["root_id"], swapped.roots[0].id);
    assert_eq!(fixture_skill["size"], expected_skill.len());
    assert_eq!(
        fixture_skill["digest"],
        format!(
            "sha256:{}",
            bbt_mux::store::digest(expected_skill.as_bytes())
        )
    );
    let skill_call = Call {
        op_id: Ulid::new(),
        tool: "skill_read".into(),
        args: json!({"uri":fixture_skill["uri"]}),
    };
    let skill_read = logic
        .client
        .control_call(seat_id, 1, skill_call.clone())
        .await
        .unwrap();
    assert_eq!(skill_read["isError"], false);
    assert_eq!(
        skill_read["structuredContent"]["skill"]["content"],
        expected_skill
    );
    let call = Call {
        op_id: Ulid::new(),
        tool: "knowledge_fetch".into(),
        args: json!({"locator":"#1"}),
    };
    let first = logic
        .client
        .control_call(seat_id, 1, call.clone())
        .await
        .unwrap();
    assert_eq!(first["isError"], false);
    assert_eq!(first["structuredContent"]["body"], "Private note body.\n");
    let search = tool(
        &logic.client,
        seat_id,
        "knowledge_search",
        json!({"query":"Private"}),
    )
    .await;
    assert_eq!(search["isError"], false);
    assert_eq!(
        search["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let graph = tool(
        &logic.client,
        seat_id,
        "graph_read",
        json!({"graph":"markdown","limit":128}),
    )
    .await;
    assert_eq!(graph["isError"], false);
    assert_eq!(
        graph["structuredContent"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let orient = tool(&logic.client, seat_id, "orient", json!({})).await;
    assert_eq!(orient["isError"], false);
    assert_eq!(
        orient["structuredContent"]["tasks"]["items"][0]["body"],
        "Private task body.\n"
    );
    let denied=logic.client.gui_record_write(Ulid::new(),json!({"path":"e2e/must-not-write.md","frontmatter":{"type":"e2e","title":"denied"},"body":"must not land"}),Origin {door:Door::Ctl,issued_by:None}).await.unwrap().unwrap_err();
    assert_eq!(denied.code, "signed_writer_unavailable");
    assert!(!root.join("e2e/must-not-write.md").exists());
    fs::write(&armed, b"armed").unwrap();
    let client = logic.client.clone();
    let held = tokio::spawn(async move {
        client
            .control_call(
                seat_id,
                1,
                Call {
                    op_id: Ulid::new(),
                    tool: "knowledge_fetch".into(),
                    args: json!({"locator":"#1"}),
                },
            )
            .await
    });
    let (mut barrier, _) = tokio::time::timeout(Duration::from_secs(30), listener.accept())
        .await
        .expect("fetch never reached its explicit barrier")
        .unwrap();
    // An unrelated admitted input completes while the provider remains held.
    tokio::time::timeout(
        Duration::from_secs(30),
        logic.client.ctl_input(
            seat_id,
            1,
            Ulid::new(),
            b"private input\n".to_vec(),
            Origin {
                door: Door::Ctl,
                issued_by: None,
            },
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let registry = Registry::new(SqliteStore::open(&run.join("account")).unwrap());
    registry
        .update(seat_id, |seat| {
            seat.role.class = Class::Worker;
            seat.role.name = "executor".into();
            Ok(())
        })
        .unwrap();
    barrier.write_all(b"x").await.unwrap();
    assert!(
        held.await
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("authority changed")
    );
    fs::remove_file(&armed).unwrap();
    // Cache a successful instruction read under the current worker grant before
    // disable, so the next replay refusal proves plugin-enablement fencing
    // independently of the earlier role-change refusal.
    let skill_call = Call {
        op_id: Ulid::new(),
        ..skill_call
    };
    let current_skill = logic
        .client
        .control_call(seat_id, 1, skill_call.clone())
        .await
        .unwrap();
    assert_eq!(current_skill["isError"], false);
    assert_eq!(
        current_skill["structuredContent"]["skill"]["content"],
        expected_skill
    );
    let back = roots(
        &logic.client,
        KnowledgeRequest::Reset {
            root: swapped.roots[0].id.clone(),
        },
    )
    .await;
    assert_eq!(back.roots[0].provider.id, "bboxy450-term-data_source-bboxy");
    assert_eq!(back.roots[0].records, Some(2));
    // Reset deselects the provider but leaves the plugin enabled and checked,
    // so its instructions keep their identity and remain readable.
    let reset_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(reset_skills["isError"], false);
    assert_eq!(
        reset_skills["structuredContent"]["skills"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(fixture_names(&reset_skills), 1);
    let still = tool(
        &logic.client,
        seat_id,
        "skill_read",
        skill_call.args.clone(),
    )
    .await;
    assert_eq!(still["isError"], false);
    assert_eq!(
        still["structuredContent"]["skill"]["content"],
        expected_skill
    );
    // One plugin's failed pin is reported for that plugin only; host guides stay.
    let copy = checked[0].trust.as_ref().unwrap().copy.clone();
    fs::set_permissions(&copy, fs::Permissions::from_mode(0o700)).unwrap();
    let failed = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    fs::set_permissions(&copy, fs::Permissions::from_mode(0o500)).unwrap();
    assert_eq!(failed["isError"], false);
    assert_eq!(fixture_names(&failed), 0);
    assert_eq!(
        failed["structuredContent"]["skills"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        failed["structuredContent"]["provider_error"],
        json!([{"origin":candidate,"plugin_sha256":unselected_skill["plugin_sha256"],
            "code":"plugin_copy_writable",
            "message":"Plugin guidance is unavailable; other guidance remains available."}])
    );
    plugin(
        &logic.client,
        PluginRequest::Disable {
            candidate: candidate.clone(),
        },
    )
    .await;
    let disabled_skills = tool(&logic.client, seat_id, "skill_list", json!({})).await;
    assert_eq!(disabled_skills["isError"], false);
    assert_eq!(
        disabled_skills["structuredContent"]["skills"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let old_skill = tool(
        &logic.client,
        seat_id,
        "skill_read",
        skill_call.args.clone(),
    )
    .await;
    assert_eq!(
        old_skill["isError"], true,
        "disabled plugin URI still returned instructions"
    );
    let old_skill = logic
        .client
        .control_call(seat_id, 1, skill_call)
        .await
        .unwrap();
    assert_eq!(
        old_skill["isError"], true,
        "cached plugin instructions bypassed plugin disable"
    );
    let replay = logic.client.control_call(seat_id, 1, call).await.unwrap();
    assert_eq!(
        replay["isError"], true,
        "old root/role result must not leak through replay"
    );
    assert_eq!(fs::read_to_string(root.join("notes/one.md")).unwrap(), note);
    assert_eq!(fs::read_to_string(root.join("tasks/two.md")).unwrap(), task);
    drop(write);
    drop(read);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

// keeps: G2 plugin seat tools — an enabled, checked plugin's contract methods
// and declared tools reach seats by role, a seat call reaches the plugin, and
// disable removes them and refuses a later call.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enabled_plugin_methods_and_declared_tools_reach_seats_by_role() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let root = repo.join(".bboxy");
    fs::create_dir_all(root.join("plugins")).unwrap();
    let source = root.join("plugins/bboxy450-term-task_system-reference");
    fs::write(
        &source,
        include_str!("fixtures/plugins/bboxy450-term-data_source-reference")
            .replacen("SYSTEM = \"data_source\"", "SYSTEM = \"task_system\"", 1)
            .replacen("TOOLS = False", "TOOLS = True", 1),
    )
    .unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
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
    let swarm = Ulid::new();
    let mut seats = Vec::new();
    for oracle in [false, true] {
        let mut seat = SeatRecord::new(Ulid::new(), swarm, "plugin tool seat", "zsh");
        if oracle {
            seat.role.class = Class::Oracle;
            seat.role.name = "oracle".into();
        }
        assert_eq!(seat.role.class == Class::Oracle, oracle);
        let mut launch = LaunchSpec::new("/bin/cat", temp.path());
        launch.seat = seat.id;
        launch.generation = seat.generation;
        seats.push(seat.id);
        logic.client.spawn(seat, launch).await.unwrap();
    }
    let (worker, oracle) = (seats[0], seats[1]);
    let plugin_tools = |catalogue: &Value| -> Vec<String> {
        let mut names: Vec<String> = catalogue["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .filter(|name| name.starts_with("plugin_"))
            .map(str::to_owned)
            .collect();
        names.sort();
        names
    };
    let catalogue = |seat| {
        let client = logic.client.clone();
        async move { client.control_catalogue(seat, 1).await.unwrap() }
    };
    assert!(plugin_tools(&catalogue(oracle).await).is_empty());

    let trusted = plugin(
        &logic.client,
        PluginRequest::Trust {
            path: source.to_str().unwrap().into(),
            sha256: None,
            grants: None,
            tier: None,
        },
    )
    .await;
    let candidate = trusted[0].candidate.id.clone();
    let checked = plugin(
        &logic.client,
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
    )
    .await;
    assert!(checked[0].conformance.as_ref().unwrap().ok, "{checked:?}");
    assert!(
        plugin_tools(&catalogue(oracle).await).is_empty(),
        "a checked but disabled plugin offered tools"
    );
    plugin(
        &logic.client,
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    )
    .await;

    // Signed transitions (delegate ... reject) require signed_record and stay
    // with the host; declared tools follow their own role.
    let worker_catalogue = catalogue(worker).await;
    assert_eq!(
        plugin_tools(&worker_catalogue),
        [
            "plugin_task_system_reference_changes_since",
            "plugin_task_system_reference_list",
            "plugin_task_system_reference_status",
            "plugin_tool_reference_echo",
        ]
    );
    let oracle_catalogue = catalogue(oracle).await;
    assert_eq!(
        plugin_tools(&oracle_catalogue),
        [
            "plugin_task_system_reference_changes_since",
            "plugin_task_system_reference_list",
            "plugin_task_system_reference_status",
            "plugin_tool_reference_audit",
            "plugin_tool_reference_echo",
        ]
    );
    let echo = worker_catalogue["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "plugin_tool_reference_echo")
        .unwrap();
    assert_eq!(echo["inputSchema"]["properties"]["text"]["type"], "string");
    assert!(echo["inputSchema"]["properties"]["op_id"].is_object());
    assert_eq!(echo["outputSchema"]["required"], json!(["result"]));
    let orient = tool(&logic.client, worker, "orient", json!({})).await;
    let mut oriented: Vec<String> = orient["structuredContent"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
        .filter(|name| name.starts_with("plugin_"))
        .collect();
    oriented.sort();
    assert_eq!(oriented, plugin_tools(&worker_catalogue));

    let echoed = tool(
        &logic.client,
        worker,
        "plugin_tool_reference_echo",
        json!({"text": "hi"}),
    )
    .await;
    assert_eq!(echoed["isError"], false, "{echoed}");
    assert_eq!(
        echoed["structuredContent"]["result"],
        json!({"tool": "tool_echo", "args": {"text": "hi"}})
    );
    let listed = tool(
        &logic.client,
        worker,
        "plugin_task_system_reference_list",
        json!({}),
    )
    .await;
    assert_eq!(listed["isError"], false, "{listed}");
    assert_eq!(
        listed["structuredContent"]["result"]["items"][0]["path"],
        "tasks/p0-fixture.md"
    );
    let code = |reply: &Value| reply["structuredContent"]["error"]["code"].clone();
    let invalid = tool(
        &logic.client,
        worker,
        "plugin_tool_reference_echo",
        json!({"text": 7}),
    )
    .await;
    assert_eq!(code(&invalid), "invalid_request", "{invalid}");
    let host_field = tool(
        &logic.client,
        worker,
        "plugin_task_system_reference_list",
        json!({"root": "/elsewhere"}),
    )
    .await;
    assert_eq!(code(&host_field), "host_field_in_arguments");
    let audit = json!({"scope": "all"});
    let refused = tool(
        &logic.client,
        worker,
        "plugin_tool_reference_audit",
        audit.clone(),
    )
    .await;
    assert_eq!(
        code(&refused),
        "unknown_tool",
        "an oracle tool served a worker"
    );
    let audited = tool(&logic.client, oracle, "plugin_tool_reference_audit", audit).await;
    assert_eq!(audited["structuredContent"]["result"]["tool"], "tool_audit");

    plugin(
        &logic.client,
        PluginRequest::Disable {
            candidate: candidate.clone(),
        },
    )
    .await;
    assert!(plugin_tools(&catalogue(oracle).await).is_empty());
    let after = tool(
        &logic.client,
        worker,
        "plugin_tool_reference_echo",
        json!({"text": "hi"}),
    )
    .await;
    assert_eq!(
        code(&after),
        "unknown_tool",
        "a disabled plugin served a call"
    );
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
