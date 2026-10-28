// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "owned SIGKILL/PTY recovery fixture"
)]
//! Decision 253: parsed debt is bounded; the independently captured backlog is
//! reported separately. The child stops at a ReadRing boundary before SIGKILL.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SocketSupervisor, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::ring::{Playback, Saved};
use bbt_mux::{Action, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

const R: u64 = 8 * 1024 * 1024;
const BURST: u64 = 20 * 1024 * 1024;

// A single-threaded finite producer keeps 967 PTYs from requiring 967 Python
// interpreters. It uses only the Rust toolchain already running this test.
const PRODUCER: &str = r#"
use std::{io::{self, Write}, path::Path, time::Duration};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    while !Path::new(&args[1]).exists() { std::thread::sleep(Duration::from_millis(50)); }
    let mut left: usize = args[2].parse().unwrap();
    let paced = left < 1024 * 1024;
    let mut out = io::stdout().lock();
    let bytes = [b'x'; 4096];
    while left > 0 {
        let n = left.min(bytes.len());
        out.write_all(&bytes[..n]).unwrap();
        left -= n;
        if paced { std::thread::sleep(Duration::from_millis(125)); }
    }
    out.write_all(b"RECOVERY_READY").unwrap();
    out.flush().unwrap();
    std::fs::write(&args[3], b"done").unwrap();
    loop { std::thread::park(); } // Only the owned supervisor ends this child.
}
"#;

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
struct DeliveryDiagnostics {
    receive_calls: u64,
    inner_receive_calls: u64,
    inner_empty: u64,
    burst_seen: u64,
    burst_held: u64,
    held_released: u64,
    burst_returned: u64,
    other_returned: u64,
    last_burst_seen_sequence: u64,
    last_burst_returned_sequence: u64,
    held_sequence: Option<u64>,
    release_requested: bool,
    client: bbt_supervisor::client::ClientDiagnostics,
}

