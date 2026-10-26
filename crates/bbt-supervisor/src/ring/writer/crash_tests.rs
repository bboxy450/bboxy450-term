#![allow(clippy::disallowed_methods)] // Explicit real-process crash/durability fixture.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::probe::Probe;
use crate::ring::{Ring, Settings};
use crate::{Action, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};
use serde_json::{Value, json};

fn crash_seats() -> usize {
    let count = std::env::var("BBT_CRASH_SEATS")
        .map(|value| value.parse().unwrap())
        .unwrap_or(2);
    assert!((1..=967).contains(&count));
    count
}

fn check_disk_reserve(root: &Path) {
    let output = Command::new("df")
        .env("LC_ALL", "C")
        .arg("-Pk")
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let available: u64 = text
        .lines()
        .last()
        .unwrap()
        .split_whitespace()
        .nth(3)
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1024;
    eprintln!("CRASH free_disk_bytes={available} min_free_bytes=10737418240");
    assert!(
        available >= 10 * 1024 * 1024 * 1024,
        "crash fixture disk reserve exhausted"
    );
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_json(path: &Path) -> Option<Value> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
#[ignore = "private child of capture_sigkill_replays_a_crc_valid_prefix_with_at_most_one_tick_lost"]
fn capture_crash_child() {
    let Some(root) = std::env::var_os("BBT_CRASH_FIXTURE_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let count = crash_seats();
    check_disk_reserve(&root);
    let supervisor = Supervisor::start(Config {
        journal_directory: Some(root.join("journal")),
        max_seats: count,
        ..Default::default()
    })
    .unwrap();
    let client = supervisor.connect().unwrap();
    let mut seats = Vec::new();
    let mut attachments = Vec::new();
    let mut probes = Vec::new();
    for i in 0..count {
        let ring_root = root.join(format!("ring-{i}"));
        Probe::create(&ring_root).unwrap();
        probes.push(Probe::open(&ring_root).unwrap().unwrap());
        let mut spec = LaunchSpec::new(std::env::var("BBT_CRASH_PRODUCER").unwrap(), &root);
        spec.args = vec![
            root.join(format!("final-{i}.json"))
                .to_string_lossy()
                .into(),
            root.join("go").to_string_lossy().into(),
        ];
        let info = client.spawn(spec).unwrap();
        let Reply::Attached { attachment, .. } = client
            .request(Action::AttachRing {
                seat: info.seat,
                generation: 1,
                directory: ring_root,
                settings: Settings::default(),
            })
            .unwrap()
        else {
            panic!()
        };
        seats.push(info.seat);
        attachments.push(attachment);
        if (i + 1) % 100 == 0 || i + 1 == count {
            check_disk_reserve(&root);
            eprintln!("CRASH staged={}/{count}", i + 1);
        }
    }
    std::fs::write(root.join("go"), []).unwrap();
    loop {
        let Reply::RingRecovery { status, .. } = client
            .request(Action::RingRecovery {
                attachment: attachments[0].clone(),
            })
            .unwrap()
        else {
            panic!()
        };
        // Every seat has admitted at least ten frames. The shared journal
        // has ordered five ticks; do not add 967 serial status RPCs here.
        if status.capture.commit_rounds >= 5 && probes.iter().all(|probe| probe.snapshot().0 >= 640)
        {
            let path = root.join("ready.json");
            let tmp = root.join("ready.tmp");
            std::fs::write(&tmp, serde_json::to_vec(&seats).unwrap()).unwrap();
            std::fs::rename(tmp, path).unwrap();
            std::thread::park();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
#[ignore = "release crash gate: cargo test -p bboxy450-harness-host --release --lib capture_sigkill -- --ignored --nocapture"]
fn capture_sigkill_replays_a_crc_valid_prefix_with_at_most_one_tick_lost() {
    use std::os::unix::process::ExitStatusExt;
    let count = crash_seats();
    // SAFETY: this process owns its fd limit; never raise it above the hard cap.
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit.rlim_cur = limit.rlim_max.min(8192);
    // SAFETY: a valid resource limit structure for this fixture process only.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("producer.rs");
    let producer = temp.path().join("producer");
    std::fs::write(&source, include_str!("crash_producer.rs")).unwrap();
    assert!(
        Command::new("rustc")
            .args(["--edition=2024", "-O"])
            .arg(&source)
            .arg("-o")
            .arg(&producer)
            .status()
            .unwrap()
            .success()
    );
    let child_log = std::fs::File::create(temp.path().join("child.log")).unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ring::writer::crash_tests::capture_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BBT_CRASH_FIXTURE_ROOT", temp.path())
            .env("BBT_CRASH_PRODUCER", &producer)
            .stdout(Stdio::null())
            .stderr(child_log)
            .spawn()
            .unwrap(),
    );
    println!("CRASH owned_supervisor_pid={} seats={count}", child.0.id());
    let until = Instant::now() + Duration::from_secs(120);
    let seats = loop {
        if let Some(seats) = read_json(&temp.path().join("ready.json")) {
            break seats;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "capture child failed before ready"
        );
        assert!(
            Instant::now() < until,
            "capture child did not commit five ticks"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    std::thread::sleep(Duration::from_millis(113));
    child.0.kill().unwrap();
    assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    assert_eq!(seats.as_array().unwrap().len(), count);
    print!(
        "{}",
        std::fs::read_to_string(temp.path().join("child.log")).unwrap()
    );
    let replay_started = Instant::now();
    let pty_capacity = measured_pty_capacity();
    // Scan the global commit boundary once; each real Ring::open below then
    // independently uses the production recovery path for its own projection.
    let mut boundaries = std::collections::HashMap::new();
    for path in super::super::journal::journal_files(&temp.path().join("journal")).unwrap() {
        let (records, _, _) = super::super::journal::scan(&path).unwrap();
        for record in records.into_iter().filter(|r| r.kind != 4) {
            boundaries.insert(
                record.key,
                (
                    record.sequence,
                    record.byte_start
                        + if record.kind == 1 {
                            record.length as u64
                        } else {
                            0
                        },
                ),
            );
        }
    }
    let mut total_written = 0;
    let mut total_committed = 0;
    for (i, seat) in seats.as_array().unwrap().iter().enumerate() {
        let until = Instant::now() + Duration::from_secs(20);
        let final_stats = loop {
            if let Some(stats) = read_json(&temp.path().join(format!("final-{i}.json"))) {
                break stats;
            }
            assert!(
                Instant::now() < until,
                "owned producer did not report successful writes after SIGHUP"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        let seat: Ulid = serde_json::from_value(seat.clone()).unwrap();
        let root = temp.path().join(format!("ring-{i}"));
        let (admitted, admitted_tick_bytes) = Probe::open(&root).unwrap().unwrap().snapshot();
        // The journal, not the stale derived ring marker, owns tick commits.
        let committed = boundaries.get(&(seat, 1)).copied().unwrap_or((0, 0));
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let status = ring.status();
        assert_eq!(
            (status.captured_sequence, status.byte_end),
            committed,
            "projection did not recover the journal boundary"
        );
        let mut bytes = Vec::new();
        let mut from = 1;
        while from <= status.captured_sequence {
            let history = ring.history(from, status.captured_sequence).unwrap();
            assert!(history.gap.is_none());
            for record in history.records {
                if let Event::Output { bytes: chunk, .. } = record.event {
                    bytes.extend(chunk);
                }
            }
            from = history.next.unwrap_or(status.captured_sequence + 1);
        }
        for (index, actual) in bytes.chunks(64).enumerate() {
            let frame = format!("{index:08x}{}", "x".repeat(56));
            assert_eq!(
                actual,
                &frame.as_bytes()[..actual.len()],
                "replayed bytes are not the exact generated prefix"
            );
        }
        let written = final_stats["written"].as_u64().unwrap();
        assert!(written >= admitted && admitted >= bytes.len() as u64);
        let pre_capture = written - admitted;
        let admitted_loss = admitted - bytes.len() as u64;
        // This producer writes 64 bytes every 2 ms: one coalescing window.
        let coalescing_limit = 64;
        println!(
            "{}",
            json!({"seat":seat,"signal":"SIGKILL","written":written,"admitted":admitted,"committed":bytes.len(),"admitted_loss":admitted_loss,"admitted_tick_bytes":admitted_tick_bytes,"pre_capture_tail":pre_capture,"measured_pty_buffer":pty_capacity,"coalescing_limit":coalescing_limit,"crc_errors":0})
        );
        assert!(pre_capture <= pty_capacity as u64 + coalescing_limit);
        assert!(
            admitted_loss <= admitted_tick_bytes,
            "lost more than the admitted data from one 20 ms tick"
        );
        assert!(!bytes.is_empty(), "a staged seat never committed output");
        total_written += written;
        total_committed += bytes.len();
        if (i + 1) % 100 == 0 || i + 1 == count {
            eprintln!(
                "CRASH replayed={}/{count} elapsed_s={:.3}",
                i + 1,
                replay_started.elapsed().as_secs_f64()
            );
        }
    }
    println!(
        "CRASH seats={count} replayed={count} crc_errors=0 total_written={total_written} total_committed={total_committed} replay_s={:.3}",
        replay_started.elapsed().as_secs_f64()
    );
}

fn measured_pty_capacity() -> usize {
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd};
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty initializes two descriptor out-parameters. No name or
    // termios/winsize pointer is supplied; both returned FDs are owned here.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        0
    );
    // SAFETY: each successful openpty FD is transferred into exactly one File.
    let (_master, mut slave) = unsafe {
        (
            std::fs::File::from_raw_fd(master),
            std::fs::File::from_raw_fd(slave),
        )
    };
    // SAFETY: fcntl operates on this fixture's owned slave descriptor only.
    let flags = unsafe { libc::fcntl(slave.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags, -1);
    // SAFETY: preserve existing flags, adding nonblocking mode on our FD.
    assert_eq!(
        unsafe { libc::fcntl(slave.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let mut written = 0;
    while written < 1024 * 1024 {
        match slave.write(&[b'x'; 64]) {
            Ok(0) => return written,
            Ok(n) => written += n,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return written,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => panic!("unexpected PTY capacity result: {result:?}"),
        }
    }
    panic!("PTY fixture exceeded its bounded capacity probe")
}
