#![allow(
    clippy::disallowed_methods,
    reason = "owned PTYs and explicit-key GUI/replay fixture"
)]

mod common;
use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::provider::{
    Host, HostId, LinkState, LocalProvider, Provider, ProviderEvent, ProviderKind,
};
use bbt_mux::registry::SeatRecord;
use bbt_mux::ring::Playback;
use bbt_mux::streams::{Replica, Screen, Update};
use bbt_mux::{Action, Client, Event, LaunchSpec, Reply, Supervisor, Ulid};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatId};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufReader};

// Deliver one large gap first, then the other seats' latest notifications.
// Delay only its replay pages, never the other seats or GUI commands.
struct HeldSeats {
    client: Client,
    order: Vec<Ulid>,
    huge: Ulid,
    hold: Arc<AtomicBool>,
    delay: Arc<AtomicBool>,
    pages: Arc<AtomicU64>,
    held: Arc<Mutex<BTreeMap<Ulid, u64>>>,
    latest: BTreeMap<Ulid, Event>,
    release: VecDeque<Event>,
}
impl SupervisorConnection for HeldSeats {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let huge = matches!(&action,Action::ReadRing {attachment,..} if attachment.seat==self.huge);
        let reply = self.client.request(action)?;
        if huge
            && self.delay.load(Ordering::Acquire)
            && matches!(&reply,Reply::RingHistory {history} if !history.records.is_empty())
        {
            self.pages.fetch_add(1, Ordering::Release);
            std::thread::sleep(Duration::from_millis(30));
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if self.hold.load(Ordering::Acquire) {
            let mut next = SupervisorConnection::receive(&mut self.client, timeout)?;
            while let Some(event) = next {
                self.held
                    .lock()
                    .unwrap()
                    .insert(event.seat(), event.sequence().unwrap());
                self.latest.insert(event.seat(), event);
                next = SupervisorConnection::receive(&mut self.client, Duration::ZERO)?;
            }
            return Ok(None);
        }
        if !self.latest.is_empty() {
            for seat in &self.order {
                if let Some(event) = self.latest.remove(seat) {
                    self.release.push_back(event);
                }
            }
        }
        if let Some(event) = self.release.pop_front() {
            return Ok(Some(event));
        }
        SupervisorConnection::receive(&mut self.client, timeout)
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
        let (read, write) = tokio::net::UnixStream::connect(endpoint.path)
            .await
            .unwrap()
            .into_split();
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
    async fn update(&mut self) -> (Ulid, Update) {
        loop {
            let line = tokio::time::timeout(
                Duration::from_secs(10),
                bbt_mux::seat_socket::read_line(&mut self.read),
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap();
            match serde_json::from_slice::<GuiReply>(&line).unwrap() {
                GuiReply::Update { seat, update } => return (Ulid::from(seat.0.0), update),
                GuiReply::Error { reason, .. } => panic!("GUI rejected fixture: {reason}"),
                _ => {}
            }
        }
    }
    async fn apply(&mut self, replicas: &mut BTreeMap<Ulid, Replica>) -> (Ulid, u64) {
        let (seat, update) = self.update().await;
        let replica = replicas.entry(seat).or_default();
        replica.apply(update).unwrap();
        let snapshot = replica.snapshot.as_ref().unwrap();
        let sequence = snapshot.sequence;
        self.send(GuiAction::Applied {
            seat: SeatId::from_u128(seat.0),
            view: snapshot.view,
            sequence,
        })
        .await;
        (seat, sequence)
    }
}

#[test]
fn four_watched_seats_advance_during_a_large_replay_and_match_serial_screens() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let order: Vec<_> = (0..4).map(|_| Ulid::new()).collect();
    let huge = order[0];
    let hold = Arc::new(AtomicBool::new(true));
    let delay = Arc::new(AtomicBool::new(false));
    let pages = Arc::new(AtomicU64::new(0));
    let held = Arc::new(Mutex::new(BTreeMap::new()));
    let logic = Logic::start(
        HeldSeats {
            client: supervisor.connect().unwrap(),
            order: order.clone(),
            huge,
            hold: hold.clone(),
            delay: delay.clone(),
            pages: pages.clone(),
            held: held.clone(),
            latest: BTreeMap::new(),
            release: VecDeque::new(),
        },
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[93; 32]),
        runtime.handle(),
    )
    .unwrap();
    let swarm = Ulid::new();
    let mut expected_bytes = BTreeMap::new();
    for (index, &seat) in order.iter().enumerate() {
        let count = if index == 0 { 4 * 1024 * 1024 } else { 256 };
        let tail = format!("\r\nSEAT-{index}-END\r\n");
        expected_bytes.insert(seat, count + tail.len());
        let mut spec = LaunchSpec::new("python3", temp.path());
        spec.seat = seat;
        spec.args = vec![
            "-c".into(),
            r#"import os,sys,time,tty
tty.setraw(1)
while not os.path.exists('go'): time.sleep(.002)
data=(b'0123456789abcdef'*(int(sys.argv[1])//16))+sys.argv[2].encode()
while data:
 n=os.write(1,data);data=data[n:]
time.sleep(60)
"#
            .into(),
            count.to_string(),
            tail,
        ];
        let info = runtime
            .block_on(logic.client.spawn(
                SeatRecord::new(seat, swarm, format!("fair {index}"), "python"),
                spec,
            ))
            .unwrap();
        println!("owned fixture seat={seat} pid={}", info.pid);
    }
    let mut gui = runtime.block_on(Gui::open(&logic));
    let mut replicas = BTreeMap::new();
    runtime.block_on(async {
        for &seat in &order {
            let snapshot = logic.client.snapshot(seat, 1).await.unwrap();
            gui.send(GuiAction::Watch {
                seat: SeatId::from_u128(seat.0),
                view: snapshot.view,
            })
            .await;
        }
        while replicas.len() < 4 {
            gui.apply(&mut replicas).await;
        }
    });
    std::fs::write(temp.path().join("go"), []).unwrap();
    let mut through = BTreeMap::new();
    let until = Instant::now() + Duration::from_secs(15);
    while through.len() < 4 {
        for &seat in &order {
            let status = runtime
                .block_on(logic.client.history(seat, 1, u64::MAX, u64::MAX))
                .unwrap()
                .status;
            if status.byte_end >= expected_bytes[&seat] as u64
                && held
                    .lock()
                    .unwrap()
                    .get(&seat)
                    .is_some_and(|seq| *seq >= status.captured_sequence)
            {
                through.insert(seat, status.captured_sequence);
            }
        }
        assert!(Instant::now() < until, "owned fixture output not captured");
        std::thread::sleep(Duration::from_millis(5));
    }
    delay.store(true, Ordering::Release);
    hold.store(false, Ordering::Release);
    let started = Instant::now();
    runtime.block_on(async {
        let mut advanced = std::collections::BTreeSet::new();
        while advanced.len() < 3 {
            let (seat, sequence) = gui.apply(&mut replicas).await;
            if seat != huge && sequence >= through[&seat] {
                advanced.insert(seat);
            }
            assert!(
                pages.load(Ordering::Acquire) <= 16,
                "other seats starved behind the huge replay"
            );
        }
        let snapshot = logic.client.snapshot(huge, 1).await.unwrap();
        assert!(
            snapshot.sequence < through[&huge],
            "other Updates arrived only after the huge replay finished"
        );
    });
    println!(
        "other three watched seats updated during replay: {:?}, huge_pages={}",
        started.elapsed(),
        pages.load(Ordering::Acquire)
    );
    delay.store(false, Ordering::Release);
    runtime.block_on(async {
        while order
            .iter()
            .any(|seat| replicas[seat].snapshot.as_ref().unwrap().sequence < through[seat])
        {
            gui.apply(&mut replicas).await;
        }
    });
    for &seat in &order {
        let mut reference = Playback::new(seat, 1, Default::default());
        let mut next = 1;
        while next <= through[&seat] {
            let history = runtime
                .block_on(logic.client.history(seat, 1, next, through[&seat]))
                .unwrap();
            assert!(history.gap.is_none());
            assert!(!history.records.is_empty());
            for record in history.records {
                reference.apply(&record.event).unwrap();
            }
            next = reference.sequence + 1;
        }
        let expected = Screen::from_term(
            &reference.term,
            reference.title,
            reference.cwd,
            reference.gap,
        );
        assert_eq!(
            replicas[&seat].snapshot.as_ref().unwrap().screen,
            expected,
            "seat {seat} differs from serial capture replay"
        );
        println!(
            "exact serial Screen seat={seat} sequence={}",
            through[&seat]
        );
    }
    drop(gui);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

struct CountReceives {
    client: Client,
    calls: Arc<AtomicU64>,
}
impl SupervisorConnection for CountReceives {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        Ok(self.client.request(action)?)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        SupervisorConnection::receive(&mut self.client, timeout)
    }
}
struct OutageLink {
    inner: HeldSeats,
    up: Arc<AtomicBool>,
    cut: Arc<AtomicBool>,
    receives: Arc<AtomicU64>,
    failures: Arc<AtomicU64>,
}
fn outage_host() -> HostId {
    HostId::Ssh {
        host: "fairness-outage".into(),
    }
}
fn outage_state(up: bool) -> LinkState {
    if up {
        LinkState::Connected { rtt_ms: 0 }
    } else {
        LinkState::Disconnected {
            reason: "fixture down".into(),
        }
    }
}
impl SupervisorConnection for OutageLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        if matches!(action, Action::ReadRing { .. }) && self.cut.swap(false, Ordering::AcqRel) {
            self.up.store(false, Ordering::Release);
        }
        if !self.up.load(Ordering::Acquire) {
            self.failures.fetch_add(1, Ordering::Relaxed);
            return Err(Box::new(bbt_mux::provider::fake::LinkDown {
                host: outage_host(),
                state: outage_state(false),
            }));
        }
        self.inner.request(action)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.receives.fetch_add(1, Ordering::Relaxed);
        if !self.up.load(Ordering::Acquire) {
            return Ok(None);
        }
        self.inner.receive(timeout)
    }
}
struct OutageProvider {
    link: Option<OutageLink>,
    up: Arc<AtomicBool>,
    reported: bool,
}
impl Provider for OutageProvider {
    fn id(&self) -> String {
        "outage-fixture".into()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<Host> {
        vec![Host {
            id: outage_host(),
            provider: self.id(),
            label: "fixture".into(),
            region: None,
            state: outage_state(self.up.load(Ordering::Acquire)),
        }]
    }
    fn link(&mut self, host: HostId) -> bbt_mux::store::Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, outage_host());
        Ok(Box::new(self.link.take().unwrap()))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        let up = self.up.load(Ordering::Acquire);
        if up == self.reported {
            return vec![];
        }
        self.reported = up;
        vec![ProviderEvent::HostState {
            host: outage_host(),
            state: outage_state(up),
        }]
    }
}

