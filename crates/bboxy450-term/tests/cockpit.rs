#[path = "support/events.rs"]
mod events;
use events::StateEvents;
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};

use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::{Ulid, seat_socket::read_line};
use bbt_proto::{
    Activity, GuiAction, GuiReply, GuiRequest, GuiSeat, ProcessStatus, SpawnSpec, SwarmId, Where,
};
use tokio::io::{AsyncWriteExt, BufReader};

async fn start(root: &Path) -> Session {
    let repository = root.join("repository");
    tokio::fs::create_dir_all(&repository).await.unwrap();
    let shell_config = root.join("zsh-config");
    tokio::fs::create_dir_all(&shell_config).await.unwrap();
    // Keep this process fixture off the user's Keychain. A per-child wrapper
    // leaves the environment of parallel Rust tests untouched.
    let executable = root.join("test-app");
    let binary = env!("CARGO_BIN_EXE_bboxy450-term").replace('\'', "'\\''");
    let custody = root.join("fixture-custody");
    let custody = custody.to_string_lossy().replace('\'', "'\\''");
    let home = root.join("home");
    tokio::fs::create_dir_all(&home).await.unwrap();
    let home = home.to_string_lossy().replace('\'', "'\\''");
    tokio::fs::write(
        &executable,
        format!("#!/bin/sh\nHOME='{home}' CLAUDE_CONFIG_DIR='{home}/.claude' CODEX_HOME='{home}/.codex' BBOXY_TEST_CUSTODY='fixture:{custody}' exec '{binary}' \"$@\"\n"),
    )
    .await
    .unwrap();
    tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
    Session::start(Config {
        executable,
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository,
        shell_config: Some(shell_config),
        ctl: false,
        test_account: None,
    })
    .await
    .unwrap()
}

fn pid(seat: &GuiSeat) -> u32 {
    match seat.state.process {
        ProcessStatus::Running { pid } => pid,
        _ => panic!("seat is not running"),
    }
}
async fn spawn(session: &mut Session, root: &Path, swarm: Ulid) -> GuiSeat {
    spawn_kind(session, root, swarm, "zsh").await
}
async fn spawn_kind(session: &mut Session, root: &Path, swarm: Ulid, kind: &str) -> GuiSeat {
    let known: Vec<_> = session
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|seat| seat.record.id)
        .collect();
    events::attach(session).await.unwrap();
    session
        .send(GuiAction::Spawn {
            id: bbt_mux::Ulid::new(),
            spec: SpawnSpec {
                swarm: SwarmId::from_u128(swarm.0),
                harness_kind: kind.into(),
                name: Some("test zsh".into()),
                role: None,
                cwd: root.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: None,
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.receive_state().await.unwrap() {
                GuiReply::Seat(seat) if !known.contains(&seat.record.id) => break seat,
                GuiReply::Seat(_)
                | GuiReply::Spawned { .. }
                | GuiReply::Ok { .. }
                | GuiReply::Hosts(_)
                | GuiReply::HostState { .. } => {}
                other => panic!("unexpected spawn response: {other:?}"),
            }
        }
    })
    .await
    .unwrap()
}

