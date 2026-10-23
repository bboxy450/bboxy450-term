#![allow(clippy::disallowed_methods)] // Controlled blocking IO/process fixtures.

mod common;
use std::io::{Cursor, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig};
use bbt_mux::mcp::{
    Call,
    bridge::{Bridge, Config as BridgeConfig, read_frame},
};
use bbt_mux::registry::{Class, Registry, SeatRecord};
use bbt_mux::store::{Events, Store, events::EventLog, sqlite::SqliteStore};
use bbt_mux::{LaunchSpec, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

struct Fixture {
    runtime: tokio::runtime::Runtime,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    config: LogicConfig,
    own: SeatRecord,
    peer: SeatRecord,
    oracle: SeatRecord,
    outside: SeatRecord,
    task: Ulid,
    foreign_task: Ulid,
    _temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy/tasks")).unwrap();
        let swarm = Ulid::new();
        let task = Ulid::new();
        let foreign_task = Ulid::new();
        let mut own = SeatRecord::new(Ulid::new(), swarm, "Own", "zsh");
        own.tasks.push(task);
        let peer = SeatRecord::new(Ulid::new(), swarm, "Peer", "zsh");
        let mut oracle = SeatRecord::new(Ulid::new(), swarm, "Oracle", "zsh");
        oracle.role.name = "custom director".into();
        oracle.role.class = Class::Oracle;
        let outside = SeatRecord::new(Ulid::new(), Ulid::new(), "Outside", "zsh");
        for (id, seat, title) in [
            (task, own.id, "Own fixture task"),
            (foreign_task, outside.id, "Foreign fixture task"),
        ] {
            std::fs::write(repository.join(format!(".bboxy/tasks/{id}.md")),format!("---\nid: {id}\ntype: task\ntitle: {title}\nstatus: running\nassignee: {seat}\nverify: 'true'\n---\nSynthetic fixture brief only.\n")).unwrap();
        }
        let config = LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            config.clone(),
            key(),
            runtime.handle(),
        )
        .unwrap();
        for record in [&own, &peer, &oracle, &outside] {
            let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
            spec.seat = record.id;
            spec.generation = record.generation;
            spec.args = vec![
                "-f".into(),
                "-c".into(),
                "while IFS= read -r line; do :; done".into(),
            ];
            runtime
                .block_on(logic.client.spawn(record.clone(), spec))
                .unwrap();
        }
        Self {
            runtime,
            logic: Some(logic),
            supervisor: Some(supervisor),
            config,
            own,
            peer,
            oracle,
            outside,
            task,
            foreign_task,
            _temp: temp,
        }
    }
    fn client(&self) -> &bbt_mux::logic::LogicClient {
        &self.logic.as_ref().unwrap().client
    }
    fn bridge(&self, seat: Ulid) -> Bridge {
        let info = self.runtime.block_on(self.client().endpoint(seat)).unwrap();
        Bridge::new(BridgeConfig {
            socket: info.path,
            seat,
            generation: info.generation,
            capability: info.capability,
        })
    }
    fn call(&self, seat: Ulid, op_id: Ulid, tool: &str, args: Value) -> Value {
        self.runtime
            .block_on(self.client().control_call(
                seat,
                1,
                Call {
                    op_id,
                    tool: tool.into(),
                    args,
                },
            ))
            .unwrap()
    }
    fn registry(&self) -> Registry {
        Registry::new(SqliteStore::open(&self.config.run_dir.join("account")).unwrap())
    }
    fn mutate(&self, seat: Ulid, op_id: Ulid, tool: &str, mut args: Value) -> Value {
        args["op_id"] = json!(op_id);
        let mut bridge = self.bridge(seat);
        initialize(&mut bridge);
        let reply = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":tool,"arguments":args}),
        );
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }
    fn audits(&self, seat: Ulid, count: usize) -> Vec<bbt_mux::store::events::Envelope> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let log = EventLog::open(&self.config.run_dir.join(format!("seat-{seat}"))).unwrap();
            let events: Vec<_> = log
                .after(0, 128)
                .unwrap()
                .into_iter()
                .filter(|event| event.event.subject == format!("seat/{seat}/control"))
                .collect();
            if events.len() >= count {
                return events;
            }
            assert!(Instant::now() < until, "audit not published");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn restart(&mut self) {
        self.logic.take().unwrap().shutdown().unwrap();
        self.logic = Some(
            Logic::start(
                self.supervisor.as_ref().unwrap().connect().unwrap(),
                self.config.clone(),
                key(),
                self.runtime.handle(),
            )
            .unwrap(),
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(logic) = self.logic.take() {
            let _ = logic.shutdown();
        }
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.shutdown();
        }
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[53; 32])
}
fn data(value: Value) -> Value {
    assert_eq!(value["isError"], false, "{value}");
    value["structuredContent"].clone()
}
fn rpc(bridge: &mut Bridge, id: u64, method: &str, params: Value) -> Value {
    bridge
        .handle(
            &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .unwrap(),
        )
        .unwrap()
}
fn initialize(bridge: &mut Bridge) {
    let value = rpc(
        bridge,
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"fixture","version":"1"},"capabilities":{}}),
    );
    assert_eq!(value["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        value["result"]["capabilities"]["tools"]["listChanged"],
        false
    );
    assert!(
        bridge
            .handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
}

#[test]
fn mcp_four_methods_forward_to_logic_and_explicit_metadata_replays_the_original_result() {
    let fixture = Fixture::new();
    let mut bridge = fixture.bridge(fixture.own.id);
    assert_eq!(
        rpc(&mut bridge, 0, "tools/list", json!({}))["error"]["code"],
        -32002
    );
    assert_eq!(rpc(&mut bridge, 0, "ping", json!({}))["result"], json!({}));
    initialize(&mut bridge);
    let list = rpc(&mut bridge, 2, "tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 12);
    assert!(
        tools
            .iter()
            .filter(|tool| matches!(
                tool["name"].as_str(),
                Some("orient" | "self_status" | "seat_list" | "seat_get" | "graph_read")
            ))
            .all(|tool| tool["annotations"]["readOnlyHint"] == true)
    );
    assert_eq!(
        rpc(&mut bridge, 3, "resources/list", json!({}))["error"]["code"],
        -32601
    );
    let op_id = Ulid::new();
    let params = json!({"name":"orient","arguments":{},"_meta":{"bboxy450/op_id":op_id}});
    let first = rpc(&mut bridge, 4, "tools/call", params.clone());
    assert_eq!(first["result"]["isError"], false);
    let data = &first["result"]["structuredContent"];
    assert_eq!(data["actor"]["seat_id"], json!(fixture.own.id));
    assert_eq!(data["actor"]["process_generation"], 1);
    assert_eq!(data["actor"]["role"]["class"], "worker");
    assert_eq!(data["actor"]["role"]["name"], "executor");
    assert_eq!(data["tasks"]["items"][0]["id"], json!(fixture.task));
    assert_eq!(data["tasks"]["items"][0]["verify"], "true");
    assert!(data["unread_count"].is_null());
    assert_eq!(data["pending_application_ack"], 0);
    let second = rpc(&mut bridge, 5, "tools/call", params);
    assert_eq!(first["result"], second["result"]);
    let audit = fixture.audits(fixture.own.id, 2);
    assert_eq!(audit[0].event.payload["tool"], "orient");
    assert_eq!(audit[0].event.generation, Some(1));
    assert_eq!(audit[1].event.payload["outcome"], "replay");
    let minted = rpc(&mut bridge, 6, "tools/call", json!({"name":"self_status"}));
    assert!(
        minted["result"]["structuredContent"]["op_id"]
            .as_str()
            .unwrap()
            .parse::<Ulid>()
            .is_ok()
    );
}

#[test]
fn mcp_role_scope_and_schema_cannot_be_spoofed_and_graph_reads_hide_foreign_items() {
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let list = fixture
        .runtime
        .block_on(fixture.client().control_catalogue(own, 1))
        .unwrap();
    let graph = list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "graph_read")
        .unwrap();
    assert_eq!(
        graph["inputSchema"]["properties"]["graph"]["enum"],
        json!(["task", "markdown"])
    );
    for (tool, args) in [
        ("seat_list", json!({"scope":"account"})),
        ("seat_get", json!({"seat_id":fixture.outside.id})),
        ("graph_read", json!({"graph":"execution"})),
        ("self_status", json!({"caller_role":"oracle"})),
        ("seat_input_text", json!({"text":"must never be typed"})),
        ("task_delegate", json!({})),
    ] {
        assert_eq!(fixture.call(own, Ulid::new(), tool, args)["isError"], true);
    }
    let peers = fixture.call(own, Ulid::new(), "seat_list", json!({}));
    assert_eq!(
        peers["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let peer = fixture.call(
        own,
        Ulid::new(),
        "seat_get",
        json!({"seat_id":fixture.peer.id}),
    );
    assert_eq!(peer["isError"], false);
    for graph in ["task", "markdown"] {
        let data = fixture.call(own, Ulid::new(), "graph_read", json!({"graph":graph}));
        let text = data.to_string();
        assert!(text.contains(&fixture.task.to_string()));
        assert!(!text.contains(&fixture.foreign_task.to_string()));
    }
    let all = fixture.call(
        fixture.oracle.id,
        Ulid::new(),
        "seat_list",
        json!({"scope":"account"}),
    );
    assert_eq!(
        all["structuredContent"]["items"].as_array().unwrap().len(),
        4
    );
    let execution = fixture.call(
        fixture.oracle.id,
        Ulid::new(),
        "graph_read",
        json!({"graph":"execution"}),
    );
    assert_eq!(execution["isError"], false);
    assert!(
        execution
            .to_string()
            .contains(&fixture.foreign_task.to_string())
    );
    let audit = fixture.audits(own, 6);
    assert_eq!(
        audit
            .iter()
            .filter(|e| e.event.payload["outcome"] == "rejected")
            .count(),
        6
    );
}

#[test]
fn mcp_journal_survives_logic_restart_and_never_replays_a_result_under_changed_authority() {
    let mut fixture = Fixture::new();
    let own = fixture.own.id;
    let op = Ulid::new();
    let first = fixture.call(own, op, "self_status", json!({}));
    fixture
        .registry()
        .update(own, |record| {
            record.name = "Changed".into();
            Ok(())
        })
        .unwrap();
    let repeated = fixture.call(own, op, "self_status", json!({}));
    assert_eq!(first, repeated);
    let store = SqliteStore::open(&fixture.config.run_dir.join(format!("seat-{own}"))).unwrap();
    let bytes = store.get(&format!("control/op/{op}")).unwrap().unwrap();
    let conflict = fixture.call(own, op, "seat_get", json!({"seat_id":own}));
    assert_eq!(conflict["structuredContent"]["error"]["code"], "conflict");
    assert_eq!(
        store.get(&format!("control/op/{op}")).unwrap().unwrap(),
        bytes
    );
    fixture.audits(own, 3);
    fixture.restart();
    assert_eq!(fixture.call(own, op, "self_status", json!({})), first);
    let oracle = fixture.oracle.id;
    let privileged = Ulid::new();
    assert_eq!(
        fixture.call(
            oracle,
            privileged,
            "graph_read",
            json!({"graph":"execution"})
        )["isError"],
        false
    );
    fixture
        .registry()
        .update(oracle, |record| {
            record.role.class = Class::Worker;
            record.role.name = "oracle".into();
            Ok(())
        })
        .unwrap();
    let old = fixture.call(
        oracle,
        privileged,
        "graph_read",
        json!({"graph":"execution"}),
    );
    assert_eq!(old["isError"], true);
    assert_eq!(old["structuredContent"]["error"]["code"], "conflict");
    assert_eq!(
        fixture.call(
            oracle,
            Ulid::new(),
            "graph_read",
            json!({"graph":"execution"})
        )["isError"],
        true
    );
}

#[test]
fn mcp_socket_refuses_bad_capabilities_and_generations_and_audit_does_not_store_arguments() {
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let endpoint = fixture
        .runtime
        .block_on(fixture.client().endpoint(own))
        .unwrap();
    for (capability, generation) in [([0; 32], 1), (endpoint.capability, 2)] {
        for action in [
            bbt_mux::seat_socket::Action::Catalogue,
            bbt_mux::seat_socket::Action::Call {
                call: Call {
                    op_id: Ulid::new(),
                    tool: "message_send".into(),
                    args: json!({"destination":format!("seats/{}/inbox",fixture.peer.id),"text":"must not execute"}),
                },
            },
        ] {
            let mut stream = UnixStream::connect(&endpoint.path).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let request = bbt_mux::seat_socket::Request {
                version: 1,
                capability,
                seat: own,
                generation,
                action,
            };
            let mut bytes = serde_json::to_vec(&request).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).unwrap();
            let mut out = Vec::new();
            let result = stream.read_to_end(&mut out);
            assert!(out.is_empty(), "unauthorized response bytes");
            assert!(
                matches!(result, Ok(0))
                    || result
                        .as_ref()
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset),
                "admission did not close the connection: {result:?}"
            );
        }
    }
    assert!(
        fixture
            .runtime
            .block_on(fixture.client().control_call(
                own,
                2,
                Call {
                    op_id: Ulid::new(),
                    tool: "orient".into(),
                    args: json!({})
                }
            ))
            .is_err()
    );
    for (tool, args) in [("bad\nname", json!({})), ("orient", json!([]))] {
        assert!(
            fixture
                .runtime
                .block_on(fixture.client().control_call(
                    own,
                    1,
                    Call {
                        op_id: Ulid::new(),
                        tool: tool.into(),
                        args
                    }
                ))
                .is_err()
        );
    }
    let marker = "DO_NOT_PERSIST_CONTROL_ARGUMENT_PAYLOAD";
    assert_eq!(
        fixture.call(own, Ulid::new(), "self_status", json!({"extra":marker}))["isError"],
        true
    );
    let audit = fixture.audits(own, 1);
    let encoded = serde_json::to_string(&audit).unwrap();
    assert!(!encoded.contains(marker));
    assert!(
        audit[0].event.payload["args_digest"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
    audit[0].verify().unwrap();
}

#[test]
fn mcp_unread_is_unknown_and_application_ack_is_a_separate_count() {
    use bbt_mux::messaging::{api, wire::SendRequest};
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let send = SendRequest::text(format!("seats/{own}/inbox"), "Synthetic MCP count fixture");
    let message = send.id;
    fixture
        .runtime
        .block_on(fixture.client().messaging(
            fixture.oracle.id,
            1,
            api::Request::Send { message: send },
        ))
        .unwrap();
    let first = fixture.call(own, Ulid::new(), "orient", json!({}));
    assert!(first["structuredContent"]["unread_count"].is_null());
    assert_eq!(first["structuredContent"]["pending_application_ack"], 1);
    fixture
        .runtime
        .block_on(fixture.client().messaging(
            own,
            1,
            api::Request::Acknowledge {
                message,
                agent: None,
            },
        ))
        .unwrap();
    let second = fixture.call(own, Ulid::new(), "orient", json!({}));
    assert!(second["structuredContent"]["unread_count"].is_null());
    assert_eq!(second["structuredContent"]["pending_application_ack"], 0);
}

#[test]
fn mcp_framing_is_bounded_and_protocol_errors_never_echo_input() {
    assert_eq!(
        read_frame(&mut Cursor::new(b"{}\nnext\n")).unwrap(),
        Some(b"{}".to_vec())
    );
    assert!(read_frame(&mut Cursor::new(b"truncated")).is_err());
    assert!(
        read_frame(&mut Cursor::new(vec![
            b'x';
            bbt_mux::mcp::bridge::MAX_FRAME + 1
        ]))
        .is_err()
    );
    let mut bridge = Bridge::new(BridgeConfig {
        socket: PathBuf::from("/does/not/exist"),
        seat: Ulid::new(),
        generation: 1,
        capability: [0; 32],
    });
    let secret = b"secret input that is not JSON";
    let error = bridge.handle(secret).unwrap();
    assert_eq!(error["error"]["code"], -32700);
    assert!(!error.to_string().contains("secret input"));
    assert_eq!(bridge.handle(b"[]").unwrap()["error"]["code"], -32600);
    initialize(&mut bridge);
    assert_eq!(
        rpc(
            &mut bridge,
            4,
            "tools/call",
            json!({"name":"orient","_meta":{"bboxy450/op_id":"invalid"}})
        )["error"]["code"],
        -32602
    );
    let unavailable = rpc(&mut bridge, 5, "tools/call", json!({"name":"orient"}));
    assert_eq!(unavailable["error"]["code"], -32001);
    assert_eq!(unavailable["error"]["data"]["status"], "unknown");
    assert_ne!(
        unavailable["error"]["data"]["op_id"]
            .as_str()
            .unwrap()
            .parse::<Ulid>()
            .unwrap(),
        Ulid::nil()
    );
    let original = Ulid::new();
    assert_eq!(
        rpc(
            &mut bridge,
            6,
            "tools/call",
            json!({"name":"orient", "_meta":{"bboxy450/op_id":original}})
        )["error"]["data"]["op_id"],
        original.to_string()
    );
    for args in [
        json!({"name":"bad\nname"}),
        json!({"name":"orient","arguments":[]}),
    ] {
        assert_eq!(
            rpc(&mut bridge, 7, "tools/call", args)["error"]["code"],
            -32602
        );
    }
}

/// Runs only by explicit request: calls the installed, authenticated clients
/// against synthetic seat state. Output is parsed in memory, never exported.
#[test]
#[ignore = "live installed-client interop; BBOXY_TEST_EXE must name the built app"]
fn mcp_installed_claude_and_codex_call_orient() {
    use std::process::{Command, Stdio};
    let fixture = Fixture::new();
    let exe = std::env::var("BBOXY_TEST_EXE").expect("set BBOXY_TEST_EXE to the built app");
    let cwd = fixture._temp.path().join("client-repo");
    std::fs::create_dir(&cwd).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&cwd)
            .status()
            .unwrap()
            .success()
    );
    let mcp_config = cwd.join("mcp-fixture.json");
    std::fs::write(
        &mcp_config,
        json!({"mcpServers":{"bboxy":{"command":exe,"args":["mcp"]}}}).to_string(),
    )
    .unwrap();
    let before = configuration_digests();
    let prompt = "This is a synthetic MCP interop fixture. Call the bboxy orient tool exactly once, then say the role name and class it returns. Use no other tools, read no files, and change no settings.";
    for (program, seat) in [("claude", fixture.own.id), ("codex", fixture.peer.id)] {
        let endpoint = fixture
            .runtime
            .block_on(fixture.client().endpoint(seat))
            .unwrap();
        let mut command = Command::new(program);
        command.current_dir(&cwd).stdin(Stdio::null());
        for name in [
            "CLAUDECODE",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_SESSION_ID",
            "CODEX_THREAD_ID",
            "CODEX_SESSION_ID",
            "CODEX_TURN_ID",
            "ICQ_SESSION_ID",
            "ICQ_SESSION_KEY",
        ] {
            command.env_remove(name);
        }
        command
            .env("BBOXY_SOCK", &endpoint.path)
            .env("BBOXY_SEAT_ID", seat.to_string())
            .env("BBOXY_GENERATION", endpoint.generation.to_string())
            .env(
                "BBOXY_CAPABILITY",
                endpoint
                    .capability
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            );
        if program == "claude" {
            command
                .args([
                    "-p",
                    prompt,
                    "--no-session-persistence",
                    "--tools",
                    "",
                    "--allowedTools",
                    "mcp__bboxy__orient",
                    "--permission-prompts",
                    "none",
                    "--output-format",
                    "json",
                    "--mcp-config",
                ])
                .arg(&mcp_config);
        } else {
            command.args(["exec","--ephemeral","--json","-s","read-only","-c","approval_policy=\"never\"","-c"])
                .arg(format!("mcp_servers.bboxy.command={}",json!(exe)))
                .args(["-c","mcp_servers.bboxy.args=[\"mcp\"]","-c","mcp_servers.bboxy.env_vars=[\"BBOXY_SOCK\",\"BBOXY_CAPABILITY\",\"BBOXY_SEAT_ID\",\"BBOXY_GENERATION\"]","-c","mcp_servers.bboxy.enabled_tools=[\"orient\"]"])
                .arg(prompt);
        }
        let (status, out, err) = bounded_client(command, Duration::from_secs(120));
        let diagnostics = String::from_utf8_lossy(&err).to_ascii_lowercase();
        assert!(
            status.success(),
            "{program} exit={status}; stdout_bytes={} stderr_bytes={} trust_text={} config_text={}",
            out.len(),
            err.len(),
            diagnostics.contains("trust"),
            diagnostics.contains("config")
        );
        let role_spoken = if program == "claude" {
            let result: Value = serde_json::from_slice(&out).expect("Claude JSON result");
            result["result"]
                .as_str()
                .is_some_and(|s| s.to_ascii_lowercase().contains("executor"))
        } else {
            out.split(|b| *b == b'\n')
                .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
                .any(|event| {
                    event["type"] == "item.completed"
                        && event["item"]["type"] == "agent_message"
                        && event["item"]["text"]
                            .as_str()
                            .is_some_and(|s| s.to_ascii_lowercase().contains("executor"))
                })
        };
        assert!(
            role_spoken,
            "{program} final output did not report the returned role"
        );
        let audits = fixture.audits(seat, 1);
        let orient = audits
            .iter()
            .find(|event| event.event.payload["tool"] == "orient")
            .unwrap();
        assert_eq!(orient.event.seat, Some(seat));
        assert_eq!(orient.event.generation, Some(1));
        assert_eq!(orient.event.payload["actor"]["role"]["class"], "worker");
        orient.verify().unwrap();
        println!(
            "client={program} exit=0 tool=orient seat={seat} generation=1 role=executor class=worker audit={} seq={} final_role_verified=true stdout_bytes={} stderr_bytes={} (raw outputs discarded)",
            orient.event.id,
            orient.seq,
            out.len(),
            err.len()
        );
    }
    assert_eq!(
        configuration_digests(),
        before,
        "named user config files changed during interop"
    );
    println!(
        "named Claude/Codex settings/config/hooks hashes unchanged; no secret/config values exported"
    );
}

