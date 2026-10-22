//! C1d: the 1000-seat gates that need no GUI (c1-live-seats.md §5, decision #10).
//!
//! Run through `sh scripts/gates.sh memory|bus|poller`. Each gate prints one
//! `GATE ...` line and fails when the value is above `GATE_BUDGET`.
//! Supervisor and logic run as threads of this process, so the process RSS
//! is theirs. Env: `GATE_SEATS` (min(1000, ptmx_max - 32)), `GATE_RATE` (per seat, or `idle`,
//! 32kB/s), `GATE_SECONDS` (load time, 20), `GATE_RUN_DIR` (parent of the
//! run directory, /tmp), `GATE_BUDGET`.

#![allow(
    clippy::disallowed_methods,
    reason = "measurement fixture IO, not the UI thread"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use std::sync::Arc;

use bbt_mux::adapters::{Evidence, WakeOutcome};
use bbt_mux::logic::{Logic, LogicConfig};
use bbt_mux::messaging::Messaging;
use bbt_mux::messaging::keys::LocalSeatKeys;
use bbt_mux::messaging::wire::SendRequest;
use bbt_mux::registry::{Class, Registry, SeatRecord};
use bbt_mux::store::{now_ms, object_dir, sqlite::SqliteStore};
use bbt_mux::{Config, LaunchSpec, Supervisor, Ulid};
use ed25519_dalek::SigningKey;

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn ptmx_max() -> Option<usize> {
    let out = Command::new("sysctl")
        .args(["-n", "kern.tty.ptmx_max"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// `GATE_SEATS`, else 1000 capped at the machine's PTY limit minus a 32 margin.
fn seats() -> usize {
    match std::env::var("GATE_SEATS") {
        Ok(v) => v.parse().expect("GATE_SEATS is a number"),
        Err(_) => ptmx_max().map_or(1000, |m| m.saturating_sub(32).min(1000)),
    }
}

/// Fails early, with the fix, when macOS cannot open this many PTYs.
fn check_pty_limit(ptys: usize) {
    if let Some(max) = ptmx_max() {
        // The test process and the firehose children open no PTYs of their own.
        assert!(
            ptys <= max,
            "kern.tty.ptmx_max={max} < {ptys} PTYs needed; raise it: sudo sysctl -w kern.tty.ptmx_max={}",
            ptys + 64
        );
    }
}

fn raise_nofile() -> (u64, u64) {
    let mut lim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: getrlimit and setrlimit read and write one valid rlimit.
    unsafe {
        assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim), 0);
        let want = lim.rlim_max.min(65_536);
        if lim.rlim_cur < want {
            lim.rlim_cur = want;
            assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &lim), 0);
        }
    }
    (lim.rlim_cur, lim.rlim_max)
}

fn open_fds() -> usize {
    (0..65_536)
        .filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } != -1)
        .count()
}

/// Resident set size of this process (supervisor and logic threads), KiB.
fn rss_kib() -> u64 {
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

fn disk_bytes(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => disk_bytes(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

/// The firehose binary next to this test's target profile (`gates.sh` builds it).
fn firehose() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let path = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("bboxy-firehose");
    assert!(
        path.exists(),
        "{} missing: cargo build --release -p bboxy-firehose",
        path.display()
    );
    path
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() as f64 * p / 100.0).ceil() as usize).clamp(1, sorted.len()) - 1]
}

fn budget(gate: &str, value: f64, unit: &str, detail: String) {
    let budget = std::env::var("GATE_BUDGET")
        .ok()
        .map(|b| b.parse::<f64>().expect("GATE_BUDGET is a number"));
    let verdict = match budget {
        Some(b) if value > b => "FAIL",
        Some(_) => "PASS",
        None => "MEASURED",
    };
    println!(
        "GATE {gate} value={value:.3} unit={unit} budget={} {verdict} ptmx_max={} {detail}",
        budget.map_or("none".into(), |b| b.to_string()),
        ptmx_max().map_or("?".into(), |m| m.to_string())
    );
    assert_ne!(
        verdict, "FAIL",
        "{gate}: {value:.3} {unit} is above the budget"
    );
}

