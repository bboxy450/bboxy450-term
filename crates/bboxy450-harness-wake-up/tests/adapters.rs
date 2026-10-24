#![allow(clippy::disallowed_methods)] // Disposable protocol fixtures, never UI.
use bboxy450_harness_wake_up::{
    Adapter, ArbiterView, Binding, Evidence, HarnessRecord, OscTap, Priority, Readiness, Session,
    State, Ulid, hook,
    launch::{LaunchContext, plan},
    native,
};
use serde_json::json;
fn binding() -> Binding {
    Binding {
        seat: Ulid::new(),
        session: "fixture-session".into(),
        pid: std::process::id(),
        process_start: native::process_start(std::process::id()).unwrap(),
        generation: 1,
        attachment_generation: 1,
    }
}
fn session(kind: &str) -> Session {
    Session::new(HarnessRecord::builtin(kind).unwrap(), binding()).unwrap()
}
fn arbiter(session: &Session) -> ArbiterView {
    ArbiterView {
        binding: session.binding.clone(),
        operator_epoch: 0,
        foreground_pid: Some(session.binding.pid),
        operator_draft: false,
        active_tool: false,
        sync_pending: false,
        screen_switch_pending: false,
        canonical: true,
        echo: true,
        bracketed_paste: true,
    }
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn adapters_claude_wire_capture_runs_through_term_callback_and_ends_only_running_rows() {
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    let mut session = session("claude");
    let bytes = include_bytes!("fixtures/osc7501-claude.bin");
    let mut sequence = 0;
    let mut saw_working = false;
    let mut saw_clear = false;
    let mut saw_done = false;
    for chunk in bytes.chunks(7) {
        sequence += 1;
        term.advance(chunk);
        for params in tap.drain().unwrap() {
            for obs in session.on_osc(&params, sequence, sequence * 10) {
                if obs.agent_id.as_deref() == Some("a9d4d919158900674") {
                    if obs.state == State::Active {
                        saw_working = true;
                        assert_eq!(obs.title.as_deref(), Some("OSC7501 child fixture"));
                        assert_eq!(session.status.subagents.len(), 1);
                    }
                    if obs.clear {
                        assert!(saw_working);
                        saw_clear = true;
                        assert!(session.status.subagents.is_empty());
                    }
                } else if obs.state == State::Idle && saw_clear && !obs.clear {
                    saw_done = true;
                }
            }
        }
    }
    assert!(saw_working && saw_clear && saw_done);
    assert_eq!(session.take_replies(), vec![b"\x1b]7501;?\x07".to_vec()]);
    assert!(
        matches!(
            session.input_guard(&arbiter(&session), 2000),
            Readiness::NotReady(_)
        ),
        "wire done never grants composer input"
    );
}

#[test]
fn adapters_codex_native_readiness_and_hooks_only_subagents() {
    let mut session = session("codex");
    let thread = session.binding.session.clone();
    session.on_osc(
        &[
            b"7501".to_vec(),
            b"state=working:id=fake:title=YWdlbnQ=".to_vec(),
        ],
        1,
        100,
    );
    assert!(session.status.subagents.is_empty());
    session.on_native(&json!({"method":"thread/status/changed","params":{"threadId":thread,"status":{"type":"active","activeFlags":["waitingOnApproval"]}}}),2,200).unwrap();
    assert!(matches!(
        session.input_guard(&arbiter(&session), 1000),
        Readiness::NotReady(_)
    ));
    session
        .on_native(
            &json!({"method":"serverRequest/resolved","params":{"threadId":thread}}),
            3,
            1100,
        )
        .unwrap();
    session.on_native(&json!({"method":"thread/status/changed","params":{"threadId":thread,"status":{"type":"idle"}}}),4,1200).unwrap();
    assert!(matches!(
        session.input_guard(&arbiter(&session), 1700),
        Readiness::Ready(_)
    ));
    let raw = hook::prepare(
        json!({"hook_event_name":"SubagentStart","agent_id":"00000000-0000-4000-8000-000000000001","agent_type":"Explore"}),
        "codex",
    )
    .unwrap();
    let obs = hook::project(&mut session, &raw, 5, 1800).unwrap();
    let frame = hook::osc(session.binding.seat, &raw).unwrap().unwrap();
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    term.advance(&frame);
    for params in tap.drain().unwrap() {
        session.on_osc(&params, 5, 1800);
    }
    assert_eq!(
        obs.agent_id.as_deref(),
        Some("00000000-0000-4000-8000-000000000001")
    );
    assert_eq!(session.status.subagents.len(), 1);
    assert!(
        hook::project(
            &mut session,
            &hook::prepare(
                json!({"hook_event_name":"SubagentStart","session_id":"parent-only"}),
                "codex"
            )
            .unwrap(),
            5,
            1800
        )
        .is_none()
    );
    assert!(
        session
            .on_native(
                &json!({"method":"turn/started","params":{"threadId":"wrong"}}),
                6,
                1900
            )
            .is_err()
    );
}

#[test]
fn adapters_launch_overlays_leave_user_config_unchanged_and_keep_raw_hook_fields() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let user = temp.path().join("user");
    std::fs::create_dir(&user).unwrap();
    let settings = user.join("settings.json");
    std::fs::write(&settings, b"operator-config").unwrap();
    let codex_home = user.join(".codex");
    std::fs::create_dir(&codex_home).unwrap();
    let config = codex_home.join("config.toml");
    let original = b"check_for_update_on_startup = true\n[mcp_servers.other]\ndefault_tools_approval_mode = \"prompt\"\n";
    std::fs::write(&config, original).unwrap();
    let id = Ulid::new();
    let context = LaunchContext {
        id,
        generation: 1,
        name: "fixture",
        role_name: "executor",
        harness_session_id: None,
        skill: "caller-owned skill content",
        cwd: temp.path(),
        role_prompt: "ROLE: EXECUTOR\nDo exactly this.",
        executable: std::path::Path::new("/tmp/our app/bin/bboxy450-term"),
        overlay_parent: temp.path(),
        user_home: &user,
        user_zdotdir: None,
        socket: std::path::Path::new("/tmp/fixture.sock"),
        capability: [7; 32],
        resume: false,
    };
    let plan = plan(&HarnessRecord::builtin("claude").unwrap(), &context).unwrap();
    let bytes = std::fs::read(plan.overlay.join("claude-settings.json")).unwrap();
    let overlay: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        overlay["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("'/tmp/our app/bin/bboxy450-term' hook")
    );
    assert_eq!(
        std::fs::metadata(plan.overlay.join("claude-settings.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(plan.args.contains(&"--append-system-prompt".into()));
    assert!(!plan.args.iter().any(|s| s.contains("bypass")));
    assert_eq!(std::fs::read(settings).unwrap(), b"operator-config");
    let mut codex_record = HarnessRecord::builtin("codex").unwrap();
    let other_approval = "mcp_servers.other.default_tools_approval_mode=\"prompt\"";
    codex_record
        .command
        .extend(["-c".into(), other_approval.into()]);
    let codex = bboxy450_harness_wake_up::launch::plan(&codex_record, &context).unwrap();
    let approval_overrides = codex
        .args
        .windows(2)
        .filter(|pair| {
            pair[0] == "-c"
                && (pair[1].contains("approval")
                    || pair[1].starts_with("check_for_update_on_startup="))
        })
        .map(|pair| pair[1].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        approval_overrides,
        [
            other_approval,
            "mcp_servers.bboxy.default_tools_approval_mode=\"auto\"",
            "check_for_update_on_startup=false"
        ]
    );
    assert_eq!(std::fs::read(config).unwrap(), original);
    let hooks = codex.args.iter().find(|s| s.starts_with("hooks=")).unwrap();
    assert!(hooks.contains("SessionStart="));
    assert!(hooks.contains("SubagentStart="));
    assert!(!hooks.contains("session_start="));
    assert!(
        !codex
            .args
            .iter()
            // Instruction prose can say "never bypass"; it is not a CLI switch.
            .filter(|a| !a.starts_with("developer_instructions="))
            .any(|a| a.contains("bypass") || a.contains("trusted_hash"))
    );
    codex.cleanup().unwrap();
    let raw = json!({"hook_event_name":"PreToolUse","tool_input":{"description":"x:y\nraw value"},"tool_use_id":"join-me","token":"do-not-export"});
    let event = hook::prepare(raw.clone(), "claude").unwrap();
    assert_eq!(event.raw["tool_input"], raw["tool_input"]);
    assert!(event.raw.get("token").is_none());
    let large = hook::prepare(
        json!({"hook_event_name":"Stop","last_message":"x".repeat(5000)}),
        "claude",
    )
    .unwrap();
    assert!(hook::osc(id, &large).unwrap().is_none());
    plan.cleanup().unwrap();
}

#[test]
fn adapters_native_claude_socket_writes_priority_without_claiming_consumption() {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("native.sock");
    runtime().block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut probe = String::new();
            assert_eq!(
                BufReader::new(stream).read_line(&mut probe).await.unwrap(),
                0
            );
            let (stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).await.unwrap();
            serde_json::from_str::<serde_json::Value>(&line).unwrap()
        });
        let id = Ulid::new();
        let mut adapter = session("claude");
        let attachment = adapter
            .attach(bboxy450_harness_wake_up::Endpoint::Claude { path: path.clone() })
            .await
            .unwrap();
        let outcome = adapter
            .wake(&attachment, id, "literal peer text", Priority::Later, false)
            .await;
        let frame = server.await.unwrap();
        assert_eq!(frame["priority"], "later");
        assert_eq!(frame["msg_id"], id.to_string());
        assert_eq!(frame["message"]["content"], "literal peer text");
        assert_eq!(outcome.evidence, Evidence::NativeWritten);
        let mut binding = adapter.binding.clone();
        binding.generation += 1;
        adapter.rebind(binding).unwrap();
        assert_eq!(
            adapter
                .wake(&attachment, Ulid::new(), "stale", Priority::Next, false)
                .await
                .evidence,
            Evidence::RecipientStored
        );
    });
}

