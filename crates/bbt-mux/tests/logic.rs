#![allow(clippy::disallowed_methods)] // Disposable IO/process test harness.
mod common;
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
    let repository = root.join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let config = fixture(temp.path());
    // Build products and repository Markdown are outside the document root.
    // This file used to abort the whole scan before task events could flow.
    std::fs::create_dir_all(config.repository.join("target")).unwrap();
    std::fs::File::create(config.repository.join("target/oversized.md"))
        .unwrap()
        .set_len(2 * 1024 * 1024 + 1)
        .unwrap();
    std::fs::write(
        config.repository.join("README.md"),
        "not a cockpit document",
    )
    .unwrap();
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
        config.repository.join(".bboxy/task.md"),
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
    let original = runtime.block_on(logic.client.spawn(record, spec)).unwrap();
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
                .close(seat, 1, Ulid::new(), position.clone(), 30_000),
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
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
            panic!()
        };
        assert_eq!(seats[0].pid, original.pid, "undo replaced the child");
        assert_eq!(seats[0].generation, original.generation);
        if seats[0].exited.is_some() {
            break;
        }
        assert!(
            Instant::now() < until,
            "supervisor failed to expire grace without logic: {seats:?}"
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
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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
        journal_directory: Some(temp.path().join("supervisor-journal")),
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
        // C1a's logic also binds gui.sock here.
        std::fs::remove_file(path.parent().unwrap().join("gui.sock")).unwrap();
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

fn ring_writer(root: &Path, bytes: usize) -> LaunchSpec {
    let mut spec = LaunchSpec::new("python3", root);
    spec.env.insert(
        "BBOXY_TEST_GATE".into(),
        root.join("gate").to_string_lossy().to_string(),
    );
    spec.env.insert(
        "BBOXY_TEST_STATS".into(),
        root.join("writer.json").to_string_lossy().to_string(),
    );
    spec.env
        .insert("BBOXY_TEST_BYTES".into(), bytes.to_string());
    spec.args = vec![
        "-c".into(),
        r#"
import os, time, json
os.write(1, b'BEFORE\n')
while not os.path.exists(os.environ['BBOXY_TEST_GATE']): time.sleep(0.005)
remaining=int(os.environ['BBOXY_TEST_BYTES']); total=remaining
chunk=b'x'*65536; latencies=[]; calls=0; started=time.perf_counter()
while remaining:
    before=time.perf_counter(); n=os.write(1, chunk[:min(remaining,len(chunk))])
    latencies.append(time.perf_counter()-before); remaining-=n; calls+=1
before=time.perf_counter(); os.write(1,b'\x1b[31mRING-TAIL\x1b[0m\n')
latencies.append(time.perf_counter()-before); calls+=1
elapsed=time.perf_counter()-started; latencies.sort()
stats={'bytes':total,'elapsed_s':elapsed,'max_write_s':max(latencies),'p99_write_s':latencies[max(0,(len(latencies)*99+99)//100-1)],'mib_s':total/(1024*1024)/elapsed,'calls':calls}
path=os.environ['BBOXY_TEST_STATS']
with open(path+'.tmp','w') as f: json.dump(stats,f)
os.replace(path+'.tmp',path)
while True: time.sleep(0.1)
"#
        .into(),
    ];
    spec
}
fn wait_for_prefix(runtime: &tokio::runtime::Runtime, logic: &Logic, seat: Ulid) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        if snapshot.screen.cells.iter().flatten().any(|c| c.c == 'B') {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn ring_seals_twenty_mib_with_logic_stopped_and_a_stalled_notification_queue_then_replays_without_gap()
 {
    assert!(sealed_twenty_mib_roundtrip() < 2.0);
}

#[test]
#[ignore = "W1.2f O0 cold-start profile; producer latency is separately reported and gated"]
fn ring_cold_replay_twenty_mib_profile() {
    let max_write = sealed_twenty_mib_roundtrip();
    println!("W12F_PROFILE cold_start_only=true producer_max_write_s={max_write}");
}

fn sealed_twenty_mib_roundtrip() -> f64 {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let config = fixture(temp.path());
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[26; 32]),
        runtime.handle(),
    )
    .unwrap();
    let size = 20 * 1024 * 1024;
    let spec = ring_writer(temp.path(), size);
    let seat = spec.seat;
    let info = runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "ring writer", "python"),
            spec,
        ))
        .unwrap();
    wait_for_prefix(&runtime, &logic, seat);
    logic.shutdown().unwrap();
    // An admitted but completely unread logic queue must not stall PTY capture.
    let stalled = supervisor.connect().unwrap();
    let attachment = stalled.attach(seat, 1).unwrap();
    std::fs::write(temp.path().join("gate"), b"go").unwrap();
    let started = Instant::now();
    let mut next_trace = Duration::from_secs(5);
    let until = Instant::now() + Duration::from_secs(60);
    while !temp.path().join("writer.json").exists() {
        if started.elapsed() >= next_trace {
            if let Ok(Reply::RingRecovery { status, .. }) = stalled.request(Action::RingRecovery {
                attachment: attachment.clone(),
            }) {
                println!(
                    "writer progress {:?}: {} captured bytes, hot {}, paused {}",
                    started.elapsed(),
                    status.byte_end,
                    status.hot_bytes,
                    status.paused
                );
            }
            next_trace += Duration::from_secs(5);
        }
        assert!(
            Instant::now() < until,
            "writer blocked while logic was down"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let stats: serde_json::Value =
        serde_json::from_slice(&std::fs::read(temp.path().join("writer.json")).unwrap()).unwrap();
    println!("20 MiB writer completed BEFORE logic restart: {stats}");
    assert_eq!(stats["bytes"], size);
    let max_write = stats["max_write_s"].as_f64().unwrap();
    let raw_size = size + 8 + b"\x1b[31mRING-TAIL\x1b[0m\r\n".len();
    let until = Instant::now() + Duration::from_secs(15);
    let mut last_observation = String::new();
    loop {
        let status = match stalled.request(Action::RingRecovery {
            attachment: attachment.clone(),
        }) {
            Ok(Reply::RingRecovery { status, .. }) => {
                last_observation = format!("{status:?}");
                status
            }
            other => {
                assert!(
                    Instant::now() < until,
                    "final capture unavailable; last={last_observation}; current={other:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
        };
        assert!(!status.paused);
        assert!(status.hot_bytes <= config.ring.capacity_bytes);
        if status.byte_end == raw_size as u64 {
            println!("capture profile BEFORE logic restart: {:?}", status.capture);
            break;
        }
        assert!(
            Instant::now() < until,
            "final capture bytes unavailable: {last_observation}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let restart = Instant::now();
    struct ReplayTrace {
        client: bbt_mux::Client,
        started: Instant,
        next_trace: usize,
        bytes: usize,
        reads: Duration,
        checkpoints: Duration,
    }
    impl bbt_mux::logic::SupervisorConnection for ReplayTrace {
        fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
            let checkpoint = matches!(action, Action::PublishCheckpoint { .. });
            let started = Instant::now();
            let reply = self.client.request(action)?;
            if checkpoint {
                self.checkpoints += started.elapsed();
            }
            if let Reply::RingHistory { history } = &reply {
                self.reads += started.elapsed();
                for record in &history.records {
                    if let bbt_mux::Event::Output { bytes, .. } = &record.event {
                        self.bytes += bytes.len();
                    }
                }
                if self.bytes >= self.next_trace {
                    println!(
                        "cold replay {:?}: {} bytes, reads {:?}, checkpoint writes {:?}",
                        self.started.elapsed(),
                        self.bytes,
                        self.reads,
                        self.checkpoints
                    );
                    self.next_trace += 4 * 1024 * 1024;
                }
            }
            Ok(reply)
        }
        fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<bbt_mux::Event>> {
            bbt_mux::logic::SupervisorConnection::receive(&mut self.client, timeout)
        }
    }
    let logic = Logic::start(
        ReplayTrace {
            client: stalled,
            started: restart,
            next_trace: 4 * 1024 * 1024,
            bytes: 0,
            reads: Duration::ZERO,
            checkpoints: Duration::ZERO,
        },
        config,
        SigningKey::from_bytes(&[26; 32]),
        runtime.handle(),
    )
    .unwrap();
    println!("20 MiB cold replay startup: {:?}", restart.elapsed());
    let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
    let text: String = snapshot
        .screen
        .cells
        .iter()
        .flatten()
        .map(|c| c.c)
        .collect();
    assert!(text.contains("RING-TAIL"));
    assert!(snapshot.screen.gap.is_none());
    assert!(
        snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .any(|c| c.c == 'R' && c.fg == bbt_core::Color::Named(bbt_core::NamedColor::Red))
    );
    let mut hash = Sha256::new();
    let mut count = 0;
    let mut from = 1;
    while from <= snapshot.sequence {
        let page = runtime
            .block_on(logic.client.history(seat, 1, from, snapshot.sequence))
            .unwrap();
        assert!(page.gap.is_none());
        for record in page.records {
            if let bbt_mux::Event::Output { bytes, .. } = record.event {
                count += bytes.len();
                hash.update(bytes);
            }
        }
        from = page.next.unwrap_or(snapshot.sequence + 1);
    }
    let mut expected = Sha256::new();
    expected.update(b"BEFORE\r\n");
    for _ in 0..320 {
        expected.update([b'x'; 65536]);
    }
    expected.update(b"\x1b[31mRING-TAIL\x1b[0m\r\n");
    assert_eq!(count, raw_size);
    let actual = hash.finalize();
    assert_eq!(actual, expected.finalize());
    println!("complete raw capture: {count} bytes, SHA-256 {actual:x}, no gap");
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
    max_write
}

#[test]
fn ring_live_logic_catches_up_coalesced_notifications_after_a_stall() {
    struct Delayed {
        client: bbt_mux::Client,
        pause: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }
    impl bbt_mux::logic::SupervisorConnection for Delayed {
        fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
            Ok(self.client.request(action)?)
        }
        fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<bbt_mux::Event>> {
            if self.pause.swap(false, std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(750));
            }
            match self.client.recv_timeout(timeout) {
                Ok(e) => Ok(Some(e)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
                Err(e) => Err(e.into()),
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let pause = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let logic = Logic::start(
        Delayed {
            client: supervisor.connect().unwrap(),
            pause: pause.clone(),
        },
        fixture(temp.path()),
        SigningKey::from_bytes(&[28; 32]),
        runtime.handle(),
    )
    .unwrap();
    let spec = ring_writer(temp.path(), 5 * 1024 * 1024);
    let seat = spec.seat;
    runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "stalled logic", "python"),
            spec,
        ))
        .unwrap();
    wait_for_prefix(&runtime, &logic, seat);
    pause.store(true, std::sync::atomic::Ordering::SeqCst);
    std::fs::write(temp.path().join("gate"), b"go").unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        let text: String = snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .map(|c| c.c)
            .collect();
        if text.contains("RING-TAIL") {
            assert!(snapshot.screen.gap.is_none());
            break;
        }
        assert!(Instant::now() < until, "live logic failed to catch up");
        std::thread::sleep(Duration::from_millis(20));
    }
    logic.shutdown().unwrap();
}

#[test]
fn ring_idle_checkpoint_indexes_small_output_without_a_shutdown_or_byte_threshold() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
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

/// Explicit 20 MiB baseline: report timings, and assert capture continuity.
#[test]
#[ignore = "W1.2f baseline instrumentation: run release ring_capture_twenty_mib_profile --ignored --nocapture"]
fn ring_capture_twenty_mib_profile_running_and_stopped() {
    use sha2::{Digest, Sha256};

    for running in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let runtime = runtime();
        let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
        let initial = Logic::start(
            supervisor.connect().unwrap(),
            fixture(temp.path()),
            SigningKey::from_bytes(&[29; 32]),
            runtime.handle(),
        )
        .unwrap();
        let size = 20 * 1024 * 1024;
        let spec = ring_writer(temp.path(), size);
        let seat = spec.seat;
        let info = runtime
            .block_on(initial.client.spawn(
                SeatRecord::new(seat, Ulid::new(), "capture profile", "python"),
                spec,
            ))
            .unwrap();
        wait_for_prefix(&runtime, &initial, seat);
        let (logic, stalled) = if running {
            (Some(initial), None)
        } else {
            initial.shutdown().unwrap();
            let client = supervisor.connect().unwrap();
            let attachment = client.attach(seat, 1).unwrap();
            (None, Some((client, attachment)))
        };
        println!(
            "W12F_PROFILE baseline_only=true running={running} seat={seat} producer_pid={} bytes={size}",
            info.pid
        );
        let started = Instant::now();
        std::fs::write(temp.path().join("gate"), b"go").unwrap();
        let until = started + Duration::from_secs(60);
        while !temp.path().join("writer.json").exists() {
            assert!(Instant::now() < until, "writer stalled: running={running}");
            std::thread::sleep(Duration::from_millis(1));
        }
        let stats: serde_json::Value =
            serde_json::from_slice(&std::fs::read(temp.path().join("writer.json")).unwrap())
                .unwrap();
        println!("W12F_PROFILE writer running={running}: {stats}");
        let raw_size = size + 8 + b"\x1b[31mRING-TAIL\x1b[0m\r\n".len();
        let status = loop {
            let status = if let Some(logic) = &logic {
                runtime
                    .block_on(logic.client.history(seat, 1, 1, 1))
                    .unwrap()
                    .status
            } else {
                let (client, attachment) = stalled.as_ref().unwrap();
                let Reply::RingRecovery { status, .. } = client
                    .request(Action::RingRecovery {
                        attachment: attachment.clone(),
                    })
                    .unwrap()
                else {
                    panic!("capture status reply mismatch")
                };
                *status
            };
            if status.byte_end == raw_size as u64
                && status.committed_sequence == status.captured_sequence
            {
                break status;
            }
            assert!(Instant::now() < until, "commit stalled: {status:?}");
            std::thread::sleep(Duration::from_millis(1));
        };
        let elapsed = started.elapsed().as_secs_f64();
        println!(
            "W12F_PROFILE committed running={running} elapsed_s={} mib_s={} stats={}",
            elapsed,
            20.0 / elapsed,
            serde_json::to_string(&status.capture).unwrap()
        );
        assert!(status.capture.peak_queue_bytes <= 64 * 1024 * 1024);
        if let Some(logic) = logic {
            logic.shutdown().unwrap();
        }
        let (client, attachment) = stalled.unwrap_or_else(|| {
            let client = supervisor.connect().unwrap();
            let attachment = client.attach(seat, 1).unwrap();
            (client, attachment)
        });
        let mut from = 1;
        let mut bytes = 0;
        let mut actual = Sha256::new();
        while from <= status.captured_sequence {
            let Reply::RingHistory { history } = client
                .request(Action::ReadRing {
                    attachment: attachment.clone(),
                    from,
                    to: status.captured_sequence,
                })
                .unwrap()
            else {
                panic!("capture history reply mismatch")
            };
            assert!(history.gap.is_none());
            assert!(!history.records.is_empty());
            for record in history.records {
                if let bbt_mux::Event::Output { bytes: output, .. } = record.event {
                    bytes += output.len();
                    actual.update(output);
                }
            }
            from = history.next.unwrap_or(status.captured_sequence + 1);
        }
        let mut expected = Sha256::new();
        expected.update(b"BEFORE\r\n");
        for _ in 0..320 {
            expected.update([b'x'; 65536]);
        }
        expected.update(b"\x1b[31mRING-TAIL\x1b[0m\r\n");
        assert_eq!(bytes, raw_size);
        assert_eq!(actual.finalize(), expected.finalize());
        println!("W12F_PROFILE exact_capture running={running} bytes={bytes} no_gap=true");
        supervisor.shutdown().unwrap();
    }
}

/// W1.2c's original 128 MiB gate, retained for comparison.
#[test]
#[ignore = "cargo test -p bbt-mux --release --locked --test logic ring_capture_throughput -- --ignored --nocapture --test-threads=1"]
fn ring_capture_throughput_with_logic_running_and_stopped() {
    for running in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let runtime = runtime();
        let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
        let config = fixture(temp.path());
        let key = SigningKey::from_bytes(&[29; 32]);
        let initial = Logic::start(
            supervisor.connect().unwrap(),
            config.clone(),
            key.clone(),
            runtime.handle(),
        )
        .unwrap();
        let size = 128 * 1024 * 1024;
        let spec = ring_writer(temp.path(), size);
        let seat = spec.seat;
        let info = runtime
            .block_on(initial.client.spawn(
                SeatRecord::new(seat, Ulid::new(), "throughput", "python"),
                spec,
            ))
            .unwrap();
        wait_for_prefix(&runtime, &initial, seat);
        let (mut logic, stalled) = if running {
            (Some(initial), None)
        } else {
            initial.shutdown().unwrap();
            let client = supervisor.connect().unwrap();
            let attachment = client.attach(seat, 1).unwrap();
            (None, Some((client, attachment)))
        };
        let start = Instant::now();
        std::fs::write(temp.path().join("gate"), b"go").unwrap();
        let until = Instant::now() + Duration::from_secs(60);
        while !temp.path().join("writer.json").exists() {
            assert!(
                Instant::now() < until,
                "writer blocked, logic running={running}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let stats: serde_json::Value =
            serde_json::from_slice(&std::fs::read(temp.path().join("writer.json")).unwrap())
                .unwrap();
        println!("128 MiB logic running={running} writer: {stats}");
        let raw_size = size + 8 + b"\x1b[31mRING-TAIL\x1b[0m\r\n".len();
        let status = loop {
            let status = if let Some(logic) = &logic {
                runtime
                    .block_on(logic.client.history(seat, 1, 1, 1))
                    .unwrap()
                    .status
            } else {
                let (client, attachment) = stalled.as_ref().unwrap();
                let Reply::RingRecovery { status, .. } = client
                    .request(Action::RingRecovery {
                        attachment: attachment.clone(),
                    })
                    .unwrap()
                else {
                    panic!()
                };
                *status
            };
            if status.byte_end == raw_size as u64
                && status.committed_sequence == status.captured_sequence
            {
                break status;
            }
            assert!(Instant::now() < until, "durable capture failed to catch up");
            std::thread::sleep(Duration::from_millis(5));
        };
        let elapsed = start.elapsed();
        let durable_mib_s = 128.0 / elapsed.as_secs_f64();
        println!(
            "128 MiB logic running={running} durable capture: {elapsed:?}, {durable_mib_s:.3} MiB/s; {:?}",
            status.capture
        );
        assert!(
            stats["max_write_s"].as_f64().unwrap() <= 0.050,
            "max write target"
        );
        assert!(
            stats["mib_s"].as_f64().unwrap() >= 30.0,
            "writer throughput target"
        );
        assert!(durable_mib_s >= 30.0, "durable throughput target");
        assert_eq!(
            status.capture.queue_full_count, 0,
            "queue exhausted during acceptance"
        );
        assert!(status.capture.peak_queue_bytes <= 64 * 1024 * 1024);
        if let Some(logic) = logic.take() {
            logic.shutdown().unwrap();
        }
        let client = if let Some((client, _)) = stalled {
            client
        } else {
            supervisor.connect().unwrap()
        };
        let restart = Instant::now();
        let recovered = Logic::start(client, config, key, runtime.handle()).unwrap();
        println!(
            "128 MiB logic running={running} replay startup: {:?}",
            restart.elapsed()
        );
        let snapshot = runtime
            .block_on(recovered.client.snapshot(seat, 1))
            .unwrap();
        let text: String = snapshot
            .screen
            .cells
            .iter()
            .flatten()
            .map(|c| c.c)
            .collect();
        assert!(text.contains("RING-TAIL"));
        assert!(snapshot.screen.gap.is_none());
        recovered.shutdown().unwrap();
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
}
