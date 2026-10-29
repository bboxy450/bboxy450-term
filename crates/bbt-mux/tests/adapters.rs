// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable native/PTY fixtures, never UI.
mod common;
use bbt_mux::adapters::{
    Adapter, ArbiterView, Binding, Evidence, HarnessRecord, OscTap, Readiness, Session, State,
    WaitingKind,
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

struct ZshStartupDiagnostics<'a> {
    root: Option<tempfile::TempDir>,
    client: &'a bbt_mux::Client,
    pid: u32,
    lookup: std::cell::RefCell<String>,
    events: std::cell::RefCell<Vec<Event>>,
}
impl ZshStartupDiagnostics<'_> {
    fn process_start(&self) -> bboxy450_harness_wake_up::Result<String> {
        let result = native::process_start(self.pid);
        *self.lookup.borrow_mut() = format!("{result:?}");
        result
    }

    fn recv(&self) -> Result<Event, std::sync::mpsc::RecvTimeoutError> {
        let event = self.client.recv_timeout(Duration::from_millis(50))?;
        self.events.borrow_mut().push(event.clone());
        Ok(event)
    }

    fn retain(&mut self) -> std::path::PathBuf {
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            if self
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Exited { .. }))
            {
                break;
            }
            if self.recv().is_err() {
                break;
            }
        }
        let seats = self.client.request(bbt_mux::Action::ListOwnedSeats);
        let events = self.events.borrow();
        let pty: Vec<u8> = events
            .iter()
            .flat_map(|event| match event {
                Event::Output { bytes, .. } => bytes.as_slice(),
                _ => &[],
            })
            .copied()
            .collect();
        let retained = self.root.take().unwrap().keep();
        let diagnostics = format!(
            "pid={}\nlookup={}\nretained={seats:?}\nevents={events:?}\n",
            self.pid,
            self.lookup.borrow()
        );
        let metadata_write = std::fs::write(retained.join("startup-diagnostics.log"), &diagnostics);
        let pty_write = std::fs::write(retained.join("startup-pty.log"), &pty);
        eprintln!(
            "zsh startup diagnostics at {} (metadata={metadata_write:?}, pty={pty_write:?}): {diagnostics}\nPTY: {:?}",
            retained.display(),
            String::from_utf8_lossy(&pty)
        );
        retained
    }
}

#[test]
fn adapters_zsh_startup_retains_exit_and_stderr_when_lookup_fails() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_owned();
    let user = root.join("user");
    std::fs::create_dir(&user).unwrap();
    // Controlled startup failure, not an assertion about the historical cause.
    std::fs::write(
        user.join(".zshenv"),
        "print -ru2 -- BBOXY_STARTUP_EXIT_41\nexit 41\n",
    )
    .unwrap();
    let record = SeatRecord::new(Ulid::new(), Ulid::new(), "startup exit fixture", "zsh");
    let mut plan = plan(
        &HarnessRecord::builtin("zsh").unwrap(),
        &LaunchContext {
            seat: &record,
            cwd: &root,
            role_prompt: "",
            executable: std::path::Path::new("/tmp/bboxy450-term"),
            overlay_parent: &root,
            user_home: &user,
            user_zdotdir: Some(&user),
            socket: std::path::Path::new("/tmp/fixture.sock"),
            capability: [9; 32],
            resume: false,
        },
    )
    .unwrap();
    plan.spec
        .env
        .insert("HOME".into(), user.to_string_lossy().into_owned());
    let supervisor = Supervisor::start(common::capture_config(&root)).unwrap();
    let client = supervisor.connect().unwrap();
    let info = client.spawn(plan.spec.clone()).unwrap();
    client.attach(info.seat, info.generation).unwrap();
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let diagnostics = ZshStartupDiagnostics {
            root: Some(temp),
            client: &client,
            pid: info.pid,
            lookup: Default::default(),
            events: Default::default(),
        };
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < until, "startup exit not observed");
            if let Ok(Event::Exited { status, .. }) = diagnostics.recv() {
                assert_eq!(status.code, 41);
                assert!(status.signal.is_none());
                break;
            }
        }
        let error = diagnostics.process_start().unwrap_err();
        assert!(error.to_string().contains("ps_status="));
        panic!("intentional startup failure exercises unwind retention");
    }));
    assert!(failed.is_err());
    let log = std::fs::read_to_string(root.join("startup-diagnostics.log")).unwrap();
    assert!(log.contains("lookup=Err("));
    assert!(log.contains("ps_status=exit status: 1"));
    assert!(log.contains("code: 41, signal: None"));
    let pty = std::fs::read_to_string(root.join("startup-pty.log")).unwrap();
    assert!(
        pty.contains("BBOXY_STARTUP_EXIT_41"),
        "stderr was lost: {pty:?}"
    );
    assert!(!log.contains("BBOXY_CAPABILITY"));
    supervisor.shutdown().unwrap();
    plan.cleanup().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