#[test]
fn adapters_codex_rpc_admits_exact_message_and_preserves_unknown_on_disconnect() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    for (disconnect, priority, malformed) in [
        (false, Priority::Next, false),
        (true, Priority::Next, false),
        (false, Priority::Now, false),
        (false, Priority::Next, true),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("native.sock");
        let binding = binding();
        let thread = binding.session.clone();
        runtime().block_on(async {
            let listener=tokio::net::UnixListener::bind(&path).unwrap();
            let server=tokio::spawn(async move {
                let (stream,_)=listener.accept().await.unwrap();let mut ws=tokio_tungstenite::accept_async(stream).await.unwrap();
                let mut mutation=None;
                let method=if priority==Priority::Now{"turn/steer"}else{"thread/queue/add"};
                for expected in ["initialize","initialized","thread/read",method] {
                    let frame=ws.next().await.unwrap().unwrap();let value:serde_json::Value=serde_json::from_str(frame.to_text().unwrap()).unwrap();assert_eq!(value["method"],expected);
                    if expected=="initialized"{continue;}
                    let result=if expected=="thread/read" {json!({"thread":{"id":thread,"status":{"type":"idle"}}})}
                        else if expected==method {mutation=Some(value.clone());if disconnect{break;}
                            if malformed {ws.send(Message::Text(json!({"id":value["id"],"error":null}).to_string().into())).await.unwrap();break;}
                            if priority==Priority::Now {assert_eq!(value["params"]["expectedTurnId"],"turn1");json!({"turnId":"turn1"})}
                            else {let mut input=value["params"]["input"].clone();input[0]["text_elements"]=json!([]);json!({"queuedSubmission":{"id":"q1","clientUserMessageId":value["params"]["clientUserMessageId"],"input":input}})}
                        }
                        else {json!({})};
                    ws.send(Message::Text(json!({"id":value["id"],"result":result}).to_string().into())).await.unwrap();
                }
                mutation.unwrap()
            });
            let endpoint=bboxy450_harness_wake_up::Endpoint::Codex{path:path.clone(),thread:binding.session.clone(),turn:Some("turn1".into())};
            if priority==Priority::Now {assert_eq!(native::wake(&binding,&endpoint,Ulid::new(),"not-authorized",priority,false).await.evidence,Evidence::RecipientStored);}
            let outcome=native::wake(&binding,&endpoint,Ulid::new(),"fixture",priority,true).await;
            assert_eq!(server.await.unwrap()["params"]["input"][0]["text"],"fixture");
            assert_eq!(outcome.evidence,if disconnect||malformed{Evidence::NativeUnknown}else{Evidence::NativeQueued});
        });
    }
}

