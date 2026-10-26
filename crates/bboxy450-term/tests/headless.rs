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
    scripted_hosts: bool,
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
            scripted_hosts: true,
        }
    }
    fn ctl(&self, args: &[&str]) -> (i32, Vec<Value>) {
        let mut command = Command::new(BIN);
        if !self.scripted_hosts {
            command.env("BBOXY_TEST_SCRIPTED_HOSTS", "off");
        }
        let out = command
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
    fn wait_stopped(&mut self) {
        let pid: libc::pid_t = self
            .pid
            .expect("started app reported its owned PID")
            .try_into()
            .unwrap();
        await_value(self, || {
            // SAFETY: signal zero only observes this fixture's recorded child.
            // It never sends a signal or provides a PID-kill fallback.
            let absent = unsafe { libc::kill(pid, 0) } < 0
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
            absent.then_some(Value::Bool(true))
        });
        self.pid = None;
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

// keeps: spec e2e-acceptance T2 (one-frame echo and exact full paste)
#[test]
#[ignore = "night: real GPU/PTY boundary, test-policy #327 B2"]
fn terminal_input_echo_frame_and_full_paste_use_a_private_account() {
    let mut f = Fixture::new();
    let account = f.root.path().join("t2-account");
    f.run = account.join("run");
    let started = f.ok(&[
        "app", "start", "--headless", "--ctl", "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = started["data"]["pid"].as_u64().map(|pid| pid as u32);
    assert_eq!(started["data"]["custody_backend"], "file");
    assert_eq!(started["data"]["windows"], 0);
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/e2e/T2.sh");
    let out = Command::new("sh").arg(script)
        .env("BBT", BIN).env("BBOXY_RUN_DIR", &f.run)
        .env("E2E_WORK", f.root.path().join("work"))
        .env("BBOXY_CTL_ISSUER", "test-account:t2")
        .current_dir(f.root.path()).stdin(Stdio::null()).output().unwrap();
    assert!(out.status.success(), "{}\n{}\n{}",
        String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr), f.log());
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec ctl-cli §1a (operator placement receipt)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn explicit_placements_capture_the_anchor_preserve_siblings_and_acknowledge_the_pane_frame() {
    let mut f = Fixture::new();
    let account = f.root.path().join("placement-account");
    f.run = account.join("run");
    let started = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = started["data"]["pid"].as_u64().map(|n| n as u32);
    let cwd = f.root.path().join("picked cwd");
    std::fs::create_dir(&cwd).unwrap();
    let (code, error) = f.ctl(&["seat", "spawn", "zsh", "--place", "right"]);
    assert_ne!(code, 0);
    assert!(
        error[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("placement_anchor_missing")
    );
    assert!(f.ok(&["seat", "ls"])["data"].as_array().unwrap().is_empty());
    fn spawn(f: &Fixture, place: &str, name: &str, cwd: &std::path::Path, op: &str) -> Value {
        f.ok(&[
            "seat",
            "spawn",
            "zsh",
            "--place",
            place,
            "--name",
            name,
            "--cwd",
            cwd.to_str().unwrap(),
            "--op",
            op,
        ])
    }
    fn id(reply: &Value) -> String {
        reply["data"]["seat"].as_str().unwrap().to_owned()
    }
    fn rect(layout: &Value, seat: &str) -> [f64; 4] {
        let p = layout["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["seat"] == seat)
            .unwrap();
        std::array::from_fn(|i| p["rect"][i].as_f64().unwrap())
    }
    fn same(a: [f64; 4], b: [f64; 4]) {
        for (x, y) in a.into_iter().zip(b) {
            assert!(
                (x - y).abs() < 0.01,
                "sibling rectangle moved: {a:?} -> {b:?}"
            );
        }
    }
    let a = spawn(&f, "tab", "anchor", &cwd, &bbt_mux::Ulid::new().to_string());
    let a = id(&a);
    let original = f.ok(&["ui", "layout"])["data"].clone();
    let swarm = original["swarm"].clone();
    let tab = original["tab"].clone();
    let b = spawn(
        &f,
        "right",
        "outer right",
        &cwd,
        &bbt_mux::Ulid::new().to_string(),
    );
    let b = id(&b);
    let focused = f.ok(&["ui", "focus", &a])["data"].clone();
    let outer = rect(&focused, &b);
    let op = bbt_mux::Ulid::new().to_string();
    let c = spawn(&f, "right", "inner right", &cwd, &op);
    let c_id = id(&c);
    assert_eq!(c["data"]["placement"]["target"], a);
    assert_eq!(c["data"]["placement"]["side"], "right");
    assert_eq!(c["data"]["layout_revision"], focused["layout_revision"]);
    assert_eq!(
        c["data"]["applied_layout_revision"],
        c["data"]["layout"]["layout_revision"]
    );
    let right = &c["data"]["layout"];
    assert_eq!(right["focused"], c_id);
    assert_eq!(right["tab"], tab);
    assert!(c["data"]["frame_seq"].as_u64().unwrap() > focused["frame_seq"].as_u64().unwrap());
    let ra = rect(right, &a);
    let rc = rect(right, &c_id);
    assert!(rc[0] > ra[0]);
    same(outer, rect(right, &b));
    // Same operator op replays the captured result after focus has moved; it
    // cannot split the newly focused sibling or launch another process.
    f.ok(&["ui", "focus", &b]);
    let retry = spawn(&f, "right", "inner right", &cwd, &op);
    assert_eq!(retry["data"], c["data"]);
    assert_eq!(f.ok(&["ui", "layout"])["data"]["focused"], b);
    assert_eq!(f.ok(&["seat", "ls"])["data"].as_array().unwrap().len(), 3);
    let before = f.ok(&["ui", "focus", &c_id])["data"].clone();
    let d = spawn(&f, "down", "below", &cwd, &bbt_mux::Ulid::new().to_string());
    let d_id = id(&d);
    assert_eq!(d["data"]["placement"]["target"], c_id);
    assert_eq!(d["data"]["placement"]["side"], "bottom");
    let down = &d["data"]["layout"];
    same(rect(&before, &a), rect(down, &a));
    same(rect(&before, &b), rect(down, &b));
    let rc = rect(down, &c_id);
    let rd = rect(down, &d_id);
    assert!(rd[1] > rc[1]);
    assert!((rd[0] - rc[0]).abs() < 0.01);
    let old = down.clone();
    let e = spawn(
        &f,
        "tab",
        "separate tab",
        &cwd,
        &bbt_mux::Ulid::new().to_string(),
    );
    assert_eq!(e["data"]["swarm"], swarm);
    assert_ne!(e["data"]["layout"]["tab"], tab);
    assert_eq!(e["data"]["layout"]["panes"].as_array().unwrap().len(), 1);
    let restored = f.ok(&["ui", "focus", &a])["data"].clone();
    for seat in [&a, &b, &c_id, &d_id] {
        same(rect(&old, seat), rect(&restored, seat));
    }
    let space = spawn(
        &f,
        "space",
        "new space",
        &cwd,
        &bbt_mux::Ulid::new().to_string(),
    );
    let s_id = id(&space);
    assert_ne!(space["data"]["swarm"], swarm);
    assert_ne!(space["data"]["layout"]["tab"], tab);
    assert_eq!(space["data"]["layout"]["focused"], s_id);
    assert_eq!(
        space["data"]["layout"]["panes"].as_array().unwrap().len(),
        1
    );
    let image = f.root.path().join("placed-space.png");
    let capture = f.ok(&["seat", "capture", &s_id, image.to_str().unwrap()]);
    assert!(image.is_file());
    assert!(
        capture["data"]["frame_seq"].as_u64().unwrap()
            >= space["data"]["frame_seq"].as_u64().unwrap()
    );
    f.ok(&["seat", "type", &s_id, "printf 'PLACED_CWD:%s\\n' \"$PWD\""]);
    f.ok(&["seat", "key", &s_id, "enter"]);
    f.ok(&[
        "seat",
        "wait",
        &s_id,
        "--text",
        "PLACED_CWD:.*picked cwd",
        "--timeout",
        "20000",
    ]);
    let restored = f.ok(&["ui", "focus", &a])["data"].clone();
    assert_eq!(restored["swarm"], swarm);
    for seat in [&a, &b, &c_id, &d_id] {
        same(rect(&old, seat), rect(&restored, seat));
    }
    assert_eq!(f.ok(&["seat", "ls"])["data"].as_array().unwrap().len(), 6);
    f.ok(&["app", "stop"]);
}

// keeps: spec attention §3 (test-account-only recording before startup)
#[test]
fn notification_recording_requires_a_test_account_before_startup() {
    let fixture = Fixture::new();
    let (code, reply) = fixture.ctl(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--notifications",
        "record",
    ]);
    assert_eq!(code, 2, "{reply:?}");
    assert!(!fixture.run.exists());
    let direct = Command::new(BIN)
        .args(["--headless", "--notifications", "record", "--run-dir"])
        .arg(&fixture.run)
        .env("BBOXY_TEST_CUSTODY", "memory")
        .output()
        .unwrap();
    assert!(!direct.status.success());
    assert!(String::from_utf8_lossy(&direct.stderr).contains("--test-account"));
    assert!(!fixture.run.exists());
}

// keeps: spec attention §5 (notification-to-produced-frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn recording_notifications_replace_mute_and_activate_the_exact_headless_pane() {
    let mut f = Fixture::new();
    let account = f.root.path().join("notification-account");
    let start = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--test-account",
        account.to_str().unwrap(),
        "--notifications",
        "record",
    ]);
    f.pid = start["data"]["pid"].as_u64().map(|pid| pid as u32);
    assert_eq!(start["data"]["windows"], 0);
    assert_eq!(start["data"]["custody_backend"], "file");
    let target = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "notification-target",
        "--place",
        "tab",
    ])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let original = f.ok(&["ui", "focus", &target]);
    let other = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "other-pane",
        "--place",
        "tab",
    ])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let background = f.ok(&["ui", "focus", &other]);
    assert_ne!(background["data"]["tab"], original["data"]["tab"]);
    let program = f.root.path().join("waiting-producer.py");
    std::fs::write(
        &program,
        r#"import base64,sys
print('R6READY',flush=True)
for raw in sys.stdin:
    question=raw.rstrip('\n')
    print('R6PRODUCER:'+question,flush=True)
    if question=='IDLE':
        print('\033]7501;state=idle\007',end='',flush=True)
    else:
        detail=base64.b64encode((question+'\nprivate second line').encode()).decode()
        print('\033]7501;state=blocked:kind=question:msg='+detail+'\007',end='',flush=True)
"#,
    )
    .unwrap();
    let launch = format!(
        "python3 -u '{}'",
        program.to_string_lossy().replace('\'', "'\\''")
    );
    f.ok(&["seat", "type", &target, &launch]);
    f.ok(&["seat", "key", &target, "enter"]);
    f.ok(&[
        "seat",
        "wait",
        &target,
        "--text",
        "R6READY",
        "--timeout",
        "20000",
    ]);
    let question = |text: &str| {
        f.ok(&["seat", "type", &target, text]);
        f.ok(&["seat", "key", &target, "enter"]);
        f.ok(&[
            "seat",
            "wait",
            &target,
            "--text",
            &format!("R6PRODUCER:{text}"),
            "--timeout",
            "20000",
        ]);
    };
    let notices = || f.ok(&["debug", "notifications"])["data"].clone();
    question("First question?");
    let first = await_value(&f, || {
        let state = notices();
        (state["available"] == true
            && state["settled"] == true
            && state["outstanding_count"] == 1
            && state["outstanding"][0]["body"] == "First question?")
            .then_some(state)
    });
    assert_eq!(first["backend"], "recording");
    assert_eq!(first["os_presentation"], false);
    assert_eq!(first["outstanding"][0]["id"], target);
    assert_eq!(first["posts"], 1);
    assert!(!first.to_string().contains("private second line"));
    question("Replacement question?");
    let replaced = await_value(&f, || {
        let state = notices();
        (state["settled"] == true
            && state["outstanding_count"] == 1
            && state["posts"] == 2
            && state["outstanding"][0]["body"] == "Replacement question?")
            .then_some(state)
    });
    assert_eq!(replaced["replacements"], 1);
    assert!(
        replaced["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["operation"] == "replace" && row["stage"] == "completion")
    );
    use bboxy450_term::ctl::{Client, Endpoint};
    use bbt_proto::messaging::Request;
    let mut client =
        Client::connect(&Endpoint::read(&f.run.join("gui.endpoint")).unwrap()).unwrap();
    let record = client
        .seats()
        .unwrap()
        .into_iter()
        .find(|seat| seat.record.id.to_string() == target)
        .unwrap()
        .record;
    client
        .messaging(
            record.swarm,
            Request::Mute {
                seat: record.id,
                muted: true,
            },
        )
        .unwrap();
    await_value(&f, || {
        let state = notices();
        (state["settled"] == true
            && state["outstanding_count"] == 0
            && state["waiting"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r["seat"] == target && r["muted"] == true)
            }))
        .then_some(state)
    });
    question("IDLE");
    await_value(&f, || {
        let state = notices();
        state["waiting"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|r| r["seat"] != target))
            .then_some(state)
    });
    question("Muted question?");
    let muted = await_value(&f, || {
        let state = notices();
        (state["settled"] == true
            && state["waiting"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r["seat"] == target && r["muted"] == true)
            }))
        .then_some(state)
    });
    assert_eq!(muted["posts"], 2, "muted waiting episode posted");
    assert_eq!(muted["outstanding_count"], 0);
    client
        .messaging(
            record.swarm,
            Request::Mute {
                seat: record.id,
                muted: false,
            },
        )
        .unwrap();
    await_value(&f, || {
        let state = notices();
        (state["settled"] == true
            && state["outstanding_count"] == 1
            && state["outstanding"][0]["id"] == target)
            .then_some(state)
    });
    let before = f.ok(&["ui", "layout"]);
    assert_eq!(before["data"]["focused"], other);
    let clicked = f.ok(&["ui", "notification-click", &target]);
    assert_eq!(clicked["data"]["focused"], target);
    assert_eq!(clicked["data"]["tab"], original["data"]["tab"]);
    assert!(
        clicked["data"]["frame_seq"].as_u64().unwrap()
            > before["data"]["frame_seq"].as_u64().unwrap()
    );
    assert!(
        clicked["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["seat"] == target && p["focused"] == true)
    );
    let handled = await_value(&f, || {
        let state = notices();
        (state["settled"] == true && state["activations"] == 1 && state["outstanding_count"] == 0)
            .then_some(state)
    });
    assert_eq!(handled["focused"], target);
    let (code, _) = f.ctl(&["ui", "notification-click", &other]);
    assert_ne!(code, 0);
    assert_eq!(f.ok(&["ui", "layout"])["data"]["focused"], target);
    assert_eq!(f.ok(&["app", "status"])["data"]["windows"], 0);
    f.ok(&["app", "stop"]);
}

