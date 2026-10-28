// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
        runtime: None,
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

// keeps: spec harness-kits §5
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

// keeps: spec harness-kits §5
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

// keeps: spec harness-kits §5
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
    let prompt = &plan
        .args
        .windows(2)
        .find(|pair| pair[0] == "--append-system-prompt")
        .unwrap()[1];
    assert!(prompt.contains("NAME: \"fixture\""));
    assert!(prompt.contains("ROLE: EXECUTOR"));
    assert!(prompt.contains("First call the bboxy MCP `orient` tool, before other work"));
    assert!(prompt.contains(&serde_json::to_string(temp.path()).unwrap()));
    assert!(plan.args.iter().any(|arg| arg == "--dangerously-skip-permissions"));
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
            "approval_policy=\"never\"",
            "mcp_servers.bboxy.default_tools_approval_mode=\"approve\"",
            "check_for_update_on_startup=false"
        ]
    );
    assert_eq!(std::fs::read(config).unwrap(), original);
    assert!(codex.args.iter().any(|arg| arg == "--yolo"));
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
    let proxy = codex.runtime.as_ref().unwrap();
    assert_eq!(proxy.program, context.executable.to_string_lossy());
    assert!(proxy.args.contains(&"--codex-proxy".into()));
    assert!(!proxy.args.contains(&"app-server".into()));
    assert!(
        codex
            .shared_runtime
            .as_ref()
            .unwrap()
            .ends_with("app-server-control/app-server-control.sock")
    );
    let url = format!("unix://{}", proxy.endpoint.display());
    assert!(codex.args.windows(2).any(|pair| pair == ["--remote", &url]));
    assert!(
        !codex
            .args
            .iter()
            .any(|arg| ["--no-daemon", "app-server"].contains(&arg.as_str()))
    );
    assert!(!codex.env.contains_key("BBOXY_HOOK_SOCKET_ONLY"));
    let instructions = codex
        .args
        .iter()
        .find_map(|arg| arg.strip_prefix("developer_instructions="))
        .unwrap();
    let instructions: String = serde_json::from_str(instructions).unwrap();
    assert!(instructions.starts_with(prompt));
    assert!(instructions.ends_with(context.skill));
    assert!(hooks.contains("bboxy-seat"));
    let wrapper = std::fs::read_to_string(codex.overlay.join("bboxy-seat")).unwrap();
    assert!(wrapper.contains(&format!("export BBOXY_SEAT_ID='{}'", context.id)));
    assert!(wrapper.contains("BBOXY_HOOK_SOCKET_ONLY=1"));
    assert!(!codex.args.iter().any(|arg| arg.contains(&"07".repeat(32))));
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