struct Rig {
    run: tempfile::TempDir,
    runtime: tokio::runtime::Runtime,
    supervisor: Supervisor,
    logic: Logic,
    swarm: Ulid,
}

impl Rig {
    fn start(max_seats: usize) -> Self {
        let parent = env_or("GATE_RUN_DIR", "/tmp");
        let run = tempfile::Builder::new()
            .prefix("c1d-gate-")
            .tempdir_in(parent)
            .unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(Config {
            max_seats,
            ..Config::default()
        })
        .unwrap();
        let repo = run.path().join(".bboxy");
        std::fs::create_dir(&repo).unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                ring: Default::default(),
                run_dir: run.path().join("run"),
                repository: repo,
            },
            SigningKey::from_bytes(&[41; 32]),
            runtime.handle(),
        )
        .unwrap();
        Self {
            run,
            runtime,
            supervisor,
            logic,
            swarm: Ulid::new(),
        }
    }

    fn spawn(&self, name: String, kind: &str, class: Class, launch: LaunchSpec) -> Ulid {
        let mut record = SeatRecord::new(launch.seat, self.swarm, name, kind);
        record.role.class = class;
        let seat = launch.seat;
        self.try_spawn(record, launch)
            .unwrap_or_else(|e| panic!("spawn of seat {seat} failed: {e}"))
    }

    fn try_spawn(&self, record: SeatRecord, launch: LaunchSpec) -> bbt_mux::store::Result<Ulid> {
        let seat = launch.seat;
        self.runtime
            .block_on(self.logic.client.spawn(record, launch))?;
        Ok(seat)
    }

    /// Spawns `n` firehose seats; on failure stops logic and reports its exit.
    fn firehose_seats(self, n: usize, rate: &str) -> (Self, Vec<Ulid>) {
        let bin = firehose();
        let started = Instant::now();
        let mut seats = Vec::with_capacity(n);
        for i in 0..n {
            let mut launch = LaunchSpec::new(bin.to_string_lossy().to_string(), self.run.path());
            launch.args = vec!["--rate".into(), rate.into(), "--loop".into()];
            if rate == "idle" {
                launch = LaunchSpec::new("/bin/sleep", self.run.path());
                launch.args = vec!["3600".into()];
            }
            let record =
                SeatRecord::new(launch.seat, self.swarm, format!("firehose-{i}"), "firehose");
            match self.try_spawn(record, launch) {
                Ok(s) => seats.push(s),
                Err(e) => {
                    let at = started.elapsed();
                    let exit = self.logic.shutdown();
                    panic!("spawn {i} of {n} failed after {at:?}: {e}; logic exit: {exit:?}");
                }
            }
        }
        (self, seats)
    }

    fn sum_capture(&self, seats: &[Ulid]) -> bbt_mux::ring::CaptureStats {
        let mut t = bbt_mux::ring::CaptureStats::default();
        for &s in seats {
            let c = self
                .runtime
                .block_on(self.logic.client.history(s, 1, 1, 1))
                .unwrap()
                .status
                .capture;
            t.records += c.records;
            t.hot_write_ns += c.hot_write_ns;
            t.encode_ns += c.encode_ns;
            t.sync_ns += c.sync_ns;
            t.marker_ns += c.marker_ns;
            t.queue_full_count += c.queue_full_count;
            t.peak_queue_bytes = t.peak_queue_bytes.max(c.peak_queue_bytes);
        }
        t
    }

    fn stop(self) -> u64 {
        let disk = disk_bytes(self.run.path());
        self.logic.shutdown().unwrap();
        self.supervisor.shutdown().unwrap();
        disk
    }
}