// keeps: spec ctl-cli §1b (headless lifecycle and capture boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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
    let stopped = f.ok(&["app", "stop"]);
    assert_eq!(stopped["data"]["running"], false);
    assert!(!f.run.join("gui.endpoint").exists());
    let (code, rows) = f.ctl(&["app", "status"]);
    assert_eq!(code, 4, "{rows:?}");
    // The endpoint disappears during logic shutdown. Wait for the independent
    // supervisor and GUI completion, not an arbitrary outage duration.
    f.wait_stopped();
}

// keeps: spec ctl-cli §1a (close/undo produced-frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn t7_headless_close_receipts_restore_the_same_tab_and_extent() {
    let mut f = Fixture::new();
    f.start();
    f.ok(&["app", "resize", "100x30"]);
    let mut ids = vec![];
    for name in ["left", "victim", "right"] {
        let row = f.ok(&["seat", "spawn", "zsh", "--name", name, "--place", "tab"]);
        let seat = row["data"]["seat"].as_str().unwrap().to_owned();
        // Every fixture seat has reached a frame before the next operation.
        f.ok(&["seat", "focus", &seat]);
        ids.push(seat);
    }
    let seat = &ids[1];
    f.ok(&["seat", "focus", seat]);
    let before_layout = f.ok(&["ui", "layout"]);
    assert_ne!(
        before_layout["data"]["size"],
        serde_json::json!([1600., 900.])
    );
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
    let first_frame = f.ok(&["seat", "capture", seat, first.to_str().unwrap()]);
    let close_frame = f.ok(&["seat", "close", seat]);
    assert_eq!(close_frame["data"]["applied"], true);
    assert_eq!(
        close_frame["data"]["layout"]["size"],
        before_layout["data"]["size"]
    );
    assert!(
        close_frame["data"]["frame_seq"].as_u64().unwrap()
            > first_frame["data"]["frame_seq"].as_u64().unwrap()
    );
    assert!(
        !close_frame["data"]["layout"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pane| pane["seat"] == seat.as_str())
    );
    // No lifecycle or capture polling: the command's frame receipt is the boundary.
    let closed = f.ok(&["app", "status"]);
    assert_eq!(
        closed["data"]["close"]["slots"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        closed["data"]["close"]["grace"].as_array().unwrap().len(),
        1
    );
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
    let undo_frame = f.ok(&["seat", "undo-close", seat]);
    assert_eq!(
        undo_frame["data"]["layout"]["size"],
        before_layout["data"]["size"]
    );
    assert!(
        undo_frame["data"]["frame_seq"].as_u64().unwrap()
            > close_frame["data"]["frame_seq"].as_u64().unwrap()
    );
    assert_eq!(undo_frame["data"]["layout"]["focused"], seat.as_str());
    assert!(
        undo_frame["data"]["layout"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pane| pane["seat"] == seat.as_str() && pane["focused"] == true)
    );
    let restored_ui = f.ok(&["app", "status"]);
    assert_eq!(
        restored_ui["data"]["close"]["slots"],
        before["data"]["close"]["slots"]
    );
    assert!(
        restored_ui["data"]["close"]["grace"]
            .as_array()
            .unwrap()
            .is_empty()
    );
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
    // One capture must succeed after the first restored frame; no retry masks the contract.
    let restored_frame = f.ok(&["seat", "capture", seat, last.to_str().unwrap()]);
    assert!(
        restored_frame["data"]["frame_seq"].as_u64().unwrap()
            >= undo_frame["data"]["frame_seq"].as_u64().unwrap()
    );
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
    f.wait_stopped();
}

