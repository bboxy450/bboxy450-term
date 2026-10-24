#![allow(clippy::disallowed_methods)] // Disposable native/PTY fixtures, never UI.
mod common;
use bbt_mux::adapters::{
    Adapter, ArbiterView, Binding, Evidence, HarnessRecord, OscTap, Readiness, Session,
    State, WaitingKind,
};
use bbt_mux::adapters::{
    hook,
    launch::{LaunchContext, plan},
    native,
};
use bbt_mux::registry::SeatRecord;
use bbt_mux::store::{Store, sqlite::SqliteStore};
use bbt_mux::{Event, InputState, Supervisor, Ulid};
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
#[test]
fn harness_defaults_follow_the_role_contract_and_validate_authority() {
    use bbt_mux::registry::{Class, Role};
    for (kind, name, class) in [
        ("claude", "oracle", Class::Oracle),
        ("codex", "executor", Class::Worker),
        ("zsh", "shell", Class::Worker),
    ] {
        let harness = HarnessRecord::builtin(kind).unwrap();
        assert_eq!(
            harness.default_role,
            Role {
                name: name.into(),
                class
            }
        );
        harness.validate().unwrap();
    }
    let mut codex = HarnessRecord::builtin("codex").unwrap();
    codex.default_role.class = Class::Oracle;
    assert_eq!(
        codex.validate().unwrap_err().to_string(),
        "codex_oracle_role_forbidden"
    );
    let claude = HarnessRecord::builtin("claude").unwrap();
    for (name, class) in [
        ("oracle", Class::Oracle),
        ("architect", Class::Oracle),
        ("creativity", Class::Worker),
        ("executor", Class::Worker),
        ("shell", Class::Worker),
    ] {
        assert_eq!(
            claude.resolve_role(Some(name)).unwrap(),
            Role {
                name: name.into(),
                class
            }
        );
    }
    assert_eq!(
        claude
            .resolve_role(Some("director"))
            .unwrap_err()
            .to_string(),
        "unknown_role_preset"
    );
    let codex = HarnessRecord::builtin("codex").unwrap();
    for name in ["oracle", "architect"] {
        assert_eq!(
            codex.resolve_role(Some(name)).unwrap_err().to_string(),
            "codex_oracle_role_forbidden"
        );
    }
    assert_eq!(
        bbt_mux::adapters::validate_role(
            "claude",
            &Role {
                name: "oracle".into(),
                class: Class::Worker
            }
        )
        .unwrap_err()
        .to_string(),
        "role_class_mismatch"
    );
}

#[test]
fn codex_launch_refuses_oracle_class_before_creating_an_overlay() {
    use bbt_mux::registry::Class;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_path_buf();
    let overlays = root.join("overlays");
    let mut seat = SeatRecord::new(Ulid::new(), Ulid::new(), "forged", "codex");
    // The class is authoritative even when a caller labels it executor.
    seat.role.class = Class::Oracle;
    let context = LaunchContext {
        seat: &seat,
        cwd: &root,
        role_prompt: "",
        executable: std::path::Path::new("/fixture/bboxy450-term"),
        overlay_parent: &overlays,
        user_home: &root,
        user_zdotdir: None,
        socket: &root.join("seat.sock"),
        capability: [0; 32],
        resume: false,
    };
    let error = plan(&HarnessRecord::builtin("codex").unwrap(), &context)
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "codex_oracle_role_forbidden");
    assert!(!overlays.exists());
}

