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
    tokio::fs::write(
        &executable,
        format!("#!/bin/sh\nBBOXY_TEST_CUSTODY='fixture:{custody}' exec '{binary}' \"$@\"\n"),
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
                harness_kind: "zsh".into(),
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
    tokio::time::timeout(Duration::from_secs(10), async {
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
    let changed = tokio::time::timeout(Duration::from_secs(10), async {
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
            tokio::time::timeout(Duration::from_secs(3), read_line(&mut reader))
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
        let _ = tokio::time::timeout(Duration::from_secs(3), write.write_all(&bytes))
            .await
            .unwrap();
        let _ = write.shutdown().await;
        let result =
            tokio::time::timeout(Duration::from_secs(3), read_line(&mut BufReader::new(read)))
                .await
                .unwrap();
        assert!(matches!(result, Ok(None) | Err(_)));
    }
    session.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn app_io_owner_observes_both_child_process_exits() {
    let root = tempfile::Builder::new().prefix("c1a-").tempdir().unwrap();
    let mut session = start(root.path()).await;
    session.logic.start_kill().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
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
    tokio::time::timeout(Duration::from_secs(5), async {
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
    assert!(
        tokio::time::timeout(Duration::from_millis(50), session.event())
            .await
            .is_err()
    );
    session.shutdown().await.unwrap();
    assert!(
        !tokio::fs::try_exists(root.path().join("run/mux.sock"))
            .await
            .unwrap()
    );
}

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