// keeps: interface app-harness — operator's 2026-10-28 launch policy
#[test]
fn fresh_and_resumed_custom_launches_always_carry_permission_bypass() {
    let root = tempfile::tempdir().unwrap();
    for kind in ["claude", "codex"] {
        for (resume, preconfigured) in [(false, false), (false, true), (true, false), (true, true)] {
            let context = LaunchContext {
                id: Ulid::new(),
                generation: 1,
                name: "permissions fixture",
                role_name: "executor",
                harness_session_id: resume.then_some("00000000-0000-4000-8000-000000000001"),
                skill: "fixture skill",
                cwd: root.path(),
                role_prompt: "fixture prompt",
                executable: std::path::Path::new("/unused/bboxy450-term"),
                overlay_parent: root.path(),
                user_home: root.path(),
                user_zdotdir: None,
                socket: std::path::Path::new("/unused/control"),
                capability: [4; 32],
                resume,
            };
            let mut record = HarnessRecord::builtin(kind).unwrap();
            record.command[0] = format!("/custom/{kind}");
            let flag = if kind == "claude" {
                "--dangerously-skip-permissions"
            } else {
                "--yolo"
            };
            if preconfigured {
                record.command.push(if kind == "codex" && resume {
                    "--dangerously-bypass-approvals-and-sandbox".into()
                } else {
                    flag.into()
                });
            }
            let launch = plan(&record, &context).unwrap();
            assert_eq!(
                launch
                    .args
                    .iter()
                    .filter(|arg| arg.as_str() == flag)
                    .count(),
                1,
                "{kind} resume={resume} preconfigured={preconfigured}"
            );
            if kind == "codex" {
                let config: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(launch.overlay.join("codex-launch.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(config["config"]["approval_policy"], "never");
                assert_eq!(config["config"]["sandbox_mode"], "danger-full-access");
                assert_eq!(config["resume"].is_string(), resume);
            }
            launch.cleanup().unwrap();
        }
    }
}
// keeps: interface app-harness — conflicting caller options cannot downgrade policy
#[test]
fn conflicting_launch_permission_selectors_refuse_without_an_overlay() {
    let root = tempfile::tempdir().unwrap();
    let context = LaunchContext {
        id: Ulid::new(),
        generation: 1,
        name: "conflict fixture",
        role_name: "executor",
        harness_session_id: None,
        skill: "fixture skill",
        cwd: root.path(),
        role_prompt: "fixture prompt",
        executable: std::path::Path::new("/unused/bboxy450-term"),
        overlay_parent: root.path(),
        user_home: root.path(),
        user_zdotdir: None,
        socket: std::path::Path::new("/unused/control"),
        capability: [4; 32],
        resume: false,
    };
    for (kind, arguments) in [
        ("claude", vec!["--permission-mode", "plan"]),
        ("claude", vec!["--permission-mode=manual"]),
        ("claude", vec!["--restricted"]),
        ("claude", vec!["--"]),
        ("codex", vec!["--"]),
        ("codex", vec!["--sandbox", "read-only"]),
        ("codex", vec!["--sandbox=workspace-write"]),
        ("codex", vec!["-sread-only"]),
        ("codex", vec!["-a", "on-request"]),
        ("codex", vec!["--ask-for-approval=on-request"]),
        ("codex", vec!["--approve-for-me"]),
        ("codex", vec!["-c", "approval_policy=\"on-request\""]),
        ("codex", vec!["--config=sandbox_mode=\"read-only\""]),
        ("codex", vec!["-capproval_policy=\"on-request\""]),
    ] {
        let mut record = HarnessRecord::builtin(kind).unwrap();
        record
            .command
            .extend(arguments.iter().map(|value| (*value).to_owned()));
        let error = match plan(&record, &context) {
            Ok(_) => panic!("accepted {kind} {arguments:?}"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("always"),
            "{kind} {arguments:?}: {error}"
        );
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            0,
            "refusal creates no overlay"
        );
    }
}

// keeps: spec harness-kits §5
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

// keeps: spec harness-kits §5
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
        let mut binding = binding();
        binding.runtime = Some(bboxy450_harness_wake_up::RuntimeBinding {
            endpoint: path.clone(),
            pid: binding.pid,
            process_start: binding.process_start.clone(),
        });
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
fn shared_codex_daemon_pins_exact_threads_and_wakes_without_touching_peers() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("shared.sock");
    runtime().block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let mut queued = std::collections::BTreeMap::<String, Vec<String>>::from([
                ("seat-one".into(), vec![]), ("seat-two".into(), vec![]),
                ("operator-unrelated".into(), vec!["leave this alone".into()]),
            ]);
            let mut methods = Vec::new();
            for _ in 0..5 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(Message::Text(frame))) = socket.next().await {
                    let request: serde_json::Value = serde_json::from_str(&frame).unwrap();
                    let method = request["method"].as_str().unwrap();
                    methods.push(method.to_owned());
                    let result = match method {
                        "initialized" => continue,
                        "initialize" => json!({}),
                        "thread/read" => {
                            let id = request["params"]["threadId"].as_str().unwrap();
                            json!({"thread":{"id":id,"status":{"type":if queued.contains_key(id) {"idle"} else {"notLoaded"}}}})
                        }
                        "thread/queue/add" => {
                            let id = request["params"]["threadId"].as_str().unwrap();
                            let input = request["params"]["input"].clone();
                            queued.get_mut(id).unwrap().push(input[0]["text"].as_str().unwrap().into());
                            json!({"threadId":id,"queuedSubmission":{"id":"queued-fixture","clientUserMessageId":request["params"]["clientUserMessageId"],"input":input}})
                        }
                        other => panic!("unexpected shared-daemon operation: {other}"),
                    };
                    socket.send(Message::Text(json!({"id":request["id"],"result":result}).to_string().into())).await.unwrap();
                }
            }
            (queued, methods)
        });
        for thread in ["seat-one", "seat-two"] {
            let mut bound = binding();
            bound.session = thread.into();
            bound.runtime = Some(native::shared_codex_runtime(&bound, &path).await.unwrap());
            assert_eq!(bound.runtime.as_ref().unwrap().pid, std::process::id());
            let endpoint = bboxy450_harness_wake_up::Endpoint::Codex { path: path.clone(), thread: thread.into(), turn: None };
            let outcome = native::wake(&bound, &endpoint, Ulid::new(), thread, Priority::Next, false).await;
            assert_eq!(outcome.evidence, Evidence::NativeQueued);
            let wrong = bboxy450_harness_wake_up::Endpoint::Codex { path: path.clone(), thread: "operator-unrelated".into(), turn: None };
            assert_eq!(native::wake(&bound, &wrong, Ulid::new(), "must not send", Priority::Next, false).await.evidence, Evidence::RecipientStored);
        }
        let mut unknown = binding();
        unknown.session = "unloaded".into();
        assert!(native::shared_codex_runtime(&unknown, &path).await.is_err());
        let (queued, methods) = tokio::time::timeout(std::time::Duration::from_secs(5), server).await.unwrap().unwrap();
        assert_eq!(queued["seat-one"], ["seat-one"]);
        assert_eq!(queued["seat-two"], ["seat-two"]);
        assert_eq!(queued["operator-unrelated"], ["leave this alone"]);
        assert_eq!(methods.iter().filter(|method| method.as_str() == "thread/queue/add").count(), 2);
        assert!(!methods.iter().any(|method| matches!(method.as_str(), "thread/loaded/list" | "thread/resume" | "thread/start")));
        assert!(path.exists());
    });
}