// keeps: spec runtime §4 (real memory-custody idle boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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

// keeps: spec ctl-cli §1b (ten real launch/result boundaries)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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
        let capture = f.ok(&["seat", "capture", seat, png.to_str().unwrap()]);
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
    f.wait_stopped();
}

// keeps: spec runtime §4 (idle probe rejects actual external invalidation)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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

// keeps: spec ctl-cli §1b (no PID fallback)
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

// keeps: spec ctl-cli §5 (own test-account admission before startup)
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

// keeps: spec ctl-cli §1b (file-account headless control boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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
    let capture = f.ok(&["seat", "capture", seat, png.to_str().unwrap()]);
    assert_eq!(capture["data"]["offscreen"], true);
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec runtime §4 (real file-custody idle boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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

// keeps: spec ctl-cli §5 (actual identity admission boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn ui_clock_refuses_a_non_test_account_even_with_a_forged_issuer() {
    let mut f = Fixture::new();
    f.start();
    for command in [
        vec!["ui", "clock", "freeze"],
        vec!["ui", "clock", "freeze", "--as", "test-account:forged"],
    ] {
        let (code, rows) = f.ctl(&command);
        assert_eq!(code, 1, "{rows:?}");
        assert_eq!(rows[0]["ok"], false);
        assert!(
            rows[0]["error"]["message"]
                .as_str()
                .unwrap()
                .contains("verified test account"),
            "{rows:?}"
        );
    }
    assert_eq!(f.ok(&["ui", "layout"])["data"]["clock"]["revision"], 0);
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec ctl-cli §1 (scripted window control)
#[test]
fn scripted_host_opt_out_is_debug_only_and_requires_a_test_account() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("selector.rs");
    let module =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cockpit/fixture_hosts.rs");
    std::fs::write(
        &source,
        format!(
            r#"#[path = {module:?}] mod fixture_hosts;
fn main() {{ println!("{{}} {{}} {{}}",
    fixture_hosts::enabled(true, true),
    fixture_hosts::enabled(false, false),
    fixture_hosts::enabled(false, true)); }}
"#
        ),
    )
    .unwrap();
    for (profile, debug, opted_out) in [
        ("debug", "yes", "false true false\n"),
        ("release", "no", "false true true\n"),
    ] {
        let bin = root.path().join(profile);
        let compile = Command::new("rustc")
            .arg("--edition=2024")
            .args(["-C", &format!("debug-assertions={debug}")])
            .arg(&source)
            .arg("-o")
            .arg(&bin)
            .output()
            .unwrap();
        assert!(compile.status.success(), "{compile:?}");
        for selector in [None, Some("off"), Some("unrecognized")] {
            let mut command = Command::new(&bin);
            command.env_remove("BBOXY_TEST_SCRIPTED_HOSTS");
            if let Some(value) = selector {
                command.env("BBOXY_TEST_SCRIPTED_HOSTS", value);
            }
            let out = command.output().unwrap();
            assert!(out.status.success(), "{out:?}");
            let expected = if selector == Some("off") {
                opted_out
            } else {
                "false true true\n"
            };
            assert_eq!(String::from_utf8(out.stdout).unwrap(), expected);
        }
        if profile == "release" {
            assert!(
                !std::fs::read(&bin)
                    .unwrap()
                    .windows(b"BBOXY_TEST_SCRIPTED_HOSTS".len())
                    .any(|bytes| bytes == b"BBOXY_TEST_SCRIPTED_HOSTS"),
                "release must not contain the test selector"
            );
        }
    }
}

