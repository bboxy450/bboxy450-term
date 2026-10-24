#![allow(clippy::disallowed_methods)] // Private process/socket fixtures, never UI.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "private ready-exec subprocess, invoked by its parent"]
fn ready_exec_child() {
    let root = std::path::PathBuf::from(std::env::var_os("BBT_READY_ROOT").unwrap());
    std::fs::write(root.join("started"), std::process::id().to_string()).unwrap();
    let error = bboxy450_harness_wake_up::launch::exec_when_ready(&[
        root.join("s").to_string_lossy().into(),
        "/bin/sh".into(),
        "-c".into(),
        "printf '%s' \"$$\" > \"$1\"".into(),
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
            .env("BBT_READY_ROOT", root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert!(Instant::now() < deadline, "private ready-exec fixture hung");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_late_runtime_execs_the_client_in_the_same_pty_process() {
    let root = tempfile::tempdir().unwrap();
    let mut child = child(root.path());
    until(|| root.path().join("started").exists());
    assert!(!root.path().join("exec-pid").exists());
    let _listener = std::os::unix::net::UnixListener::bind(root.path().join("s")).unwrap();
    until(|| child.0.try_wait().unwrap().is_some());
    assert!(child.0.wait().unwrap().success());
    let pid: u32 = std::fs::read_to_string(root.path().join("exec-pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(pid, child.0.id(), "exec introduced a second client process");
    assert!(!root.path().join("error").exists());
}

#[test]
fn a_missing_runtime_fails_with_a_named_readiness_result() {
    let root = tempfile::tempdir().unwrap();
    let mut child = child(root.path());
    until(|| child.0.try_wait().unwrap().is_some());
    assert_eq!(child.0.wait().unwrap().code(), Some(31));
    assert_eq!(
        std::fs::read_to_string(root.path().join("error")).unwrap(),
        "seat runtime readiness deadline elapsed"
    );
    assert!(!root.path().join("exec-pid").exists());
}
