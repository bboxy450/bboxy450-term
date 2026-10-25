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
    let other = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "other-tab",
        "--place",
        "tab",
    ]);
    let other = other["data"]["seat"].as_str().unwrap();
    let focused = f.ok(&["seat", "focus", other]);
    assert_eq!(focused["data"]["applied"], true);
    assert!(focused["data"]["frame_seq"].as_u64().unwrap() > 0);
    // Text is readable while off screen. Images refuse instead of capturing
    // whichever pane happens to occupy the old rectangle.
    f.ok(&["seat", "screen", seat]);
    let hidden = f.root.path().join("off-screen.png");
    let (code, rows) = f.ctl(&["seat", "capture", seat, hidden.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert_eq!(rows[0]["error"]["kind"], "not_on_screen");
    assert_eq!(rows[0]["error"]["hint"], "ctl seat focus <seat>");
    let focused = f.ok(&["seat", "focus", seat]);
    assert_eq!(focused["data"]["seat"], seat);
    assert!(focused["data"]["generation"].as_u64().unwrap() > 0);
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
fn t7_headless_close_uses_the_window_intent_and_restores_the_same_split() {
    let mut f = Fixture::new();
    f.start();
    let mut ids = vec![];
    for name in ["left", "victim", "right"] {
        let row = f.ok(&["seat", "spawn", "zsh", "--name", name]);
        ids.push(row["data"]["seat"].as_str().unwrap().to_owned());
    }
    let seat = &ids[1];
    f.ok(&["seat", "focus", seat]);
    let before = await_value(&f, || {
        let row = f.ok(&["app", "status"]);
        (row["data"]["close"]["slots"].as_array()?.len() == 3).then_some(row)
    });
    assert_eq!(before["data"]["close"]["complete"], true);
    let row = f.ok(&["seat", "ls"]);
    let original = row["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == seat.as_str())
        .unwrap()
        .clone();
    let first = f.root.path().join("t7-before.png");
    f.ok(&["seat", "capture", seat, first.to_str().unwrap()]);
    f.ok(&["seat", "close", seat]);
    let closed = await_value(&f, || {
        let row = f.ok(&["app", "status"]);
        let close = &row["data"]["close"];
        (close["slots"].as_array()?.len() == 2 && close["grace"].as_array()?.len() == 1)
            .then_some(row)
    });
    assert_eq!(closed["data"]["close"]["grace"][0]["seat"], *seat);
    assert_eq!(closed["data"]["close"]["grace"][0]["undo"], true);
    assert!(
        closed["data"]["close"]["grace"][0]["remaining_ms"]
            .as_u64()
            .unwrap()
            <= 60_000
    );
    let hidden = f.root.path().join("t7-hidden.png");
    assert_ne!(
        f.ctl(&["seat", "capture", seat, hidden.to_str().unwrap()])
            .0,
        0
    );
    f.ok(&["seat", "undo-close", seat]);
    await_value(&f, || {
        let row = f.ok(&["app", "status"]);
        (row["data"]["close"]["slots"] == before["data"]["close"]["slots"]
            && row["data"]["close"]["grace"].as_array()?.is_empty())
        .then_some(row)
    });
    let row = f.ok(&["seat", "ls"]);
    let restored = row["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == seat.as_str())
        .unwrap();
    assert_eq!(
        restored["record"]["generation"],
        original["record"]["generation"]
    );
    assert_eq!(restored["state"]["process"], original["state"]["process"]);
    let last = f.root.path().join("t7-restored.png");
    // Capture succeeds only if the restored tab is actually on screen.
    await_value(&f, || {
        let (code, rows) = f.ctl(&["seat", "capture", seat, last.to_str().unwrap()]);
        (code == 0).then(|| rows.last().unwrap().clone())
    });
    let size = |path| {
        let png = std::fs::read(path).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        png[16..24].to_vec()
    };
    assert_eq!(
        size(first),
        size(last),
        "undo changed the actual pane extent"
    );
    f.ok(&["app", "stop"]);
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
fn unattended_without_a_test_account_is_refused_before_native_custody() {
    let root = tempfile::tempdir().unwrap();
    let out = Command::new(BIN)
        .args(["--headless", "--unattended"])
        .env("BBOXY_TEST_CUSTODY", "memory")
        .current_dir(root.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let log = String::from_utf8(out.stderr).unwrap();
    assert!(log.contains("unattended requires --test-account"), "{log}");
    assert!(!root.path().join("run").exists());
}

#[test]
fn unattended_headless_control_uses_its_own_file_account() {
    let mut f = Fixture::new();
    let account = f.root.path().join("account");
    f.run = account.join("run");
    let row = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = row["data"]["pid"].as_u64().map(|pid| pid as u32);
    assert_eq!(row["data"]["custody_backend"], "file");
    assert_eq!(row["data"]["unattended"], true);
    assert_eq!(row["data"]["windows"], 0);
    let spawned = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "file-proof",
        "--place",
        "tab",
    ]);
    let seat = spawned["data"]["seat"].as_str().unwrap();
    f.ok(&["seat", "type", seat, "printf 'FILE-ACCOUNT-42\\n'"]);
    f.ok(&["seat", "key", seat, "enter"]);
    f.ok(&[
        "seat",
        "wait",
        seat,
        "--text",
        "(?m)^FILE-ACCOUNT-42$",
        "--timeout",
        "20000",
    ]);
    let status = f.ok(&["app", "status"]);
    assert_eq!(status["data"]["custody_backend"], "file");
    assert_eq!(status["data"]["unattended"], true);
    let png = f.root.path().join("file-account.png");
    let capture = await_value(&f, || {
        let (code, rows) = f.ctl(&["seat", "capture", seat, png.to_str().unwrap()]);
        (code == 0).then(|| rows.last().unwrap().clone())
    });
    assert_eq!(capture["data"]["offscreen"], true);
    assert!(!f.log().contains("signing held"), "{}", f.log());
    f.ok(&["app", "stop"]);
    await_value(&f, || {
        f.log()
            .contains("headless GUI stopped: Ok(())")
            .then_some(Value::Bool(true))
    });
    f.pid = None;
}

#[test]
fn unattended_idle_smoke_keeps_file_custody_without_a_window() {
    let root = tempfile::tempdir().unwrap();
    let account = root.path().join("account");
    let out = Command::new(BIN)
        .args([
            "--headless",
            "--smoke",
            "--app-ready",
            "--unattended",
            "--test-account",
        ])
        .arg(&account)
        .current_dir(root.path())
        .env("BBOXY_TEST_CUSTODY", "memory")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let log = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success(), "{log}");
    let status: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(status["custody_backend"], "file");
    assert_eq!(status["unattended"], true);
    assert_eq!(status["windows"], 0);
    assert!(log.contains("idle passed: frames=0 callbacks=0"), "{log}");
    assert!(
        log.contains("cockpit children reaped after orderly shutdown"),
        "{log}"
    );
}

fn ui_node<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    if value["kind"] == name {
        return Some(value);
    }
    value["children"]
        .as_array()?
        .iter()
        .find_map(|v| ui_node(v, name))
}
fn png_pixels(path: &std::path::Path) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut bytes).unwrap();
    assert_eq!(frame.color_type, png::ColorType::Rgba);
    bytes.truncate(frame.buffer_size());
    (frame.width, frame.height, bytes)
}