impl Drop for ZshStartupDiagnostics<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() && self.root.is_some() {
            self.retain();
        }
    }
}

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
// keeps: decision #262
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
    codex.validate().unwrap();
    for kind in ["claude", "codex", "pi", "omp", "zsh"] {
        let harness = HarnessRecord::builtin(kind).unwrap();
        for (name, class) in [
            ("oracle", Class::Oracle),
            ("architect", Class::Oracle),
            ("creativity", Class::Worker),
            ("executor", Class::Worker),
            ("shell", Class::Worker),
        ] {
            assert_eq!(
                harness.resolve_role(Some(name)).unwrap(),
                Role {
                    name: name.into(),
                    class
                }
            );
        }
        assert_eq!(
            harness
                .resolve_role(Some("director"))
                .unwrap_err()
                .to_string(),
            "unknown_role_preset"
        );
    }
    bbt_mux::adapters::validate_role(
        "claude",
        &Role {
            name: "oracle".into(),
            class: Class::Worker,
        },
    )
    .unwrap();
}

// keeps: decision #262
#[test]
fn codex_launch_carries_the_explicit_oracle_role_into_its_overlay() {
    use bbt_mux::registry::Class;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_path_buf();
    let overlays = root.join("overlays");
    let mut seat = SeatRecord::new(Ulid::new(), Ulid::new(), "configured", "codex");
    seat.role.name = "oracle".into();
    seat.role.class = Class::Oracle;
    let context = LaunchContext {
        seat: &seat,
        cwd: &root,
        role_prompt: "Coordinate the swarm.",
        executable: std::path::Path::new("/fixture/bboxy450-term"),
        overlay_parent: &overlays,
        user_home: &root,
        user_zdotdir: None,
        socket: &root.join("seat.sock"),
        capability: [0; 32],
        resume: false,
    };
    let plan = plan(&HarnessRecord::builtin("codex").unwrap(), &context).unwrap();
    assert!(plan.overlay.exists());
    let args = plan.spec.args.join(" ");
    assert!(args.contains("Coordinate the swarm."));
    assert!(args.contains("oracle"));
    plan.cleanup().unwrap();
}

// keeps: decision #262
#[test]
fn logic_refuses_a_forged_codex_role_class_without_admitting_a_seat() {
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
                env: vec![],
                cwd: directory.path().into(),
                role_prompt: String::new(),
                executable: "/fixture/bboxy450-term".into(),
                user_home: directory.path().into(),
                user_zdotdir: None,
                resume: false,
            },
        ))
        .unwrap_err();
    assert_eq!(error.to_string(), "configured_role_class_mismatch");
    let mut spec = bbt_mux::LaunchSpec::new("/bin/zsh", directory.path());
    spec.seat = record.id;
    spec.generation = record.generation;
    let error = runtime
        .block_on(logic.client.spawn(record.clone(), spec))
        .unwrap_err();
    assert_eq!(error.to_string(), "configured_role_class_mismatch");
    assert!(
        runtime
            .block_on(logic.client.gui_seats())
            .unwrap()
            .is_empty()
    );
    assert!(!run_dir.join(format!("seat-{}", record.id)).exists());
    logic.shutdown().unwrap();
}