// keeps: mux-daemon GUI independence; normal close preserves work, hard quit ends it.
#[ignore = "night: real process boundary; run explicitly for lifecycle changes"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::disallowed_methods, reason = "private process fixture; no UI thread")]
async fn normal_detach_keeps_work_and_window_state_until_hard_quit() {
    async fn file(path: &Path) -> String {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(text) = tokio::fs::read_to_string(path).await
                    && !text.is_empty()
                {
                    return text;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.unwrap()
    }
    let root = tempfile::tempdir().unwrap();
    let mut session = start(root.path()).await;
    let supervisor = session.supervisor.id().unwrap();
    let logic = session.logic.id().unwrap();
    let swarm = Ulid::new();
    let seat = spawn(&mut session, root.path(), swarm).await;
    let shell = pid(&seat);
    let before = root.path().join("before");
    session.send(GuiAction::Input {
        seat: bbt_proto::SeatId::from_u128(seat.record.id.0), generation: seat.record.generation, id: Ulid::new(),
        bytes: format!("KEEP_WORK=575931; printf '%s\\n' $$ > '{}'\r", before.display()).into_bytes(),
    }).await.unwrap();
    assert_eq!(file(&before).await.trim(), shell.to_string());
    let seat_id = seat.record.id;
    let tab = Ulid::new();
    let state: bbt_proto::gui::WindowState = serde_json::from_value(serde_json::json!({
        "version":1, "size":[1440.0,900.0], "sidebar_width":280.0,
        "sidebar_scroll":12.0, "sidebar_shown":true, "open_swarms":[swarm],
        "swarms":[{"id":swarm,"tabs":[{"id":tab,"name":"work","shortcut":1,
            "expanded":true,"seats":[seat_id],"tree":{"kind":"seat","id":seat_id}}]}],
        "stage":{"swarm":swarm,"tab":tab,"focused":seat_id},
        "scroll":[{"seat":seat_id,"top_row":23,"live":false}]
    })).unwrap();
    session.detach(Some(state.clone())).await.unwrap();
    let mut reattached = start(root.path()).await;
    assert_eq!(reattached.supervisor.id(), Some(supervisor));
    assert_eq!(reattached.logic.id(), Some(logic));
    assert_eq!(reattached.endpoint.window.as_ref().unwrap().state.as_ref(), Some(&state));
    let restored = reattached.list().await.unwrap().into_iter()
        .find(|row| row.record.id == seat.record.id).unwrap();
    assert_eq!(restored.record.generation, seat.record.generation);
    assert_eq!(pid(&restored), shell);
    let after = root.path().join("after");
    reattached.send(GuiAction::Input {
        seat: bbt_proto::SeatId::from_u128(seat.record.id.0), generation: seat.record.generation, id: Ulid::new(),
        bytes: format!("printf '%s %s\\n' \"$KEEP_WORK\" $$ > '{}'\r", after.display()).into_bytes(),
    }).await.unwrap();
    assert_eq!(file(&after).await.trim(), format!("575931 {shell}"));
    reattached.shutdown().await.unwrap();
    for owner in [supervisor, logic, shell] {
        assert!(!std::process::Command::new("/bin/ps")
            .args(["-p", &owner.to_string(), "-o", "pid="])
            .output().unwrap().status.success(), "hard quit kept process {owner}");
    }
}

// keeps: Finder launch PATH and mux-daemon UI independence.
#[ignore = "night: real zsh/backend/PTY boundary; run explicitly for PATH changes"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::disallowed_methods, reason = "private launch fixture, never UI")]
async fn changed_login_path_refreshes_logic_and_launches_through_a_retained_supervisor() {
    async fn file(path: &Path) -> String {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(text) = tokio::fs::read_to_string(path).await
                    && !text.is_empty() { return text; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.unwrap()
    }
    let root = tempfile::tempdir().unwrap();
    let shell = root.path().join("zsh-config");
    tokio::fs::create_dir_all(&shell).await.unwrap();
    tokio::fs::write(shell.join(".zshrc"), "export PATH=/usr/bin:/bin\n").await.unwrap();
    let mut old = start(root.path()).await;
    let supervisor = old.supervisor.id().unwrap();
    let logic = old.logic.id().unwrap();
    let swarm = Ulid::new();
    let running = spawn(&mut old, root.path(), swarm).await;
    let cookie = root.path().join("cookie");
    old.send(GuiAction::Input {
        seat: bbt_proto::SeatId::from_u128(running.record.id.0),
        generation: running.record.generation, id: Ulid::new(),
        bytes: format!("KEEP_PATH_WORK=kept; printf '%s' $$ > '{}'\r", cookie.display()).into_bytes(),
    }).await.unwrap();
    assert_eq!(file(&cookie).await, pid(&running).to_string());
    old.detach(None).await.unwrap();

    let bin = root.path().join("login-only-bin");
    tokio::fs::create_dir(&bin).await.unwrap();
    let launched = root.path().join("claude-started");
    let executable = bin.join("claude");
    tokio::fs::write(&executable, format!(
        "#!/bin/sh\nprintf '%s' \"$PATH\" > '{}'\nexec /bin/cat\n", launched.display()
    )).await.unwrap();
    tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).await.unwrap();
    tokio::fs::write(shell.join(".zshrc"), format!(
        "export PATH='{}:/usr/bin:/bin'\n", bin.display()
    )).await.unwrap();

    let mut refreshed = start(root.path()).await;
    assert_eq!(refreshed.supervisor.id(), Some(supervisor));
    assert_ne!(refreshed.logic.id(), Some(logic));
    let current = refreshed.list().await.unwrap().into_iter()
        .find(|row| row.record.id == running.record.id).unwrap();
    assert_eq!(pid(&current), pid(&running));
    assert_eq!(current.record.generation, running.record.generation);
    let new = spawn_kind(&mut refreshed, root.path(), swarm, "claude").await;
    assert!(pid(&new) > 0);
    assert_eq!(file(&launched).await, format!("{}:/usr/bin:/bin", bin.display()));
    let kept = root.path().join("kept");
    refreshed.send(GuiAction::Input {
        seat: bbt_proto::SeatId::from_u128(running.record.id.0),
        generation: running.record.generation, id: Ulid::new(),
        bytes: format!("printf '%s:%s' \"$KEEP_PATH_WORK\" $$ > '{}'\r", kept.display()).into_bytes(),
    }).await.unwrap();
    assert_eq!(file(&kept).await, format!("kept:{}", pid(&running)));
    refreshed.shutdown().await.unwrap();
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn real_processes_list_spawn_and_push_unwatched_exit_state_on_one_gui_connection() {
    let root = tempfile::Builder::new().prefix("c1a-").tempdir().unwrap();
    let mut session = start(root.path()).await;
    assert!(session.list().await.unwrap().is_empty());
    let swarm = Ulid::new();
    let first = spawn(&mut session, root.path(), swarm).await;
    let second = spawn(&mut session, root.path(), swarm).await;
    assert_ne!(first.record.id, second.record.id);
    assert!(pid(&first) > 0 && pid(&second) > 0);
    let listed = session.list().await.unwrap();
    assert_eq!(listed.len(), 2);
    for seat in listed {
        let wire = serde_json::to_value(seat).unwrap();
        assert!(wire.get("screen").is_none() && wire.get("cells").is_none());
    }
    let status = tokio::process::Command::new("/bin/kill")
        .args(["-KILL", &pid(&first).to_string()])
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let changed = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let GuiReply::Seat(seat) = session.receive_state().await.unwrap()
                && seat.record.id == first.record.id
                && matches!(seat.state.process, ProcessStatus::Exited { .. })
            {
                break seat;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(changed.state.activity, Activity::Errored);
    assert!(matches!(
        changed.state.process,
        ProcessStatus::Exited { status } if status!=0
    ));
    let listed = session.list().await.unwrap();
    assert!(listed.iter().any(|seat| seat.record.id == second.record.id
        && matches!(seat.state.process, ProcessStatus::Running { .. })));
    let endpoint_root = session.endpoint.root.clone();
    session.shutdown().await.unwrap();
    assert!(!tokio::fs::try_exists(endpoint_root).await.unwrap());
    assert!(
        !tokio::fs::try_exists(root.path().join("run/mux.sock"))
            .await
            .unwrap()
    );
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn gui_endpoint_keeps_private_modes_and_rejects_bad_admission_and_frames() {
    let root = tempfile::Builder::new().prefix("c1a-").tempdir().unwrap();
    let session = start(root.path()).await;
    let path = session.endpoint.root.join("gui.sock");
    assert_eq!(
        tokio::fs::metadata(&session.endpoint.root)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        tokio::fs::metadata(&path)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    for (version, capability) in [
        (bbt_proto::gui::VERSION - 1, session.endpoint.capability),
        (bbt_proto::gui::VERSION, [0; 32]),
    ] {
        let stream = tokio::net::UnixStream::connect(&path).await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version,
            capability,
            action: GuiAction::ListSeats,
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        let mut reader = BufReader::new(read);
        // Each refusal names itself, then the socket closes: a wrong capability says
        // admission was rejected, as a wrong version says it mismatched.
        let expected = if capability == session.endpoint.capability {
            "GUI socket version mismatch"
        } else {
            "GUI socket admission rejected"
        };
        let reply = read_line(&mut reader).await.unwrap().unwrap();
        assert!(matches!(
            serde_json::from_slice::<GuiReply>(&reply).unwrap(),
            GuiReply::Error { reason, .. } if reason == expected
        ));
        assert!(
            tokio::time::timeout(Duration::from_secs(30), read_line(&mut reader))
                .await
                .unwrap()
                .unwrap()
                .is_none()
        );
    }
    // Admission is decoded before the action: a v1 Spawn has no op id and
    // cannot deserialize as the current action, but still gets the named refusal.
    let stream = tokio::net::UnixStream::connect(&path).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut bytes = serde_json::to_vec(&serde_json::json!({
        "version": bbt_proto::gui::VERSION - 1,
        "capability": session.endpoint.capability,
        "action": {"kind":"Spawn","body":{"spec":{}}}
    }))
    .unwrap();
    bytes.push(b'\n');
    write.write_all(&bytes).await.unwrap();
    let mut reader = BufReader::new(read);
    let reply = read_line(&mut reader).await.unwrap().unwrap();
    assert!(matches!(
        serde_json::from_slice::<GuiReply>(&reply).unwrap(),
        GuiReply::Error { reason, .. } if reason == "GUI socket version mismatch"
    ));
    assert!(read_line(&mut reader).await.unwrap().is_none());
    for bytes in [b"{".to_vec(), vec![b'x'; 1024 * 1024 + 1]] {
        let stream = tokio::net::UnixStream::connect(&path).await.unwrap();
        let (read, mut write) = stream.into_split();
        let _ = tokio::time::timeout(Duration::from_secs(30), write.write_all(&bytes))
            .await
            .unwrap();
        let _ = write.shutdown().await;
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            read_line(&mut BufReader::new(read)),
        )
        .await
        .unwrap();
        assert!(matches!(result, Ok(None) | Err(_)));
    }
    session.shutdown().await.unwrap();
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn app_io_owner_observes_both_child_process_exits() {
    let root = tempfile::Builder::new().prefix("c1a-").tempdir().unwrap();
    let mut session = start(root.path()).await;
    session.logic.start_kill().unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if matches!(
                session.event().await.unwrap(),
                Event::Exited {
                    process: "logic",
                    ..
                }
            ) {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(session.supervisor.try_wait().unwrap().is_none());
    session.supervisor.start_kill().unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if matches!(
                session.event().await.unwrap(),
                Event::Exited {
                    process: "supervisor",
                    ..
                }
            ) {
                break;
            }
        }
    })
    .await
    .unwrap();
    session.shutdown().await.unwrap();
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn cancelled_exit_watch_preserves_orderly_child_shutdown() {
    let root = tempfile::Builder::new()
        .prefix("c1a-watch-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    assert!(session.list().await.unwrap().is_empty());
    // The live IO loop repeatedly cancels this wait when another command wins.
    use std::{
        future::Future,
        task::{Context, Waker},
    };
    assert!(
        std::pin::pin!(session.event())
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    session.shutdown().await.unwrap();
    assert!(
        !tokio::fs::try_exists(root.path().join("run/mux.sock"))
            .await
            .unwrap()
    );
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn same_run_directory_relaunches_and_spawns_after_orderly_shutdown() {
    let root = tempfile::Builder::new()
        .prefix("c1a-relaunch-")
        .tempdir()
        .unwrap();
    let swarm = Ulid::new();
    for _ in 0..2 {
        let mut session = start(root.path()).await;
        let seat = spawn(&mut session, root.path(), swarm).await;
        assert!(pid(&seat) > 0);
        session.shutdown().await.unwrap();
        assert!(
            !tokio::fs::try_exists(root.path().join("run/mux.sock"))
                .await
                .unwrap()
        );
    }
}