#[test]
fn logic_refuses_codex_oracle_authority_without_admitting_a_seat() {
    use bbt_mux::logic::{Logic, LogicConfig};
    use bbt_mux::registry::Class;
    let directory = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(directory.path())).unwrap();
    let run_dir = directory.path().join("run");
    let repository = directory.path().join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run_dir.clone(),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[31; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "forged", "codex");
    record.role.class = Class::Oracle;
    let error = runtime
        .block_on(logic.client.spawn_harness(
            record.clone(),
            HarnessRecord::builtin("codex").unwrap(),
            bbt_mux::adapters::launch::LaunchOptions {
                cwd: directory.path().into(),
                role_prompt: String::new(),
                executable: "/fixture/bboxy450-term".into(),
                user_home: directory.path().into(),
                user_zdotdir: None,
                resume: false,
            },
        ))
        .unwrap_err();
    assert_eq!(error.to_string(), "codex_oracle_role_forbidden");
    let mut spec = bbt_mux::LaunchSpec::new("/bin/zsh", directory.path());
    spec.seat = record.id;
    spec.generation = record.generation;
    let error = runtime
        .block_on(logic.client.spawn(record.clone(), spec))
        .unwrap_err();
    assert_eq!(error.to_string(), "codex_oracle_role_forbidden");
    assert!(
        runtime
            .block_on(logic.client.gui_seats())
            .unwrap()
            .is_empty()
    );
    assert!(!run_dir.join(format!("seat-{}", record.id)).exists());
    logic.shutdown().unwrap();
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
fn adapters_literal_paste_requires_positive_readiness_and_submit_false_has_no_enter() {
    use bbt_mux::adapters::guard::LiteralInput;
    for bracketed in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let journal = SqliteStore::open(temp.path()).unwrap();
        let mut session = session("claude");
        let mut view = arbiter(&session);
        view.bracketed_paste = bracketed;
        let input = LiteralInput {
            id: Ulid::new(),
            text: "literal text",
            submit: false,
        };
        let refused = session
            .submit_literal(&view, 1600, input, &journal, |_| {
                panic!("idle without a positive native observation must not write")
            })
            .unwrap();
        assert_eq!(refused.evidence, Evidence::RecipientStored);
        let file = native::ClaudeSessionFile {
            pid: session.binding.pid,
            session_id: session.binding.session.clone(),
            process_start: session.binding.process_start.clone(),
            status: "idle".into(),
            waiting_for: None,
            messaging_socket_path: None,
        };
        session.observe_claude_session(&file, 5, 1000).unwrap();
        let outcome = session
            .submit_literal(&view, 1600, input, &journal, |bytes| {
                assert_eq!(
                    bytes,
                    if bracketed {
                        b"\x1b[200~literal text\x1b[201~".to_vec()
                    } else {
                        b"literal text".to_vec()
                    }
                );
                assert!(!bytes.ends_with(b"\r"));
                Ok(bbt_mux::InputReceipt {
                    input_id: input.id,
                    state: InputState::PtyWritten,
                    bytes_written: bytes.len(),
                    bytes_total: bytes.len(),
                })
            })
            .unwrap();
        assert_eq!(outcome.evidence, Evidence::NativeWritten);
        assert!(matches!(
            session.input_guard(&view, 1601),
            Readiness::NotReady(_)
        ));
        let replay = session
            .submit_literal(&view, 1601, input, &journal, |_| {
                panic!("must not paste twice")
            })
            .unwrap();
        assert_eq!(replay, outcome);
    }
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
    let observed_at = Instant::now();
    let until = observed_at + Duration::from_secs(2);
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
            "live probe observation exceeded 2s: elapsed={:?}, status={status:?}, screen_seq={}, probe_ok={}",
            observed_at.elapsed(),
            snapshot.sequence,
            text.contains("PROBE_OK")
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    println!(
        "live OSC probe and reported working state observed in {:?}",
        observed_at.elapsed()
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
fn adapters_literal_guard_negatives_write_no_bytes() {
    for case in [
        "active",
        "waiting",
        "subagent",
        "expired",
        "generation",
        "attachment",
        "pid",
        "draft",
        "tool",
        "sync",
        "screen",
        "foreground",
        "canonical",
        "echo",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let shell = matches!(case, "canonical" | "echo");
        let mut session = session(if shell { "zsh" } else { "claude" });
        if shell {
            session.on_osc(&[b"133".to_vec(), b"B".to_vec()], 5, 1000);
        } else {
            let file = native::ClaudeSessionFile {
                pid: session.binding.pid,
                session_id: session.binding.session.clone(),
                process_start: session.binding.process_start.clone(),
                status: "idle".into(),
                waiting_for: None,
                messaging_socket_path: None,
            };
            session.observe_claude_session(&file, 5, 1000).unwrap();
        }
        let mut view = arbiter(&session);
        let now = if case == "expired" { 3001 } else { 1600 };
        match case {
            "active" => session.status.state = State::Active,
            "waiting" => session.status.state = State::Waiting,
            "subagent" => session
                .status
                .subagents
                .push(bbt_mux::adapters::observe::Subagent {
                    id: "fixture-child".into(),
                    title: "fixture".into(),
                    state: State::Active,
                    waiting_kind: None,
                    grade: bbt_mux::adapters::Grade::Reported,
                    started_sequence: 5,
                }),
            "generation" => view.binding.generation += 1,
            "attachment" => view.binding.attachment_generation += 1,
            "pid" => view.binding.pid += 1,
            "draft" => view.operator_draft = true,
            "tool" => view.active_tool = true,
            "sync" => view.sync_pending = true,
            "screen" => view.screen_switch_pending = true,
            "foreground" => view.foreground_pid = None,
            "canonical" => view.canonical = false,
            "echo" => view.echo = false,
            _ => {}
        }
        let result = session
            .submit_literal(
                &view,
                now,
                bbt_mux::adapters::guard::LiteralInput {
                    id: Ulid::new(),
                    text: "never write",
                    submit: true,
                },
                &SqliteStore::open(temp.path()).unwrap(),
                |_| panic!("{case} wrote bytes"),
            )
            .unwrap();
        assert_eq!(result.evidence, Evidence::RecipientStored, "{case}");
    }
}