// keeps: spec harness-adapters §5, a dialog never receives a PTY paste.
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
    let outcome = session
        .submit_pty(
            &arbiter(&session),
            1600,
            Ulid::new(),
            "peer data",
            &journal,
            |_| unreachable!("dialog must never receive a paste"),
        )
        .unwrap();
    assert_eq!(outcome.evidence, Evidence::RecipientStored);
    let mut wrong = session.binding.clone();
    wrong.session = "another-session".into();
    assert!(native::read_claude_session(&path, &wrong).is_err());
    wrong = session.binding.clone();
    wrong.process_start = "old start".into();
    assert!(native::read_claude_session(&path, &wrong).is_err());
}

// keeps: spec harness-adapters
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

// keeps: spec harness-adapters
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
    let diagnostics = ZshStartupDiagnostics {
        root: Some(temp),
        client: &client,
        pid: info.pid,
        lookup: Default::default(),
        events: Default::default(),
    };
    let att = client.attach(record.id, 1).unwrap();
    let mut bind = binding();
    bind.seat = record.id;
    bind.pid = info.pid;
    bind.process_start = diagnostics.process_start().unwrap();
    let mut session = Session::new(HarnessRecord::builtin("zsh").unwrap(), bind).unwrap();
    let mut term = bbt_core::Term::new(80, 24, 0);
    let tap = OscTap::install(&mut term);
    let until = Instant::now() + Duration::from_secs(5);
    let mut prompt = false;
    while !prompt {
        assert!(Instant::now() < until, "zsh prompt marker missing");
        if let Ok(Event::Output { seq, bytes, .. }) = diagnostics.recv() {
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
        if let Ok(Event::Output { seq, bytes, .. }) = diagnostics.recv() {
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
                env: vec![],
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

// keeps: spec harness-adapters
#[test]
fn adapters_literal_guard_negatives_write_no_bytes() {
    use bbt_mux::adapters::WakeCause;
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
        let cause = match case {
            "active" | "subagent" => WakeCause::TurnActive,
            "waiting" => WakeCause::DialogPending,
            "expired" => WakeCause::ComposerReadinessExpired,
            "generation" | "attachment" | "pid" => WakeCause::InputBindingChanged,
            "draft" | "tool" | "sync" | "screen" => WakeCause::InputOwned,
            "foreground" => WakeCause::ForegroundChanged,
            "canonical" | "echo" => WakeCause::ShellModeInvalid,
            _ => unreachable!(),
        };
        assert_eq!(result.cause, Some(cause), "{case}");
        assert!(!result.note.is_empty() && result.note.len() <= 640);
    }
}

// keeps: interface app-harness — legacy/future unknown journal facts never authorize another writer.
#[test]
fn wake_causes_replayed_unknowns_block_native_and_pty_resubmission() {
    use bbt_mux::adapters::{Endpoint, Priority, WakeCause, WakeOutcome};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for cause in [
        None,
        Some("future-peer-state"),
        Some("native-queue-admitted"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("native.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut session = session("claude");
        let attachment = runtime
            .block_on(session.attach(Endpoint::Claude { path }))
            .unwrap();
        drop(listener.accept().unwrap()); // Only the read-only attachment probe.
        let id = Ulid::new();
        let journal = SqliteStore::open(&root.path().join("journal")).unwrap();
        let mut wire =
            json!({"evidence":"native-unknown","native_id":null,"note":"retained ambiguous write"});
        if let Some(cause) = cause {
            wire["cause"] = json!(cause);
        }
        journal
            .put(
                &format!("adapter_input/{id}"),
                &serde_json::to_vec(&wire).unwrap(),
            )
            .unwrap();
        let outcome = session
            .submit_pty(
                &arbiter(&session),
                1000,
                id,
                "must not write",
                &journal,
                |_| {
                    panic!("journal replay called the PTY writer");
                },
            )
            .unwrap();
        assert_eq!(outcome.evidence, Evidence::NativeUnknown);
        assert_eq!(outcome.note, "retained ambiguous write");
        assert!(!session.can_attempt_native(id));
        let native = runtime.block_on(session.wake(
            &attachment,
            id,
            "must not resend",
            Priority::Next,
            false,
        ));
        assert_eq!(native.evidence, Evidence::NativeUnknown);
        assert_eq!(native.cause, Some(WakeCause::PreviousOutcomeUnknown));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        // Mux and kit retain the same legacy absent-cause byte contract.
        let legacy = r#"{"evidence":"native-unknown","native_id":null,"note":"opaque"}"#;
        assert_eq!(
            serde_json::to_string(&serde_json::from_str::<WakeOutcome>(legacy).unwrap()).unwrap(),
            legacy
        );
    }
}

// keeps: interface app-harness — every returned unknown fences the same delivery ID.
#[test]
fn wake_causes_failed_result_journal_fences_same_id_native_and_pty() {
    use bbt_mux::adapters::{Endpoint, Priority, WakeCause, WakeOutcome};
    use bbt_mux::store::Result;
    use std::cell::Cell;
    struct FailResultJournal {
        inner: SqliteStore,
        puts: Cell<usize>,
    }
    impl Store for FailResultJournal {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            self.inner.get(key)
        }
        fn put(&self, key: &str, value: &[u8]) -> Result<()> {
            self.puts.set(self.puts.get() + 1);
            if self.puts.get() == 2 {
                Err(bbt_mux::store::invalid(
                    "controlled final journal write failure",
                ))
            } else {
                self.inner.put(key, value)
            }
        }
        fn list(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
            self.inner.list(prefix)
        }
        fn sql(
            &self,
            query: &str,
            args: &[rusqlite::types::Value],
        ) -> Result<Vec<Vec<rusqlite::types::Value>>> {
            self.inner.sql(query, args)
        }
        fn alarm(&self, key: &str, deadline_ms: u64) -> Result<()> {
            self.inner.alarm(key, deadline_ms)
        }
        fn due_alarms(&self, now_ms: u64) -> Result<Vec<String>> {
            self.inner.due_alarms(now_ms)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("native.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut session = session("claude");
    let attachment = runtime
        .block_on(session.attach(Endpoint::Claude { path }))
        .unwrap();
    drop(listener.accept().unwrap()); // Only the read-only attachment probe.
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
    let journal = FailResultJournal {
        inner: SqliteStore::open(&root.path().join("journal")).unwrap(),
        puts: Cell::new(0),
    };
    let mut writes = 0;
    let outcome = session
        .submit_pty(&view, 1600, id, "write once", &journal, |bytes| {
            writes += 1;
            let pending: WakeOutcome = serde_json::from_slice(
                &journal
                    .get(&format!("adapter_input/{id}"))
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(pending.evidence, Evidence::NativeUnknown);
            assert_eq!(pending.cause, Some(WakeCause::PtySubmissionPending));
            Ok(bbt_mux::InputReceipt {
                input_id: id,
                state: InputState::PtyWritten,
                bytes_written: bytes.len(),
                bytes_total: bytes.len(),
            })
        })
        .unwrap();
    assert_eq!(outcome.evidence, Evidence::NativeUnknown);
    assert_eq!(outcome.cause, Some(WakeCause::PtyResultUnjournaled));
    assert_eq!(writes, 1);
    assert_eq!(journal.puts.get(), 2);
    assert!(
        !session.can_attempt_native(id),
        "returned unknown did not fence native retry: {outcome:?}"
    );
    let native =
        runtime.block_on(session.wake(&attachment, id, "must not resend", Priority::Next, false));
    assert_eq!(native.evidence, Evidence::NativeUnknown);
    assert_eq!(native.cause, Some(WakeCause::PreviousOutcomeUnknown));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let replay = session
        .submit_pty(&view, 1800, id, "write once", &journal, |_| {
            panic!("second PTY write")
        })
        .unwrap();
    assert_eq!(replay.evidence, Evidence::NativeUnknown);
    assert_eq!(journal.puts.get(), 2);
    assert_eq!(writes, 1);
}
