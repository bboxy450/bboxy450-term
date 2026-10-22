#![allow(clippy::disallowed_methods)] // Disposable native/PTY fixtures, never UI.
use bbt_mux::adapters::{
    Adapter, ArbiterView, Binding, Evidence, HarnessRecord, OscTap, Priority, Readiness, Session,
    State, WaitingKind,
};
use bbt_mux::adapters::{
    hook,
    launch::{LaunchContext, plan},
    native,
};
use bbt_mux::registry::SeatRecord;
use bbt_mux::store::{Store, sqlite::SqliteStore};
use bbt_mux::{Config, Event, InputState, Supervisor, Ulid};
use serde_json::json;
use std::time::{Duration, Instant};

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
    let bytes = include_bytes!("../../../.bboxy/runs/osc7501-claude.bin");
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
fn adapters_sessions_file_dialog_blocks_every_write_and_rejects_pid_session_reuse() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("session.json");
    let mut session = session("claude");
    let record = json!({"pid":session.binding.pid,"sessionId":session.binding.session,"procStart":session.binding.process_start,"status":"waiting","waitingFor":"permission"});
    std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let file = native::read_claude_session(&path, &session.binding).unwrap();
    session.observe_claude_session(&file, 10, 1000).unwrap();
    assert_eq!(session.status.waiting_kind, Some(WaitingKind::Permission));
    let journal = SqliteStore::open(temp.path()).unwrap();
    let mut writes = 0;
    let outcome = session
        .submit_pty(
            &arbiter(&session),
            1600,
            Ulid::new(),
            "peer data",
            &journal,
            |_| {
                writes += 1;
                unreachable!("dialog must never receive a paste")
            },
        )
        .unwrap();
    assert_eq!(writes, 0);
    assert_eq!(outcome.evidence, Evidence::RecipientStored);
    let mut wrong = session.binding.clone();
    wrong.session = "another-session".into();
    assert!(native::read_claude_session(&path, &wrong).is_err());
    wrong = session.binding.clone();
    wrong.process_start = "old start".into();
    assert!(native::read_claude_session(&path, &wrong).is_err());
}