struct CrashLink {
    inner: SocketSupervisor,
    burst_seat: Ulid,
    held: Arc<AtomicBool>,
    frozen: Arc<AtomicBool>,
    latest: Option<Event>,
    byte_ends: BTreeMap<u64, u64>,
    ready: Arc<Mutex<BTreeSet<Ulid>>>,
    worker: Option<std::thread::ThreadId>,
    side_saves: Arc<AtomicU64>,
    synchronous_saves: Arc<AtomicU64>,
    delivery: Arc<Mutex<DeliveryDiagnostics>>,
}
impl CrashLink {
    fn observe_client(&self) {
        let mut delivery = self.delivery.lock().unwrap();
        delivery.client = self.inner.diagnostics(self.burst_seat, 1);
        delivery.held_sequence = self.latest.as_ref().and_then(Event::sequence);
    }
}
impl SupervisorConnection for CrashLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let worker = *self
            .worker
            .get_or_insert_with(|| std::thread::current().id());
        if let Action::PublishCheckpoint { attachment, .. } = &action
            && self.ready.lock().unwrap().contains(&attachment.seat)
        {
            if std::thread::current().id() == worker {
                self.synchronous_saves.fetch_add(1, Ordering::Relaxed);
            } else {
                self.side_saves.fetch_add(1, Ordering::Relaxed);
            }
        }
        let burst_from = match &action {
            Action::ReadRing {
                attachment, from, ..
            } if attachment.seat == self.burst_seat => Some(*from),
            Action::ReadRingBatch { reads, .. } => reads
                .iter()
                .find(|read| read.attachment.seat == self.burst_seat)
                .map(|read| read.from),
            _ => None,
        };
        let burst_read = burst_from.is_some() && !self.held.load(Ordering::Acquire);
        let parsed_end = burst_from
            .and_then(|from| self.byte_ends.get(&from.saturating_sub(1)))
            .copied()
            .unwrap_or(0);
        if burst_read && parsed_end >= R + R / 2 {
            // The previous page has returned to Worker and its cursor/accounting
            // has been published. This thread cannot parse another byte. No
            // checkpoint owns the serial stream while ReadRing owns its lock.
            self.observe_client();
            self.frozen.store(true, Ordering::Release);
            loop {
                std::thread::park();
            }
        }
        let reply = self.inner.request(action)?;
        self.observe_client();
        if burst_read {
            let histories: Vec<_> = match &reply {
                Reply::RingHistory { history } => vec![history.as_ref()],
                Reply::RingHistoryBatch { results } => results
                    .iter()
                    .filter_map(|row| row.result.as_deref().ok())
                    .collect(),
                _ => vec![],
            };
            for history in histories {
                for record in &history.records {
                    if record.event.seat() != self.burst_seat {
                        continue;
                    }
                    let seq = record.event.sequence().unwrap();
                    let previous = self.byte_ends[&(seq - 1)];
                    let bytes = match &record.event {
                        Event::Output { bytes, .. } => bytes.len() as u64,
                        _ => 0,
                    };
                    self.byte_ends.insert(seq, previous + bytes);
                }
            }
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        {
            let mut delivery = self.delivery.lock().unwrap();
            delivery.receive_calls = delivery.receive_calls.saturating_add(1);
        }
        if !self.held.load(Ordering::Acquire)
            && let Some(event) = self.latest.take()
        {
            {
                let mut delivery = self.delivery.lock().unwrap();
                delivery.held_released = delivery.held_released.saturating_add(1);
                delivery.burst_returned = delivery.burst_returned.saturating_add(1);
                delivery.last_burst_returned_sequence = event.sequence().unwrap();
            }
            self.observe_client();
            return Ok(Some(event));
        }
        let mut wait = timeout;
        loop {
            let event = self.inner.receive(wait)?;
            {
                let mut delivery = self.delivery.lock().unwrap();
                delivery.inner_receive_calls = delivery.inner_receive_calls.saturating_add(1);
                if event.is_none() {
                    delivery.inner_empty = delivery.inner_empty.saturating_add(1);
                }
            }
            let Some(event) = event else {
                self.observe_client();
                return Ok(None);
            };
            if event.seat() == self.burst_seat {
                let mut delivery = self.delivery.lock().unwrap();
                delivery.burst_seen = delivery.burst_seen.saturating_add(1);
                delivery.last_burst_seen_sequence = event.sequence().unwrap();
            }
            if event.seat() == self.burst_seat && self.held.load(Ordering::Acquire) {
                self.latest = Some(event);
                {
                    let mut delivery = self.delivery.lock().unwrap();
                    delivery.burst_held = delivery.burst_held.saturating_add(1);
                }
                self.observe_client();
                wait = Duration::ZERO;
            } else {
                {
                    let mut delivery = self.delivery.lock().unwrap();
                    if event.seat() == self.burst_seat {
                        delivery.burst_returned = delivery.burst_returned.saturating_add(1);
                        delivery.last_burst_returned_sequence = event.sequence().unwrap();
                    } else {
                        delivery.other_returned = delivery.other_returned.saturating_add(1);
                    }
                }
                self.observe_client();
                return Ok(Some(event));
            }
        }
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}
fn config(root: &Path) -> LogicConfig {
    LogicConfig {
        run_dir: root.join("logic"),
        repository: root.join("repository"),
        ring: Default::default(),
    }
}
fn write_json(path: &Path, value: &Value) {
    let temporary = path.with_extension("writing");
    std::fs::write(&temporary, serde_json::to_vec(value).unwrap()).unwrap();
    std::fs::rename(temporary, path).unwrap();
}

fn check_disk(root: &Path) {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(root.as_os_str().as_bytes()).unwrap();
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: the path is NUL-terminated; stat points to writable storage and
    // is only read after a successful statvfs initialized it.
    assert_eq!(
        unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) },
        0
    );
    let stat = unsafe { stat.assume_init() };
    let free = u128::from(stat.f_bavail) * u128::from(stat.f_frsize);
    assert!(
        free >= 10 * 1024 * 1024 * 1024,
        "recovery fixture aborted: free disk {free} below 10 GiB"
    );
}

