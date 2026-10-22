#![allow(clippy::disallowed_methods)] // Disposable IO/process test harness.
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SocketSupervisor};
use bbt_mux::registry::{Position, SeatRecord};
use bbt_mux::seat_socket::{Action as SocketAction, Request};
use bbt_mux::streams::{Replica, Screen, Snapshot, Update, View, ViewStream};
use bbt_mux::{Action, Config, LaunchSpec, Reply, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use serde_json::json;
use tokio::io::{AsyncWriteExt, BufReader};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}
fn fixture(root: &Path) -> LogicConfig {
    let repository = root.join(".bboxy");
    std::fs::create_dir_all(&repository).unwrap();
    LogicConfig {
        ring: Default::default(),
        run_dir: root.join("run"),
        repository,
    }
}
fn launch() -> LaunchSpec {
    let mut spec = LaunchSpec::new("/bin/zsh", std::env::temp_dir());
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        r"printf '\033[31mHELLO\033[0m\n'; read -r ignored".into(),
    ];
    spec
}
fn settled(runtime: &tokio::runtime::Runtime, logic: &Logic, seat: Ulid) -> Snapshot {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        if snapshot.screen.cells.iter().flatten().any(|c| c.c == 'H') {
            return snapshot;
        }
        assert!(Instant::now() < until, "fixture output not observed");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn view_stream_coalesces_and_replica_refuses_wrong_baselines_atomically() {
    let mut term = bbt_core::Term::new(10, 2, 0);
    let view = View {
        seat: Ulid::new(),
        generation: 1,
        view_generation: Ulid::new(),
    };
    let make = |term: &bbt_core::Term, sequence| Snapshot {
        view,
        sequence,
        screen: Screen::from_term(term, String::new(), None, None),
    };
    let mut stream = ViewStream::default();
    stream.offer(make(&term, 0)).unwrap();
    let first = stream.next_update().unwrap();
    term.advance(b"one");
    stream.offer(make(&term, 1)).unwrap();
    term.advance(b"two");
    stream.offer(make(&term, 2)).unwrap();
    assert!(stream.next_update().is_none());
    let mut replica = Replica::default();
    replica.apply(first).unwrap();
    stream.applied(view, 0).unwrap();
    let delta = stream.next_update().unwrap();
    replica.apply(delta.clone()).unwrap();
    assert_eq!(
        replica.snapshot.as_ref().unwrap().screen,
        make(&term, 2).screen
    );
    let before = replica.snapshot.clone();
    assert!(replica.apply(delta).is_err());
    assert_eq!(replica.snapshot, before);
    assert!(stream.applied(view, 999).is_err());
    stream.applied(view, 2).unwrap();
    // Palette/cursor changes are meaningful even without a dirty cell row.
    term.advance(b"\x1b]4;1;rgb:12/34/56\x07\x1b[5 q");
    let metadata = make(&term, 3);
    assert_ne!(
        metadata.screen.palette,
        before.as_ref().unwrap().screen.palette
    );
    assert_ne!(
        metadata.screen.cursor_style,
        before.as_ref().unwrap().screen.cursor_style
    );
    stream.offer(metadata.clone()).unwrap();
    let delta = stream.next_update().unwrap();
    let wire = serde_json::to_value(&delta).unwrap();
    assert!(wire.get("delta").is_none());
    assert_eq!(wire["sequence"], 3);
    let Update::Delta { delta: body } = &delta else {
        panic!("expected metadata delta")
    };
    assert!(body.rows.is_empty());
    replica
        .apply(serde_json::from_value(wire).unwrap())
        .unwrap();
    assert_eq!(replica.snapshot, Some(metadata));
    stream.applied(view, 3).unwrap();
    let changed = View {
        generation: 2,
        ..view
    };
    let mut next = make(&term, 0);
    next.view = changed;
    stream.offer(next).unwrap();
    assert!(matches!(
        stream.next_update(),
        Some(Update::Snapshot { .. })
    ));
}

#[test]
fn logic_reconnect_keeps_same_supervisor_child_and_exact_screen() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let config = fixture(temp.path());
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[20; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = launch();
    let record = SeatRecord::new(spec.seat, Ulid::new(), "fixture", "zsh");
    let seat = spec.seat;
    let info = runtime.block_on(logic.client.spawn(record, spec)).unwrap();
    let before = settled(&runtime, &logic, seat);
    logic.shutdown().unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config,
        SigningKey::from_bytes(&[20; 32]),
        runtime.handle(),
    )
    .unwrap();
    let after = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
    assert_eq!(before.screen, after.screen);
    assert_eq!(before.sequence, after.sequence);
    assert_ne!(before.view.view_generation, after.view.view_generation);
    let endpoint = runtime.block_on(logic.client.endpoint(seat)).unwrap();
    assert!(endpoint.path.exists());
    logic.shutdown().unwrap();
    let client = supervisor.connect().unwrap();
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert_eq!(seats[0].pid, info.pid);
}