#[test]
fn adapters_positive_ready_is_consumed_and_journals_unknown_without_retyping() {
    let temp = tempfile::tempdir().unwrap();
    let mut session = session("claude");
    let file = native::ClaudeSessionFile {
        pid: session.binding.pid,
        session_id: session.binding.session.clone(),
        process_start: session.binding.process_start.clone(),
        status: "idle".into(),
        waiting_for: None,
        messaging_socket_path: None,
    };
    session.observe_claude_session(&file, 5, 1000).unwrap();
    let view = arbiter(&session);
    let id = Ulid::new();
    let journal = SqliteStore::open(temp.path()).unwrap();
    let mut writes = 0;
    let outcome = session
        .submit_pty(&view, 1600, id, "literal\nmessage", &journal, |bytes| {
            writes += 1;
            assert_eq!(bytes, b"\x1b[200~literal\nmessage\x1b[201~\r");
            assert!(
                journal
                    .get(&format!("adapter_input/{id}"))
                    .unwrap()
                    .is_some()
            );
            Ok(bbt_mux::InputReceipt {
                input_id: id,
                state: InputState::PartialUnknown,
                bytes_written: 3,
                bytes_total: bytes.len(),
            })
        })
        .unwrap();
    assert_eq!(outcome.evidence, Evidence::NativeUnknown);
    session.observe_claude_session(&file, 6, 1700).unwrap();
    assert_eq!(
        session
            .submit_pty(&view, 1800, id, "literal\nmessage", &journal, |_| {
                writes += 1;
                unreachable!()
            })
            .unwrap(),
        outcome
    );
    assert_eq!(writes, 1);
    let Readiness::Ready(token) = session.input_guard(&view, 1800) else {
        panic!()
    };
    let mut changed = view.clone();
    changed.operator_epoch += 1;
    assert!(
        session
            .prepare_input(token, &changed, 1800, Ulid::new(), "text")
            .is_err()
    );
    let mut plain = view.clone();
    plain.bracketed_paste = false;
    let Readiness::Ready(token) = session.input_guard(&plain, 1800) else {
        panic!()
    };
    assert!(
        session
            .prepare_input(token, &plain, 1800, Ulid::new(), "tab\tcompletion")
            .is_err()
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
    let record = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "claude");
    let context = LaunchContext {
        seat: &record,
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
    assert!(plan.spec.args.contains(&"--append-system-prompt".into()));
    assert!(!plan.spec.args.iter().any(|s| s.contains("bypass")));
    assert_eq!(std::fs::read(settings).unwrap(), b"operator-config");
    let codex =
        bbt_mux::adapters::launch::plan(&HarnessRecord::builtin("codex").unwrap(), &context)
            .unwrap();
    let hooks = codex
        .spec
        .args
        .iter()
        .find(|s| s.starts_with("hooks="))
        .unwrap();
    assert!(hooks.contains("SessionStart="));
    assert!(hooks.contains("SubagentStart="));
    assert!(!hooks.contains("session_start="));
    assert!(
        !codex
            .spec
            .args
            .iter()
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
    assert!(hook::osc(record.id, &large).unwrap().is_none());
    plan.cleanup().unwrap();
}

#[test]
fn adapters_real_zsh_overlay_reports_prompt_133_b_and_command_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let user = temp.path().join("user");
    std::fs::create_dir(&user).unwrap();
    let record = SeatRecord::new(Ulid::new(), Ulid::new(), "shell fixture", "zsh");
    let context = LaunchContext {
        seat: &record,
        cwd: temp.path(),
        role_prompt: "",
        executable: std::path::Path::new("/tmp/bboxy450-term"),
        overlay_parent: temp.path(),
        user_home: &user,
        user_zdotdir: Some(&user),
        socket: std::path::Path::new("/tmp/fixture.sock"),
        capability: [9; 32],
        resume: false,
    };
    let plan = plan(&HarnessRecord::builtin("zsh").unwrap(), &context).unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let info = client.spawn(plan.spec.clone()).unwrap();
    let att = client.attach(record.id, 1).unwrap();
    let mut bind = binding();
    bind.seat = record.id;
    bind.pid = info.pid;
    bind.process_start = native::process_start(info.pid).unwrap();
    let mut session = Session::new(HarnessRecord::builtin("zsh").unwrap(), bind).unwrap();
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    let until = Instant::now() + Duration::from_secs(5);
    let mut prompt = false;
    while !prompt {
        assert!(Instant::now() < until, "zsh prompt marker missing");
        if let Ok(Event::Output { seq, bytes, .. }) = client.recv_timeout(Duration::from_millis(50))
        {
            term.advance(&bytes);
            for params in tap.drain().unwrap() {
                session.on_osc(&params, seq, 1000);
            }
            prompt = session.status.source == "133" && session.status.state == State::Idle;
        }
    }
    assert!(
        session
            .cwd
            .as_ref()
            .is_some_and(|s| s.starts_with("file://"))
    );
    assert!(matches!(
        session.input_guard(&arbiter(&session), 1100),
        Readiness::Ready(_)
    ));
    client
        .input(&att, Ulid::new(), b"printf BBOXY_FIXTURE\n")
        .unwrap();
    let mut active = false;
    while !active {
        assert!(Instant::now() < until, "zsh preexec marker missing");
        if let Ok(Event::Output { seq, bytes, .. }) = client.recv_timeout(Duration::from_millis(50))
        {
            term.advance(&bytes);
            for params in tap.drain().unwrap() {
                for obs in session.on_osc(&params, seq, 1200) {
                    active |= obs.state == State::Active;
                }
            }
        }
    }
    client.kill(&att).unwrap();
    drop(supervisor);
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
            .attach(bbt_mux::adapters::Endpoint::Claude { path: path.clone() })
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
            let endpoint=bbt_mux::adapters::Endpoint::Codex{path:path.clone(),thread:binding.session.clone(),turn:Some("turn1".into())};
            if priority==Priority::Now {assert_eq!(native::wake(&binding,&endpoint,Ulid::new(),"not-authorized",priority,false).await.evidence,Evidence::RecipientStored);}
            let outcome=native::wake(&binding,&endpoint,Ulid::new(),"fixture",priority,true).await;
            assert_eq!(server.await.unwrap()["params"]["input"][0]["text"],"fixture");
            assert_eq!(outcome.evidence,if disconnect||malformed{Evidence::NativeUnknown}else{Evidence::NativeQueued});
        });
    }
}

