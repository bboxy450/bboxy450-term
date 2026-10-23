//! Real private-process recovery, with isolated custody and fixture harnesses.
#![allow(
    clippy::disallowed_methods,
    reason = "headless integration tests own their Tokio runtimes"
)]

use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::{Ulid, streams::Replica};
use bbt_proto::{
    GuiAction, GuiReply, GuiSeat, ProcessStatus, Screen, SeatId, SpawnSpec, SwarmId, Update, View,
    Where,
};
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};

async fn script(path: &Path, text: String) {
    tokio::fs::write(path, text).await.unwrap();
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
}
async fn start(root: &Path) -> Session {
    tokio::fs::create_dir_all(root.join("repository"))
        .await
        .unwrap();
    tokio::fs::create_dir_all(root.join("zsh-config"))
        .await
        .unwrap();
    // Fixture binaries record actual argv, then act as PTY readers. No native
    // Claude/Codex session, account or approval store is touched.
    for harness in ["claude", "codex"] {
        script(&root.join(harness), format!("#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\nprintf 'fixture-ready\\n'\nwhile IFS= read -r line; do printf 'fixture: %s\\n' \"$line\"; done\n", root.join(format!("{harness}-argv")).display())).await;
    }
    let executable = root.join("test-app");
    script(
        &executable,
        format!(
            "#!/bin/sh\nPATH='{}':\"$PATH\" BBOXY_TEST_CUSTODY='fixture:{}' exec '{}' \"$@\"\n",
            root.display(),
            root.join("fixture-custody").display(),
            env!("CARGO_BIN_EXE_bboxy450-term")
        ),
    )
    .await;
    Session::start(Config {
        executable,
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository: root.join("repository"),
        shell_config: Some(root.join("zsh-config")),
    })
    .await
    .unwrap()
}
fn pid(seat: &GuiSeat) -> u32 {
    match seat.state.process {
        ProcessStatus::Running { pid } => pid,
        _ => panic!("not running: {seat:?}"),
    }
}
fn id(seat: &GuiSeat) -> SeatId {
    SeatId::from_u128(seat.record.id.0)
}
async fn spawn(session: &mut Session, root: &Path, harness: &str, resume: Option<&str>) -> GuiSeat {
    let known: Vec<_> = session
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.record.id)
        .collect();
    session
        .send(GuiAction::Spawn {
            id: bbt_mux::Ulid::new(),
            spec: SpawnSpec {
                swarm: SwarmId::from_u128(57),
                harness_kind: harness.into(),
                name: Some(harness.into()),
                role: None,
                cwd: root.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: resume.map(str::to_owned),
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Seat(seat) if !known.contains(&seat.record.id) => return seat,
                GuiReply::Error { reason, .. } => panic!("spawn: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}
async fn watch(session: &mut Session, seat: &GuiSeat) {
    session
        .send(GuiAction::Watch {
            seat: id(seat),
            view: View {
                seat: seat.record.id,
                generation: seat.record.generation,
                view_generation: Ulid::nil(),
            },
        })
        .await
        .unwrap();
}
async fn update(session: &mut Session, seat: SeatId) -> Update {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Update { seat: got, update } if got == seat => return update,
                GuiReply::Error { reason, .. } => panic!("GUI: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}
fn cursor(update: &Update) -> (View, u64) {
    match update {
        Update::Snapshot { snapshot } => (snapshot.view, snapshot.sequence),
        Update::Delta { delta } => (delta.view, delta.sequence),
    }
}
async fn apply(session: &mut Session, seat: SeatId, replica: &mut Replica, update: Update) {
    let (view, sequence) = cursor(&update);
    replica.apply(update).unwrap();
    // The Replica's new state is available before acknowledging, as in IO.
    session
        .send(GuiAction::Applied {
            seat,
            view,
            sequence,
        })
        .await
        .unwrap();
}
fn text(screen: &Screen) -> String {
    screen.cells.iter().flatten().map(|c| c.c).collect()
}
async fn input(session: &mut Session, seat: SeatId, generation: u64, bytes: &[u8]) {
    session
        .send(GuiAction::Input {
            seat,
            generation,
            id: Ulid::new(),
            bytes: bytes.into(),
        })
        .await
        .unwrap();
}
async fn until_text(session: &mut Session, seat: SeatId, replica: &mut Replica, wanted: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let next = update(session, seat).await;
            apply(session, seat, replica, next).await;
            if replica
                .snapshot
                .as_ref()
                .unwrap()
                .screen
                .cells
                .iter()
                .any(|row| {
                    let line: String = row.iter().map(|cell| cell.c).collect();
                    line.trim() == wanted
                })
            {
                break;
            }
        }
    })
    .await
    .unwrap();
}
async fn restarted(session: &mut Session) -> Vec<GuiSeat> {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Event::Reply(reply) = session.event().await.unwrap()
                && let GuiReply::Seats(seats) = *reply
            {
                return seats;
            }
        }
    })
    .await
    .unwrap()
}
async fn durable(root: &Path, seat: &GuiSeat, sequence: u64) {
    let path = root.join(format!(
        "run/seat-{}/ring-{}/committed",
        seat.record.id, seat.record.generation
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(bytes) = tokio::fs::read(&path).await
                && bytes.len() > 4
                && let Ok(boundary) =
                    serde_json::from_slice::<serde_json::Value>(&bytes[..bytes.len() - 4])
                && boundary["sequence"].as_u64().is_some_and(|n| n >= sequence)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logic_death_keeps_pty_generation_rewatches_and_preserves_ack_backpressure() {
    let root = tempfile::Builder::new()
        .prefix("c1c-logic-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let seat = spawn(&mut session, root.path(), "zsh", None).await;
    let unwatched = spawn(&mut session, root.path(), "zsh", None).await;
    let supervisor_pid = session.supervisor.id();
    let logic_pid = session.logic.id();
    watch(&mut session, &seat).await;
    let mut replica = Replica::default();
    input(
        &mut session,
        id(&seat),
        seat.record.generation,
        b"printf 'before-restart\\n'\n",
    )
    .await;
    until_text(&mut session, id(&seat), &mut replica, "before-restart").await;
    let previous_view = replica.snapshot.as_ref().unwrap().view;
    session.logic.start_kill().unwrap();
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
    // Cancelling polling must not cancel the stored startup task.
    let mut early_catalogue = None;
    for _ in 0..3 {
        if let Ok(Ok(Event::Reply(reply))) =
            tokio::time::timeout(Duration::from_millis(10), session.event()).await
            && let GuiReply::Seats(seats) = *reply
        {
            early_catalogue = Some(seats);
            break;
        }
    }
    let seats = match early_catalogue {
        Some(seats) => seats,
        None => restarted(&mut session).await,
    };
    let restored = seats
        .iter()
        .find(|s| s.record.id == seat.record.id)
        .unwrap();
    assert_eq!(seats.len(), 2);
    let restored_unwatched = seats
        .iter()
        .find(|item| item.record.id == unwatched.record.id)
        .unwrap();
    assert_eq!(pid(restored_unwatched), pid(&unwatched));
    assert_eq!(
        restored_unwatched.record.generation,
        unwatched.record.generation
    );
    assert_eq!(pid(restored), pid(&seat));
    assert_eq!(restored.record.generation, seat.record.generation);
    assert_eq!(session.supervisor.id(), supervisor_pid);
    assert_ne!(session.logic.id(), logic_pid);
    let first = update(&mut session, id(&seat)).await;
    let (view, _) = cursor(&first);
    assert_eq!(view.generation, previous_view.generation);
    assert_ne!(view.view_generation, previous_view.view_generation);
    assert!(matches!(first, Update::Snapshot { .. }));
    input(
        &mut session,
        id(&seat),
        seat.record.generation,
        b"printf 'after-restart\\n'\n",
    )
    .await;
    let window = tokio::time::timeout(Duration::from_millis(250), async {
        loop {
            assert!(
                !matches!(session.receive().await.unwrap(), GuiReply::Update { .. }),
                "second update before Applied"
            );
        }
    })
    .await;
    assert!(window.is_err());
    apply(&mut session, id(&seat), &mut replica, first).await;
    until_text(&mut session, id(&seat), &mut replica, "after-restart").await;
    println!(
        "logic recovery: seat={} pid={} generation={} supervisor={:?} logic {:?}->{:?}; fresh view; 250ms ack hold",
        seat.record.id,
        pid(&seat),
        seat.record.generation,
        supervisor_pid,
        logic_pid,
        session.logic.id()
    );
    session.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_death_replays_ring_with_gap_same_ids_new_generation_and_native_resume_argv() {
    let root = tempfile::Builder::new()
        .prefix("c1c-super-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let zsh = spawn(&mut session, root.path(), "zsh", None).await;
    let claude = spawn(
        &mut session,
        root.path(),
        "claude",
        Some("fixture-claude-session"),
    )
    .await;
    let codex = spawn(
        &mut session,
        root.path(),
        "codex",
        Some("fixture-codex-thread"),
    )
    .await;
    watch(&mut session, &zsh).await;
    let mut replica = Replica::default();
    input(
        &mut session,
        id(&zsh),
        zsh.record.generation,
        b"printf 'durable-before-supervisor\\n'\n",
    )
    .await;
    until_text(
        &mut session,
        id(&zsh),
        &mut replica,
        "durable-before-supervisor",
    )
    .await;
    session
        .send(GuiAction::Resize {
            seat: id(&zsh),
            generation: zsh.record.generation,
            cols: 100,
            rows: 30,
        })
        .await
        .unwrap();
    loop {
        let next = update(&mut session, id(&zsh)).await;
        apply(&mut session, id(&zsh), &mut replica, next).await;
        if replica.snapshot.as_ref().unwrap().screen.cols == 100 {
            break;
        }
    }
    durable(
        root.path(),
        &zsh,
        replica.snapshot.as_ref().unwrap().sequence,
    )
    .await;
    let old_supervisor = session.supervisor.id();
    let old_logic = session.logic.id();
    // These actions were formed against the displayed old process and held
    // across the outage. Neither may reach the replacement process.
    let stale_id = Ulid::new();
    let stale_file = root.path().join("stale-input-ran");
    let stale_input = GuiAction::Input {
        seat: id(&zsh),
        generation: zsh.record.generation,
        id: stale_id,
        bytes: format!(": > '{}'\n", stale_file.display()).into_bytes(),
    };
    let stale_resize = GuiAction::Resize {
        seat: id(&zsh),
        generation: zsh.record.generation,
        cols: 77,
        rows: 7,
    };
    session.supervisor.start_kill().unwrap();
    let seats = restarted(&mut session).await;
    assert_eq!(seats.len(), 3);
    for old in [&zsh, &claude, &codex] {
        let new = seats.iter().find(|s| s.record.id == old.record.id).unwrap();
        assert_eq!(new.record.generation, old.record.generation + 1);
        assert_ne!(pid(new), pid(old));
        assert_eq!(new.record.harness_session_id, old.record.harness_session_id);
    }
    assert_ne!(session.supervisor.id(), old_supervisor);
    assert_ne!(session.logic.id(), old_logic);
    let next = update(&mut session, id(&zsh)).await;
    assert!(matches!(next, Update::Snapshot { .. }));
    apply(&mut session, id(&zsh), &mut replica, next).await;
    let screen = &replica.snapshot.as_ref().unwrap().screen;
    assert_eq!((screen.cols, screen.rows), (100, 30));
    assert!(text(screen).contains("durable-before-supervisor"));
    let gap = screen.gap.as_ref().unwrap();
    assert!(
        gap.contains("generation 1") && gap.contains("uncommitted output unavailable"),
        "{gap}"
    );
    for (harness, flag, session_id) in [
        ("claude", "--resume", "fixture-claude-session"),
        ("codex", "resume", "fixture-codex-thread"),
    ] {
        let argv = tokio::fs::read_to_string(root.path().join(format!("{harness}-argv")))
            .await
            .unwrap();
        let args: Vec<_> = argv.lines().collect();
        assert_eq!(
            args.windows(2)
                .filter(|pair| pair == &[flag, session_id])
                .count(),
            2,
            "{argv}"
        );
    }
    session.send(stale_input).await.unwrap();
    session.send(stale_resize).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        for dropped in 1..=2 {
            loop {
                match session.receive().await.unwrap() {
                    GuiReply::Error { op, reason } => {
                        if dropped == 1 {
                            assert_eq!(op, stale_id, "Input rejection lost its operation ID");
                        }
                        assert!(reason.contains("StaleGeneration"), "{reason}");
                        assert!(
                            reason.contains("displayed generation 1, current 2"),
                            "{reason}"
                        );
                        assert!(reason.contains(&format!("dropped={dropped}")), "{reason}");
                        break;
                    }
                    GuiReply::Update { seat, update } => {
                        apply(&mut session, seat, &mut replica, update).await;
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .unwrap();
    input(
        &mut session,
        id(&zsh),
        replica.snapshot.as_ref().unwrap().view.generation,
        b"printf 'echo-after-supervisor\\n'; stty size\n",
    )
    .await;
    until_text(&mut session, id(&zsh), &mut replica, "30 100").await;
    assert!(!tokio::fs::try_exists(stale_file).await.unwrap());
    let screen = &replica.snapshot.as_ref().unwrap().screen;
    assert_eq!((screen.cols, screen.rows), (100, 30));
    assert!(text(screen).contains("echo-after-supervisor"));
    println!(
        "supervisor recovery: 3 stable seat IDs, gen1->2, zsh/Claude/Codex new PIDs; 100x30, retained screen, explicit gap, actual resume argv, post-restart echo; stale Input ID preserved, drops1/2 counted, stale input file absent, stale resize rejected at PTY30x100"
    );
    session.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_during_pending_recovery_reaps_children_without_restarting() {
    let root = tempfile::Builder::new()
        .prefix("c1c-stop-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let seat = spawn(&mut session, root.path(), "zsh", None).await;
    session.logic.start_kill().unwrap();
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
    session.shutdown().await.unwrap();
    assert!(
        !tokio::fs::try_exists(root.path().join("run/mux.sock"))
            .await
            .unwrap()
    );
    let status = tokio::process::Command::new("/bin/ps")
        .args(["-p", &pid(&seat).to_string(), "-o", "comm="])
        .output()
        .await
        .unwrap();
    assert!(!status.status.success(), "owned seat survived shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partial_startup_survives_cancelled_poll_and_shutdown_reaps_its_child() {
    let root = tempfile::Builder::new()
        .prefix("c1c-handshake-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    // Replace only this fixture's executable. Its next logic child records its
    // own PID, consumes bootstrap, then holds readiness on the inherited pipe.
    script(&root.path().join("test-app"), format!(
        "#!/bin/sh\nif [ \"$1\" = logic ]; then\n  echo $$ > '{}'\n  read bootstrap\n  read stop\n  exit 0\nfi\nBBOXY_TEST_CUSTODY=memory exec '{}' \"$@\"\n",
        root.path().join("pending-pid").display(), env!("CARGO_BIN_EXE_bboxy450-term")
    )).await;
    session.logic.start_kill().unwrap();
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
    let pending_pid = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = tokio::fs::read_to_string(root.path().join("pending-pid")).await {
                break pid.trim().to_owned();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(20), session.event())
            .await
            .is_err()
    );
    // Known loss returns promptly; the owner can retain this stable-ID command
    // and its independent close channel remains selectable during startup.
    assert!(
        tokio::time::timeout(
            Duration::from_millis(20),
            session.send(GuiAction::ListSeats)
        )
        .await
        .unwrap()
        .is_err()
    );
    session.shutdown().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = tokio::process::Command::new("/bin/ps")
                .args(["-p", &pending_pid, "-o", "comm="])
                .output()
                .await
                .unwrap();
            if !status.status.success() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    println!(
        "cancelled readiness future retained startup; shutdown reaped recorded replacement PID {pending_pid}"
    );
}

async fn raw_supervisor(root: &Path) -> (tokio::process::Child, bbt_mux::logic::SocketSupervisor) {
    use tokio::io::{AsyncWriteExt, BufReader};
    #[derive(serde::Deserialize)]
    struct Ready {
        socket: std::path::PathBuf,
        capability: [u8; 32],
    }
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("supervisor")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let config = Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository: root.into(),
        shell_config: None,
    };
    let mut bytes = serde_json::to_vec(&config).unwrap();
    bytes.push(b'\n');
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
    let line = tokio::time::timeout(
        Duration::from_secs(5),
        bbt_mux::seat_socket::read_line(&mut BufReader::new(child.stdout.take().unwrap())),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let ready: Ready = serde_json::from_slice(&line).unwrap();
    let client =
        bbt_mux::logic::SocketSupervisor::connect(&ready.socket, ready.capability).unwrap();
    (child, client)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovered_generation_admission_rejects_unknown_and_conflicting_launches_but_exact_retry_is_idempotent()
 {
    use bbt_mux::{Action, Reply, logic::SupervisorConnection};
    use tokio::io::AsyncWriteExt;
    let root = tempfile::Builder::new()
        .prefix("c1c-admit-")
        .tempdir()
        .unwrap();
    let (mut first, mut client) = raw_supervisor(root.path()).await;
    let mut spec = bbt_mux::LaunchSpec::new("/bin/cat", root.path());
    let info = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("spawn mismatch"),
    };
    let attachment = match client
        .request(Action::AttachRing {
            seat: info.seat,
            generation: 1,
            directory: root.path().join(format!("run/seat-{}/ring-1", info.seat)),
            settings: Default::default(),
        })
        .unwrap()
    {
        Reply::Attached { attachment, .. } => attachment,
        _ => panic!("attach mismatch"),
    };
    client
        .request(Action::Input {
            attachment: attachment.clone(),
            input_id: Ulid::new(),
            bytes: b"durable-generation\n".to_vec(),
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Reply::RingRecovery { status, .. } = client
                .request(Action::RingRecovery {
                    attachment: attachment.clone(),
                })
                .unwrap()
                && status.committed_sequence > 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    first.start_kill().unwrap();
    first.wait().await.unwrap();
    drop(client);
    let predecessor = root.path().join(format!("run/seat-{}/ring-1", spec.seat));
    // Recovery must also work with a valid previous boundary and a torn current
    // marker, using Ring's existing validation/repair rather than a new format.
    tokio::fs::copy(predecessor.join("committed"), predecessor.join("previous"))
        .await
        .unwrap();
    tokio::fs::write(predecessor.join("committed"), b"torn marker")
        .await
        .unwrap();
    let (mut second, mut client) = raw_supervisor(root.path()).await;
    assert!(
        client
            .request(Action::Spawn { spec: spec.clone() })
            .is_err(),
        "recovered old generation was reused"
    );
    let mut gap = spec.clone();
    gap.generation = 3;
    assert!(
        client.request(Action::Spawn { spec: gap }).is_err(),
        "generation gap admitted"
    );
    spec.generation = 2;
    let occupied = root.path().join(format!("run/seat-{}/ring-2", spec.seat));
    tokio::fs::create_dir(&occupied).await.unwrap();
    assert!(
        client
            .request(Action::Spawn { spec: spec.clone() })
            .is_err(),
        "occupied destination generation was reused"
    );
    tokio::fs::remove_dir(occupied).await.unwrap();
    let info = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("resume mismatch"),
    };
    let retry = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("retry mismatch"),
    };
    assert_eq!(retry.pid, info.pid);
    let mut conflict = spec.clone();
    conflict.args.push("different".into());
    assert!(client.request(Action::Spawn { spec: conflict }).is_err());
    let mut unknown = spec.clone();
    unknown.seat = Ulid::new();
    let unknown_root = root.path().join(format!("run/seat-{}", unknown.seat));
    assert!(
        client.request(Action::Spawn { spec: unknown }).is_err(),
        "unknown seat started at generation2"
    );
    assert!(
        !tokio::fs::try_exists(unknown_root).await.unwrap(),
        "admission created a ring for an unknown seat"
    );
    let mut corrupt = spec.clone();
    corrupt.seat = Ulid::new();
    let corrupt_ring = root
        .path()
        .join(format!("run/seat-{}/ring-1", corrupt.seat));
    tokio::fs::create_dir_all(&corrupt_ring).await.unwrap();
    tokio::fs::write(corrupt_ring.join("committed"), b"invalid boundary")
        .await
        .unwrap();
    assert!(client.request(Action::Spawn { spec: corrupt }).is_err());
    assert_eq!(
        tokio::fs::read_to_string(corrupt_ring.join("committed"))
            .await
            .unwrap(),
        "invalid boundary",
        "invalid predecessor was changed during admission"
    );
    assert_eq!(std::fs::read_dir(&corrupt_ring).unwrap().count(), 1);
    let mut empty = spec;
    empty.seat = Ulid::new();
    let empty_ring = root.path().join(format!("run/seat-{}/ring-1", empty.seat));
    tokio::fs::create_dir_all(&empty_ring).await.unwrap();
    assert!(
        client.request(Action::Spawn { spec: empty }).is_err(),
        "empty ring directory proved a generation"
    );
    assert!(
        tokio::fs::read_dir(&empty_ring)
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .is_none(),
        "admission initialized an empty predecessor"
    );
    second
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"stop\n")
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), second.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    println!(
        "admission: valid previous boundary authorized gen1 -> gen2; old/gap/occupied/unknown/empty/corrupt/conflicting generations rejected; identical gen2 retry returned PID {}",
        info.pid
    );
}
