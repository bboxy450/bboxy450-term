#![allow(clippy::disallowed_methods)] // Owned process/socket fixtures, never UI.

use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

use bbt_supervisor::protocol::CompanionSpec;
use bbt_supervisor::*;

const WATCHDOG: Duration = Duration::from_secs(30);

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
#[ignore = "private service subprocess, exercised by the parent tests"]
fn service_fixture() {
    // SAFETY: read this fixture's own session/process-group IDs only.
    assert_eq!(
        unsafe { libc::getsid(0) },
        std::process::id() as libc::pid_t
    );
    assert_eq!(
        unsafe { libc::getpgrp() },
        std::process::id() as libc::pid_t
    );
    // SAFETY: read only this service's own tty and standard descriptor state.
    unsafe {
        if let Ok(lifetime) = std::fs::File::open("/dev/tty") {
            assert_eq!(libc::tcgetpgrp(lifetime.as_raw_fd()), libc::getpgrp());
        }
        for fd in 0..=2 {
            assert_eq!(libc::isatty(fd), 0, "service stdio acquired a tty");
        }
    }
    let socket = std::env::var_os("BBT_SERVICE_SOCKET").unwrap();
    let pid = std::env::var_os("BBT_SERVICE_PIDFILE").unwrap();
    std::fs::write(pid, std::process::id().to_string()).unwrap();
    match std::env::var("BBT_SERVICE_MODE").unwrap().as_str() {
        "fail" => std::process::exit(17),
        "no-ready" => loop {
            std::thread::park();
        },
        "ready" => {
            let listener = std::os::unix::net::UnixListener::bind(socket).unwrap();
            for stream in listener.incoming() {
                drop(stream.unwrap());
            }
        }
        _ => panic!("unexpected fixture mode"),
    }
}

fn plan(root: &Path, mode: &str) -> LaunchSpec {
    let mut spec = LaunchSpec::new("/bin/zsh", root);
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        "printf 'service-pty-ready\\n'; exec /bin/sleep 600".into(),
    ];
    spec.companion = Some(Box::new(CompanionSpec {
        program: std::env::current_exe().unwrap().to_string_lossy().into(),
        args: vec![
            "--ignored".into(),
            "--exact".into(),
            "service_fixture".into(),
            "--nocapture".into(),
        ],
        env: [
            (
                "BBT_SERVICE_SOCKET".into(),
                root.join("s").to_string_lossy().into(),
            ),
            (
                "BBT_SERVICE_PIDFILE".into(),
                root.join("pid").to_string_lossy().into(),
            ),
            ("BBT_SERVICE_MODE".into(), mode.into()),
        ]
        .into(),
        ready_socket: root.join("s"),
    }));
    spec
}

