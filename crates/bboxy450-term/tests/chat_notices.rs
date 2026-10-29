// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app/PTY fixture"
)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
const TASK: &str = "Private notice task";

// Adapted from bboxy450-term tests/swarm_workdir.rs::Fixture:
// https://github.com/bboxy450/bboxy450-term — reduced to private close/expiry control.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    app: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        for name in ["home", "bin", "repository/.bboxy/tasks"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        fs::write(
            root.join("home/.zprofile"),
            format!(
                "export PATH='{}:/usr/bin:/bin'\n",
                root.join("bin").display()
            ),
        )
        .unwrap();
        fs::write(root.join("home/.zshrc"), "PROMPT='%# '\n").unwrap();
        fs::write(root.join("bin/claude"), "#!/bin/sh\nprintf '%s\\n' \"$$\" > \"$HOME/mock-$BBOXY_SEAT_ID.pid\"\nprintf '\\nCHAT-NOTICE-HISTORY:%s\\n' \"$BBOXY_SEAT_ID\"\nexec /bin/cat\n").unwrap();
        fs::set_permissions(root.join("bin/claude"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("repository/.bboxy/tasks/notice.md"), format!("---\nid: {}\nnumber: 1\ntype: task\ntitle: {TASK}\nstatus: open\nassignee: none\n---\nPrivate task panel sentinel.\n", bbt_mux::Ulid(1))).unwrap();
        Self {
            directory,
            root,
            app: None,
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(BIN);
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("ZDOTDIR", self.root.join("home"))
            .env("CODEX_HOME", self.root.join("home/.codex"))
            .env("CLAUDE_CONFIG_DIR", self.root.join("home/.claude"))
            .env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }
    fn log(&self) -> String {
        fs::read_to_string(self.root.join("app.log")).unwrap_or_default()
    }
    fn ctl(&self, args: &[&str]) -> Value {
        let output = self
            .command()
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {} {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            self.log()
        );
        let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        reply["data"].clone()
    }
    fn wait<T>(&self, label: &str, seconds: u64, mut ready: impl FnMut() -> Option<T>) -> T {
        let until = Instant::now() + Duration::from_secs(seconds);
        loop {
            if let Some(value) = ready() {
                return value;
            }
            assert!(Instant::now() < until, "{label}: {}", self.log());
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn start(&mut self) {
        let log = fs::File::create(self.root.join("app.log")).unwrap();
        self.app = Some(
            self.command()
                .args(["--headless", "--ctl", "--unattended", "--test-account"])
                .arg(self.root.join("account"))
                .arg("--run-dir")
                .arg(self.root.join("run"))
                .arg("--repository")
                .arg(self.root.join("repository"))
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        self.wait("private GUI startup", 60, || {
            if !self.root.join("run/gui.endpoint").exists() {
                return None;
            }
            let output = self
                .command()
                .args(["ctl", "app", "status", "--run-dir"])
                .arg(self.root.join("run"))
                .output()
                .unwrap();
            if !output.status.success() {
                return None;
            }
            let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
            (reply["data"]["gui_ready"] == true && reply["data"]["backend_ready"] == true)
                .then_some(())
        });
    }
    fn spawn(&self, swarm: &str, name: &str) -> String {
        let seat = self.ctl(&[
            "seat", "spawn", "claude", "--place", "tab", "--swarm", swarm, "--name", name,
        ])["seat"]
            .as_str()
            .unwrap()
            .to_owned();
        self.ctl(&[
            "seat",
            "wait",
            &seat,
            "--text",
            &format!("CHAT-NOTICE-HISTORY:{seat}"),
            "--timeout",
            "30000",
        ]);
        seat
    }
    fn row(&self, seat: &str) -> Value {
        self.ctl(&["seat", "ls"])
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["record"]["id"] == seat)
            .unwrap()
            .clone()
    }
    fn history(&self, seat: &str, to: u64) -> Vec<bbt_proto::gui::HistoryRecord> {
        let endpoint =
            bboxy450_term::ctl::Endpoint::read(&self.root.join("run/gui.endpoint")).unwrap();
        bboxy450_term::ctl::Client::connect(&endpoint)
            .unwrap()
            .history(seat.parse().unwrap(), 1, to)
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let directory = std::mem::replace(&mut self.directory, tempfile::tempdir().unwrap());
            eprintln!(
                "failed private chat notice fixture retained: {}",
                directory.keep().display()
            );
        }
        let owners: Vec<_> = ["supervisor", "logic"]
            .into_iter()
            .filter_map(|name| {
                let value: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).ok()?,
                )
                .ok()?;
                value["process"]["pid"]
                    .as_u64()
                    .and_then(|pid| i32::try_from(pid).ok())
            })
            .collect();
        let _ = self
            .command()
            .args(["ctl", "app", "stop", "--run-dir"])
            .arg(self.root.join("run"))
            .output();
        if let Some(mut app) = self.app.take() {
            if app.try_wait().ok().flatten().is_none() {
                let _ = app.kill();
            }
            let _ = app.wait();
        }
        for pid in owners {
            // Only backend owners recorded by this fixture's private run directory.
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
        }
    }
}

