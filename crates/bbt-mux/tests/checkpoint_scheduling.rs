#![allow(
    clippy::disallowed_methods,
    reason = "owned, explicit-key PTY checkpoint fixtures"
)]

mod common;

use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::provider::{
    Host, HostId, LinkState, LocalProvider, Provider, ProviderEvent, ProviderKind,
};
use bbt_mux::registry::{Location, SeatRecord};
use bbt_mux::ring::Saved;
use bbt_mux::{Action, Client, Event, LaunchSpec, Reply, Supervisor, Ulid};
use ed25519_dalek::SigningKey;

const R: u64 = 8 * 1024 * 1024;

#[derive(Default)]
struct SaveGate {
    held: AtomicBool,
    entered: AtomicBool,
}

struct OtherHost(Option<Client>);
impl Provider for OtherHost {
    fn id(&self) -> String {
        "other".into()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<Host> {
        vec![Host {
            id: HostId::Ssh {
                host: "other".into(),
            },
            provider: self.id(),
            label: "owned second supervisor".into(),
            region: None,
            state: LinkState::Connected { rtt_ms: 0 },
        }]
    }
    fn link(&mut self, host: HostId) -> bbt_mux::store::Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, self.hosts()[0].id);
        Ok(Box::new(self.0.take().unwrap()))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        vec![]
    }
}