fn absent(pid: u32) {
    // SAFETY: signal zero observes the recorded fixture PID; it sends no signal.
    assert_eq!(
        unsafe { libc::kill(pid as libc::pid_t, 0) },
        -1,
        "owned service remains live"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

fn exit(client: &Client, seat: Ulid) {
    let deadline = Instant::now() + WATCHDOG;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "owned seat never published exit");
        if let Event::Exited { seat: id, .. } = client.recv_timeout(left).unwrap()
            && id == seat
        {
            return;
        }
    }
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn companion_survives_client_restart_and_is_reaped_before_seat_exit() {
    let root = tempfile::tempdir().unwrap();
    let server = Supervisor::start(Config::default()).unwrap();
    let client = server.connect().unwrap();
    let spec = plan(root.path(), "ready");
    let info = client.spawn(spec.clone()).unwrap();
    let service = info.companion.clone().unwrap();
    assert_ne!(service.pid, info.pid);
    assert_eq!(service.ready_socket, root.path().join("s"));
    assert_eq!(
        client.spawn(spec).unwrap(),
        info,
        "retry launched a second service"
    );
    let _attachment = client.attach(info.seat, info.generation).unwrap();
    let deadline = Instant::now() + WATCHDOG;
    let mut output = Vec::new();
    while !output
        .windows(b"service-pty-ready".len())
        .any(|v| v == b"service-pty-ready")
    {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "PTY did not start after service readiness");
        if let Event::Output { bytes, .. } = client.recv_timeout(left).unwrap() {
            output.extend(bytes);
        }
    }
    drop(client);
    let client = server.connect().unwrap();
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!("wrong list reply")
    };
    assert_eq!(seats.len(), 1);
    assert_eq!(seats[0].pid, info.pid, "client restart replaced the PTY");
    assert_eq!(
        seats[0].companion, info.companion,
        "client restart replaced the owned service"
    );
    assert_eq!(seats[0].generation, info.generation);
    let attachment = client.attach(info.seat, info.generation).unwrap();
    let deadline_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    client
        .request(Action::ScheduleClose {
            attachment,
            close_id: Ulid::new(),
            deadline_ms,
        })
        .unwrap();
    exit(&client, info.seat);
    absent(service.pid);
    server.shutdown().unwrap();
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn companion_failure_ends_the_seat_and_reaps_the_service() {
    let root = tempfile::tempdir().unwrap();
    let server = Supervisor::start(Config::default()).unwrap();
    let client = server.connect().unwrap();
    let info = client.spawn(plan(root.path(), "fail")).unwrap();
    client.attach(info.seat, info.generation).unwrap();
    exit(&client, info.seat);
    absent(info.companion.unwrap().pid);
    absent(info.pid);
    server.shutdown().unwrap();
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn unready_companion_never_holds_the_serial_client_or_recovery_inventory() {
    let root = tempfile::tempdir().unwrap();
    let server = Supervisor::start(Config::default()).unwrap();
    let client = server.connect().unwrap();
    let info = client.spawn(plan(root.path(), "no-ready")).unwrap();
    let service = info.companion.clone().unwrap();
    // The same serial client can immediately list, input and spawn another seat:
    // none of these operations depends on the first service binding its socket.
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!("wrong inventory reply")
    };
    assert_eq!(seats.as_slice(), std::slice::from_ref(&info));
    assert!(!service.ready_socket.exists());
    let mut other = LaunchSpec::new("/bin/sleep", root.path());
    other.args = vec!["600".into()];
    let other = client.spawn(other).unwrap();
    let attachment = client.attach(other.seat, other.generation).unwrap();
    client.kill(&attachment).unwrap();
    exit(&client, other.seat);
    drop(client);
    let client = server.connect().unwrap();
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!("wrong recovery inventory reply")
    };
    assert!(seats.iter().any(|seat| seat == &info));
    let attachment = client.attach(info.seat, info.generation).unwrap();
    client.kill(&attachment).unwrap();
    exit(&client, info.seat);
    absent(service.pid);
    server.shutdown().unwrap();
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
#[ignore = "private supervisor subprocess, exercised by the parent test"]
fn supervisor_process_fixture() {
    let root = std::env::var_os("BBT_SERVICE_ROOT").unwrap();
    let root = Path::new(&root);
    let server = Supervisor::start(Config::default()).unwrap();
    let client = server.connect().unwrap();
    let info = client.spawn(plan(root, "ready")).unwrap();
    std::fs::write(root.join("seat.json"), serde_json::to_vec(&info).unwrap()).unwrap();
    loop {
        std::thread::park();
    }
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn companion_dies_when_its_supervisor_process_is_killed() {
    let root = tempfile::tempdir().unwrap();
    let mut owner = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "supervisor_process_fixture",
            "--nocapture",
        ])
        .env("BBT_SERVICE_ROOT", root.path())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + WATCHDOG;
    let info: SeatInfo = loop {
        assert!(
            owner.try_wait().unwrap().is_none(),
            "supervisor fixture exited"
        );
        if let Ok(bytes) = std::fs::read(root.path().join("seat.json"))
            && let Ok(info) = serde_json::from_slice(&bytes)
            && root.path().join("s").exists()
        {
            break info;
        }
        if Instant::now() >= deadline {
            owner.kill().unwrap();
            owner.wait().unwrap();
            panic!("private service never bound its readiness socket");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let service = info.companion.unwrap();
    owner.kill().unwrap();
    let status = owner.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    // No graceful Drop/TERM path can run in the killed supervisor. The kernel
    // hangup must retire both its existing PTY child and its private service.
    let mut survivors = Vec::new();
    for pid in [service.pid, info.pid] {
        loop {
            // SAFETY: observe only the exact fixture PID; no signal is sent.
            if unsafe { libc::kill(pid as libc::pid_t, 0) } == -1 {
                absent(pid);
                break;
            }
            if Instant::now() >= deadline {
                // SAFETY: clean up the exact child recorded by our owned
                // supervisor fixture, which has not exited during observation.
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGKILL);
                }
                survivors.push(pid);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(
        survivors.is_empty(),
        "owned children survived supervisor SIGKILL: {survivors:?}"
    );
}
