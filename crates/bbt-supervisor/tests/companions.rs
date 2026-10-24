#![allow(clippy::disallowed_methods)] // Owned process/socket fixtures, never UI.

use std::path::Path;
use std::time::{Duration, Instant};

use bbt_supervisor::protocol::CompanionSpec;
use bbt_supervisor::*;

const WATCHDOG: Duration = Duration::from_secs(15);

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
    assert!(
        std::fs::File::open("/dev/tty").is_err(),
        "service inherited a controlling tty"
    );
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
