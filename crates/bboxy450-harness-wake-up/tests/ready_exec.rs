#![allow(clippy::disallowed_methods)] // Private process/socket fixtures, never UI.

use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

// keeps: decision #279
#[test]
#[ignore = "private hook subprocess, invoked by its parent"]
fn hook_transport_child() {
    struct Sink;
    impl bboxy450_harness_wake_up::hook::HookSink for Sink {
        fn send(
            &self,
            seat: bboxy450_harness_wake_up::Ulid,
            event: bboxy450_harness_wake_up::hook::ShimEvent,
            _until: Instant,
        ) -> bboxy450_harness_wake_up::Result<()> {
            std::fs::write(
                std::env::var_os("BBT_HOOK_RESULT").unwrap(),
                serde_json::to_vec(&(seat, event))?,
            )?;
            Ok(())
        }
    }
    bboxy450_harness_wake_up::hook::run_cli(
        &[
            "--agent".into(),
            "codex".into(),
            r#"{"hook_event_name":"SessionStart","session_id":"private-hook-session"}"#.into(),
        ],
        &Sink,
    );
}

struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// keeps: decision #279
#[test]
#[ignore = "private ready-exec subprocess, invoked by its parent"]
fn ready_exec_child() {
    let root = std::path::PathBuf::from(std::env::var_os("BBT_READY_ROOT").unwrap());
    let error = bboxy450_harness_wake_up::launch::exec_when_ready(&[
        root.join("s").to_string_lossy().into(),
        "/bin/sh".into(),
        "-c".into(),
        "printf '%s' \"$$\" > \"$1\"; /usr/bin/env > \"$1.env\"".into(),
        "ready-exec".into(),
        root.join("exec-pid").to_string_lossy().into(),
    ])
    .unwrap_err();
    std::fs::write(root.join("error"), error.to_string()).unwrap();
    std::process::exit(31);
}

fn child(root: &Path) -> Owned {
    Owned(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "ready_exec_child", "--nocapture"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", "fixture-home")
            .env("CODEX_HOME", "fixture-codex")
            .env("OPENAI_API_KEY", "fixture-auth")
            .env("BBOXY_CAPABILITY", "fixture-admission")
            .envs([
                ("CODEX_THREAD_ID", "parent-thread"),
                ("ICQ_CODEX_SOCKET", "parent-socket"),
                ("CLAUDE_CODE_SESSION_ID", "parent-claude"),
                ("CLAUDE_CODE_MESSAGING_SOCKET", "parent-claude-socket"),
                ("PI_SESSION_ID", "parent-pi"),
                ("PI_SESSION_FILE", "parent-pi-file"),
                ("ICQ_HARNESS_DESCRIPTOR", "parent-descriptor"),
            ])
            .env("BBT_READY_ROOT", root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn drain_closed(reader: &mut (impl Read + AsRawFd)) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let mut descriptor = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: the descriptor is borrowed from this fixture's live reader.
        let ready = unsafe { libc::poll(&mut descriptor, 1, 30_000) };
        if ready < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        assert!(ready > 0, "private fixture hung or its pipe failed");
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break, // PTY EOF.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("private fixture read failed: {error}"),
        }
        assert!(output.len() <= 64 * 1024, "private fixture output overflow");
    }
    output
}

// keeps: decision #279
#[test]
fn a_ready_runtime_execs_the_client_in_the_same_pty_process() {
    let root = tempfile::tempdir().unwrap();
    // Delay and deadline behavior are checked with the launch module's fake
    // clock. This boundary check starts with its own selected listener ready.
    let _listener = std::os::unix::net::UnixListener::bind(root.path().join("s")).unwrap();
    let mut child = child(root.path());
    drain_closed(&mut child.0.stdout.take().unwrap());
    assert!(child.0.wait().unwrap().success());
    let pid: u32 = std::fs::read_to_string(root.path().join("exec-pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(pid, child.0.id(), "exec introduced a second client process");
    let env = std::fs::read_to_string(root.path().join("exec-pid.env")).unwrap();
    for key in [
        "CODEX_THREAD_ID",
        "ICQ_CODEX_SOCKET",
        "CLAUDE_CODE_SESSION_ID",
        "CLAUDE_CODE_MESSAGING_SOCKET",
        "PI_SESSION_ID",
        "PI_SESSION_FILE",
        "ICQ_HARNESS_DESCRIPTOR",
    ] {
        assert!(!env.lines().any(|line| line.starts_with(&format!("{key}="))));
    }
    for value in [
        "HOME=fixture-home",
        "CODEX_HOME=fixture-codex",
        "OPENAI_API_KEY=fixture-auth",
        "BBOXY_CAPABILITY=fixture-admission",
    ] {
        assert!(env.lines().any(|line| line == value), "lost {value}");
    }
    assert!(!root.path().join("error").exists());
}

// keeps: decision #279
#[test]
fn private_runtime_hooks_use_socket_even_with_a_controlling_tty() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;

    for socket_only in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let result = root.path().join("hook.json");
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: openpty initializes both owned descriptor out-parameters.
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
        // SAFETY: each descriptor is transferred to exactly one owner.
        let (master, slave) =
            unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
        let slave_fd = slave.as_raw_fd();
        // SAFETY: change only this fixture's private master descriptor.
        unsafe {
            assert_ne!(
                libc::fcntl(master.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC),
                -1
            );
        }
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "hook_transport_child",
                "--nocapture",
            ])
            .env(
                "BBOXY_SEAT_ID",
                bboxy450_harness_wake_up::Ulid::new().to_string(),
            )
            .env("BBT_HOOK_RESULT", &result)
            .env_remove("CLAUDE_CODE_MESSAGING_SOCKET")
            .env_remove("BBOXY_HOOK_SOCKET_ONLY")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if socket_only {
            command.env("BBOXY_HOOK_SOCKET_ONLY", "1");
        }
        // SAFETY: the child installs only its own controlling tty, without
        // allocation or shared Rust locks between fork and exec.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1
                    || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1
                    || libc::tcsetpgrp(slave_fd, libc::getpgrp()) == -1
                    || libc::fcntl(slave_fd, libc::F_SETFD, 0) == -1
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = Owned(command.spawn().unwrap());
        drop(slave);
        // Drain while waiting: a controlling-tty session leader can wait for
        // pending output during exit. Drop the master before child cleanup on
        // panic so that this fixture cannot strand its own child in tty close.
        let mut master = std::fs::File::from(master);
        let bytes = drain_closed(&mut master);
        assert!(child.0.wait().unwrap().success());
        if socket_only {
            assert!(
                bytes.is_empty(),
                "hook wrote {} bytes into the lifetime tty",
                bytes.len()
            );
            let delivered: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&result).expect("hook skipped its authenticated sink"),
            )
            .unwrap();
            assert_eq!(delivered[1]["event"], "SessionStart");
            assert_eq!(delivered[1]["raw"]["session_id"], "private-hook-session");
        } else {
            assert!(!result.exists(), "ordinary PTY hook bypassed its OSC path");
            assert!(
                bytes
                    .windows(b"\x1b]7502;".len())
                    .any(|part| part == b"\x1b]7502;")
            );
        }
    }
}