fn named<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    if node["kind"].as_str() == Some(key) {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|node| named(node, key))
}
fn section<'a>(frame: &'a Value, key: &str) -> Option<&'a Value> {
    frame["tree"]
        .as_array()?
        .iter()
        .find_map(|node| named(node, key))
}

#[test]
#[ignore = "private ctl chat notice placement, eight-second timer redraw and retained history"]
fn closed_harness_notice_expires_from_chat_without_losing_history() {
    let mut fixture = Fixture::new();
    fixture.start();
    let swarm = fixture.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = fixture.spawn(&swarm, "Notice target");
    let peer = fixture.spawn(&swarm, "Notice peer");
    fixture.ctl(&["app", "resize", "180x64"]);
    fixture.wait("separate populated chat/task sections", 20, || {
        let frame = fixture.ctl(&["ui", "layout"]);
        (section(&frame, "named:conversation/chat-section").is_some()
            && section(&frame, "named:conversation/task-section")
                .is_some_and(|node| node.to_string().contains(TASK)))
        .then_some(())
    });
    let before = fixture.row(&first);
    let peer_before = fixture.row(&peer);
    let pid: i32 = fs::read_to_string(fixture.root.join(format!("home/mock-{first}.pid")))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(pid > 1);
    assert_eq!(
        before["state"]["process"]["pid"].as_i64(),
        Some(i64::from(pid))
    );
    fixture.ctl(&["ui", "focus", &first]);
    let closed = fixture.ctl(&["ui", "key", "cmd-w"]);
    assert_eq!(closed["focused"], peer);
    let grace = fixture.ctl(&["app", "status"]);
    assert_eq!(grace["close"]["grace"][0]["seat"], first);
    assert_eq!(grace["close"]["grace"][0]["undo"], true);
    // The PID was written by this exact private mock and matched before closure.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
    let notice = fixture.wait("confirmed retained close notice", 15, || {
        let status = fixture.ctl(&["app", "status"]);
        status["close"]["notices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|notice| notice["seat"] == first)
            .cloned()
    });
    let retained = fixture.row(&first);
    assert_eq!(retained["record"]["lifecycle"], "closed_retained");
    assert_eq!(
        notice["operation"],
        retained["record"]["close"]["operation"]
    );
    assert_eq!(notice["generation"], before["record"]["generation"]);
    assert_eq!(notice["swarm"], swarm);
    assert!((1..=8000).contains(&notice["remaining_ms"].as_u64().unwrap()));
    let text = notice["text"].as_str().unwrap();
    assert_eq!(text, "Notice target closed; its history is kept");
    fixture.wait("notice rendered inside chat only", 5, || {
        let frame = fixture.ctl(&["ui", "layout"]);
        let chat = section(&frame, "named:conversation/chat-section")?;
        let tasks = section(&frame, "named:conversation/task-section")?;
        assert!(!tasks.to_string().contains(text));
        chat.to_string().contains(text).then_some(())
    });
    let visible = fixture.ctl(&["app", "status"]);
    assert!(
        visible["close"]["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["seat"] == first)
    );
    let mut last_visible_frame = visible["frame_seq"].as_u64().unwrap();
    let history = fixture.history(&first, u64::MAX);
    let bytes: Vec<_> = history
        .iter()
        .flat_map(|row| match row {
            bbt_proto::gui::HistoryRecord::Bytes { bytes, .. } => bytes.as_slice(),
            bbt_proto::gui::HistoryRecord::Gap { .. } => {
                panic!("private retained history has a gap")
            }
        })
        .copied()
        .collect();
    assert!(String::from_utf8_lossy(&bytes).contains(&format!("CHAT-NOTICE-HISTORY:{first}")));
    let through = history
        .iter()
        .filter_map(|row| match row {
            bbt_proto::gui::HistoryRecord::Bytes { sequence, .. } => Some(*sequence),
            _ => None,
        })
        .max()
        .unwrap();

    // Status is a readback, not a redraw request. Observe the timer's own frame
    // after the last status that still contained the active exact-seat notice.
    let expired = fixture.wait("unattended expiry must produce a new frame", 12, || {
        let status = fixture.ctl(&["app", "status"]);
        let frame = status["frame_seq"].as_u64().unwrap();
        if status["close"]["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["seat"] == first)
        {
            last_visible_frame = frame;
            None
        } else {
            (frame > last_visible_frame).then_some(status)
        }
    });
    assert!(expired["close"]["grace"].as_array().unwrap().is_empty());
    let frame = fixture.ctl(&["ui", "layout"]);
    assert!(frame["frame_seq"].as_u64().unwrap() >= expired["frame_seq"].as_u64().unwrap());
    for key in [
        "named:conversation/chat-section",
        "named:conversation/task-section",
    ] {
        assert!(
            !section(&frame, key).unwrap().to_string().contains(text),
            "{key}"
        );
    }
    assert_eq!(fixture.history(&first, through), history);
    assert_eq!(
        fixture.row(&first)["record"]["generation"],
        before["record"]["generation"]
    );
    assert_eq!(
        fixture.row(&peer)["state"]["process"],
        peer_before["state"]["process"]
    );
}
