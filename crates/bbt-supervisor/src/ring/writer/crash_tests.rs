#![allow(clippy::disallowed_methods)] // Explicit real-process crash/durability fixture.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::probe::Probe;
use crate::ring::{Ring, Settings};
use crate::{Action, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};
use serde_json::{Value, json};

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

#[test]
#[ignore = "private child of capture_sigkill_replays_a_crc_valid_prefix_with_at_most_one_tick_lost"]
fn capture_crash_child() {
    let Some(root) = std::env::var_os("BBT_CRASH_FIXTURE_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut seats = Vec::new();
    let mut attachments = Vec::new();
    for i in 0..2 {
        let ring_root = root.join(format!("ring-{i}"));
        Probe::create(&ring_root).unwrap();
        let mut spec = LaunchSpec::new("python3", &root);
        spec.env.insert(
            "BBT_FIXTURE_FINAL".into(),
            root.join(format!("final-{i}.json"))
                .to_string_lossy()
                .into(),
        );
        spec.args = vec![
            "-c".into(),
            r#"
import os,time,signal,json
stopping=False
def stop(*_):
 global stopping
 stopping=True
signal.signal(signal.SIGHUP,stop)
written=0; times=[]; i=0
try:
 while not stopping:
  frame=(f'{i:08x}'+'x'*56).encode(); pos=0
  while pos<len(frame):
   try: n=os.write(1,frame[pos:])
   except OSError: stopping=True; break
   pos+=n; written+=n; times.append([written,time.monotonic_ns()])
  i+=1
  time.sleep(.002)
finally:
 p=os.environ['BBT_FIXTURE_FINAL']
 with open(p+'.tmp','w') as f: json.dump({'written':written,'times':times},f)
 os.replace(p+'.tmp',p)
"#
            .into(),
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
    }
    loop {
        let mut ready = true;
        for attachment in &attachments {
            let Reply::RingRecovery { status, .. } = client
                .request(Action::RingRecovery {
                    attachment: attachment.clone(),
                })
                .unwrap()
            else {
                panic!()
            };
            ready &= status.capture.ordered_commits >= 5;
        }
        if ready {
            let path = root.join("ready.json");
            let tmp = root.join("ready.tmp");
            std::fs::write(&tmp, serde_json::to_vec(&seats).unwrap()).unwrap();
            std::fs::rename(tmp, path).unwrap();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
#[ignore = "release crash gate: cargo test -p bbt-supervisor --release --lib capture_sigkill -- --ignored --nocapture"]
fn capture_sigkill_replays_a_crc_valid_prefix_with_at_most_one_tick_lost() {
    use std::os::unix::process::ExitStatusExt;
    let temp = tempfile::tempdir().unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ring::writer::crash_tests::capture_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BBT_CRASH_FIXTURE_ROOT", temp.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(10);
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
    for (i, seat) in seats.as_array().unwrap().iter().enumerate() {
        let until = Instant::now() + Duration::from_secs(5);
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
        let marker = std::fs::read(root.join("committed")).unwrap();
        let marker: Value = serde_json::from_slice(&marker[..marker.len() - 4]).unwrap();
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let status = ring.status();
        assert_eq!(
            status.captured_sequence,
            marker["sequence"].as_u64().unwrap(),
            "current marker failed CRC validation"
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
        for (offset, &actual) in bytes.iter().enumerate() {
            let frame = format!("{:08x}{}", offset / 64, "x".repeat(56));
            assert_eq!(
                actual,
                frame.as_bytes()[offset % 64],
                "replayed bytes are not the exact generated prefix"
            );
        }
        let written = final_stats["written"].as_u64().unwrap();
        assert!(written >= admitted && admitted >= bytes.len() as u64);
        let pre_capture = written - admitted;
        let admitted_loss = admitted - bytes.len() as u64;
        let pty_capacity = measured_pty_capacity();
        println!(
            "{}",
            json!({"seat":seat,"signal":"SIGKILL","written":written,"admitted":admitted,"committed":bytes.len(),"admitted_loss":admitted_loss,"admitted_tick_bytes":admitted_tick_bytes,"pre_capture_tail":pre_capture,"measured_pty_buffer":pty_capacity,"coalescing_limit":crate::protocol::OUTPUT_CHUNK_BYTES,"crc_errors":0})
        );
        assert!(
            admitted_loss <= admitted_tick_bytes,
            "lost more than the admitted data from one 20 ms tick"
        );
    }
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