// keeps: spec harness-kits §2 — honest-specific-failure public-note contract
#[test]
fn codex_bootstrap_and_unloaded_threads_never_admit_or_retry_a_submission() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("boot.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut bootstrap = binding();
    bootstrap.session = format!("process-{}", bootstrap.pid);
    bootstrap.runtime = Some(bboxy450_harness_wake_up::RuntimeBinding {
        endpoint: path.clone(),
        pid: bootstrap.pid,
        process_start: bootstrap.process_start.clone(),
    });
    let endpoint = bboxy450_harness_wake_up::Endpoint::Codex {
        path,
        thread: bootstrap.session.clone(),
        turn: None,
    };
    let outcome = runtime().block_on(native::wake(
        &bootstrap,
        &endpoint,
        Ulid::new(),
        "bootstrap",
        Priority::Next,
        false,
    ));
    assert_eq!(outcome.evidence, Evidence::RecipientStored);
    assert!(outcome.note.contains("not yet bound"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );

    for status in ["notLoaded", "systemError"] {
        let path = root.path().join(format!("{status}.sock"));
        let mut bound = binding();
        bound.runtime = Some(bboxy450_harness_wake_up::RuntimeBinding {
            endpoint: path.clone(),
            pid: bound.pid,
            process_start: bound.process_start.clone(),
        });
        let thread = bound.session.clone();
        runtime().block_on(async {
            let listener = tokio::net::UnixListener::bind(&path).unwrap();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let mut methods = Vec::new();
                for expected in ["initialize", "initialized", "thread/read"] {
                    let frame = ws.next().await.unwrap().unwrap();
                    let value: serde_json::Value =
                        serde_json::from_str(frame.to_text().unwrap()).unwrap();
                    assert_eq!(value["method"], expected);
                    methods.push(expected);
                    if expected == "initialized" {
                        continue;
                    }
                    let result = if expected == "thread/read" {
                        json!({"thread":{"id":thread,"status":{"type":status}}})
                    } else {
                        json!({})
                    };
                    ws.send(Message::Text(
                        json!({"id":value["id"],"result":result}).to_string().into(),
                    ))
                    .await
                    .unwrap();
                }
                let next = tokio::time::timeout(std::time::Duration::from_secs(30), ws.next())
                    .await
                    .unwrap();
                assert!(
                    !matches!(next, Some(Ok(Message::Text(_)))),
                    "unknown preflight admitted a mutation"
                );
                methods
            });
            let endpoint = bboxy450_harness_wake_up::Endpoint::Codex {
                path,
                thread: bound.session.clone(),
                turn: None,
            };
            let outcome = native::wake(
                &bound,
                &endpoint,
                Ulid::new(),
                "do not mutate",
                Priority::Next,
                false,
            )
            .await;
            assert_eq!(outcome.evidence, Evidence::NativeUnknown);
            assert!(outcome.note.contains("no mutation or automatic retry"));
            assert_eq!(
                server.await.unwrap(),
                ["initialize", "initialized", "thread/read"]
            );
        });
    }
}

// keeps: spec harness-kits §5
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

// keeps: spec harness-kits §5
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