/// Gate 1: 1000 headless seats under the firehose, none watched; RSS of supervisor + logic.
#[test]
#[ignore = "C1d gate 1: sh scripts/gates.sh memory"]
fn gate_memory() {
    let n = seats();
    check_pty_limit(n);
    let rate = env_or("GATE_RATE", "32kB/s");
    let secs: u64 = env_or("GATE_SECONDS", "20").parse().unwrap();
    raise_nofile();
    let rig = Rig::start(n.max(1));
    let base = rss_kib();
    let t = Instant::now();
    let (rig, seats) = rig.firehose_seats(n, &rate);
    let spawn = t.elapsed();
    let mut peak = 0;
    let mut samples = Vec::new();
    for _ in 0..secs {
        std::thread::sleep(Duration::from_secs(1));
        let r = rss_kib();
        peak = peak.max(r);
        samples.push(r);
    }
    let last = *samples.last().unwrap_or(&base);
    let cap = rig.sum_capture(&seats);
    let disk = rig.stop();
    budget(
        "memory",
        last as f64 / 1024.0,
        "MiB",
        format!(
            "seats={n} rate={rate} load_s={secs} baseline_mib={:.1} peak_mib={:.1} per_seat_kib={:.1} spawn_s={:.1} records={} sync_s={:.1} marker_s={:.1} queue_full={} disk_peak_mib={:.1}",
            base as f64 / 1024.0,
            peak as f64 / 1024.0,
            (last.saturating_sub(base)) as f64 / n.max(1) as f64,
            spawn.as_secs_f64(),
            cap.records,
            cap.sync_ns as f64 / 1e9,
            cap.marker_ns as f64 / 1e9,
            cap.queue_full_count,
            disk as f64 / 1048576.0
        ),
    );
}