#[test]
fn down_host_replay_parks_and_idle_local_polling_stays_bounded_until_reconnect() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let local = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let remote = Supervisor::start(common::capture_config(&temp.path().join("remote"))).unwrap();
    let seat = Ulid::new();
    let up = Arc::new(AtomicBool::new(true));
    let cut = Arc::new(AtomicBool::new(false));
    let hold = Arc::new(AtomicBool::new(true));
    let held = Arc::new(Mutex::new(BTreeMap::new()));
    let local_calls = Arc::new(AtomicU64::new(0));
    let remote_calls = Arc::new(AtomicU64::new(0));
    let failures = Arc::new(AtomicU64::new(0));
    let remote_link = OutageLink {
        inner: HeldSeats {
            client: remote.connect().unwrap(),
            order: vec![seat],
            huge: seat,
            hold: hold.clone(),
            delay: Arc::new(AtomicBool::new(false)),
            pages: Arc::new(AtomicU64::new(0)),
            held: held.clone(),
            latest: BTreeMap::new(),
            release: VecDeque::new(),
        },
        up: up.clone(),
        cut: cut.clone(),
        receives: remote_calls.clone(),
        failures: failures.clone(),
    };
    let logic = Logic::start_with_providers(
        vec![
            Box::new(LocalProvider::new(CountReceives {
                client: local.connect().unwrap(),
                calls: local_calls.clone(),
            })),
            Box::new(OutageProvider {
                link: Some(remote_link),
                up: up.clone(),
                reported: true,
            }),
        ],
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[94; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut spec = LaunchSpec::new("python3", temp.path());
    spec.seat = seat;
    spec.args = vec![
        "-c".into(),
        r#"import os,time,tty
tty.setraw(1)
while not os.path.exists('go'): time.sleep(.002)
data=b'x'*(1024*1024)+b'\r\nOUTAGE-END\r\n'
while data:
 n=os.write(1,data);data=data[n:]
time.sleep(60)
"#
        .into(),
    ];
    let mut record = SeatRecord::new(seat, Ulid::new(), "outage fixture", "python");
    record.location = bbt_mux::registry::Location::Ssh {
        host: "fairness-outage".into(),
    };
    let info = runtime.block_on(logic.client.spawn(record, spec)).unwrap();
    println!("owned outage fixture seat={seat} pid={}", info.pid);
    std::fs::write(temp.path().join("go"), []).unwrap();
    let until = Instant::now() + Duration::from_secs(15);
    let through = loop {
        let status = runtime
            .block_on(logic.client.history(seat, 1, u64::MAX, u64::MAX))
            .unwrap()
            .status;
        if status.byte_end >= 1024 * 1024 + 14
            && held
                .lock()
                .unwrap()
                .get(&seat)
                .is_some_and(|seq| *seq >= status.captured_sequence)
        {
            break status.captured_sequence;
        }
        assert!(Instant::now() < until, "outage fixture was not captured");
        std::thread::sleep(Duration::from_millis(5));
    };
    cut.store(true, Ordering::Release);
    hold.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(5);
    while failures.load(Ordering::Acquire) == 0 {
        assert!(
            Instant::now() < until,
            "capture never reached the injected outage"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let first_failures = failures.load(Ordering::Acquire);
    let local_before = local_calls.load(Ordering::Relaxed);
    let remote_before = remote_calls.load(Ordering::Relaxed);
    let started = Instant::now();
    std::thread::sleep(Duration::from_secs(1));
    let elapsed = started.elapsed();
    let local_count = local_calls.load(Ordering::Relaxed) - local_before;
    let remote_count = remote_calls.load(Ordering::Relaxed) - remote_before;
    let bound = (elapsed.as_millis() as u64 * 110).div_ceil(1000);
    assert!(
        local_count <= bound,
        "idle local host spun: {local_count} receives in {elapsed:?}, bound {bound}"
    );
    assert!(
        remote_count <= bound,
        "down host spun: {remote_count} receives in {elapsed:?}, bound {bound}"
    );
    assert_eq!(
        failures.load(Ordering::Acquire),
        first_failures,
        "parked capture was retried before an up event"
    );
    println!(
        "down-host idle {:?}: local_receives={local_count}, remote_receives={remote_count}, bound={bound}, capture_retries=0",
        elapsed
    );
    up.store(true, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if runtime
            .block_on(logic.client.snapshot(seat, 1))
            .unwrap()
            .sequence
            >= through
        {
            break;
        }
        assert!(
            Instant::now() < until,
            "up transition did not resume parked replay"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    logic.shutdown().unwrap();
    local.shutdown().unwrap();
    remote.shutdown().unwrap();
}