// keeps: spec ctl-cli §1 (scripted window control)
#[test]
#[ignore = "night: real GPU/protocol boundary, test-policy #327 B2"]
fn test_account_ui_clock_freezes_pixels_steps_focus_and_resumes_live_time() {
    let mut f = Fixture::new();
    // Only this fixture needs a quiescent backend: the provider script has an
    // independent real-clock link drop, unrelated to frozen animation time.
    f.scripted_hosts = false;
    let account = f.root.path().join("c");
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
    assert_eq!(started["data"]["custody_backend"], "file");
    assert_eq!(started["data"]["windows"], 0);
    let frozen = f.ok(&["ui", "clock", "freeze"]);
    assert_eq!(frozen["data"]["clock"]["frozen"], true);
    let mut seats = vec![];
    for name in ["clock-first", "clock-second"] {
        let row = f.ok(&["seat", "spawn", "zsh", "--name", name, "--place", "tab"]);
        let seat = row["data"]["seat"].as_str().unwrap().to_owned();
        f.ok(&[
            "seat",
            "type",
            &seat,
            "printf '\\nCLOCK-READY\\n'; exec cat",
        ]);
        f.ok(&["seat", "key", &seat, "enter"]);
        f.ok(&[
            "seat",
            "wait",
            &seat,
            "--text",
            "(?m)^CLOCK-READY$",
            "--timeout",
            "20000",
        ]);
        seats.push(seat);
    }
    // Settle the launch swells before beginning the new focus episode.
    f.ok(&["ui", "focus", &seats[1]]);
    f.ok(&["ui", "clock", "advance", "900"]);
    let focus = f.ok(&["ui", "focus", &seats[0]]);
    let before_step = f.ok(&["ui", "clock", "freeze"]);
    let capture = |name: &str| {
        let path = f.root.path().join(name);
        let reply = f.ok(&["seat", "capture", "window", path.to_str().unwrap()]);
        assert!(
            reply["data"]["frame_seq"].as_u64().unwrap()
                >= before_step["data"]["frame_seq"].as_u64().unwrap()
        );
        png_pixels(&path)
    };
    let first = capture("clock-first.png");
    let second = capture("clock-second.png");
    assert_eq!(
        first, second,
        "wall time and capture work cannot advance a frozen animation"
    );
    let stepped = f.ok(&["ui", "clock", "advance", "450"]);
    assert_eq!(
        stepped["data"]["clock"]["time_ms"].as_u64().unwrap(),
        before_step["data"]["clock"]["time_ms"].as_u64().unwrap() + 450
    );
    assert!(
        stepped["data"]["frame_seq"].as_u64().unwrap()
            > before_step["data"]["frame_seq"].as_u64().unwrap()
    );
    let geometry = |frame: &Value| {
        frame["data"]["panes"].as_array().unwrap().iter().map(|pane| {
            serde_json::json!({"seat": pane["seat"], "rect": pane["rect"], "cols": pane["cols"], "rows": pane["rows"], "focused": pane["focused"]})
        }).collect::<Vec<_>>()
    };
    assert_eq!(
        geometry(&stepped),
        geometry(&focus),
        "the swell is paint only"
    );
    let peak = capture("clock-peak.png");
    assert_eq!((first.0, first.1), (peak.0, peak.1));
    assert_ne!(
        first.2, peak.2,
        "the 450 ms peak must reach the actual GPU image"
    );
    let node_rect = |name| {
        stepped["data"]["tree"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|node| ui_node(node, name))
            .unwrap_or_else(|| panic!("missing solved node {name}"))["rect"]
            .clone()
    };
    let pane = node_rect("named:pane");
    let new_swarm = node_rect("named:new-swarm");
    let dock = node_rect("named:conversation/rows");
    // The empty sidebar gap is outside the resting rim and every sidebar
    // block: moving waves cannot satisfy this assertion in place of a swell.
    let x0 = (pane[0].as_f64().unwrap() - 32.) as usize;
    let x1 = (pane[0].as_f64().unwrap() - 8.) as usize;
    let y0 = (new_swarm[1].as_f64().unwrap() + new_swarm[3].as_f64().unwrap() + 16.) as usize;
    let y1 = (dock[1].as_f64().unwrap() - 16.) as usize;
    assert!(x1 > x0 && y1 > y0 && x1 < first.0 as usize && y1 < first.1 as usize);
    let mut swell_pixels = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (y * first.0 as usize + x) * 4;
            swell_pixels += usize::from(first.2[at..at + 4] != peak.2[at..at + 4]);
        }
    }
    assert!(
        swell_pixels > 0,
        "the 450 ms frame has no swell beyond the resting rim"
    );
    let resumed = f.ok(&["ui", "clock", "resume"]);
    assert_eq!(resumed["data"]["clock"]["frozen"], false);
    assert!(
        resumed["data"]["clock"]["time_ms"].as_u64().unwrap()
            >= stepped["data"]["clock"]["time_ms"].as_u64().unwrap()
    );
    let (code, rows) = f.ctl(&["ui", "clock", "advance", "1"]);
    assert_eq!(code, 1);
    assert!(
        rows[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("clock_not_frozen")
    );
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec ctl-cli §1a (chrome and foreground frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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
    let second = f.ok(&["ui", "layout"]);
    assert_eq!(second["data"]["focused"], b);
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
    f.wait_stopped();
}

// keeps: spec ctl-cli §1a (keyboard end-state frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
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
    let closed = f.ok(&["ui", "layout"]);
    assert!(closed["data"]["panes"].as_array().unwrap().is_empty());
    assert!(
        closed["data"]["tree"]
            .as_array()
            .unwrap()
            .iter()
            .any(|root| closed_row(root, &seat))
    );
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
    f.wait_stopped();
}

