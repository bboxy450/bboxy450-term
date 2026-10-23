#![allow(
    clippy::disallowed_methods,
    reason = "owned PTYs and explicit-key GUI/replay fixture"
)]

mod common;
use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
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
