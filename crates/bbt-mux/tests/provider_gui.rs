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
            version: 1,
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
    clock.set(300);
    let mut connected = false;
    while !connected {
        if let GuiReply::HostState { host } = rt.block_on(gui.recv()) {
            connected = matches!(host.location, bbt_proto::Where::Ssh { .. })
                && host.state == HostState::Connected { rtt_ms: 40 };
        }
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
    logic.shutdown().unwrap();
    println!(
        "D1c: one supervisor/lease/journal, three labelled PTYs, portable GUI hosts/transitions, local input during SSH drop, gap-free reconnect and exact PID/label recovery after logic restart"
    );
}
