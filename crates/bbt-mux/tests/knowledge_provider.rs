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
    let trusted = plugin(
        &logic.client,
        PluginRequest::Trust {
            path: source.to_str().unwrap().into(),
            sha256: None,
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
    plugin(
        &logic.client,
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    )
    .await;
    let before = roots(&logic.client, KnowledgeRequest::Status).await;
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
    let mut seat = SeatRecord::new(seat_id, swarm, "private provider reader", "zsh");
    seat.role.class = Class::Oracle;
    seat.role.name = "oracle".into();
    let mut launch = LaunchSpec::new("/bin/cat", temp.path());
    launch.seat = seat.id;
    launch.generation = seat.generation;
    logic.client.spawn(seat, launch).await.unwrap();
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
    let back = roots(
        &logic.client,
        KnowledgeRequest::Reset {
            root: swapped.roots[0].id.clone(),
        },
    )
    .await;
    assert_eq!(back.roots[0].provider.id, "bboxy450-term-data_source-bboxy");
    assert_eq!(back.roots[0].records, Some(2));
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