fn bounded_client(
    mut command: std::process::Command,
    timeout: Duration,
) -> (std::process::ExitStatus, Vec<u8>, Vec<u8>) {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn().expect("start installed client");
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(1024 * 1024).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(256 * 1024).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > timeout {
            // SAFETY: this is the newly-created disposable client's own group.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            let status = child.wait().unwrap();
            let out = out.join().unwrap();
            let err = err.join().unwrap();
            panic!(
                "client timeout, status={status}, stdout_bytes={}, stderr_bytes={}; raw outputs discarded",
                out.len(),
                err.len()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // End only descendants of the disposable process group before joining its
    // pipe readers; an MCP helper must not keep a completed fixture open.
    // SAFETY: this is the newly-created disposable client's own group.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    (status, out.join().unwrap(), err.join().unwrap())
}

fn configuration_digests() -> std::collections::BTreeMap<PathBuf, Option<String>> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    [
        ".claude/settings.json",
        ".claude/settings.local.json",
        ".codex/config.toml",
        ".codex/hooks.json",
    ]
    .into_iter()
    .map(|path| {
        let path = home.join(path);
        let digest = match std::fs::read(&path) {
            Ok(bytes) => Some(bbt_mux::store::digest(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("cannot hash named config: {error}"),
        };
        (path, digest)
    })
    .collect()
}

#[test]
fn mcp_messages_bind_the_sender_retry_once_reply_list_and_ack_through_w24() {
    let mut f = Fixture::new();
    let own = f.own.id;
    let peer = f.peer.id;
    let destination = format!("seats/{peer}/inbox");
    let op = Ulid::new();
    let args = json!({"destination":destination,"text":"private fixture payload","priority":9,"attachments":["[[fixture-artifact]]"]});
    let first = f.mutate(own, op, "message_send", args.clone());
    let sent = data(first.clone());
    assert_eq!(sent["status"], "accepted");
    assert_eq!(sent["kind"], "stored");
    let envelope = &sent["message"]["envelope"];
    assert_eq!(envelope["id"], op.to_string());
    assert_eq!(envelope["seat"], own.to_string());
    assert_eq!(envelope["role"]["class"], "worker");
    assert_eq!(envelope["priority_class"], "worker");
    assert_eq!(f.mutate(own, op, "message_send", args.clone()), first);
    let changed = f.mutate(
        own,
        op,
        "message_send",
        json!({"destination":destination,"text":"changed"}),
    );
    assert_eq!(changed["structuredContent"]["error"]["code"], "conflict");
    f.restart();
    assert_eq!(f.mutate(own, op, "message_send", args), first);
    let inbox = data(f.call(peer, Ulid::new(), "message_list", json!({})));
    assert_eq!(inbox["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(inbox["page"]["items"][0]["envelope"]["id"], op.to_string());
    let receipts = data(f.call(own, Ulid::new(), "message_receipts", json!({"message":op})));
    assert_ne!(
        receipts["page"]["items"][0]["evidence"],
        json!(bbt_mux::adapters::Evidence::ApplicationAcknowledged)
    );
    assert_eq!(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "message_ack",
            json!({"message":op})
        )["isError"],
        true
    );
    let acknowledged = data(f.mutate(peer, Ulid::new(), "message_ack", json!({"message":op})));
    assert_eq!(acknowledged["status"], "applied");
    let receipts = data(f.call(own, Ulid::new(), "message_receipts", json!({"message":op})));
    assert_eq!(
        receipts["page"]["items"][0]["evidence"],
        json!(bbt_mux::adapters::Evidence::ApplicationAcknowledged)
    );
    let reply = data(f.mutate(
        peer,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{own}/inbox"),"text":"reply","reply_to":op}),
    ));
    assert_eq!(
        reply["message"]["envelope"]["thread_id"],
        envelope["thread_id"]
    );
    let thread = data(f.call(
        own,
        Ulid::new(),
        "message_list",
        json!({"thread":envelope["thread_id"],"limit":1}),
    ));
    assert_eq!(thread["page"]["items"].as_array().unwrap().len(), 1);
    let next = thread["page"]["next"].as_u64().unwrap();
    let second = data(f.call(
        own,
        Ulid::new(),
        "message_list",
        json!({"thread":envelope["thread_id"],"after":next,"limit":1}),
    ));
    assert_eq!(second["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        second["page"]["items"][0]["envelope"]["body"]["text"],
        "reply"
    );
    let audits = f.audits(own, 7);
    let serialized = serde_json::to_string(&audits).unwrap();
    assert!(!serialized.contains("private fixture payload"));
    assert!(!serialized.contains("fixture-artifact"));
    assert!(
        audits
            .iter()
            .any(|event| event.event.payload["outcome"] == "admitted")
    );
    assert!(
        audits
            .iter()
            .any(|event| event.event.payload["outcome"] == "replay")
    );
}

#[test]
fn mcp_channel_tools_keep_worker_links_and_recheck_revoked_reads_before_cached_results() {
    let f = Fixture::new();
    let own = f.own.id;
    let oracle = f.oracle.id;
    let folder = "channels/fixture";
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":folder,"kind":"folder"}}),
    ));
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":["read","write","link"]}),
    ));
    let path = format!("{folder}/private-node-marker");
    data(f.mutate(own,Ulid::new(),"ch_subscribe",json!({"folder":folder,"name":"private-node-marker","filter":{"kinds":["chunk"]},"rate":{"batch_ms":100,"coalesce":true}})));
    let read_op = Ulid::new();
    let before = data(f.call(own, read_op, "ch_ls", json!({"path":folder})));
    let node = &before["nodes"][0];
    assert_eq!(node["path"], path);
    assert_eq!(node["target"], format!("seats/{own}/inbox"));
    assert_eq!(node["filter"]["kinds"], json!(["chunk"]));
    assert_eq!(node["filter"]["classes"], json!([]));
    assert_eq!(node["rate"]["batch_ms"], 100);
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_pause",
        json!({"path":path,"paused":true}),
    ));
    assert_eq!(
        data(f.call(own, Ulid::new(), "ch_ls", json!({"path":folder})))["nodes"][0]["paused"],
        true
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_pause",
        json!({"path":path,"paused":false}),
    ));
    let worker = f
        .runtime
        .block_on(f.client().control_catalogue(own, 1))
        .unwrap();
    let admin = f
        .runtime
        .block_on(f.client().control_catalogue(oracle, 1))
        .unwrap();
    assert_eq!(worker["tools"].as_array().unwrap().len(), 12);
    assert_eq!(admin["tools"].as_array().unwrap().len(), 15);
    for tool in [
        "ch_link",
        "ch_pause",
        "ch_grant",
        "seat_input_text",
        "task_delegate",
        "seat_mute",
        "seat_unmute",
    ] {
        assert!(
            !worker["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["name"] == tool)
        );
        assert_eq!(f.call(own, Ulid::new(), tool, json!({}))["isError"], true);
    }
    let others = format!("swarms/{}/{}", f.own.swarm, f.peer.id);
    assert_eq!(
        f.mutate(own, Ulid::new(), "ch_unsubscribe", json!({"path":others}))["isError"],
        true
    );
    let invalid = f.mutate(
        own,
        Ulid::new(),
        "ch_subscribe",
        json!({"folder":folder,"name":"../escape"}),
    );
    assert_eq!(invalid["isError"], true);
    let epoch =
        data(f.call(own, Ulid::new(), "self_status", json!({})))["actor"]["grant_epoch"].clone();
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":[]}),
    ));
    assert_eq!(
        data(f.call(own, Ulid::new(), "self_status", json!({})))["actor"]["grant_epoch"],
        epoch
    );
    let denied = f.call(own, read_op, "ch_ls", json!({"path":folder}));
    assert_eq!(denied["isError"], true);
    assert!(
        !denied.to_string().contains("private-node-marker"),
        "cached authorized read escaped current grants"
    );
    assert_eq!(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":format!("{folder}/*"),"text":"revoked","kind":"chunk"})
        )["isError"],
        true
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":["read","write","link"]}),
    ));
    data(f.mutate(own, Ulid::new(), "ch_unsubscribe", json!({"path":path})));
    assert!(
        data(f.call(own, Ulid::new(), "ch_ls", json!({"path":folder})))["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mcp_message_visibility_and_receipt_pages_preserve_current_w24_scope() {
    let f = Fixture::new();
    let own = f.own.id;
    let peer = f.peer.id;
    let oracle = f.oracle.id;
    let private = data(f.mutate(
        oracle,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{peer}/inbox"),"text":"third party private payload"}),
    ));
    let private_id = private["message"]["envelope"]["id"].clone();
    assert!(
        data(f.call(own, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.call(
            own,
            Ulid::new(),
            "message_receipts",
            json!({"message":private_id})
        )["isError"],
        true
    );
    assert_eq!(
        f.call(
            f.outside.id,
            Ulid::new(),
            "message_receipts",
            json!({"message":private_id})
        )["isError"],
        true
    );
    let admin_page = data(f.call(
        oracle,
        Ulid::new(),
        "message_list",
        json!({"channel":format!("seats/{peer}/inbox")}),
    ));
    assert_eq!(admin_page["page"]["items"].as_array().unwrap().len(), 1);
    let broadcast = data(f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("swarms/{}/*",f.own.swarm),"kind":"chunk","text":"chunk"}),
    ));
    let broadcast_id = broadcast["message"]["envelope"]["id"].clone();
    let recipient = data(f.call(
        peer,
        Ulid::new(),
        "message_receipts",
        json!({"message":broadcast_id}),
    ));
    assert_eq!(recipient["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(recipient["page"]["items"][0]["recipient"], peer.to_string());
    let mut cursor = 0;
    let mut count = 0;
    loop {
        let page = data(f.call(
            own,
            Ulid::new(),
            "message_receipts",
            json!({"message":broadcast_id,"after":cursor,"limit":1}),
        ));
        count += page["page"]["items"].as_array().unwrap().len();
        if let Some(next) = page["page"]["next"].as_u64() {
            assert!(next > cursor);
            cursor = next;
        } else {
            break;
        }
    }
    assert_eq!(count, 3);
    assert_eq!(
        data(f.call(
            oracle,
            Ulid::new(),
            "message_receipts",
            json!({"message":broadcast_id})
        ))["page"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":format!("seats/{peer}/inbox"),"seat":own,"rights":[]}),
    ));
    let denied = f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{peer}/inbox"),"text":"explicitly revoked DM"}),
    );
    assert_eq!(denied["isError"], true);
    assert!(denied.to_string().contains("write"));
    // Even a controller-created foreign sink in this tree cannot bypass the
    // registry's swarm fence at the W2.4 send boundary.
    let foreign_folder = format!("seats/{}", f.outside.id);
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":foreign_folder,"kind":"folder"}}),
    ));
    let foreign_sink = format!("{foreign_folder}/inbox");
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":foreign_sink,"kind":"sink"}}),
    ));
    let foreign = f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":foreign_sink,"text":"no cross-swarm delivery"}),
    );
    assert_eq!(foreign["isError"], true);
    assert!(foreign.to_string().contains("swarm"));
    assert!(
        data(f.call(f.outside.id, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.mutate(
            oracle,
            Ulid::new(),
            "ch_grant",
            json!({"path":format!("swarms/{}",f.own.swarm),"seat":f.outside.id,"rights":["write"]})
        )["isError"],
        true
    );
}

#[test]
fn mcp_mutation_argument_ids_are_visible_retryable_and_cannot_spoof_authority() {
    let f = Fixture::new();
    let own = f.own.id;
    let mut bridge = f.bridge(own);
    initialize(&mut bridge);
    let args = json!({"destination":format!("seats/{}/inbox",f.peer.id),"text":"fixture"});
    let missing = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"message_send","arguments":args}),
    );
    assert_eq!(missing["error"]["code"], -32602);
    let op = Ulid::new();
    for invalid_id in [json!("not-a-ulid"), json!(Ulid::nil())] {
        let mut invalid = args.clone();
        invalid["op_id"] = invalid_id;
        assert_eq!(
            rpc(
                &mut bridge,
                2,
                "tools/call",
                json!({"name":"message_send","arguments":invalid})
            )["error"]["code"],
            -32602
        );
    }
    let mut original = args.clone();
    original["op_id"] = json!(op);
    let first = rpc(
        &mut bridge,
        3,
        "tools/call",
        json!({"name":"message_send","arguments":original}),
    );
    assert_eq!(first["result"]["isError"], false);
    let compatible = rpc(
        &mut bridge,
        4,
        "tools/call",
        json!({"name":"message_send","arguments":args,"_meta":{"bboxy450/op_id":op}}),
    );
    assert_eq!(compatible["result"], first["result"]);
    let mismatch = rpc(
        &mut bridge,
        5,
        "tools/call",
        json!({"name":"message_send","arguments":original,"_meta":{"bboxy450/op_id":Ulid::new()}}),
    );
    assert_eq!(mismatch["error"]["code"], -32602);
    assert!(
        mismatch["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mismatch")
    );
    for (field, value) in [
        ("sender", json!(f.oracle.id)),
        ("role", json!("oracle")),
        ("operator", json!(true)),
        ("swarm_id", json!(f.outside.swarm)),
        ("process_generation", json!(2)),
    ] {
        let mut forged = args.clone();
        forged[field] = value;
        assert_eq!(
            f.mutate(own, Ulid::new(), "message_send", forged)["isError"],
            true
        );
    }
    for extra in [
        json!({"kind":"task"}),
        json!({"kind":"presence"}),
        json!({"text":"x".repeat(16385)}),
        json!({"text":"\u{1b}[escape"}),
    ] {
        let mut invalid = args.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            f.mutate(own, Ulid::new(), "message_send", invalid)["isError"],
            true
        );
    }
    assert_eq!(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":format!("swarms/{}/*",f.own.swarm),"text":"plain progress"})
        )["isError"],
        true
    );
    for (tool, args) in [
        ("message_list", json!({"limit":129})),
        ("message_receipts", json!({"message":op,"limit":0})),
        ("ch_ls", json!({"path":"../escape"})),
    ] {
        assert_eq!(f.call(own, Ulid::new(), tool, args)["isError"], true);
    }
    assert_eq!(
        data(f.call(own, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