struct HeldOutput {
    client: Client,
    seat: Ulid,
    held: Arc<AtomicBool>,
    latest: Option<Event>,
    saves: Arc<Mutex<BTreeMap<Ulid, Vec<u64>>>>,
    gate: Arc<SaveGate>,
}
impl SupervisorConnection for HeldOutput {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let saved = match &action {
            Action::PublishCheckpoint { checkpoint, .. } => Some(Saved::decode(checkpoint)?),
            _ => None,
        };
        if saved.as_ref().is_some_and(|saved| saved.byte_end > 0)
            && self.gate.held.load(Ordering::Acquire)
        {
            self.gate.entered.store(true, Ordering::Release);
            while self.gate.held.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let reply = self.client.request(action)?;
        if let Some(saved) = saved {
            assert!(
                matches!(reply, Reply::CheckpointCommitted { sequence } if sequence == saved.sequence)
            );
            self.saves
                .lock()
                .unwrap()
                .entry(saved.seat)
                .or_default()
                .push(saved.byte_end);
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if !self.held.load(Ordering::Acquire)
            && let Some(event) = self.latest.take()
        {
            return Ok(Some(event));
        }
        let mut timeout = timeout;
        loop {
            let Some(event) = SupervisorConnection::receive(&mut self.client, timeout)? else {
                return Ok(None);
            };
            if event.seat() == self.seat && self.held.load(Ordering::Acquire) {
                self.latest = Some(event);
                timeout = Duration::ZERO;
            } else {
                return Ok(Some(event));
            }
        }
    }
}

struct Fixture {
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    other_supervisor: Option<Supervisor>,
    runtime: tokio::runtime::Runtime,
    held: Arc<AtomicBool>,
    saves: Arc<Mutex<BTreeMap<Ulid, Vec<u64>>>>,
    seat: Ulid,
    gate: Arc<SaveGate>,
    temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self::with_other_host(false)
    }
    fn with_other_host(other_host: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
        let seat = Ulid::new();
        let held = Arc::new(AtomicBool::new(true));
        let saves = Arc::new(Mutex::new(BTreeMap::new()));
        let gate = Arc::new(SaveGate::default());
        let repository = temp.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let mut providers: Vec<Box<dyn Provider>> =
            vec![Box::new(LocalProvider::new(HeldOutput {
                client: supervisor.connect().unwrap(),
                seat,
                held: held.clone(),
                latest: None,
                saves: saves.clone(),
                gate: gate.clone(),
            }))];
        let other_supervisor = other_host.then(|| {
            Supervisor::start(common::capture_config(&temp.path().join("other"))).unwrap()
        });
        if let Some(other) = &other_supervisor {
            providers.push(Box::new(OtherHost(Some(other.connect().unwrap()))));
        }
        let logic = Logic::start_with_providers(
            providers,
            LogicConfig {
                run_dir: temp.path().join("logic"),
                repository,
                ring: Default::default(),
            },
            SigningKey::from_bytes(&[42; 32]),
            runtime.handle(),
        )
        .unwrap();
        logic.client.enable_capture_diagnostics();
        Self {
            logic: Some(logic),
            supervisor: Some(supervisor),
            other_supervisor,
            runtime,
            held,
            saves,
            seat,
            gate,
            temp,
        }
    }
    fn logic(&self) -> &Logic {
        self.logic.as_ref().unwrap()
    }
    fn spawn(&self, seat: Ulid, bytes: u64) {
        self.spawn_at(seat, bytes, Location::Local, "go");
    }
    fn spawn_at(&self, seat: Ulid, bytes: u64, location: Location, go: &str) {
        let go = self.temp.path().join(go);
        let script = "import os,sys,time\nfrom pathlib import Path\np=Path(sys.argv[1])\nwhile not p.exists(): time.sleep(.01)\nleft=int(sys.argv[2])\nwhile left:\n n=os.write(1,b'x'*min(left,16384)); left-=n\nos.write(1,b'COMPLETE')\ntime.sleep(300)\n";
        let mut spec = LaunchSpec::new("python3", self.temp.path());
        spec.seat = seat;
        spec.args = vec![
            "-u".into(),
            "-c".into(),
            script.into(),
            go.to_string_lossy().into(),
            bytes.to_string(),
        ];
        let mut record = SeatRecord::new(seat, Ulid::new(), "checkpoint fixture", "fixture");
        record.location = location;
        self.runtime
            .block_on(self.logic().client.spawn(record, spec))
            .unwrap();
    }
    fn row(&self, seat: Ulid) -> Option<bbt_mux::supervisor::capture_diagnostics::Seat> {
        self.logic()
            .client
            .capture_diagnostics()
            .seats
            .into_iter()
            .find(|r| r.seat == seat)
    }
    fn start_and_capture(&self, bytes: u64) -> (u64, u64) {
        std::fs::write(self.temp.path().join("go"), b"go").unwrap();
        let mut last = 0;
        let mut progress = Instant::now();
        loop {
            let history = self
                .runtime
                .block_on(self.logic().client.history(self.seat, 1, 1, 1))
                .unwrap();
            if history.status.byte_end != last {
                last = history.status.byte_end;
                progress = Instant::now();
            }
            if last >= bytes + 8 {
                return (history.status.captured_sequence, last);
            }
            assert!(
                progress.elapsed() < Duration::from_secs(30),
                "capture stopped advancing: {last}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn catch_up(&self, through: u64, end: u64) {
        self.catch_up_with_rows(through, end, || self.row(self.seat));
    }
    fn catch_up_with_rows(
        &self,
        through: u64,
        end: u64,
        mut row: impl FnMut() -> Option<bbt_mux::supervisor::capture_diagnostics::Seat>,
    ) {
        const NO_PROGRESS: Duration = Duration::from_secs(30);
        self.held.store(false, Ordering::Release);
        let mut last = 0;
        let mut last_sequence = 0;
        let mut progress = Instant::now();
        loop {
            let mut stale = true;
            if let Some(row) = row() {
                assert!(
                    row.parsed_byte_end.saturating_sub(row.checkpoint_byte_end) <= 2 * R,
                    "parsed debt: {row:?}"
                );
                if row.parsed_byte_end != last {
                    last = row.parsed_byte_end;
                    last_sequence = last_sequence.max(row.applied_sequence);
                    progress = Instant::now();
                    stale = false;
                }
                if row.applied_sequence < through {
                    assert_eq!(
                        row.idle_checkpoints, 0,
                        "backlogged seat got an idle save: {row:?}"
                    );
                }
                if row.applied_sequence >= through && row.parsed_byte_end >= end {
                    return;
                }
            }
            if stale {
                // Recorder writes are best-effort. A lost final row must not
                // turn a completed parser into a permanent watchdog failure.
                let snapshot = self.runtime.block_on(async {
                    tokio::time::timeout(
                        NO_PROGRESS.saturating_sub(progress.elapsed()),
                        self.logic().client.snapshot(self.seat, 1),
                    )
                    .await
                    .expect("authoritative parser query did not answer within 30 s")
                    .unwrap()
                });
                // through is the captured ring watermark for end, so this
                // authoritative cursor covers those same bytes.
                if snapshot.sequence >= through {
                    return;
                }
                if snapshot.sequence > last_sequence {
                    last_sequence = snapshot.sequence;
                    progress = Instant::now();
                }
            }
            assert!(
                progress.elapsed() < NO_PROGRESS,
                "parser stopped advancing at {last}; diagnostics={}",
                serde_json::to_string(&self.logic().client.capture_diagnostics()).unwrap()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn finish(mut self) {
        self.logic.take().unwrap().shutdown().unwrap();
        drop(self.supervisor.take());
        drop(self.other_supervisor.take());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.gate.held.store(false, Ordering::Release);
        if let Some(logic) = self.logic.take() {
            let _ = logic.shutdown();
        }
        drop(self.supervisor.take());
        drop(self.other_supervisor.take());
    }
}

// keeps: decision #253
#[test]
fn a_stale_recorder_row_cannot_hide_authoritative_parser_completion() {
    let fixture = Fixture::new();
    fixture.spawn(fixture.seat, 64 * 1024);
    let (through, end) = fixture.start_and_capture(64 * 1024);
    let stale = fixture.row(fixture.seat).unwrap();
    assert!(stale.parsed_byte_end < end);
    fixture.catch_up_with_rows(through, end, || Some(stale.clone()));
    assert!(
        fixture
            .runtime
            .block_on(fixture.logic().client.snapshot(fixture.seat, 1))
            .unwrap()
            .sequence
            >= through
    );
    fixture.finish();
}

// keeps: decision #253
#[test]
fn a_held_checkpoint_does_not_hold_snapshot_or_status_service() {
    let fixture = Fixture::with_other_host(true);
    fixture.spawn(fixture.seat, 20 * 1024 * 1024);
    let other = Ulid::new();
    fixture.spawn(other, 64);
    let remote = Ulid::new();
    fixture.spawn_at(
        remote,
        20 * 1024 * 1024,
        Location::Ssh {
            host: "other".into(),
        },
        "remote-go",
    );
    let small = Ulid::new();
    fixture.spawn_at(
        small,
        64,
        Location::Ssh {
            host: "other".into(),
        },
        "after-pause-go",
    );
    let (through, end) = fixture.start_and_capture(20 * 1024 * 1024);
    fixture.gate.held.store(true, Ordering::Release);
    fixture.held.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(30);
    while !fixture.gate.entered.load(Ordering::Acquire) {
        assert!(
            Instant::now() < until,
            "save never reached its controlled gate"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    // The gate remains held until both replies arrive. This is a causal
    // assertion, not a throughput/deadline claim about a loaded machine.
    fixture.runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            fixture.logic().client.snapshot(other, 1).await.unwrap();
            fixture
                .logic()
                .client
                .adapter_status(other, 1)
                .await
                .unwrap();
        })
        .await
        .expect("local commands blocked behind the held checkpoint");
    });
    assert!(fixture.gate.held.load(Ordering::Acquire));
    std::fs::write(fixture.temp.path().join("remote-go"), b"go").unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    while fixture
        .row(remote)
        .is_none_or(|row| !row.checkpoint_paused || row.parsed_byte_end < R)
    {
        fixture
            .runtime
            .block_on(fixture.logic().client.snapshot(remote, 1))
            .unwrap();
        assert!(
            Instant::now() < until,
            "other host stopped parsing behind a held local save: {:?}",
            fixture.row(remote)
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let paused = fixture.row(remote).unwrap();
    assert!(paused.parsed_byte_end - paused.checkpoint_byte_end <= 2 * R);
    assert!(paused.checkpoint_pauses > 0);
    let seat = fixture
        .runtime
        .block_on(fixture.logic().client.gui_seat(remote))
        .unwrap();
    assert!(seat.state.title.contains("Replay paused for checkpoint"));
    std::fs::write(fixture.temp.path().join("after-pause-go"), b"go").unwrap();
    while fixture
        .row(small)
        .is_none_or(|row| row.parsed_byte_end < 72)
    {
        fixture
            .runtime
            .block_on(fixture.logic().client.snapshot(small, 1))
            .unwrap();
        assert!(
            Instant::now() < until,
            "a debt-paused seat held the other seat"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(fixture.gate.held.load(Ordering::Acquire));
    fixture.gate.held.store(false, Ordering::Release);
    fixture.catch_up(through, end);
    let until = Instant::now() + Duration::from_secs(30);
    while fixture
        .row(remote)
        .is_none_or(|row| row.parsed_byte_end < 20 * 1024 * 1024 + 8)
    {
        fixture.runtime.block_on(async {
            tokio::time::timeout(
                until.saturating_duration_since(Instant::now()),
                fixture.logic().client.snapshot(remote, 1),
            )
            .await
            .expect("paused-seat progress query exceeded the existing deadline")
            .unwrap();
        });
        assert!(
            Instant::now() < until,
            "matching checkpoint did not resume the paused seat"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    fixture.finish();
}

// keeps: decision #253
#[test]
fn caught_up_idle_checkpoints_do_not_checkpoint_a_twenty_mib_backlog() {
    let fixture = Fixture::new();
    let idle = Ulid::new();
    fixture.spawn(fixture.seat, 20 * 1024 * 1024);
    fixture.spawn(idle, 64);
    let (through, end) = fixture.start_and_capture(20 * 1024 * 1024);
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        let idle_ready = fixture.row(idle).is_some_and(|r| r.idle_checkpoints > 0);
        assert_eq!(
            fixture.row(fixture.seat).map_or(0, |r| r.idle_checkpoints),
            0
        );
        if idle_ready {
            break;
        }
        assert!(
            Instant::now() < until,
            "caught-up seat did not get its idle checkpoint"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    fixture.catch_up(through, end);
    fixture.finish();
}

// keeps: decision #253
#[test]
fn hundred_mib_partial_replay_forces_r_spaced_checkpoints_and_never_exceeds_two_r_debt() {
    let fixture = Fixture::new();
    fixture.spawn(fixture.seat, 100 * 1024 * 1024);
    let (through, end) = fixture.start_and_capture(100 * 1024 * 1024);
    fixture.catch_up(through, end);
    let mut previous = 0;
    let mut checkpoints = 0;
    for &byte_end in fixture.saves.lock().unwrap().get(&fixture.seat).unwrap() {
        assert!(byte_end >= previous);
        assert!(
            byte_end - previous <= R,
            "checkpoint gap {} exceeds R",
            byte_end - previous
        );
        previous = byte_end;
        checkpoints += 1;
    }
    assert!(
        checkpoints >= 12,
        "only {checkpoints} checkpoints for100MiB"
    );
    println!("100MiB: checkpoints={checkpoints} confirmed_bytes={previous} parsed_end={end}");
    fixture.finish();
}