/// Gate 2: 1000 synthetic seats (decision #19: bus fan-out needs no PTYs), an
/// Oracle burst of 30 messages at 10/s to the whole swarm through W2.4
/// messaging: the real ledger and weighted fair queue on a wall clock, pumped
/// every 100 ms the way logic's `pump_messages` pumps (finish last pump's
/// deliveries, drain the outbox, prepare one batch per target, at most 32 in
/// flight). Native delivery is instant, so a batch finishes on the next pump;
/// latency is send to finish. The Oracle's own inbox copy is not counted.
#[test]
#[ignore = "C1d gate 2: sh scripts/gates.sh bus"]
fn gate_bus() {
    let n: usize = env_or("GATE_SEATS", "1000").parse().unwrap();
    let parent = env_or("GATE_RUN_DIR", "/tmp");
    let run = tempfile::Builder::new()
        .prefix("c1d-gate-")
        .tempdir_in(parent)
        .unwrap();
    let registry =
        Registry::new(SqliteStore::open(&object_dir(run.path(), "account").unwrap()).unwrap());
    let mut messaging = Messaging::open(
        run.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[43; 32]))),
    )
    .unwrap();
    let swarm = Ulid::new();
    let mut oracle = SeatRecord::new(Ulid::new(), swarm, "oracle", "claude");
    oracle.role.class = Class::Oracle;
    let t = Instant::now();
    registry.insert(&oracle).unwrap();
    messaging.sync_seat(&oracle).unwrap();
    for i in 1..n {
        let r = SeatRecord::new(Ulid::new(), swarm, format!("seat-{i}"), "claude");
        registry.insert(&r).unwrap();
        messaging.sync_seat(&r).unwrap();
    }
    let setup = t.elapsed();
    let recipients = n - 1;
    let expected = 30 * recipients;
    let mut sent: BTreeMap<u64, u64> = BTreeMap::new();
    let mut lat: Vec<f64> = Vec::with_capacity(expected);
    let mut pumps = Vec::new();
    let mut sends: Vec<f64> = Vec::new();
    let mut in_flight: Vec<(Ulid, bbt_mux::messaging::ledger::Work)> = Vec::new();
    let start = Instant::now();
    let deadline = start + Duration::from_secs(env_or("GATE_SECONDS", "600").parse().unwrap());
    let mut next_pump = start;
    let mut k = 0u32;
    while lat.len() < expected && Instant::now() < deadline {
        let due = start + Duration::from_millis(100 * k as u64);
        if k < 30 && Instant::now() >= due {
            let req = SendRequest::text(format!("swarms/{swarm}"), format!("burst {k}"));
            let ts = Instant::now();
            let stored = messaging.send(oracle.id, 1, req, now_ms()).unwrap();
            sends.push(ts.elapsed().as_secs_f64() * 1000.0);
            if std::env::var("GATE_DEBUG").is_ok() {
                eprintln!("send {k} took {:?}", ts.elapsed());
            }
            sent.insert(stored.sequence, now_ms());
            k += 1;
        }
        if Instant::now() >= next_pump {
            let p = Instant::now();
            for (sw, work) in in_flight.drain(..) {
                let done = now_ms();
                let outcome = WakeOutcome {
                    evidence: Evidence::NativeWritten,
                    note: "gate".into(),
                    native_id: None,
                };
                messaging.finish(sw, &work, &outcome, done, false).unwrap();
                if work.recipient != oracle.id {
                    lat.extend(
                        work.messages
                            .iter()
                            .filter_map(|m| sent.get(&m.sequence))
                            .map(|s| done.saturating_sub(*s) as f64),
                    );
                }
            }
            let t_fin = p.elapsed();
            let targets = messaging.ready(now_ms()).unwrap();
            let t_ready = p.elapsed();
            let mut n_events = 0;
            for (stream, draft) in messaging.events().unwrap() {
                messaging.event_sent(&stream, draft.id).unwrap();
                n_events += 1;
            }
            let t_events = p.elapsed();
            let n_targets = targets.len();
            for (sw, seat) in targets {
                if in_flight.len() >= 32 {
                    break;
                }
                if let Some(work) = messaging.prepare(sw, seat, now_ms()).unwrap() {
                    in_flight.push((sw, work));
                }
            }
            if std::env::var("GATE_DEBUG").is_ok() {
                eprintln!(
                    "pump sent={k} delivered={} finish={:?} ready={:?} events={n_events} {:?} targets={n_targets} prepared={} total={:?}",
                    lat.len(),
                    t_fin,
                    t_ready - t_fin,
                    t_events - t_ready,
                    in_flight.len(),
                    p.elapsed()
                );
            }
            pumps.push(p.elapsed().as_secs_f64() * 1000.0);
            next_pump += Duration::from_millis(100);
            continue;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let drained = start.elapsed();
    lat.sort_by(f64::total_cmp);
    pumps.sort_by(f64::total_cmp);
    sends.sort_by(f64::total_cmp);
    let disk = disk_bytes(run.path());
    let row_bytes: f64 = rusqlite::Connection::open(
        run.path()
            .join(format!("swarm-{swarm}"))
            .join("object.sqlite"),
    )
    .and_then(|db| {
        db.query_row(
            "SELECT COALESCE(AVG(LENGTH(data)),0) FROM msg_deliveries",
            [],
            |r| r.get(0),
        )
    })
    .unwrap_or(0.0);
    if std::env::var("GATE_DEBUG").is_ok() {
        eprintln!("delivery_row_bytes={row_bytes:.0}");
    }
    assert!(!lat.is_empty(), "no deliveries");
    budget(
        "bus",
        percentile(&lat, 99.0),
        "ms",
        format!(
            "seats={n} synthetic=1 recipients={recipients} messages=30 rate=10/s delivered={}/{expected} p50_ms={:.0} max_ms={:.0} drained_s={:.1} send_p50_ms={:.0} send_max_ms={:.0} pump_p99_ms={:.1} pump_max_ms={:.1} setup_s={:.1} disk_peak_mib={:.1}",
            lat.len(),
            percentile(&lat, 50.0),
            lat[lat.len() - 1],
            drained.as_secs_f64(),
            percentile(&sends, 50.0),
            sends[sends.len() - 1],
            percentile(&pumps, 99.0),
            pumps[pumps.len() - 1],
            setup.as_secs_f64(),
            disk as f64 / 1048576.0
        ),
    );
    assert_eq!(
        lat.len(),
        expected,
        "not every recipient got every message before the deadline"
    );
}

/// Python writer: once `gate` exists, writes `bytes` in 4 KiB chunks and records
/// every `os.write` latency (seconds) to `stats` as JSON.
fn probe(dir: &Path, i: usize, bytes: usize) -> (LaunchSpec, PathBuf) {
    let stats = dir.join(format!("probe-{i}.json"));
    let mut spec = LaunchSpec::new("python3", dir);
    spec.env.insert(
        "GATE".into(),
        dir.join("gate").to_string_lossy().to_string(),
    );
    spec.env
        .insert("STATS".into(), stats.to_string_lossy().to_string());
    spec.env.insert("BYTES".into(), bytes.to_string());
    spec.args = vec![
        "-c".into(),
        r#"
import os, time, json
while not os.path.exists(os.environ['GATE']): time.sleep(0.01)
left=int(os.environ['BYTES']); chunk=b'y'*4095+b'\n'; lat=[]
while left:
    t=time.perf_counter(); n=os.write(1, chunk[:min(left,len(chunk))]); lat.append(time.perf_counter()-t); left-=n
p=os.environ['STATS']
with open(p+'.tmp','w') as f: json.dump(lat,f)
os.replace(p+'.tmp',p)
while True: time.sleep(1)
"#
        .into(),
    ];
    (spec, stats)
}

/// Gate 3: 1000 PTYs under the firehose with the open-file limit raised; p99 of
/// PTY record writes, timed by probe seats whose writes block until the
/// supervisor's poller drains them (as W1.2c measured the writer).
#[test]
#[ignore = "C1d gate 3: sh scripts/gates.sh poller"]
fn gate_poller() {
    let n = seats();
    let probes: usize = env_or("GATE_PROBES", "8").parse().unwrap();
    assert!(n > probes);
    check_pty_limit(n);
    let rate = env_or("GATE_RATE", "32kB/s");
    let secs: u64 = env_or("GATE_SECONDS", "20").parse().unwrap();
    let (soft, hard) = raise_nofile();
    let rig = Rig::start(n);
    let fds_before = open_fds();
    let (rig, mut seats) = rig.firehose_seats(n - probes, &rate);
    let mut stats = Vec::new();
    for i in 0..probes {
        let (launch, path) = probe(rig.run.path(), i, 2 << 20);
        seats.push(rig.spawn(
            format!("probe-{i}"),
            "probe",
            bbt_mux::registry::Class::Worker,
            launch,
        ));
        stats.push(path);
    }
    let fds = open_fds();
    std::thread::sleep(Duration::from_secs(secs / 2));
    std::fs::write(rig.run.path().join("gate"), b"go").unwrap();
    let deadline = Instant::now() + Duration::from_secs(secs.max(60));
    let mut lat: Vec<f64> = Vec::new();
    for p in &stats {
        while !p.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        let v: Vec<f64> =
            serde_json::from_slice(&std::fs::read(p).expect("probe finished before the deadline"))
                .unwrap();
        lat.extend(v.into_iter().map(|s| s * 1000.0));
    }
    lat.sort_by(f64::total_cmp);
    let cap = rig.sum_capture(&seats);
    let disk = rig.stop();
    budget(
        "poller",
        percentile(&lat, 99.0),
        "ms",
        format!(
            "ptys={n} firehose={} rate={rate} probes={probes} writes={} p50_ms={:.3} max_ms={:.3} fds={} fds_before_seats={fds_before} nofile_soft={soft} nofile_hard={hard} ring_records={} ring_mean_write_us={:.1} queue_full={} disk_peak_mib={:.1}",
            n - probes,
            lat.len(),
            percentile(&lat, 50.0),
            lat[lat.len() - 1],
            fds,
            cap.records,
            cap.hot_write_ns as f64 / cap.records.max(1) as f64 / 1000.0,
            cap.queue_full_count,
            disk as f64 / 1048576.0
        ),
    );
}