#[test]
fn adapters_logic_launch_retains_overlay_until_exit_and_authenticates_shim_fallback() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let user = temp.path().join("user");
    std::fs::create_dir(&user).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let logic = bbt_mux::logic::Logic::start(
        supervisor.connect().unwrap(),
        bbt_mux::logic::LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[62; 32]),
        runtime.handle(),
    )
    .unwrap();
    let record = SeatRecord::new(Ulid::new(), Ulid::new(), "adapter integration", "zsh");
    let id = record.id;
    let swarm = record.swarm;
    runtime
        .block_on(logic.client.spawn_harness(
            record,
            HarnessRecord::builtin("zsh").unwrap(),
            bbt_mux::adapters::launch::LaunchOptions {
                cwd: temp.path().into(),
                role_prompt: String::new(),
                executable: "/tmp/bboxy450-term".into(),
                user_home: user.clone(),
                user_zdotdir: Some(user),
                resume: false,
            },
        ))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let status = runtime
            .block_on(logic.client.adapter_status(id, 1))
            .unwrap();
        if status.source == "133" && status.state == State::Idle {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let root = temp.path().join("run").join(format!("seat-{id}"));
    let overlays = root.join("overlays");
    assert_eq!(std::fs::read_dir(&overlays).unwrap().count(), 1);
    let endpoint = runtime.block_on(logic.client.endpoint(id)).unwrap();
    runtime.block_on(async {
        let event = hook::prepare(
            json!({"hook_event_name":"PermissionRequest","tool_use_id":"permission-1"}),
            "claude",
        )
        .unwrap();
        for admitted in [false, true] {
            let stream = tokio::net::UnixStream::connect(&endpoint.path)
                .await
                .unwrap();
            let (read, mut write) = stream.into_split();
            let request = bbt_mux::seat_socket::Request {
                version: 1,
                seat: id,
                generation: 1,
                capability: if admitted {
                    endpoint.capability
                } else {
                    [0; 32]
                },
                action: bbt_mux::seat_socket::Action::Hook {
                    event: event.clone(),
                },
            };
            let mut bytes = serde_json::to_vec(&request).unwrap();
            bytes.push(b'\n');
            write.write_all(&bytes).await.unwrap();
            let mut line = String::new();
            let count = BufReader::new(read).read_line(&mut line).await.unwrap();
            if admitted {
                assert!(count > 0);
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&line).unwrap()["kind"],
                    "HookObserved"
                );
            } else {
                assert_eq!(count, 0, "bad capability must not reach the hook observer");
            }
        }
    });
    runtime
        .block_on(logic.client.close(
            id,
            1,
            Ulid::new(),
            bbt_mux::registry::Position {
                swarm,
                tab: Ulid::new(),
                split_path: vec![0],
                ratios: vec![],
                layout_revision: 1,
            },
            0,
        ))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while std::fs::read_dir(&overlays).unwrap().count() != 0 {
        assert!(
            Instant::now() < until,
            "overlay was not retired after process exit"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    logic.shutdown().unwrap();
}

#[test]
fn adapters_live_logic_answers_bel_probe_on_the_same_pty_without_replaying_it() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let logic = bbt_mux::logic::Logic::start(
        supervisor.connect().unwrap(),
        bbt_mux::logic::LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[63; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut launch = bbt_mux::LaunchSpec::new("/bin/zsh", temp.path());
    launch.args=vec!["-f".into(),"-c".into(),r#"stty raw -echo; printf '\033]7501;?\007'; got=$(dd bs=1 count=9 2>/dev/null); if [[ $got == $'\e]7501;?\a' ]]; then printf '%s' "$BBOXY_START_FRAME"; printf 'PROBE_OK\r\n\033]7501;state=working:app=fixture\007'; fi; sleep 30"#.into()];
    let id = launch.seat;
    let start = hook::prepare(
        json!({"hook_event_name":"SessionStart","session_id":"native-fixture-session"}),
        "claude",
    )
    .unwrap();
    launch.env.insert(
        "BBOXY_START_FRAME".into(),
        String::from_utf8(hook::osc(id, &start).unwrap().unwrap()).unwrap(),
    );
    runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(id, Ulid::new(), "probe fixture", "claude"),
            launch,
        ))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let status = runtime
            .block_on(logic.client.adapter_status(id, 1))
            .unwrap();
        let snapshot = runtime.block_on(logic.client.snapshot(id, 1)).unwrap();
        let text: String = snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .map(|c| c.c)
            .collect();
        if status.state == State::Active && status.source == "7501" && text.contains("PROBE_OK") {
            break;
        }
        assert!(
            Instant::now() < until,
            "live probe was not echoed before the harness deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        runtime
            .block_on(logic.client.record(id))
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("native-fixture-session")
    );
    logic.shutdown().unwrap();
    let logic = bbt_mux::logic::Logic::start(
        supervisor.connect().unwrap(),
        bbt_mux::logic::LogicConfig {
            run_dir: temp.path().join("run"),
            repository: temp.path().join(".bboxy"),
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[63; 32]),
        runtime.handle(),
    )
    .unwrap();
    assert_eq!(
        runtime
            .block_on(logic.client.adapter_status(id, 1))
            .unwrap()
            .source,
        "unobserved"
    );
    assert_eq!(
        runtime
            .block_on(logic.client.record(id))
            .unwrap()
            .harness_session_id
            .as_deref(),
        Some("native-fixture-session")
    );
    logic.shutdown().unwrap();
}

#[test]
fn adapters_custom_harness_cadence_and_reported_idle_never_create_readiness() {
    let mut record = HarnessRecord::builtin("zsh").unwrap();
    record.kind = "user-defined-harness".into();
    record.state_source = bbt_mux::adapters::StateSource::Cadence;
    let mut session = Session::new(record, binding()).unwrap();
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    term.advance(include_bytes!("../../../lab/web/logs/codex-session.txt"));
    for params in tap.drain().unwrap() {
        session.on_osc(&params, 1, 1000);
    }
    session.output(1000);
    session.tick(1, 1000);
    assert_eq!(session.status.state, State::Active);
    session.tick(1, 4000);
    assert_eq!(session.status.state, State::Idle);
    assert_eq!(session.status.grade, bbt_mux::adapters::Grade::Inferred);
    session.on_osc(
        &[b"7501".to_vec(), b"state=idle:app=custom".to_vec()],
        2,
        4500,
    );
    assert_eq!(session.status.grade, bbt_mux::adapters::Grade::Reported);
    assert!(matches!(
        session.input_guard(&arbiter(&session), 5000),
        Readiness::OperatorHandoff
    ));
}
