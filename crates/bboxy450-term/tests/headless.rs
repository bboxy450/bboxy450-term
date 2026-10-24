#![allow(
    clippy::disallowed_methods,
    reason = "isolated CLI integration fixture, never the UI thread"
)]
//! U-1: plain-shell lifecycle and real PTY input/capture without an NSWindow.
use serde_json::Value;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");

struct Fixture {
    root: tempfile::TempDir,
    run: PathBuf,
    pid: Option<u32>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        std::fs::create_dir(root.path().join("shell")).unwrap();
        Self {
            root,
            run,
            pid: None,
        }
    }
    fn ctl(&self, args: &[&str]) -> (i32, Vec<Value>) {
        let out = Command::new(BIN)
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(&self.run)
            .current_dir(self.root.path())
            .env("BBOXY_TEST_CUSTODY", "memory")
            .env("ZDOTDIR", self.root.path().join("shell"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let rows = stdout
            .lines()
            .map(|s| serde_json::from_str(s).unwrap_or_else(|e| panic!("{e}: {s}")))
            .collect();
        (out.status.code().unwrap_or(-1), rows)
    }
    fn ok(&self, args: &[&str]) -> Value {
        let (code, rows) = self.ctl(args);
        assert_eq!(code, 0, "{args:?}: {rows:?}; {}", self.log());
        assert!(!rows.is_empty(), "{args:?}: no JSON reply");
        assert!(rows.iter().all(|r| r["ok"] == true), "{rows:?}");
        rows.last().unwrap().clone()
    }
    fn log(&self) -> String {
        std::fs::read_to_string(self.run.join("app.log")).unwrap_or_default()
    }
    fn start(&mut self) -> Value {
        let row = self.ok(&["app", "start", "--headless", "--ctl"]);
        self.pid = row["data"]["pid"].as_u64().map(|n| n as u32);
        row
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.run.join("gui.endpoint").exists() && self.ctl(&["app", "stop"]).0 == 0 {
            return;
        }
        // Failure-only cleanup of the exact child created by this fixture.
        // The shipped ctl stop command has no signal/PID fallback.
        if let Some(pid) = self.pid {
            let owned = Command::new("/bin/ps")
                .args(["-p", &pid.to_string(), "-o", "args="])
                .output()
                .ok()
                .is_some_and(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .contains(self.run.to_string_lossy().as_ref())
                });
            if owned {
                // SAFETY: this exact PID was created by our CLI, and ps still
                // identifies the fixture's unique run directory in its argv.
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
    }
}

fn await_value(fixture: &Fixture, mut read: impl FnMut() -> Option<Value>) -> Value {
    let start = Instant::now();
    loop {
        if let Some(v) = read() {
            return v;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "fixture did not converge: {}",
            fixture.log()
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn headless_cli_starts_real_gui_types_captures_and_stops_without_a_window() {
    let mut f = Fixture::new();
    let started = f.start();
    let status = &started["data"];
    assert_eq!(status["mode"], "headless");
    assert_eq!(status["windows"], 0);
    assert_eq!(status["backend_ready"], true);
    assert_eq!(status["gui_ready"], true);
    assert_eq!(status["custody_backend"], "memory");
    assert!(status["generation"].as_u64().unwrap() > 0);
    assert!(status["frame_seq"].as_u64().unwrap() > 0);
    assert!(!started.to_string().contains("capability"));
    let again = f.ok(&["app", "start", "--headless", "--ctl"]);
    assert_eq!(again["data"]["generation"], status["generation"]);
    let status = f.ok(&["app", "status"]);
    assert_eq!(status["data"]["running"], true);
    assert!(status["data"]["events_ready"].is_boolean());
    let spawn = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "headless-proof",
        "--place",
        "tab",
    ]);
    let seat = spawn["data"]["seat"].as_str().unwrap();
    f.ok(&["seat", "type", seat, "printf '\\nHEADLESS-CTL-42\\n'"]);
    f.ok(&["seat", "key", seat, "enter"]);
    f.ok(&[
        "seat",
        "wait",
        seat,
        "--text",
        "(?m)^HEADLESS-CTL-42$",
        "--timeout",
        "20000",
    ]);
    let screen = f.ok(&["seat", "screen", seat]);
    assert!(
        screen["data"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "HEADLESS-CTL-42")
    );
    f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "other-tab",
        "--place",
        "tab",
    ]);
    f.ok(&["seat", "focus", seat]);
    let png = f.root.path().join("pane.png");
    let before_capture = f.ok(&["app", "status"]);
    let capture = f.ok(&["seat", "capture", seat, png.to_str().unwrap()]);
    assert_eq!(capture["data"]["offscreen"], true);
    assert_eq!(capture["data"]["stale"], false);
    assert!(capture["data"]["frame_presented_at"].is_null());
    assert!(capture["data"]["rendered_at_ms"].as_u64().unwrap() > 0);
    assert!(
        capture["data"]["frame_seq"].as_u64().unwrap()
            > before_capture["data"]["frame_seq"].as_u64().unwrap(),
        "capture renders a fresh frame even while the terminal is idle"
    );
    assert!(
        std::fs::read(png)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert!(f.log().contains("windows=0"));
    assert!(!f.log().contains("native scene presented"));
    assert!(!f.log().contains("signing held"));
    let stopped = f.ok(&["app", "stop"]);
    assert_eq!(stopped["data"]["running"], false);
    assert!(!f.run.join("gui.endpoint").exists());
    let (code, rows) = f.ctl(&["app", "status"]);
    assert_eq!(code, 4, "{rows:?}");
    // The endpoint disappears during logic shutdown. Wait for the independent
    // supervisor and GUI completion, not an arbitrary outage duration.
    await_value(&f, || {
        f.log()
            .contains("headless GUI stopped: Ok(())")
            .then_some(Value::Bool(true))
    });
    f.pid = None;
}

#[test]
fn headless_smoke_uses_the_idle_probe_with_no_window_or_operator() {
    let f = Fixture::new();
    let out = Command::new(BIN)
        .args(["--headless", "--smoke", "--run-dir"])
        .arg(&f.run)
        .current_dir(f.root.path())
        .env("BBOXY_TEST_CUSTODY", "memory")
        .env("ZDOTDIR", f.root.path().join("shell"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let log = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success(), "{log}");
    assert!(log.contains("idle passed: frames=0 callbacks=0"), "{log}");
    assert!(log.contains("windows=0 mode=headless"), "{log}");
    assert!(log.contains("headless GUI stopped: Ok(())"), "{log}");
}

#[test]
fn headless_consumes_launch_results_so_ten_seats_are_placed_and_capturable() {
    let mut f = Fixture::new();
    f.start();
    for index in 1..=10 {
        let name = format!("headless-launch-{index}");
        let spawn = f.ok(&["seat", "spawn", "zsh", "--name", &name, "--place", "tab"]);
        let seat = spawn["data"]["seat"].as_str().unwrap();
        let marker = format!("HEADLESS-SPAWN-{index}");
        f.ok(&["seat", "type", seat, &format!("printf '{marker}\\n'")]);
        f.ok(&["seat", "key", seat, "enter"]);
        f.ok(&[
            "seat",
            "wait",
            seat,
            "--text",
            &format!("(?m)^{marker}$"),
            "--timeout",
            "20000",
        ]);
        let png = f.root.path().join(format!("seat-{index}.png"));
        let capture = await_value(&f, || {
            let (code, rows) = f.ctl(&["seat", "capture", seat, png.to_str().unwrap()]);
            assert!(
                !f.log().contains("spawn_queue_full"),
                "launch {index} was not placed: {}",
                f.log()
            );
            (code == 0).then(|| rows.last().unwrap().clone())
        });
        assert_eq!(capture["data"]["seat"], seat);
        assert_eq!(capture["data"]["offscreen"], true);
        assert_eq!(capture["data"]["stale"], false);
        assert!(
            std::fs::read(png)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
        eprintln!("launch {index}: real PTY marker and offscreen pane capture verified");
    }
    f.ok(&["app", "stop"]);
    await_value(&f, || {
        f.log()
            .contains("headless GUI stopped: Ok(())")
            .then_some(Value::Bool(true))
    });
    f.pid = None;
}

#[test]
fn idle_probe_rejects_an_external_lifecycle_wake() {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let mut f = Fixture::new();
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&f.run)
        .unwrap();
    let log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(f.run.join("app.log"))
        .unwrap();
    let mut child = Command::new(BIN)
        .args(["--headless", "--smoke", "--ctl", "--run-dir"])
        .arg(&f.run)
        .current_dir(f.root.path())
        .env("BBOXY_TEST_CUSTODY", "memory")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .unwrap();
    f.pid = Some(child.id());
    await_value(&f, || {
        f.log()
            .contains("idle probe begins:")
            .then_some(Value::Bool(true))
    });
    f.ok(&["app", "status"]);
    let status = await_value(&f, || {
        child
            .try_wait()
            .unwrap()
            .map(|status| Value::Bool(status.success()))
    });
    assert_eq!(status, false, "{}", f.log());
    let log = f.log();
    let measured = log
        .lines()
        .find(|line| line.starts_with("idle FAILED:"))
        .unwrap();
    assert!(!measured.contains("callbacks=0 "), "{log}");
    eprintln!("expected active-probe rejection: {measured}");
    assert!(
        log.contains("cockpit children reaped after orderly shutdown"),
        "{log}"
    );
    f.pid = None;
}

#[test]
fn stopping_an_unavailable_endpoint_is_named_and_never_uses_a_pid() {
    let f = Fixture::new();
    let (code, rows) = f.ctl(&["app", "stop"]);
    assert_eq!(code, 4, "{rows:?}");
    assert_eq!(rows[0]["error"]["kind"], "endpoint_unavailable");
    assert!(
        rows[0]["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("debug ps")
    );
    assert!(!f.run.exists());
}

#[test]
fn an_unavailable_unattended_selector_never_falls_back_to_the_keychain() {
    let root = tempfile::tempdir().unwrap();
    let out = Command::new(BIN)
        .args(["--headless", "--unattended"])
        .current_dir(root.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let log = String::from_utf8(out.stderr).unwrap();
    assert!(log.contains("no Keychain fallback"), "{log}");
    assert!(!root.path().join("run").exists());
}