// keeps: spec mux-daemon
#[test]
#[ignore = "owned subprocess of checkpoint_bounded_recovery"]
fn checkpoint_recovery_child() {
    let Ok(root) = std::env::var("R5G_RECOVERY_CHILD") else {
        return;
    };
    let root = PathBuf::from(root);
    let input: Value =
        serde_json::from_slice(&std::fs::read(root.join("input.json")).unwrap()).unwrap();
    let seats: Vec<Ulid> = serde_json::from_value(input["seats"].clone()).unwrap();
    let runtime = runtime();
    let held = Arc::new(AtomicBool::new(true));
    let frozen = Arc::new(AtomicBool::new(false));
    let ready = Arc::new(Mutex::new(BTreeSet::new()));
    let side_saves = Arc::new(AtomicU64::new(0));
    let synchronous_saves = Arc::new(AtomicU64::new(0));
    let delivery = Arc::new(Mutex::new(DeliveryDiagnostics::default()));
    let logic = Logic::start(
        CrashLink {
            inner: SocketSupervisor::connect(
                &root.join("supervisor.sock"),
                serde_json::from_value(input["capability"].clone()).unwrap(),
            )
            .unwrap(),
            burst_seat: seats[0],
            held: held.clone(),
            frozen: frozen.clone(),
            latest: None,
            byte_ends: BTreeMap::from([(0, 0)]),
            ready: ready.clone(),
            worker: None,
            side_saves: side_saves.clone(),
            synchronous_saves: synchronous_saves.clone(),
            delivery: delivery.clone(),
        },
        config(&root),
        SigningKey::from_bytes(&[93; 32]),
        runtime.handle(),
    )
    .unwrap();
    // Lossless audit is confined to this pre-kill fixture process; the timed
    // restart below starts a fresh logic with the default recorder.
    logic.client.enable_fixture_capture_diagnostics();
    let go = root.join("go");
    // Finite output avoids a growing ramp dataset. Small seats trickle at
    // 32 KiB/s; the large seat provides a genuine partially parsed backlog.
    // Match stall/load.rs's 967/50 gate: the first 50 seats share one swarm,
    // the remainder share a second. The earlier one-swarm-per-seat workload
    // exposed O(swarms) message-pump cost (r5g-delivery-diagnostics.log).
    // Oracle d6aff179 declares this correction; messaging performance is separate.
    let swarms = [Ulid::new(), Ulid::new()];
    for (i, &seat) in seats.iter().enumerate() {
        let mut launch = LaunchSpec::new(root.join("producer").to_string_lossy(), &root);
        launch.seat = seat;
        launch.args = vec![
            go.to_string_lossy().into(),
            if i == 0 { BURST } else { 64 * 1024 }.to_string(),
            root.join("done")
                .join(seat.to_string())
                .to_string_lossy()
                .into(),
        ];
        runtime
            .block_on(logic.client.spawn(
                SeatRecord::new(
                    seat,
                    swarms[usize::from(i >= 50)],
                    "recovery fixture",
                    "fixture",
                ),
                launch,
            ))
            .unwrap();
        ready.lock().unwrap().insert(seat);
        // Populate a row even if this seat has not received output yet.
        runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        write_json(&root.join("progress.json"), &json!({"spawned":i+1}));
    }
    let endpoint = runtime.block_on(logic.client.gui_endpoint()).unwrap();
    write_json(
        &root.join("endpoints.json"),
        &json!({"root":endpoint.path.parent().unwrap()}),
    );
    std::fs::write(&go, b"go").unwrap();
    // Completion files acknowledge finite stdout writes, not capture. Capture
    // is checked by exact ring byte counts after SIGKILL. Avoid 967 serial
    // observer RPCs while the worker is supposed to be ingesting this burst.
    let mut completed = 0;
    let mut advanced = Instant::now();
    while completed < seats.len() {
        let count = std::fs::read_dir(root.join("done")).unwrap().count();
        if count != completed {
            completed = count;
            advanced = Instant::now();
            write_json(
                &root.join("progress.json"),
                &json!({"producers_done":count}),
            );
        }
        assert!(
            advanced.elapsed() < Duration::from_secs(90),
            "finite producers made no progress"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut byte_end = 0;
    let mut advanced = Instant::now();
    while byte_end < BURST + 14 {
        let history = runtime
            .block_on(logic.client.history(seats[0], 1, 1, 1))
            .unwrap();
        if history.status.byte_end != byte_end {
            byte_end = history.status.byte_end;
            advanced = Instant::now();
        }
        assert!(
            advanced.elapsed() < Duration::from_secs(90),
            "burst capture made no progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    delivery.lock().unwrap().release_requested = true;
    held.store(false, Ordering::Release);
    let snapshot = || {
        let mut value = serde_json::to_value(logic.client.capture_diagnostics()).unwrap();
        value["delivery"] = serde_json::to_value(*delivery.lock().unwrap()).unwrap();
        value["side_saves_after_readiness"] = json!(side_saves.load(Ordering::Relaxed));
        value["synchronous_saves_after_readiness"] =
            json!(synchronous_saves.load(Ordering::Relaxed));
        value
    };
    let until = Instant::now() + Duration::from_secs(90);
    while !frozen.load(Ordering::Acquire) {
        assert!(
            Instant::now() < until,
            "did not reach the mid-replay kill boundary; {}",
            serde_json::to_string(&snapshot()).unwrap()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    write_json(&root.join("frozen.json"), &snapshot());
    loop {
        std::thread::park();
    }
}

struct OwnedChild(Option<Child>, PathBuf);
impl OwnedChild {
    fn kill(&mut self) {
        if let Some(mut child) = self.0.take() {
            child.kill().unwrap();
            assert!(!child.wait().unwrap().success());
        }
        self.cleanup();
    }
    fn cleanup(&self) {
        if let Ok(bytes) = std::fs::read(self.1.join("endpoints.json"))
            && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
            && let Some(path) = value["root"].as_str()
        {
            let _ = std::fs::remove_dir_all(path); // Only our reaped child's namespace.
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.cleanup();
    }
}

// keeps: decision #253
#[test]
fn checkpoint_bounded_recovery() {
    // The full-count invocation is coordinated with the Oracle: two 967-PTY
    // fixtures cannot share this machine's 999-PTY ceiling.
    let count = std::env::var("R5G_SEATS").map_or(4, |n| n.parse::<usize>().unwrap());
    assert!((2..=967).contains(&count));
    // #254 / 11b00370: only the full-count gate has a calibrated wall budget.
    // Default functional fixtures retain every correctness assertion without it.
    let budget = if count == 967 {
        let ms = std::env::var("R5G_BUDGET_MS")
            .expect("967-seat gate requires the calibrated R5G_BUDGET_MS")
            .parse::<u64>()
            .expect("R5G_BUDGET_MS must be a positive integer");
        assert!(ms > 0, "R5G_BUDGET_MS must be positive");
        Some(Duration::from_millis(ms))
    } else {
        None
    };
    let temp = tempfile::Builder::new()
        .prefix("rook-recovery-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = temp.path();
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    check_disk(root);
    std::fs::write(root.join("producer.rs"), PRODUCER).unwrap();
    let compiler = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "-O"])
        .arg(root.join("producer.rs"))
        .arg("-o")
        .arg(root.join("producer"))
        .output()
        .unwrap();
    assert!(
        compiler.status.success(),
        "finite producer compile: {}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    std::fs::create_dir(root.join("repository")).unwrap();
    std::fs::create_dir(root.join("done")).unwrap();
    let supervisor = Supervisor::start(Config {
        socket_path: Some(root.join("supervisor.sock")),
        journal_directory: Some(root.join("journal")),
        max_seats: count,
        ..Default::default()
    })
    .unwrap();
    supervisor.enable_capture_diagnostics();
    let seats: Vec<_> = (0..count).map(|_| Ulid::new()).collect();
    write_json(
        &root.join("input.json"),
        &json!({"seats":seats,"capability":supervisor.socket_capability()}),
    );
    let stderr = std::fs::File::create(root.join("child.stderr")).unwrap();
    let mut child = OwnedChild(
        Some(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "checkpoint_recovery_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("R5G_RECOVERY_CHILD", root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(stderr)
                .spawn()
                .unwrap(),
        ),
        root.to_owned(),
    );
    let mut progress = Vec::new();
    let mut advanced = Instant::now();
    let mut disk_at = Instant::now();
    while !root.join("frozen.json").exists() {
        if disk_at.elapsed() >= Duration::from_secs(1) {
            check_disk(root);
            disk_at = Instant::now();
        }
        let current = std::fs::read(root.join("progress.json")).unwrap_or_default();
        if current != progress {
            progress = current;
            advanced = Instant::now();
        }
        let exit = child.0.as_mut().unwrap().try_wait().unwrap();
        assert!(
            exit.is_none(),
            "child exited {exit:?}: {}; supervisor_notification={}",
            std::fs::read_to_string(root.join("child.stderr")).unwrap(),
            serde_json::to_string(
                &supervisor
                    .capture_diagnostics()
                    .seats
                    .into_iter()
                    .find(|row| row.seat == seats[0])
            )
            .unwrap()
        );
        assert!(
            advanced.elapsed() < Duration::from_secs(120),
            "recovery child made no progress: {}",
            std::fs::read_to_string(root.join("child.stderr")).unwrap()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill(); // Actual SIGKILL, not orderly Logic::shutdown.
    let frozen_bytes = std::fs::read(root.join("frozen.json")).unwrap();
    if let Some(path) = std::env::var_os("R5G_RECOVERY_EVIDENCE") {
        // Archive before any assertion; tempfile cleanup still reaps all PIDs.
        // create_new preserves earlier evidence if a label is accidentally reused.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, &frozen_bytes).unwrap();
        println!(
            "checkpoint_recovery frozen_archive={} bytes={}",
            Path::new(&path).display(),
            frozen_bytes.len()
        );
    }
    let frozen: Value = serde_json::from_slice(&frozen_bytes).unwrap();
    println!("checkpoint_recovery delivery={}", frozen["delivery"]);
    println!(
        "checkpoint_recovery capture_phase_totals={}",
        frozen["totals"]
    );
    let actor = supervisor.capture_diagnostics();
    println!(
        "checkpoint_recovery supervisor_notification={}",
        serde_json::to_string(&actor.seats.iter().find(|row| row.seat == seats[0])).unwrap()
    );
    assert_eq!(frozen["fixture_lossless"], true);
    assert_eq!(frozen["missed"], 0);
    assert_eq!(frozen["untracked_seat_samples"], 0);
    assert_eq!(frozen["synchronous_saves_after_readiness"], 0);
    assert!(frozen["side_saves_after_readiness"].as_u64().unwrap() > 0);
    let rows: BTreeMap<Ulid, &Value> = frozen["seats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (serde_json::from_value(row["seat"].clone()).unwrap(), row))
        .collect();
    assert_eq!(rows.len(), count);
    // Probe only after the old owner has died, using one admitted serial link.
    let probe = supervisor.connect().unwrap();
    let Reply::Seats { seats: owned } = probe.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert_eq!(owned.len(), count);
    assert!(
        owned.iter().all(|info| info.exited.is_none()),
        "a producer exited before the controlled kill/restart"
    );
    let owned_pids: BTreeMap<_, _> = owned.iter().map(|info| (info.seat, info.pid)).collect();
    let mut tails = BTreeMap::new();
    let mut expected = BTreeMap::new();
    for &seat in &seats {
        let directory =
            bbt_mux::store::object_dir(&config(root).run_dir, &format!("seat-{seat}")).unwrap();
        let Reply::Attached { attachment, .. } = probe
            .request(Action::AttachRing {
                seat,
                generation: 1,
                directory: directory.join("ring-1"),
                settings: Default::default(),
            })
            .unwrap()
        else {
            panic!()
        };
        let Reply::RingRecovery {
            mut status,
            mut checkpoint,
        } = probe
            .request(Action::RingRecovery {
                attachment: attachment.clone(),
            })
            .unwrap()
        else {
            panic!()
        };
        let expected_bytes = if seat == seats[0] { BURST } else { 64 * 1024 } + 14;
        let mut previous = status.byte_end;
        let mut advanced = Instant::now();
        while status.byte_end < expected_bytes {
            let Reply::RingRecovery {
                status: next,
                checkpoint: saved,
            } = probe
                .request(Action::RingRecovery {
                    attachment: attachment.clone(),
                })
                .unwrap()
            else {
                panic!()
            };
            if next.byte_end != previous {
                previous = next.byte_end;
                advanced = Instant::now();
            }
            assert!(
                advanced.elapsed() < Duration::from_secs(90),
                "ring capture did not reach the producer's completed byte count"
            );
            status = next;
            checkpoint = saved;
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            status.byte_end, expected_bytes,
            "unexpected producer output"
        );
        let row = rows[&seat];
        let parsed = row["parsed_byte_end"].as_u64().unwrap();
        let confirmed = row["checkpoint_byte_end"].as_u64().unwrap();
        assert!(
            parsed >= confirmed && parsed - confirmed <= 2 * R,
            "parsed debt: {row}"
        );
        println!(
            "checkpoint_recovery seat={seat} parsed_debt={} backlog={} parsed={parsed} confirmed={confirmed} tail={}",
            parsed - confirmed,
            status.byte_end.saturating_sub(parsed),
            status.byte_end
        );
        if seat == seats[0] {
            assert!(
                parsed >= R && parsed < status.byte_end,
                "SIGKILL was not mid-replay"
            );
            assert_eq!(row["idle_checkpoints"], 0, "idle save on the held backlog");
        }
        let mut playback = Playback::restore(Saved::decode(&checkpoint.unwrap()).unwrap());
        while playback.sequence < status.captured_sequence {
            let Reply::RingHistory { history } = probe
                .request(Action::ReadRing {
                    attachment: attachment.clone(),
                    from: playback.sequence + 1,
                    to: status.captured_sequence,
                })
                .unwrap()
            else {
                panic!()
            };
            assert!(history.gap.is_none() && !history.records.is_empty());
            for record in history.records {
                playback.apply(&record.event).unwrap();
            }
        }
        expected.insert(
            seat,
            bbt_mux::streams::Screen::from_term(
                &playback.term,
                playback.title.clone(),
                playback.cwd.clone(),
                playback.gap.clone(),
            ),
        );
        tails.insert(seat, status.captured_sequence);
    }
    drop(probe);
    let runtime = runtime();
    let started = Instant::now();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config(root),
        SigningKey::from_bytes(&[93; 32]),
        runtime.handle(),
    )
    .unwrap();
    for &seat in &seats {
        let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
        assert!(snapshot.sequence >= tails[&seat]);
        assert_eq!(snapshot.screen, expected[&seat]);
    }
    let elapsed = started.elapsed();
    if let Some(budget) = budget {
        assert!(
            elapsed <= budget,
            "all {count} seats live in {elapsed:?}, above calibrated budget {budget:?}"
        );
    }
    assert!(!logic.client.capture_diagnostics().fixture_lossless);
    logic.shutdown().unwrap();
    let probe = supervisor.connect().unwrap();
    let Reply::Seats { seats: recovered } = probe.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert_eq!(recovered.len(), count);
    for info in recovered {
        assert_eq!(info.pid, owned_pids[&info.seat]);
        assert!(
            info.exited.is_none(),
            "recovery returned an exited producer"
        );
    }
    drop(probe);
    supervisor.shutdown().unwrap();
    for pid in owned_pids.values() {
        let mut status = 0;
        // SAFETY: these are this fixture's recorded direct children, the status
        // pointer is valid, and WNOHANG never waits for a still-running child.
        let result = unsafe { libc::waitpid(*pid as libc::pid_t, &mut status, libc::WNOHANG) };
        assert_eq!(
            result, -1,
            "supervisor failed to reap its owned child {pid}"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
    println!(
        "checkpoint_recovery reaped_owned_pids={:?}",
        owned_pids.values().collect::<Vec<_>>()
    );
    println!(
        "checkpoint_recovery seats={count} every_seat_live_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}