#[test]
fn codex_hook_observer_joins_only_the_loaded_private_thread_without_overrides() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("hooks.sock");
    let mut bound = binding();
    bound.runtime = Some(bboxy450_harness_wake_up::RuntimeBinding {
        endpoint: path.clone(),
        pid: bound.pid,
        process_start: bound.process_start.clone(),
    });
    let thread = bound.session.clone();
    runtime().block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut methods = Vec::new();
            for method in ["initialize", "initialized", "thread/loaded/list", "thread/read", "thread/resume"] {
                let frame = ws.next().await.unwrap().unwrap();
                let v: serde_json::Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
                assert_eq!(v["method"], method); methods.push(method);
                if method == "initialized" { continue; }
                let result = match method {
                    "thread/loaded/list" => json!({"data":[thread],"nextCursor":null}),
                    "thread/read" => json!({"thread":{"id":thread,"status":{"type":"idle"}}}),
                    "thread/resume" => {
                        assert_eq!(v["params"], json!({"threadId":thread,"excludeTurns":true}));
                        let event = json!({"method":"hook/completed","params":{"threadId":thread,"run":{
                            "id":"run-1","eventName":"sessionStart","source":"user","sourcePath":"/fixture/.codex/hooks.json",
                            "status":"failed","statusMessage":"Hook exited with code 1","durationMs":7,
                            "entries":[{"kind":"context","text":"model context must not be retained"},{"kind":"error","text":"actual hook stderr"}]}}});
                        ws.send(Message::Text(event.to_string().into())).await.unwrap();
                        json!({"thread":{"id":thread,"status":{"type":"idle"}}})
                    },
                    _ => json!({}),
                };
                ws.send(Message::Text(json!({"id":v["id"],"result":result}).to_string().into())).await.unwrap();
            }
            let next = tokio::time::timeout(std::time::Duration::from_secs(60), ws.next()).await.unwrap();
            assert!(!matches!(next, Some(Ok(Message::Text(_)))), "observer sent a control request after joining");
            methods
        });
        let mut observer = native::codex_hook_observer(&bound).await.unwrap();
        let failure = observer.next_failure().await.unwrap();
        assert_eq!(failure.id, "run-1"); assert_eq!(failure.event, "sessionStart");
        assert_eq!(failure.exit_code, Some(1)); assert_eq!(failure.duration_ms, Some(7));
        assert_eq!(failure.source, "user"); assert_eq!(failure.source_path, "/fixture/.codex/hooks.json");
        assert_eq!(failure.stderr, "actual hook stderr");
        drop(observer);
        assert_eq!(server.await.unwrap(), ["initialize", "initialized", "thread/loaded/list", "thread/read", "thread/resume"]);
    });
}

#[test]
fn hook_failure_projection_rejects_other_threads_and_keeps_only_a_bounded_error_tail() {
    let mut event = json!({"method":"hook/completed","params":{"threadId":"owned","run":{
        "id":"one","eventName":"sessionStart","source":"user","sourcePath":"/fixture/hooks.json",
        "status":"failed","statusMessage":"failure; exit unavailable","durationMs":null,
        "entries":[{"kind":"context","text":"private prompt"},{"kind":"feedback","text":"model input"},
            {"kind":"error","text":"界".repeat(2000)}]}}});
    assert!(native::codex_hook_failure(&event, "another").is_none());
    let failure = native::codex_hook_failure(&event, "owned").unwrap();
    assert_eq!(failure.exit_code, None);
    assert_eq!(failure.duration_ms, None);
    assert_eq!(failure.stderr, "界".repeat(1365));
    event["params"]["run"]["status"] = json!("completed");
    assert!(native::codex_hook_failure(&event, "owned").is_none());
    event["params"]["run"]["statusMessage"] = json!("hook exit code -2");
    assert_eq!(
        native::codex_hook_failure(&event, "owned")
            .unwrap()
            .exit_code,
        Some(-2)
    );
    event["params"]["run"]["id"] = json!("x".repeat(257));
    assert!(native::codex_hook_failure(&event, "owned").is_none());
}

#[test]
fn hook_observer_never_resumes_an_unloaded_ambiguous_or_stale_thread() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    for (loaded, cursor, status) in [
        (json!([]), json!(null), "idle"),
        (json!(["fixture-session", "other"]), json!(null), "idle"),
        (json!(["fixture-session"]), json!("more"), "idle"),
        (json!(["fixture-session"]), json!(null), "notLoaded"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("s");
        let mut bound = binding();
        bound.runtime = Some(bboxy450_harness_wake_up::RuntimeBinding {
            endpoint: path.clone(),
            pid: bound.pid,
            process_start: bound.process_start.clone(),
        });
        runtime().block_on(async {
            let listener = tokio::net::UnixListener::bind(&path).unwrap();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(Message::Text(frame))) = ws.next().await {
                    let request: serde_json::Value = serde_json::from_str(&frame).unwrap();
                    let result = match request["method"].as_str().unwrap() {
                        "initialize" => json!({}),
                        "initialized" => continue,
                        "thread/loaded/list" => json!({"data":loaded,"nextCursor":cursor}),
                        "thread/read" => {
                            json!({"thread":{"id":"fixture-session","status":{"type":status}}})
                        }
                        method => panic!("unverified observer sent {method}"),
                    };
                    ws.send(Message::Text(
                        json!({"id":request["id"],"result":result})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                }
            });
            assert!(native::codex_hook_observer(&bound).await.is_err());
            tokio::time::timeout(std::time::Duration::from_secs(60), server)
                .await
                .unwrap()
                .unwrap();
        });
    }
    let mut stale = binding();
    stale.process_start = "different-start".into();
    assert!(
        runtime()
            .block_on(native::codex_hook_observer(&stale))
            .is_err()
    );
}