/// ctl-cli §1a: `seat wait --on-screen` and `--frame-seq` read the owner's produced layout,
/// and a capture refused `not_on_screen` names the frame it checked.
// keeps: spec ctl-cli §1a (frame wait and hidden-capture boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn seat_wait_reads_produced_frames_and_not_on_screen_names_its_frame() {
    let mut f = Fixture::new();
    f.start();
    let spawn = |name: &str, place: Option<&str>| {
        let mut args = vec!["seat", "spawn", "zsh", "--name", name];
        if let Some(place) = place {
            args.extend(["--place", place]);
        }
        f.ok(&args)["data"]["seat"].as_str().unwrap().to_owned()
    };
    let first = spawn("first", None);
    let on = f.ok(&["seat", "wait", &first, "--on-screen", "--timeout", "15000"]);
    let frame = on["data"]["frame_seq"].as_u64().unwrap();
    assert!(on["data"]["pane"]["cols"].as_u64().unwrap() > 0, "{on}");
    // An idle window draws nothing; output in the pane produces the next frame.
    f.ok(&["seat", "type", &first, "echo next-frame"]);
    f.ok(&["seat", "key", &first, "enter"]);
    let next = (frame + 1).to_string();
    let later = f.ok(&[
        "seat",
        "wait",
        &first,
        "--frame-seq",
        &next,
        "--timeout",
        "15000",
    ]);
    assert!(
        later["data"]["frame_seq"].as_u64().unwrap() > frame,
        "{later}"
    );

    // A new tab takes the window; the first seat's pane leaves the screen.
    let second = spawn("second", Some("tab"));
    f.ok(&["seat", "wait", &second, "--on-screen", "--timeout", "15000"]);
    let shot = f.root.path().join("hidden.png");
    let (code, rows) = f.ctl(&["seat", "capture", "first", shot.to_str().unwrap()]);
    assert_eq!(code, 1, "{rows:?}");
    assert_eq!(rows[0]["error"]["kind"], "not_on_screen", "{rows:?}");
    assert_eq!(rows[0]["error"]["seat"], first, "{rows:?}");
    assert!(rows[0]["error"]["frame_seq"].is_u64(), "{rows:?}");
    assert!(!shot.exists());
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec ctl-cli §1a (historical produced-frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn ctl_ui_scroll_reads_the_produced_historical_view_without_changing_focus() {
    let mut f = Fixture::new();
    let account = f.root.path().join("scroll-account");
    f.run = account.join("run");
    let start = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = start["data"]["pid"].as_u64().map(|n| n as u32);
    assert_eq!(start["data"]["custody_backend"], "file");
    let seat = f.ok(&["seat", "spawn", "zsh", "--name", "scroll-pane"])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let initial = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        layout["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pane| pane["seat"] == seat)
            .then_some(layout)
    });
    let pane = |layout: &Value| {
        layout["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["seat"] == seat)
            .unwrap()
            .clone()
    };
    assert_eq!(
        pane(&initial)["scroll"],
        serde_json::json!({"top_row":null,"live":true})
    );
    let peer = f.ok(&["seat", "spawn", "zsh", "--name", "scroll-peer"])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        layout["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pane| pane["seat"] == peer)
            .then_some(layout)
    });
    f.ok(&["ui", "focus", &peer]);
    let line_count = pane(&initial)["rows"].as_u64().unwrap() * 4 + 120;
    let script = format!("for i in {{1..{line_count}}}; do printf 'scroll-%03d\\n' $i; done\r");
    f.ok(&["seat", "type", &seat, &script]);
    let last_line = format!("scroll-{line_count:03}");
    f.ok(&[
        "seat",
        "wait",
        &seat,
        "--text",
        &last_line,
        "--timeout",
        "15000",
    ]);
    let before = f.ok(&["ui", "layout"]);
    assert_eq!(before["data"]["focused"], peer);
    let top = f.ok(&["ui", "scroll", &seat, "top"]);
    let metrics = f.ok(&["debug", "metrics"]);
    let meter = &metrics["data"]["ui"]["history"];
    assert_eq!(metrics["data"]["ui"]["available"], true);
    assert_eq!(meter["complete"], true);
    let observed = meter["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == seat)
        .unwrap();
    assert!(observed["retained_history_bytes"].as_u64().unwrap() <= 4_000_000);
    assert!(observed["pages_fetched"].as_u64().unwrap() > 0);
    assert!(observed["pages_resident"].as_u64().unwrap() <= 1);
    assert_eq!(
        pane(&top)["scroll"],
        serde_json::json!({"top_row":0,"live":false})
    );
    assert_eq!(top["data"]["focused"], before["data"]["focused"]);
    assert_eq!(
        top["data"]["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pane| pane["seat"] == peer)
            .unwrap()["scroll"]["live"],
        true
    );
    assert_eq!(top["data"]["tab"], before["data"]["tab"]);
    assert!(
        top["data"]["frame_seq"].as_u64().unwrap() > before["data"]["frame_seq"].as_u64().unwrap()
    );
    let top_png = f.root.path().join("scroll-top.png");
    f.ok(&["seat", "capture", &seat, top_png.to_str().unwrap()]);
    let at = f.ok(&["ui", "scroll", "scroll-pane", "row", "40"]);
    assert_eq!(
        pane(&at)["scroll"],
        serde_json::json!({"top_row":40,"live":false})
    );
    let at_png = f.root.path().join("scroll-row40.png");
    f.ok(&["seat", "capture", &seat, at_png.to_str().unwrap()]);
    assert!(
        png_pixels(&top_png).2 != png_pixels(&at_png).2,
        "the actual pane pixels must change with the historical row"
    );
    let page = pane(&at)["rows"].as_u64().unwrap().saturating_sub(1).max(1);
    let down = f.ok(&["ui", "scroll", &seat, "page-down"]);
    assert_eq!(pane(&down)["scroll"]["top_row"], 40 + page);
    let up = f.ok(&["ui", "scroll", &seat, "page-up"]);
    assert_eq!(pane(&up)["scroll"]["top_row"], 40);
    let same = f.ok(&["ui", "scroll", &seat, "row", "40"]);
    assert_eq!(
        pane(&same)["scroll"]["top_row"],
        40,
        "an unchanged cached row still completes"
    );
    let anchored_frame = same["data"]["frame_seq"].as_u64().unwrap();
    f.ok(&["seat", "rename", &seat, "scroll-renamed"]);
    let renamed = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"]);
        (layout["data"]["frame_seq"].as_u64().unwrap() > anchored_frame).then_some(layout)
    });
    assert_eq!(
        pane(&renamed)["scroll"]["top_row"],
        40,
        "metadata publication must keep the historical display"
    );
    let bottom = f.ok(&["ui", "scroll", &seat, "bottom"]);
    assert_eq!(pane(&bottom)["scroll"]["live"], true);
    let bottom_again = f.ok(&["ui", "scroll", &seat, "bottom"]);
    assert_eq!(
        pane(&bottom_again)["scroll"]["live"],
        true,
        "already-live is a completed command"
    );
    let far = f.ok(&["ui", "scroll", &seat, "row", "18446744073709551615"]);
    assert_eq!(
        pane(&far)["scroll"]["live"],
        true,
        "u64::MAX must clamp at the tail, never wrap to row zero"
    );
    let known = pane(&far)["scroll"]["top_row"].as_u64().unwrap();
    assert!(known >= line_count.saturating_sub(pane(&far)["rows"].as_u64().unwrap()));
    f.ok(&["ui", "open", "launcher"]);
    let (code, rows) = f.ctl(&["ui", "scroll", &seat, "top"]);
    assert_eq!(code, 1);
    assert_eq!(rows[0]["error"]["kind"], "refused");
    assert!(
        rows[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not_on_screen"),
        "{rows:?}"
    );
    f.ok(&["ui", "key", "esc"]);
    assert_eq!(pane(&f.ok(&["ui", "layout"]))["scroll"]["live"], true);
    assert_eq!(f.ok(&["seat", "ls"])["data"].as_array().unwrap().len(), 2);
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

/// ctl-cli §1a: `app resize` replies only after the first produced frame at the new size, with
/// that frame's layout; a read or capture right after the reply needs no wait.
// keeps: spec ctl-cli §1a (resize produced-frame boundary)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn app_resize_replies_with_the_first_frame_at_the_new_size() {
    let mut f = Fixture::new();
    f.start();
    let seat = f.ok(&["seat", "spawn", "zsh", "--name", "only"])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = await_value(&f, || {
        let row = f.ok(&["ui", "layout"]);
        let laid_out = row["data"]["panes"]
            .as_array()?
            .iter()
            .any(|pane| pane["seat"] == seat.as_str());
        laid_out.then_some(row)
    });
    let row = f.ok(&["app", "resize", "100x40"]);
    let data = &row["data"];
    let frame = data["frame_seq"].as_u64().unwrap();
    assert!(
        frame > before["data"]["frame_seq"].as_u64().unwrap(),
        "{row}"
    );
    assert_ne!(data["size"], before["data"]["size"], "{row}");
    let pane = data["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pane| pane["seat"] == seat.as_str())
        .unwrap_or_else(|| panic!("no pane in the reply: {row}"));
    assert!(pane["cols"].as_u64().unwrap() > 0, "{pane}");
    // The reply's frame is current: what is read right after is that frame or a later one.
    let after = f.ok(&["ui", "layout"]);
    assert!(
        after["data"]["frame_seq"].as_u64().unwrap() >= frame,
        "{after}"
    );
    assert_eq!(after["data"]["size"], data["size"]);
    let shot = f.root.path().join("resized.png");
    f.ok(&["seat", "capture", &seat, shot.to_str().unwrap()]);
    assert!(shot.exists());
    let (code, rows) = f.ctl(&["app", "resize", "1x1"]);
    assert_eq!(code, 2, "{rows:?}");
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec ctl-cli §1 (scripted window control)
#[test]
#[ignore = "night: real GPU/protocol boundary, test-policy #327 B2"]
fn retained_gap_fixture_requires_actual_test_account_and_preserves_live_bytes() {
    let mut ordinary = Fixture::new();
    ordinary.start();
    let seat = ordinary.ok(&["seat", "spawn", "zsh"])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let (code, refused) = ordinary.ctl(&[
        "debug",
        "fault",
        "ring-gap",
        &seat,
        "1..1",
        "--as",
        "test-account:spoof",
    ]);
    assert_ne!(code, 0);
    assert!(
        format!("{refused:?}").contains("test_account_required"),
        "{refused:?}"
    );
    ordinary.ok(&["app", "stop"]);
    ordinary.wait_stopped();

    let mut f = Fixture::new();
    let account = f.root.path().join("history-account");
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
    let seat = f.ok(&["seat", "spawn", "zsh"])["data"]["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ok(&["ui", "focus", &seat]);
    f.ok(&["seat", "type", &seat, "printf 'HISTORY_LIVE_ORIGINAL\\n'\r"]);
    f.ok(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^HISTORY_LIVE_ORIGINAL$",
        "--timeout",
        "15000",
    ]);
    let screen = f.ok(&["seat", "screen", &seat]);
    let through = screen["data"]["sequence"].as_u64().unwrap();
    // Keep output advancing after the read: its next sequence is no longer
    // necessarily future by the time the fault request reaches logic.
    f.ok(&[
        "seat",
        "type",
        &seat,
        "printf 'HISTORY_AFTER_SNAPSHOT\\n'\r",
    ]);
    f.ok(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^HISTORY_AFTER_SNAPSHOT$",
        "--timeout",
        "30000",
    ]);
    assert!(
        f.ok(&["seat", "screen", &seat])["data"]["sequence"]
            .as_u64()
            .unwrap()
            > through
    );
    let invalid = format!("{0}..{0}", u64::MAX);
    let (code, refused) = f.ctl(&["debug", "fault", "ring-gap", &seat, &invalid]);
    assert_ne!(code, 0, "{refused:?}");
    assert!(
        refused[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("ring_gap_invalid_range"),
        "{refused:?}"
    );
    let range = format!("1..{through}");
    let op = bbt_mux::Ulid::new().to_string();
    let installed = f.ok(&["debug", "fault", "ring-gap", &seat, &range, "--op", &op]);
    assert_eq!(installed["data"]["to"], through);
    assert_eq!(
        f.ok(&["debug", "fault", "ring-gap", &seat, &range, "--op", &op])["data"],
        installed["data"]
    );
    let live = f.ok(&["seat", "screen", &seat]);
    assert!(
        live["data"]["rows"]
            .to_string()
            .contains("HISTORY_LIVE_ORIGINAL")
    );
    assert_eq!(live["data"]["generation"], screen["data"]["generation"]);
    let history = f.ok(&["seat", "screen", &seat, "--scrollback", &range]);
    assert!(
        !history["data"]["rows"]
            .to_string()
            .contains("HISTORY_LIVE_ORIGINAL")
    );
    assert!(history["data"]["rows"].to_string().contains("gap"));
    let changed = format!("{through}..{through}");
    if through > 1 {
        assert_ne!(
            f.ctl(&["debug", "fault", "ring-gap", &seat, &changed, "--op", &op])
                .0,
            0
        );
    }
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}

// keeps: spec messaging-attention §3 MA-8 — actual ctl uses operator authority and one replayable cell edit.
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn headless_operator_attention_set_reset_and_replay_use_a_private_account() {
    let mut f = Fixture::new();
    let account = f.root.path().join("attention-account");
    f.run = account.join("run");
    let start = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = start["data"]["pid"].as_u64().map(|pid| pid as u32);
    let spawned = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "policy receiver",
        "--cwd",
        f.root.path().to_str().unwrap(),
    ]);
    let seat = spawned["data"]["seat"].as_str().unwrap();
    let op = bbt_mux::Ulid::new().to_string();
    let args = [
        "seat",
        "attention",
        seat,
        "#room",
        "event",
        "wake",
        "--op",
        &op,
    ];
    let set = f.ok(&args);
    assert_eq!(set["data"]["seat_id"], seat);
    assert_eq!(
        set["data"]["policy"]["rules"][0]["destination"],
        "channels/room"
    );
    assert_eq!(f.ok(&args)["data"]["policy"], set["data"]["policy"]);
    let (code, reused) = f.ctl(&[
        "seat",
        "attention",
        seat,
        "#room",
        "event",
        "digest",
        "--op",
        &op,
    ]);
    assert_ne!(code, 0, "{reused:?}");
    assert!(format!("{reused:?}").contains("operation ID belongs to different arguments or authority"), "{reused:?}");
    f.ok(&[
        "seat",
        "attention",
        seat,
        "#hidden",
        "conversation",
        "inbox",
    ]);
    let reset = f.ok(&["seat", "attention", seat, "#room", "event", "default"]);
    let rules = reset["data"]["policy"]["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["destination"], "channels/hidden");
    assert_eq!(
        f.ok(&["seat", "attention", seat])["data"]["policy"],
        reset["data"]["policy"]
    );
    f.ok(&["seat", "attention", seat, "direct", "event", "digest"]);
    let final_policy = f.ok(&["seat", "attention", seat]);
    assert!(
        final_policy["data"]["policy"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule["destination"] == format!("seats/{seat}/inbox"))
    );
    let direct = final_policy["data"]["defaults"]["direct_conversation"].as_str().unwrap();
    let expected = match direct { "wake" => "inbox", "inbox" => "digest", "digest" => "wake", _ => panic!("invalid default") };
    f.ok(&["ui", "open", "settings"]);
    f.ok(&["ui", "key", "tab"]);
    f.ok(&["ui", "key", "tab"]);
    for _ in 0..9 { f.ok(&["ui", "key", "down"]); }
    let layout = f.ok(&["ui", "layout"]);
    fn find_name(value: &Value) -> bool {
        if value["kind"] == "named:messaging/seat/name" {
            return value.to_string().contains("policy receiver");
        }
        value["children"].as_array().into_iter().flatten().any(find_name)
    }
    assert!(layout["data"]["tree"].as_array().unwrap().iter().any(find_name), "Settings did not show its focused seat: {layout}");
    f.ok(&["ui", "key", "tab"]); // first policy cell, after the navigation
    let applied = f.ok(&["ui", "key", "right"]);
    let readback = f.ok(&["seat", "attention", seat]);
    assert!(readback["data"]["policy"]["rules"].as_array().unwrap().iter().any(|rule|
        rule["destination"] == format!("seats/{seat}/inbox") && rule["class"] == "conversation" && rule["mode"] == expected),
        "Settings intent did not reach its operator setter: {readback}");
    println!("MA8 real operator setter/reset/replay and Settings key effect passed; frame={}", applied["data"]["frame_seq"]);
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}
// keeps: decision #344 rules 1–3; T5 first cut (window state persistence is separate)
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn app_relaunch_attaches_to_surviving_owners_and_refuses_a_second_gui() {
    relaunch_after_app_death(false);
}

// keeps: decision #344 rule 2; replacement logic takes the surviving supervisor lease
#[test]
#[ignore = "night: real GUI/process boundary, test-policy #327 B2"]
fn app_relaunch_replaces_dead_logic_without_restarting_its_supervisor_or_ptys() {
    relaunch_after_app_death(true);
}

fn relaunch_after_app_death(replace_logic: bool) {
    let mut fixture = Fixture::new();
    let account = fixture.root.path().join("account");
    let start = fixture.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    fixture.pid = Some(start["data"]["pid"].as_u64().unwrap() as u32);
    assert_eq!(start["data"]["custody_backend"], "file");
    let owner = |name: &str| -> u32 {
        let bytes = std::fs::read(fixture.run.join(format!("{name}.owner.json"))).unwrap();
        let row: Value = serde_json::from_slice(&bytes).unwrap();
        row["process"]["pid"].as_u64().unwrap() as u32
    };
    let supervisor = owner("supervisor");
    let logic = owner("logic");
    let seat =
        fixture.ok(&["seat", "spawn", "zsh", "--name", "T5 surviving shell"])["data"]["seat"]
            .as_str()
            .unwrap()
            .to_owned();
    fixture.ok(&[
        "seat",
        "type",
        &seat,
        "T5_COOKIE=55119; printf 'T5_READY %s\\n' $$\n",
    ]);
    fixture.ok(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^T5_READY [0-9]+$",
        "--timeout",
        "30000",
    ]);
    let before = fixture.ok(&["seat", "ls"])["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == seat)
        .unwrap()
        .clone();
    let mut crash_guard = CrashOwners::new(&[logic, supervisor]);
    let app = fixture.pid.take().unwrap();
    // SAFETY: this exact app PID was started and recorded by this private fixture.
    assert_eq!(unsafe { libc::kill(app as i32, libc::SIGKILL) }, 0);
    await_value(&fixture, || {
        crate_process_absent(app).then_some(Value::Bool(true))
    });
    if replace_logic {
        // SAFETY: this is the exact logic child started and recorded by our fixture.
        assert_eq!(unsafe {libc::kill(logic as i32,libc::SIGKILL)},0);
        await_value(&fixture, || crate_process_absent(logic).then_some(Value::Bool(true)));
    }
    let restarted = fixture.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    fixture.pid = Some(restarted["data"]["pid"].as_u64().unwrap() as u32);
    crash_guard.armed = false;
    assert_eq!(owner("supervisor"), supervisor);
    let current_logic = owner("logic");
    assert_eq!(current_logic == logic, !replace_logic);
    let after = fixture.ok(&["seat", "ls"])["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == seat)
        .unwrap()
        .clone();
    assert_eq!(
        before["record"]["generation"],
        after["record"]["generation"]
    );
    assert_eq!(before["state"]["process"], after["state"]["process"]);
    fixture.ok(&[
        "seat",
        "type",
        &seat,
        "printf 'T5_KEPT %s %s\\n' \"$T5_COOKIE\" $$\n",
    ]);
    fixture.ok(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^T5_KEPT 55119 [0-9]+$",
        "--timeout",
        "30000",
    ]);
    let second = Command::new(BIN)
        .args(["--headless", "--ctl", "--unattended", "--test-account"])
        .arg(&account)
        .arg("--run-dir")
        .arg(&fixture.run)
        .current_dir(fixture.root.path())
        .env("ZDOTDIR", fixture.root.path().join("shell"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("app_already_running"));
    assert_eq!(owner("supervisor"), supervisor);
    assert_eq!(owner("logic"), current_logic);
    fixture.ok(&["app", "stop"]);
    fixture.wait_stopped();
    assert!(crate_process_absent(supervisor));
    assert!(crate_process_absent(logic));
    assert!(crate_process_absent(current_logic));
    eprintln!("{}", serde_json::json!({
        "case":"T5-discovery", "replace_logic":replace_logic, "old_app":app,
        "new_app":restarted["data"]["pid"], "supervisor":supervisor,
        "old_logic":logic, "new_logic":current_logic, "seat":seat,
        "before_process":before["state"]["process"], "after_process":after["state"]["process"],
        "before_generation":before["record"]["generation"], "after_generation":after["record"]["generation"],
        "second_gui_refused":true, "shell_cookie_kept":true, "owners_gone_after_stop":true
    }));
}
fn crate_process_absent(pid: u32) -> bool {
    // SAFETY: zero only observes this fixture's recorded process, sending no signal.
    (unsafe { libc::kill(pid as i32, 0) }) < 0
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

// Failure-only cleanup of the exact backend processes created by this fixture.
struct CrashOwners { processes:Vec<(u32,Vec<u8>)>, armed:bool }
impl CrashOwners {
    fn fingerprint(pid:u32)->Option<Vec<u8>> {
        let out=Command::new("/bin/ps").args(["-p",&pid.to_string(),"-o","lstart=,args="]).output().ok()?;
        out.status.success().then_some(out.stdout)
    }
    fn new(pids:&[u32])->Self {
        Self {processes:pids.iter().map(|pid|(*pid,Self::fingerprint(*pid).unwrap())).collect(),armed:true}
    }
}
impl Drop for CrashOwners {
    fn drop(&mut self) {
        if !self.armed {return;}
        for (pid,fingerprint) in &self.processes {
            if Self::fingerprint(*pid).as_ref()==Some(fingerprint) {
                // SAFETY: PID and process birth/command still match our recorded child.
                unsafe {libc::kill(*pid as i32,libc::SIGTERM);}
            }
        }
        let end=Instant::now()+Duration::from_secs(5);
        while self.processes.iter().any(|(pid,_)|!crate_process_absent(*pid)) && Instant::now()<end {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