#[test]
fn ctl_ui_relays_real_chrome_layout_focus_and_a_foreground_launcher() {
    let mut f = Fixture::new();
    let account = f.root.path().join("ui-account");
    f.run = account.join("run");
    let started = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = started["data"]["pid"].as_u64().map(|n| n as u32);
    assert_eq!(started["data"]["custody_backend"], "file");
    assert_eq!(f.ok(&["seat", "ls"])["data"].as_array().unwrap().len(), 0);
    let op = bbt_mux::Ulid::new().to_string();
    let opened = f.ok(&["ui", "key", "cmd-t", "--op", &op]);
    assert_eq!(opened["data"]["launcher_open"], true);
    let frame = opened["data"]["frame_seq"]
        .as_u64()
        .expect("produced UI frame");
    assert!(frame > 0);
    let read = f.ok(&["ui", "layout"]);
    assert_eq!(read["data"]["launcher_open"], true);
    assert!(read["data"]["frame_seq"].as_u64().unwrap() >= frame);
    let closed = f.ok(&["ui", "key", "esc"]);
    assert_eq!(closed["data"]["launcher_open"], false);
    assert!(closed["data"]["frame_seq"].as_u64().unwrap() > frame);
    assert_eq!(
        f.ok(&["seat", "ls"])["data"].as_array().unwrap().len(),
        0,
        "opening and dismissing the launcher never spawns"
    );
    // Retained layout is replayed, with no repeat of the original cmd-t effect.
    assert_eq!(
        f.ok(&["ui", "key", "cmd-t", "--op", &op])["data"]["launcher_open"],
        true
    );
    assert_eq!(f.ok(&["ui", "layout"])["data"]["launcher_open"], false);
    let (code, rows) = f.ctl(&["ui", "key", "cmd-g", "--op", &op]);
    assert_eq!(code, 1);
    assert_eq!(rows[0]["error"]["kind"], "op_id_reused");
    let (code, _) = f.ctl(&["ui", "key", "enter"]);
    assert_eq!(code, 1, "chrome never injects an Enter into a PTY");

    let a = f.ok(&[
        "seat", "spawn", "zsh", "--name", "ui-first", "--place", "tab",
    ])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let b = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "ui-second",
        "--place",
        "tab",
    ])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        (layout["data"]["focused"] == b).then_some(layout)
    });
    let focused = f.ok(&["ui", "focus", "ui-first"]);
    assert_eq!(focused["data"]["focused"], a);
    assert_ne!(focused["data"]["tab"], Value::Null);
    assert_ne!(focused["data"]["tab"], second["data"]["tab"]);
    assert!(
        focused["data"]["frame_seq"].as_u64().unwrap()
            > second["data"]["frame_seq"].as_u64().unwrap()
    );
    let focus_png = f.root.path().join("ui-focus-ack.png");
    let capture = f.ok(&["seat", "capture", &a, focus_png.to_str().unwrap()]);
    assert!(
        focus_png.is_file(),
        "focus reply must already have a capturable pane: {capture}"
    );
    assert!(
        capture["data"]["frame_seq"].as_u64().unwrap()
            >= focused["data"]["frame_seq"].as_u64().unwrap()
    );
    let ids: Vec<_> = f.ok(&["seat", "ls"])["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["record"]["id"].clone())
        .collect();
    assert!(
        focused["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|pane| ids.contains(&pane["seat"]))
    );
    await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        let pane = layout["data"]["panes"]
            .as_array()?
            .iter()
            .find(|p| p["seat"] == a)?;
        let screen = f.ok(&["seat", "screen", &a]);
        (pane["cols"] == screen["data"]["cols"]
            && pane["rows"].as_u64()
                == screen["data"]["rows"]
                    .as_array()
                    .map(|rows| rows.len() as u64))
        .then_some(layout)
    });
    // Print every cell's background, rather than relying on erase-display's
    // background rule. Reversing the layers would leave this area opaque red.
    let pane = &focused["data"]["panes"][0];
    let width = pane["cols"].as_u64().unwrap().saturating_sub(1).max(1);
    let lines = pane["rows"].as_u64().unwrap() + 2;
    let fill = format!(
        r"printf '\033[41m'; for i in {{1..{lines}}}; do printf '%{width}s\n' UI-OVERLAY; done; printf '\033[0m\nUI-OVERLAY-DONE\n'"
    );
    f.ok(&["seat", "type", &a, &fill]);
    f.ok(&["seat", "key", &a, "enter"]);
    f.ok(&[
        "seat",
        "wait",
        &a,
        "--text",
        "(?m)^UI-OVERLAY-DONE$",
        "--timeout",
        "10000",
    ]);
    let before = f.root.path().join("ui-before.png");
    f.ok(&["seat", "capture", "window", before.to_str().unwrap()]);
    let opened = f.ok(&["ui", "open", "launcher"]);
    let picker = opened["data"]["tree"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|tree| ui_node(tree, "named:picker"))
        .expect("actual solved picker");
    let after = f.root.path().join("ui-launcher.png");
    f.ok(&["seat", "capture", "window", after.to_str().unwrap()]);
    let (w, h, before) = png_pixels(&before);
    let (aw, ah, after) = png_pixels(&after);
    assert_eq!((w, h), (aw, ah));
    let pane = &opened["data"]["panes"][0]["rect"];
    let panel = &picker["rect"];
    let rect = |v: &Value, i| v[i].as_f64().unwrap();
    let x0 = rect(pane, 0).max(rect(panel, 0)) as usize + 16;
    let y0 = rect(pane, 1).max(rect(panel, 1)) as usize + 16;
    let x1 = (rect(pane, 0) + rect(pane, 2)).min(rect(panel, 0) + rect(panel, 2)) as usize - 16;
    let y1 = (rect(pane, 1) + rect(pane, 3)).min(rect(panel, 1) + rect(panel, 3)) as usize - 16;
    assert!(x1 > x0 && y1 > y0, "picker covers the actual pane");
    let (mut changed, mut red_before, mut red_after) = (0, 0, 0);
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (y * w as usize + x) * 4;
            changed += usize::from(before[at..at + 4] != after[at..at + 4]);
            let red = |p: &[u8]| {
                p[0] > 120
                    && u16::from(p[0]) > u16::from(p[1]) * 2
                    && u16::from(p[0]) > u16::from(p[2]) * 2
            };
            red_before += usize::from(red(&before[at..at + 4]));
            red_after += usize::from(red(&after[at..at + 4]));
        }
    }
    assert!(
        changed > (x1 - x0) * (y1 - y0) / 3,
        "launcher must paint above the terminal: {changed} changed pixels"
    );
    assert!(
        red_before > (x1 - x0) * (y1 - y0) * 3 / 4,
        "fixture must print opaque red cells across the covered pane: {red_before}"
    );
    assert!(
        red_after < red_before / 3,
        "launcher must cover those cells, rather than paint behind them: {red_before}->{red_after}"
    );
    if let Some(dir) = std::env::var_os("BBOXY_C3_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(
            f.root.path().join("ui-before.png"),
            dir.join("ui-before.png"),
        )
        .unwrap();
        std::fs::copy(
            f.root.path().join("ui-launcher.png"),
            dir.join("ui-launcher.png"),
        )
        .unwrap();
        std::fs::write(
            dir.join("ui-layout.json"),
            serde_json::to_vec_pretty(&opened).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("foreground-verdict.json"), serde_json::to_vec_pretty(&serde_json::json!({"changed":changed,"covered":(x1-x0)*(y1-y0),"red_before":red_before,"red_after":red_after,"custody":"file","windows":0})).unwrap()).unwrap();
    }
    f.ok(&["ui", "key", "esc"]);
    for target in ["graph", "settings"] {
        assert_eq!(f.ok(&["ui", "open", target])["data"]["page"], target);
        assert_eq!(f.ok(&["ui", "key", "esc"])["data"]["page"], Value::Null);
    }
    assert_eq!(
        f.ok(&["ui", "open", "docs", "#999999"])["data"]["document"],
        "#999999"
    );
    assert_eq!(f.ok(&["ui", "key", "esc"])["data"]["document"], Value::Null);
    assert_eq!(f.ok(&["ui", "layout"])["data"]["focused"], a);
    f.ok(&["app", "stop"]);
    f.pid = None;
}