#[test]
fn adapters_custom_harness_cadence_and_reported_idle_never_create_readiness() {
    let mut record = HarnessRecord::builtin("zsh").unwrap();
    record.kind = "user-defined-harness".into();
    record.state_source = bboxy450_harness_wake_up::StateSource::Cadence;
    let mut session = Session::new(record, binding()).unwrap();
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    term.advance(include_bytes!("fixtures/codex-session.txt"));
    for params in tap.drain().unwrap() {
        session.on_osc(&params, 1, 1000);
    }
    session.output(1000);
    session.tick(1, 1000);
    assert_eq!(session.status.state, State::Active);
    session.tick(1, 4000);
    assert_eq!(session.status.state, State::Idle);
    assert_eq!(
        session.status.grade,
        bboxy450_harness_wake_up::Grade::Inferred
    );
    session.on_osc(
        &[b"7501".to_vec(), b"state=idle:app=custom".to_vec()],
        2,
        4500,
    );
    assert_eq!(
        session.status.grade,
        bboxy450_harness_wake_up::Grade::Reported
    );
    assert!(matches!(
        session.input_guard(&arbiter(&session), 5000),
        Readiness::OperatorHandoff
    ));
}

#[test]
fn literal_preparation_consumes_readiness_and_unknown_never_retries() {
    use bboxy450_harness_wake_up::{LiteralInput, WakeOutcome};
    for bracketed in [false, true] {
        for submit in [false, true] {
            let mut session = session("claude");
            let mut view = arbiter(&session);
            view.bracketed_paste = bracketed;
            assert!(matches!(
                session.input_guard(&view, 1600),
                Readiness::NotReady(_)
            ));
            let file = native::ClaudeSessionFile {
                pid: session.binding.pid,
                session_id: session.binding.session.clone(),
                process_start: session.binding.process_start.clone(),
                status: "idle".into(),
                waiting_for: None,
                messaging_socket_path: None,
            };
            session.observe_claude_session(&file, 5, 1000).unwrap();
            let Readiness::Ready(token) = session.input_guard(&view, 1600) else {
                panic!("positive proof missing")
            };
            let id = Ulid::new();
            let bytes = session
                .prepare_literal(
                    token,
                    &view,
                    1600,
                    LiteralInput {
                        id,
                        text: "literal",
                        submit,
                    },
                )
                .unwrap();
            let mut expected = if bracketed {
                b"\x1b[200~literal\x1b[201~".to_vec()
            } else {
                b"literal".to_vec()
            };
            if submit {
                expected.push(b'\r');
            }
            assert_eq!(bytes, expected);
            assert!(matches!(
                session.input_guard(&view, 1601),
                Readiness::NotReady(_)
            ));
            session.record_outcome(id, &WakeOutcome::unknown("write ambiguous"));
            session.observe_claude_session(&file, 6, 1700).unwrap();
            assert!(!session.can_attempt_native(id));
            let Readiness::Ready(token) = session.input_guard(&view, 1800) else {
                panic!("fresh positive proof missing")
            };
            assert!(
                session
                    .prepare_literal(
                        token,
                        &view,
                        1800,
                        LiteralInput {
                            id,
                            text: "literal",
                            submit
                        }
                    )
                    .is_err()
            );
        }
    }
}
