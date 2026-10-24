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
    std::thread::sleep(Duration::from_secs(300));
}
"#;

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
        let burst_read = matches!(&action, Action::ReadRing { attachment, .. }
            if attachment.seat == self.burst_seat && !self.held.load(Ordering::Acquire));
        let parsed_end = match &action {
            Action::ReadRing { from, .. } => self
                .byte_ends
                .get(&from.saturating_sub(1))
                .copied()
                .unwrap_or(0),
            _ => 0,
        };
        if burst_read && parsed_end >= R + R / 2 {
            // The previous page has returned to Worker and its cursor/accounting
            // has been published. This thread cannot parse another byte. No
            // checkpoint owns the serial stream while ReadRing owns its lock.
            self.frozen.store(true, Ordering::Release);
            loop {
                std::thread::park();
            }
        }
        let reply = self.inner.request(action)?;
        if burst_read && let Reply::RingHistory { history } = &reply {
            for record in &history.records {
                let seq = record.event.sequence().unwrap();
                let previous = self.byte_ends[&(seq - 1)];
                let bytes = match &record.event {
                    Event::Output { bytes, .. } => bytes.len() as u64,
                    _ => 0,
                };
                self.byte_ends.insert(seq, previous + bytes);
            }
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if !self.held.load(Ordering::Acquire)
            && let Some(event) = self.latest.take()
        {
            return Ok(Some(event));
        }
        let mut wait = timeout;
        loop {
            let Some(event) = self.inner.receive(wait)? else {
                return Ok(None);
            };
            if event.seat() == self.burst_seat && self.held.load(Ordering::Acquire) {
                self.latest = Some(event);
                wait = Duration::ZERO;
            } else {
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
        },
        config(&root),
        SigningKey::from_bytes(&[93; 32]),
        runtime.handle(),
    )
    .unwrap();
    logic.client.enable_capture_diagnostics();
    let go = root.join("go");
    // Finite output avoids a growing ramp dataset. Small seats trickle at
    // 32 KiB/s; the large seat provides a genuine partially parsed backlog.
    for (i, &seat) in seats.iter().enumerate() {
        let mut launch = LaunchSpec::new(root.join("producer").to_string_lossy(), &root);
        launch.seat = seat;
        launch.args = vec![
            go.to_string_lossy().into(),
            if i == 0 { BURST } else { 64 * 1024 }.to_string(),
        ];
        runtime
            .block_on(logic.client.spawn(
                SeatRecord::new(seat, Ulid::new(), "recovery fixture", "fixture"),
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
    for (i, &seat) in seats.iter().enumerate().skip(1) {
        let mut previous = 0;
        let mut advanced = Instant::now();
        loop {
            let history = runtime
                .block_on(logic.client.history(seat, 1, 1, 1))
                .unwrap();
            if history.status.byte_end >= 64 * 1024 + 14 {
                break;
            }
            if history.status.byte_end != previous {
                previous = history.status.byte_end;
                advanced = Instant::now();
            }
            assert!(
                advanced.elapsed() < Duration::from_secs(90),
                "trickling capture stopped"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        write_json(&root.join("progress.json"), &json!({"captured":i+1}));
    }
    held.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(90);
    while !frozen.load(Ordering::Acquire) {
        assert!(
            Instant::now() < until,
            "did not reach the mid-replay kill boundary; {}",
            serde_json::to_string(&logic.client.capture_diagnostics()).unwrap()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut snapshot = serde_json::to_value(logic.client.capture_diagnostics()).unwrap();
    snapshot["side_saves_after_readiness"] = json!(side_saves.load(Ordering::Relaxed));
    snapshot["synchronous_saves_after_readiness"] =
        json!(synchronous_saves.load(Ordering::Relaxed));
    write_json(&root.join("frozen.json"), &snapshot);
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

#[test]
fn checkpoint_bounded_recovery() {
    // The full-count invocation is coordinated with the Oracle: two 967-PTY
    // fixtures cannot share this machine's 999-PTY ceiling.
    let count = std::env::var("R5G_SEATS").map_or(4, |n| n.parse::<usize>().unwrap());
    assert!((2..=967).contains(&count));
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
    let supervisor = Supervisor::start(Config {
        socket_path: Some(root.join("supervisor.sock")),
        journal_directory: Some(root.join("journal")),
        max_seats: count,
        ..Default::default()
    })
    .unwrap();
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
            "child exited {exit:?}: {}",
            std::fs::read_to_string(root.join("child.stderr")).unwrap()
        );
        assert!(
            advanced.elapsed() < Duration::from_secs(120),
            "recovery child made no progress: {}",
            std::fs::read_to_string(root.join("child.stderr")).unwrap()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill(); // Actual SIGKILL, not orderly Logic::shutdown.
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(root.join("frozen.json")).unwrap()).unwrap();
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
        let Reply::RingRecovery { status, checkpoint } = probe
            .request(Action::RingRecovery {
                attachment: attachment.clone(),
            })
            .unwrap()
        else {
            panic!()
        };
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
    println!(
        "checkpoint_recovery seats={count} every_seat_live_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    logic.shutdown().unwrap();
}