#[test]
fn a_watched_task_edit_flows_through_the_durable_bus_with_its_original_doc_identity() {
    use bbt_mux::bus::{Consumer, Delivery};
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let config = fixture(temp.path());
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[23; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut consumer = Consumer::gui("doc/>");
    consumer.filter.push("task/>".into());
    let subscription = runtime
        .block_on(logic.client.bus.subscribe(consumer))
        .unwrap();
    let id = Ulid::new();
    let swarm = Ulid::new();
    std::fs::write(
        config.repository.join("task.md"),
        format!("---\nid: {id}\ntype: task\nstatus: running\nswarm_id: {swarm}\n---\nfixture\n"),
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut subjects = std::collections::BTreeSet::new();
    while subjects.len() < 3 {
        for delivery in runtime
            .block_on(logic.client.bus.poll(&subscription, 8))
            .unwrap()
        {
            let Delivery::Event {
                stream, envelope, ..
            } = delivery
            else {
                panic!("unexpected gap")
            };
            envelope.verify().unwrap();
            assert!(envelope.seq > 0);
            if envelope.event.subject.starts_with("task/") {
                assert_eq!(stream, format!("swarm-{swarm}"));
                assert_eq!(envelope.event.payload["to"], "running");
            }
            subjects.insert(envelope.event.subject.clone());
        }
        assert!(
            Instant::now() < until,
            "watch did not publish doc/task events"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        subjects,
        std::collections::BTreeSet::from([
            format!("doc/{id}/created"),
            "doc/path/task.md/created".into(),
            format!("task/{id}/status")
        ])
    );
    logic.shutdown().unwrap();
}

#[test]
fn supervisor_expires_grace_after_logic_stops_and_undo_preserves_process() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let config = fixture(temp.path());
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config,
        SigningKey::from_bytes(&[21; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = launch();
    let record = SeatRecord::new(spec.seat, Ulid::new(), "fixture", "zsh");
    let seat = spec.seat;
    let swarm = record.swarm;
    runtime.block_on(logic.client.spawn(record, spec)).unwrap();
    settled(&runtime, &logic, seat);
    let position = Position {
        swarm,
        tab: Ulid::new(),
        split_path: vec![0],
        ratios: vec![0.5],
        layout_revision: 1,
    };
    runtime
        .block_on(
            logic
                .client
                .close(seat, 1, Ulid::new(), position.clone(), 500),
        )
        .unwrap();
    let record = runtime.block_on(logic.client.record(seat)).unwrap();
    assert!(
        runtime
            .block_on(logic.client.undo(seat, 1, record.lifecycle_revision, 99))
            .is_err()
    );
    runtime
        .block_on(logic.client.undo(seat, 1, record.lifecycle_revision, 1))
        .unwrap();
    runtime
        .block_on(logic.client.close(seat, 1, Ulid::new(), position, 150))
        .unwrap();
    logic.shutdown().unwrap();
    let client = supervisor.connect().unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
            panic!()
        };
        if seats[0].exited.is_some() {
            break;
        }
        assert!(
            Instant::now() < until,
            "supervisor failed to expire grace without logic"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

async fn socket_request(path: &Path, request: &Request) -> serde_json::Value {
    let stream = tokio::net::UnixStream::connect(path).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut bytes = serde_json::to_vec(request).unwrap();
    bytes.push(b'\n');
    for part in bytes.chunks(7) {
        write.write_all(part).await.unwrap();
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let line = tokio::time::timeout(
        Duration::from_secs(3),
        bbt_mux::seat_socket::read_line(&mut reader),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    serde_json::from_slice(&line).unwrap()
}

#[test]
fn real_seat_socket_binds_capability_generation_and_returns_explicit_history_gap() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        fixture(temp.path()),
        SigningKey::from_bytes(&[22; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = launch();
    let seat = spec.seat;
    runtime
        .block_on(
            logic
                .client
                .spawn(SeatRecord::new(seat, Ulid::new(), "fixture", "zsh"), spec),
        )
        .unwrap();
    settled(&runtime, &logic, seat);
    let endpoint = runtime.block_on(logic.client.endpoint(seat)).unwrap();
    let request = Request {
        version: 1,
        capability: endpoint.capability,
        seat,
        generation: 1,
        action: SocketAction::Attach,
    };
    assert_eq!(
        runtime.block_on(socket_request(&endpoint.path, &request))["kind"],
        "Snapshot"
    );
    let request = Request {
        action: SocketAction::GetHistory { from: 0, to: 100 },
        ..request
    };
    let history = runtime.block_on(socket_request(&endpoint.path, &request));
    assert_eq!(history["kind"], "History");
    assert!(history["history"]["gap"].is_object());
    assert!(!history["history"]["records"].as_array().unwrap().is_empty());
    runtime.block_on(async {
        let stream = tokio::net::UnixStream::connect(&endpoint.path)
            .await
            .unwrap();
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut wrong = request;
        wrong.generation = 2;
        let mut bytes = serde_json::to_vec(&wrong).unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_secs(2),
                bbt_mux::seat_socket::read_line(&mut reader)
            )
            .await
            .unwrap()
            .unwrap()
            .is_none()
        );
    });
}

#[test]
fn hard_killed_separate_logic_process_recovers_the_same_zsh_view() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let config = fixture(temp.path());
    let supervisor_path = config.run_dir.join("supervisor.sock");
    let supervisor = Supervisor::start(Config {
        socket_path: Some(supervisor_path.clone()),
        ..Default::default()
    })
    .unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[23; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = launch();
    let seat = spec.seat;
    let info = runtime
        .block_on(
            logic
                .client
                .spawn(SeatRecord::new(seat, Ulid::new(), "fixture", "zsh"), spec),
        )
        .unwrap();
    let before = settled(&runtime, &logic, seat);
    logic.shutdown().unwrap();
    let ready = temp.path().join("ready.json");
    for _ in 0..2 {
        if ready.exists() {
            std::fs::remove_file(&ready).unwrap();
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "external_logic_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("BBOXY_TEST_LOGIC_FIXTURE", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let bootstrap = json!({"run":config.run_dir,"repository":config.repository,"supervisor":supervisor_path,"capability":supervisor.socket_capability(),"seat":seat,"ready":ready});
        writeln!(child.stdin.take().unwrap(), "{bootstrap}").unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("logic fixture exited {status}");
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(20));
        }
        let endpoint: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&ready).unwrap()).unwrap();
        let capability: [u8; 32] = serde_json::from_value(endpoint["capability"].clone()).unwrap();
        let path = std::path::PathBuf::from(endpoint["path"].as_str().unwrap());
        let response = runtime.block_on(socket_request(
            &path,
            &Request {
                version: 1,
                capability,
                seat,
                generation: 1,
                action: SocketAction::Snapshot,
            },
        ));
        let after: Snapshot = serde_json::from_value(response["snapshot"].clone()).unwrap();
        assert_eq!(before.screen, after.screen);
        child.kill().unwrap();
        child.wait().unwrap();
        // The fixture owner has reaped the exact killed process. Removing its
        // stale test-only endpoint is not production socket takeover policy.
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }
    let client = supervisor.connect().unwrap();
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert_eq!(seats[0].pid, info.pid);
}

#[test]
#[ignore = "subprocess fixture, invoked by the hard-kill acceptance test"]
fn external_logic_fixture() {
    if std::env::var_os("BBOXY_TEST_LOGIC_FIXTURE").is_none() {
        return;
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    let input: serde_json::Value = serde_json::from_str(&line).unwrap();
    let runtime = runtime();
    let capability = serde_json::from_value(input["capability"].clone()).unwrap();
    let seat = serde_json::from_value(input["seat"].clone()).unwrap();
    let supervisor =
        SocketSupervisor::connect(Path::new(input["supervisor"].as_str().unwrap()), capability)
            .unwrap();
    let logic = Logic::start(
        supervisor,
        LogicConfig {
            ring: Default::default(),
            run_dir: input["run"].as_str().unwrap().into(),
            repository: input["repository"].as_str().unwrap().into(),
        },
        SigningKey::from_bytes(&[23; 32]),
        runtime.handle(),
    )
    .unwrap();
    let endpoint = runtime.block_on(logic.client.endpoint(seat)).unwrap();
    use std::os::unix::fs::OpenOptionsExt;
    let ready = Path::new(input["ready"].as_str().unwrap());
    let pending = ready.with_extension("tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .unwrap();
    write!(
        file,
        "{}",
        json!({"path":endpoint.path,"capability":endpoint.capability})
    )
    .unwrap();
    file.sync_all().unwrap();
    std::fs::rename(pending, ready).unwrap();
    loop {
        std::thread::park();
    }
}

#[test]
fn ring_full_during_logic_absence_backpressures_then_replays_the_red_tail_without_a_gap() {
    use bbt_mux::ring::Saved;
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let mut config = fixture(temp.path());
    config.ring.capacity_bytes = 256 * 1024;
    config.ring.checkpoint_bytes = 32 * 1024;
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[26; 32]),
        runtime.handle(),
    )
    .unwrap();
    let gate = temp.path().join("gate");
    let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
    spec.env
        .insert("BBOXY_TEST_GATE".into(), gate.to_string_lossy().to_string());
    spec.args=vec!["-f".into(),"-c".into(),r#"printf 'BEFORE\n'; while [[ ! -f "$BBOXY_TEST_GATE" ]]; do sleep 0.02; done; printf '\033[31m'; p=${(l:1024::x:)}; for i in {1..640}; do printf '%s' "$p"; done; printf '\nAFTER-TAIL\n'; read -r ignored"#.into()];
    let seat = spec.seat;
    let info = runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "ring fixture", "zsh"),
            spec,
        ))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        if snapshot.screen.cells.iter().flatten().any(|c| c.c == 'B') {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    logic.shutdown().unwrap();
    std::fs::write(&gate, b"go").unwrap();
    let root = config.run_dir.join(format!("seat-{seat}/ring-1"));
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let length = std::fs::read(root.join("committed"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b[..b.len() - 4]).ok())
            .and_then(|v| v["length"].as_u64())
            .unwrap_or(0);
        if length >= config.ring.capacity_bytes * 3 / 4 - 256 {
            break;
        }
        assert!(
            Instant::now() < until,
            "supervisor did not capture while logic was absent"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let client = supervisor.connect().unwrap();
    let attachment = client.attach(seat, 1).unwrap();
    let Reply::RingRecovery { status, checkpoint } = client
        .request(Action::RingRecovery {
            attachment: attachment.clone(),
        })
        .unwrap()
    else {
        panic!()
    };
    assert!(status.paused);
    assert!(status.hot_bytes <= config.ring.capacity_bytes);
    let saved = Saved::decode(&checkpoint.unwrap()).unwrap();
    assert!(status.committed_sequence > saved.sequence);
    let Reply::RingHistory { history } = client
        .request(Action::ReadRing {
            attachment,
            from: saved.sequence + 1,
            to: status.committed_sequence,
        })
        .unwrap()
    else {
        panic!()
    };
    assert!(history.records.iter().any(|r| r.row.is_none()));
    let logic = Logic::start(
        client,
        config,
        SigningKey::from_bytes(&[26; 32]),
        runtime.handle(),
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(20);
    let snapshot = loop {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        let text: String = snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .map(|c| c.c)
            .collect();
        if text.contains("AFTER-TAIL") {
            break snapshot;
        }
        assert!(
            Instant::now() < until,
            "capture failed to resume after checkpoint publication"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(snapshot.screen.gap.is_none());
    assert!(
        snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .any(|c| c.c == 'A' && c.fg == bbt_core::Color::Named(bbt_core::NamedColor::Red))
    );
    let history = runtime
        .block_on(logic.client.history(seat, 1, 1, snapshot.sequence))
        .unwrap();
    assert!(history.gap.is_some()); // Old hot history was safely evicted, not a capture loss.
    assert!(history.status.byte_end >= 655360);
    logic.shutdown().unwrap();
    let Reply::Seats { seats } = supervisor
        .connect()
        .unwrap()
        .request(Action::ListOwnedSeats)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(seats[0].pid, info.pid);
}

#[test]
fn ring_idle_checkpoint_indexes_small_output_without_a_shutdown_or_byte_threshold() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let mut config = fixture(temp.path());
    config.ring.idle_ms = 30;
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config,
        SigningKey::from_bytes(&[27; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = launch();
    let seat = spec.seat;
    runtime
        .block_on(
            logic
                .client
                .spawn(SeatRecord::new(seat, Ulid::new(), "idle ring", "zsh"), spec),
        )
        .unwrap();
    let snapshot = settled(&runtime, &logic, seat);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let history = runtime
            .block_on(logic.client.history(seat, 1, 1, snapshot.sequence))
            .unwrap();
        assert!(history.status.byte_end < 64 * 1024);
        if history.records.iter().all(|r| r.row.is_some()) {
            break;
        }
        assert!(
            Instant::now() < until,
            "idle output was not checkpointed/indexed"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    logic.shutdown().unwrap();
}