#[test]
fn ctl_ui_keys_acknowledge_spawn_mute_and_undo_only_after_their_end_state_frame() {
    let mut f = Fixture::new();
    let account = f.root.path().join("ui-end-state-account");
    f.run = account.join("run");
    let started = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = started["data"]["pid"].as_u64().map(|pid| pid as u32);
    f.ok(&["ui", "open", "launcher"]);
    // The stock empty-swarm menu is Claude, Codex, pi, omp, zsh. Enter is
    // Launch/go; the Right arrow opens that preset's placement submenu.
    let spawned = f.ok(&["ui", "key", "down", "down", "down", "down", "enter"]);
    assert_eq!(spawned["data"]["launcher_open"], false);
    let panes = spawned["data"]["panes"].as_array().unwrap();
    assert_eq!(panes.len(), 1, "the spawn reply must show the placed pane");
    let seat = panes[0]["seat"].as_str().unwrap().to_owned();
    let png = f.root.path().join("ui-spawn-ack.png");
    let capture = f.ok(&["seat", "capture", &seat, png.to_str().unwrap()]);
    assert!(png.is_file(), "{capture}");
    assert!(
        capture["data"]["frame_seq"].as_u64().unwrap()
            >= spawned["data"]["frame_seq"].as_u64().unwrap()
    );
    // Mute must refresh metadata without relying on a later process-state
    // transition: the plain shell has already settled to idle.
    f.ok(&["seat", "wait", &seat, "--state", "idle"]);
    let muted = f.ok(&["ui", "key", "cmd-m"]);
    let seats = f.ok(&["seat", "ls"]);
    let record = seats["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == seat)
        .unwrap();
    assert_eq!(
        record["record"]["muted"], true,
        "mute reply must follow authoritative metadata"
    );
    assert!(
        muted["data"]["frame_seq"].as_u64().unwrap()
            > spawned["data"]["frame_seq"].as_u64().unwrap()
    );
    f.ok(&["seat", "close", &seat]);
    fn closed_row(node: &Value, seat: &str) -> bool {
        node["kind"].as_str().is_some_and(|kind| {
            kind.starts_with("named:") && kind.ends_with(&format!("closed/{seat}"))
        }) || node["children"]
            .as_array()
            .is_some_and(|children| children.iter().any(|child| closed_row(child, seat)))
    }
    await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        (layout["data"]["panes"].as_array()?.is_empty()
            && layout["data"]["tree"]
                .as_array()?
                .iter()
                .any(|root| closed_row(root, &seat)))
        .then_some(layout)
    });
    let restored = f.ok(&["ui", "key", "cmd-z"]);
    assert!(
        restored["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pane| pane["seat"] == seat)
    );
    let png = f.root.path().join("ui-undo-ack.png");
    let capture = f.ok(&["seat", "capture", &seat, png.to_str().unwrap()]);
    assert!(png.is_file(), "{capture}");
    assert!(
        capture["data"]["frame_seq"].as_u64().unwrap()
            >= restored["data"]["frame_seq"].as_u64().unwrap()
    );
    f.ok(&["app", "stop"]);
    f.pid = None;
}
