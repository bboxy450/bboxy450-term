#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bbt_mux::{
    Config, LaunchSpec, Supervisor, Ulid,
    logic::{Logic, LogicConfig},
    provider::{HostId, fake::Clock, shared::fixture_providers},
    registry::{Location, Registry, SeatRecord},
    store::{object_dir, sqlite::SqliteStore},
};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, HostState, ProcessStatus, SeatId};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::io::{AsyncWriteExt, BufReader};

fn prepare_gui_identity(root: &std::path::Path, key: &ed25519_dalek::SigningKey) {
    // Explicit fixture custody: the supplied key is available at both fixture
    // addresses, without touching the login Keychain or inventing a device id.
    struct Provided<'a>(&'a ed25519_dalek::SigningKey);
    impl bbt_mux::identity::DeviceKeyStore for Provided<'_> {
        fn custody(&self) -> bbt_mux::identity::Custody {
            bbt_mux::identity::Custody::Memory
        }
        fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<ed25519_dalek::SigningKey>> {
            Ok(Some(self.0.clone()))
        }
        fn create(&self, _: Ulid, _: &ed25519_dalek::SigningKey) -> bbt_mux::identity::Result<()> {
            Err(bbt_mux::identity::PendingReason::JournalConflict)
        }
    }
    let bbt_mux::identity::LocalIssuerOutcome::Ready(saved) =
        bbt_mux::identity::local_dev_issuer_with_store(root, &Provided(key))
    else {
        panic!("explicit GUI fixture identity unavailable")
    };
    assert_eq!(saved.verifying_key(), key.verifying_key());
}
#[derive(Default)]
struct ManualClock(AtomicU64);
impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl ManualClock {
    fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
    fn set(&self, ms: u64) {
        assert!(ms >= self.now_ms());
        self.0.store(ms, Ordering::SeqCst);
    }
}
struct Gui {
    read: BufReader<tokio::net::unix::OwnedReadHalf>,
    write: tokio::net::unix::OwnedWriteHalf,
    capability: [u8; 32],
}
impl Gui {
    async fn open(logic: &Logic) -> Self {
        let endpoint = logic.client.gui_endpoint().await.unwrap();
        let stream = tokio::net::UnixStream::connect(endpoint.path)
            .await
            .unwrap();
        let (read, write) = stream.into_split();
        Self {
            read: BufReader::new(read),
            write,
            capability: endpoint.capability,
        }
    }
    async fn send(&mut self, action: GuiAction) {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        self.write.write_all(&bytes).await.unwrap();
    }
    async fn recv(&mut self) -> GuiReply {
        let bytes = tokio::time::timeout(
            Duration::from_secs(5),
            bbt_mux::seat_socket::read_line(&mut self.read),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    async fn spawn(&mut self, id: Ulid, spec: bbt_proto::SpawnSpec) -> Result<SeatId, String> {
        self.send(GuiAction::Spawn { id, spec }).await;
        loop {
            match self.recv().await {
                GuiReply::Spawned { op, seat } if op == id => return Ok(seat),
                GuiReply::Error { op, reason } if op == id => return Err(reason),
                _ => {}
            }
        }
    }
    async fn input(&mut self, seat: Ulid, text: &str) {
        let id = Ulid::new();
        self.send(GuiAction::Input {
            seat: SeatId::from_u128(seat.0),
            generation: 1,
            id,
            bytes: text.as_bytes().to_vec(),
        })
        .await;
        loop {
            match self.recv().await {
                GuiReply::Ok { op } if op == id => return,
                GuiReply::Error { reason, .. } => panic!("input rejected: {reason}"),
                _ => {}
            }
        }
    }
}

#[test]
fn operator_spawn_replays_across_connections_logic_and_supervisor_restart_without_relaunch() {
    use bbt_mux::{
        identity::{Custody, DeviceKeyStore, PendingReason},
        store::Store,
    };
    use bbt_proto::seat_state::{Placement, Side};
    #[derive(Default)]
    struct Keys(std::sync::Mutex<std::collections::BTreeMap<Ulid, ed25519_dalek::SigningKey>>);
    impl DeviceKeyStore for Keys {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<ed25519_dalek::SigningKey>> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }
        fn create(
            &self,
            id: Ulid,
            key: &ed25519_dalek::SigningKey,
        ) -> bbt_mux::identity::Result<()> {
            use std::collections::btree_map::Entry;
            match self.0.lock().unwrap().entry(id) {
                Entry::Vacant(entry) => {
                    entry.insert(key.clone());
                    Ok(())
                }
                Entry::Occupied(_) => Err(PendingReason::JournalConflict),
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let config = LogicConfig {
        run_dir: temp.path().join("run"),
        repository,
        ring: Default::default(),
    };
    // Recovery validates predecessor rings under the supervisor socket's
    // parent, which is the same run directory logic uses for seat storage.
    let supervisor_root = object_dir(temp.path(), "run").unwrap();
    let supervisor = || Config {
        socket_path: Some(supervisor_root.join("mux.sock")),
        journal_directory: Some(supervisor_root.join("journal")),
        ..Default::default()
    };
    let server = Supervisor::start(supervisor()).unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let keys = Arc::new(Keys::default());
    let start = |server: &Supervisor| {
        Logic::start_custodied_with_store(
            server.connect().unwrap(),
            config.clone(),
            keys.clone(),
            rt.handle(),
        )
        .unwrap()
    };
    let logic = start(&server);
    let device = rt
        .block_on(logic.client.identity_status())
        .unwrap()
        .device_id
        .unwrap();
    let spec = bbt_proto::SpawnSpec {
        swarm: bbt_proto::SwarmId::from_u128(77),
        harness_kind: "zsh".into(),
        name: Some("operator replay".into()),
        role: None,
        cwd: temp.path().to_string_lossy().into_owned(),
        location: bbt_proto::Where::Local,
        resume: None,
        placement: Some(Placement::NewTab {
            tab: bbt_proto::TabId::from_u128(78),
        }),
    };
    let op = Ulid::new();
    let mut gui = rt.block_on(Gui::open(&logic));
    let seat = rt.block_on(gui.spawn(op, spec.clone())).unwrap();
    let first = rt.block_on(logic.client.gui_seat(Ulid(seat.0.0))).unwrap();
    let ProcessStatus::Running { pid: first_pid } = first.state.process else {
        panic!("first process is not running")
    };
    drop(gui);
    let mut gui = rt.block_on(Gui::open(&logic));
    assert_eq!(rt.block_on(gui.spawn(op, spec.clone())).unwrap(), seat);
    let mut changed = spec.clone();
    changed.name = Some("conflicting reuse".into());
    assert!(
        rt.block_on(gui.spawn(op, changed))
            .unwrap_err()
            .contains("conflict")
    );
    assert_eq!(rt.block_on(logic.client.gui_seats()).unwrap().len(), 1);
    let account = SqliteStore::open(&object_dir(&config.run_dir, "account").unwrap()).unwrap();
    let saved: serde_json::Value =
        serde_json::from_slice(&account.get(&format!("control/op/{op}")).unwrap().unwrap())
            .unwrap();
    assert_eq!(saved["actor"], serde_json::json!({"device_id":device}));
    assert_eq!(
        saved["result"]["structuredContent"]["seat"],
        serde_json::json!(Ulid(seat.0.0))
    );
    // Simulate a crash after durable admission but before outcome publication.
    let unknown = Ulid::new();
    let args = serde_json::to_value(&spec).unwrap();
    account.put(&format!("control/op/{unknown}"),&serde_json::to_vec(&serde_json::json!({
        "actor":{"device_id":device},"tool":"gui_spawn", "args_digest":bbt_mux::store::digest(&serde_json::to_vec(&args).unwrap()),
        "result":{"isError":true,"structuredContent":{"error":{"code":"unknown","message":"fixture admitted before crash"}}}
    })).unwrap()).unwrap();
    assert!(
        rt.block_on(gui.spawn(unknown, spec.clone()))
            .unwrap_err()
            .starts_with("unknown:")
    );
    drop(gui);
    logic.shutdown().unwrap();
    let logic = start(&server);
    let mut gui = rt.block_on(Gui::open(&logic));
    assert_eq!(rt.block_on(gui.spawn(op, spec.clone())).unwrap(), seat);
    let recovered = rt.block_on(logic.client.gui_seat(Ulid(seat.0.0))).unwrap();
    assert_eq!(
        recovered.state.process,
        ProcessStatus::Running { pid: first_pid }
    );
    assert_eq!(rt.block_on(logic.client.gui_seats()).unwrap().len(), 1);
    drop(gui);
    logic.shutdown().unwrap();
    server.shutdown().unwrap();
    let server = Supervisor::start(supervisor()).unwrap();
    let logic = start(&server);
    rt.block_on(logic.client.recover_seats()).unwrap();
    let recovered = rt.block_on(logic.client.gui_seat(Ulid(seat.0.0))).unwrap();
    assert_eq!(recovered.state.generation, first.state.generation + 1);
    let ProcessStatus::Running { pid: second_pid } = recovered.state.process else {
        panic!("recovered process is not running")
    };
    assert_ne!(first_pid, second_pid);
    let mut gui = rt.block_on(Gui::open(&logic));
    assert_eq!(rt.block_on(gui.spawn(op, spec.clone())).unwrap(), seat);
    assert!(
        rt.block_on(gui.spawn(unknown, spec.clone()))
            .unwrap_err()
            .starts_with("unknown:")
    );
    let mut stale = spec;
    stale.placement = Some(Placement::Split {
        tab: bbt_proto::TabId::from_u128(78),
        target: seat,
        target_generation: first.state.generation,
        side: Side::Right,
    });
    assert!(
        rt.block_on(gui.spawn(Ulid::new(), stale))
            .unwrap_err()
            .contains("placement_anchor_changed")
    );
    let seats = rt.block_on(logic.client.gui_seats()).unwrap();
    assert_eq!(seats.len(), 1);
    assert_eq!(
        seats[0].state.process,
        ProcessStatus::Running { pid: second_pid }
    );
    drop(gui);
    logic.shutdown().unwrap();
    server.shutdown().unwrap();
}
fn wait_text(
    rt: &tokio::runtime::Runtime,
    logic: &Logic,
    clock: &ManualClock,
    seat: Ulid,
    text: &str,
) -> bbt_mux::streams::Snapshot {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        clock.advance(20);
        if let Ok(snapshot) = rt.block_on(logic.client.snapshot(seat, 1)) {
            let content: String = snapshot
                .screen
                .cells
                .iter()
                .flatten()
                .map(|c| c.c)
                .collect();
            if content.contains(text) {
                return snapshot;
            }
        }
        assert!(Instant::now() < until, "missing {text}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn one_supervisor_routes_host_gui_states_and_keeps_remote_ptys_and_labels_after_logic_restart() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let config = LogicConfig {
        run_dir: temp.path().join("run"),
        repository,
        ring: Default::default(),
    };
    let server = Supervisor::start(Config {
        journal_directory: Some(temp.path().join("one-journal")),
        ..Default::default()
    })
    .unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let clock = Arc::new(ManualClock::default());
    let key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    prepare_gui_identity(&config.run_dir, &key);
    let logic = Logic::start_with_providers(
        fixture_providers(server.connect().unwrap(), [], clock.clone()).unwrap(),
        config.clone(),
        key.clone(),
        rt.handle(),
    )
    .unwrap();
    let mut gui = rt.block_on(Gui::open(&logic));
    rt.block_on(gui.send(GuiAction::ListHosts));
    let hosts = loop {
        if let GuiReply::Hosts(hosts) = rt.block_on(gui.recv()) {
            break hosts;
        }
    };
    assert_eq!(hosts.len(), 4);
    assert_eq!(hosts.iter().filter(|h| h.region.is_some()).count(), 2);
    assert!(
        hosts
            .iter()
            .any(|h| h.address.as_deref() == Some("agent@build-box.invalid"))
    );
    assert!(
        hosts
            .iter()
            .filter(|h| !matches!(h.location, bbt_proto::Where::Local))
            .all(|h| h.state == HostState::Connecting)
    );
    let spawn = |name: &str, location| GuiAction::Spawn {
        id: bbt_mux::Ulid::new(),
        spec: bbt_proto::SpawnSpec {
            swarm: bbt_proto::SwarmId::from_u128(7),
            harness_kind: "zsh".into(),
            name: Some(name.into()),
            role: None,
            cwd: temp.path().to_string_lossy().into(),
            location,
            resume: None,
            placement: None,
        },
    };
    rt.block_on(gui.send(spawn(
        "not-created",
        bbt_proto::Where::Ssh {
            host: "build-box".into(),
        },
    )));
    loop {
        if let GuiReply::Error { reason, .. } = rt.block_on(gui.recv()) {
            assert!(reason.starts_with("LinkDown:"));
            break;
        }
    }
    assert!(rt.block_on(logic.client.gui_seats()).unwrap().is_empty());
    clock.set(300);
    let mut connected = false;
    while !connected {
        if let GuiReply::HostState { host } = rt.block_on(gui.recv()) {
            connected = matches!(host.location, bbt_proto::Where::Ssh { .. })
                && host.state == HostState::Connected { rtt_ms: 40 };
        }
    }
    let mut gui_spawned = Vec::new();
    for (name, location) in [
        (
            "gui-ssh",
            bbt_proto::Where::Ssh {
                host: "build-box".into(),
            },
        ),
        (
            "gui-cloud",
            bbt_proto::Where::Cloud {
                provider: "nebula".into(),
                host: "worker-east".into(),
            },
        ),
    ] {
        rt.block_on(gui.send(spawn(name, location.clone())));
        let seat = loop {
            match rt.block_on(gui.recv()) {
                GuiReply::Seat(seat) if seat.record.name == name => break seat,
                GuiReply::Error { reason, .. } => panic!("remote GUI spawn rejected: {reason}"),
                _ => {}
            }
        };
        let expected = match location {
            bbt_proto::Where::Ssh { host } => Location::Ssh { host },
            bbt_proto::Where::Cloud { provider, host } => Location::Cloud { provider, host },
            bbt_proto::Where::Local => Location::Local,
        };
        assert_eq!(seat.record.location, expected);
        let ProcessStatus::Running { pid } = seat.state.process else {
            panic!("GUI spawn is not running")
        };
        let id = seat.record.id;
        // The output marker is split across two commands so PTY echo of the
        // input itself cannot satisfy this assertion.
        rt.block_on(gui.input(id, &format!("printf 'echo-'; printf '{name}\\n'\n")));
        wait_text(&rt, &logic, &clock, id, &format!("echo-{name}"));
        gui_spawned.push((id, expected, pid));
    }
    let gate = temp.path().join("release");
    let ack = temp.path().join("written");
    let mut launched = Vec::new();
    for (name, location) in [
        ("local", Location::Local),
        (
            "ssh",
            Location::Ssh {
                host: "build-box".into(),
            },
        ),
        (
            "cloud",
            Location::Cloud {
                provider: "nebula".into(),
                host: "worker-east".into(),
            },
        ),
    ] {
        let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
        spec.args = vec![
            "-f".into(),
            "-c".into(),
            format!(
                "printf '{name} ready\\n'; while read -r line; do if [[ $line == arm ]]; then printf 'armed\\n'; while [[ ! -e '{}' ]]; do sleep 0.01; done; printf 'during-drop\\n'; : > '{}'; else printf '%s\\n' \"$line\"; fi; done",
                gate.display(),
                ack.display()
            ),
        ];
        let mut record = SeatRecord::new(spec.seat, Ulid::from(7), name, "zsh");
        record.location = location.clone();
        let info = rt.block_on(logic.client.spawn(record, spec)).unwrap();
        rt.block_on(gui.input(info.seat, &format!("echo-{name}\n")));
        wait_text(&rt, &logic, &clock, info.seat, &format!("echo-{name}"));
        launched.push((info, location));
    }
    let remote = launched[1].0.seat;
    rt.block_on(gui.input(remote, "arm\n"));
    wait_text(&rt, &logic, &clock, remote, "armed");
    clock.set(12000);
    std::fs::write(&gate, b"go").unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !ack.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    rt.block_on(gui.input(launched[0].0.seat, "local-still-alive\n"));
    // The local link drains shared traffic while the remote link is down.
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let s = rt
            .block_on(logic.client.snapshot(launched[0].0.seat, 1))
            .unwrap();
        let text: String = s.screen.cells.iter().flatten().map(|c| c.c).collect();
        if text.contains("local-still-alive") {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let remote_held = rt.block_on(logic.client.snapshot(remote, 1)).unwrap();
    assert!(
        !remote_held
            .screen
            .cells
            .iter()
            .flatten()
            .map(|c| c.c)
            .collect::<String>()
            .contains("during-drop")
    );
    clock.set(18000);
    let restored = wait_text(&rt, &logic, &clock, remote, "during-drop");
    assert!(
        rt.block_on(logic.client.history(remote, 1, 1, restored.sequence))
            .unwrap()
            .gap
            .is_none()
    );
    drop(gui);
    logic.shutdown().unwrap();
    let registry =
        Registry::new(SqliteStore::open(&object_dir(&config.run_dir, "account").unwrap()).unwrap());
    let owners = registry
        .list()
        .unwrap()
        .into_iter()
        .map(|s| (s.id, HostId::from(&s.location)));
    let logic = Logic::start_with_providers(
        fixture_providers(server.connect().unwrap(), owners, clock.clone()).unwrap(),
        config,
        key,
        rt.handle(),
    )
    .unwrap();
    clock.advance(400);
    for (info, location) in launched {
        wait_text(&rt, &logic, &clock, info.seat, "ready");
        let seat = rt.block_on(logic.client.gui_seat(info.seat)).unwrap();
        assert_eq!(seat.record.location, location);
        assert_eq!(seat.state.process, ProcessStatus::Running { pid: info.pid });
    }
    wait_text(&rt, &logic, &clock, remote, "during-drop");
    for (id, location, pid) in gui_spawned {
        let seat = rt.block_on(logic.client.gui_seat(id)).unwrap();
        assert_eq!(seat.record.location, location);
        assert_eq!(seat.state.process, ProcessStatus::Running { pid });
    }
    logic.shutdown().unwrap();
    println!(
        "D1c: one supervisor/lease/journal, five labelled PTYs (two spawned through GUI), portable GUI hosts/transitions, local input during SSH drop, gap-free reconnect and exact PID/label recovery after logic restart"
    );
}
