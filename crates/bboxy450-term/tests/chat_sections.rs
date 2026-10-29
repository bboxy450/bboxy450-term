// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from this project's tests/swarm_workdir.rs private Fixture and vertical split test.
// Changes: backend-arriving chat, independent Tasks scroll, and third-handle persistence.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app/PTY fixture"
)]
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
const DEADLINE: Duration = Duration::from_secs(15);

struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    app: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::create_dir(root.join("home")).unwrap();
        let tasks = root.join("repository/.bboxy/tasks");
        fs::create_dir_all(&tasks).unwrap();
        fs::write(root.join("home/.zshrc"), "PROMPT='%# '\n").unwrap();
        for number in 1..=24 {
            let id = bbt_mux::Ulid(100 + number);
            fs::write(tasks.join(format!("{id}.md")), format!(
                "---\nid: {id}\nnumber: {number}\ntype: task\ntitle: Private section task {number:02}\nstatus: open\nassignee: none\nverify: 'true'\n---\nPrivate repository task body.\n"
            )).unwrap();
        }
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
    fn log(&self) -> String {
        fs::read_to_string(self.root.join("app.log")).unwrap_or_default()
    }
    fn start(&mut self, theme: &str) {
        let log = fs::File::create(self.root.join("app.log")).unwrap();
        self.app = Some(
            self.command()
                .args([
                    "--headless",
                    "--ctl",
                    "--unattended",
                    "--theme",
                    theme,
                    "--test-account",
                ])
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
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                self.app.as_mut().unwrap().try_wait().unwrap().is_none(),
                "{}",
                self.log()
            );
            let output = self
                .command()
                .args(["ctl", "app", "status", "--run-dir"])
                .arg(self.root.join("run"))
                .output()
                .unwrap();
            if output.status.success() {
                let status: Value = serde_json::from_slice(&output.stdout).unwrap();
                if status["data"]["gui_ready"] == true && status["data"]["backend_ready"] == true {
                    break;
                }
            }
            assert!(
                Instant::now() < deadline,
                "app did not become ready: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn wait_frame(&self, what: &str, condition: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let frame = self.ctl(&["ui", "layout"]);
            if condition(&frame) {
                return frame;
            }
            assert!(Instant::now() < deadline, "{what}: {frame}\n{}", self.log());
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn incoming(&self, swarm: &str, marker: &str) {
        // Deliberately bypass UiRequest::Compose, whose explicit send follows the tail.
        let body = format!(
            "{marker}\nIncoming backend message.\nSecond retained line.\nThird retained line."
        );
        let sent = self.ctl(&[
            "message",
            "send",
            &format!("swarms/{swarm}"),
            &body,
            "--swarm",
            swarm,
        ]);
        assert_eq!(sent["kind"], "stored", "{sent}");
    }
    fn owners(&self) -> Vec<u64> {
        ["supervisor", "logic"]
            .map(|name| {
                let owner: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).unwrap(),
                )
                .unwrap();
                owner["process"]["pid"].as_u64().unwrap()
            })
            .into()
    }
    fn process(&self, seat: &str) -> (Value, Value) {
        let rows = self.ctl(&["seat", "ls"]);
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["record"]["id"] == seat)
            .unwrap();
        (
            row["state"]["process"].clone(),
            row["record"]["generation"].clone(),
        )
    }
    fn drag(&self, key: &str, delta: f64) -> Value {
        let frame = self.ctl(&["ui", "layout"]);
        let handle = frame["regions"][key]
            .as_array()
            .unwrap_or_else(|| panic!("missing {key}: {frame}"));
        let x = (handle[0].as_f64().unwrap() + handle[2].as_f64().unwrap() / 2.)
            .round()
            .to_string();
        let y = (handle[1].as_f64().unwrap() + handle[3].as_f64().unwrap() / 2.).round();
        self.ctl(&[
            "ui",
            "drag",
            &x,
            &y.to_string(),
            &x,
            &(y + delta).max(0.).to_string(),
        ])
    }
    fn capture(&self, name: &str, frame: &Value) {
        if let Some(directory) = std::env::var_os("BBOXY_CHAT_SECTIONS_EVIDENCE") {
            let directory = PathBuf::from(directory);
            fs::create_dir_all(&directory).unwrap();
            let path = directory.join(format!("{name}.png"));
            let captured = self.ctl(&["seat", "capture", "window", path.to_str().unwrap()]);
            assert!(
                captured["frame_seq"].as_u64().unwrap() >= frame["frame_seq"].as_u64().unwrap()
            );
            assert!(fs::read(&path).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
            fs::write(
                directory.join(format!("{name}.json")),
                serde_json::to_vec_pretty(frame).unwrap(),
            )
            .unwrap();
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let old = std::mem::replace(&mut self.directory, tempfile::tempdir().unwrap());
            eprintln!(
                "failed private chat sections fixture retained: {}",
                old.keep().display()
            );
        }
        let owners: Vec<u32> = ["supervisor", "logic"]
            .into_iter()
            .filter_map(|name| {
                let owner: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).ok()?,
                )
                .ok()?;
                owner["process"]["pid"].as_u64().map(|pid| pid as u32)
            })
            .collect();
        let _ = self
            .command()
            .args(["ctl", "app", "stop", "--run-dir"])
            .arg(self.root.join("run"))
            .output();
        if let Some(mut child) = self.app.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        for pid in owners {
            // Only backend owner PIDs recorded inside this fixture's private run directory.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
    }
}

fn section(frame: &Value, name: &str) -> [f64; 2] {
    let value = &frame["sidebar_sections"][name];
    let offset = value["offset"]
        .as_f64()
        .unwrap_or_else(|| panic!("missing {name} receipt: {frame}"));
    let max = value["max"].as_f64().unwrap();
    assert!(offset.is_finite() && max.is_finite() && offset >= 0. && offset <= max);
    if name == "tasks" {
        assert_eq!(value["following"], false);
    }
    [offset, max]
}
fn rect(frame: &Value, key: &str) -> [f64; 4] {
    fn find<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
        if node["kind"].as_str() == Some(key) {
            return Some(node);
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| find(child, key))
    }
    let node = frame["tree"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|node| find(node, key))
        .unwrap_or_else(|| panic!("missing {key}: {frame}"));
    std::array::from_fn(|i| node["rect"][i].as_f64().unwrap())
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.1
}
fn follows(frame: &Value) -> bool {
    let [offset, max] = section(frame, "chat");
    frame["sidebar_sections"]["chat"]["following"] == true && near(offset, max)
}

#[test]
#[ignore = "private ctl chat/task scrolling, third resize and UI restart; root warm gate"]
fn incoming_chat_follows_tail_manual_scroll_and_tasks_resize_are_independent() {
    let mut f = Fixture::new();
    f.start("dark");
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let seat = f.ctl(&["seat", "spawn", "zsh", "--place", "tab", "--swarm", &swarm])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ctl(&[
        "ui",
        "swarm-workdir",
        &swarm,
        f.root.join("repository").to_str().unwrap(),
    ]);
    f.ctl(&["app", "resize", "180x72"]);
    f.drag("sidebar-conversation-resize", -180.);
    let loaded = f.wait_frame("repository Tasks loaded in their own viewport", |frame| {
        frame["sidebar_sections"]["tasks"]["max"]
            .as_f64()
            .is_some_and(|max| max > 100.)
            && frame.to_string().contains("Private section task")
    });
    let chat = rect(&loaded, "named:conversation/chat-section");
    let tasks_rect = rect(&loaded, "named:conversation/task-section");
    assert!(chat[3] > 0. && tasks_rect[3] > 0.);
    assert!(near(chat[1] + chat[3], tasks_rect[1]));
    let tasks = section(&loaded, "tasks");
    for number in 0..18 {
        f.incoming(&swarm, &format!("INCOMING-{number:02}"));
    }
    let tail = f.wait_frame(
        "incoming messages advance the same view to its tail",
        |frame| {
            frame["sidebar_sections"]["chat"]["max"]
                .as_f64()
                .is_some_and(|max| max > 400.)
                && frame.to_string().contains("INCOMING-17")
                && follows(frame)
        },
    );
    assert_eq!(section(&tail, "tasks"), tasks, "arriving chat moved Tasks");
    f.capture("dark-tail", &tail);
    let up = f.ctl(&["ui", "chat-wheel", "-160"]);
    let manual = section(&up, "chat");
    assert!(manual[0] < manual[1] - 100.);
    assert_eq!(up["sidebar_sections"]["chat"]["following"], false);
    f.incoming(&swarm, "ARRIVED-WHILE-SCROLLED-UP");
    let retained = f.wait_frame("arriving message preserves manual chat offset", |frame| {
        section(frame, "chat")[1] > manual[1] + 1.
    });
    assert!(near(section(&retained, "chat")[0], manual[0]), "{retained}");
    assert_eq!(retained["sidebar_sections"]["chat"]["following"], false);
    let moved_tasks = f.ctl(&["ui", "task-wheel", "84"]);
    let scrolled_tasks = section(&moved_tasks, "tasks");
    assert!(scrolled_tasks[0] > tasks[0] + 40.);
    assert_eq!(section(&moved_tasks, "chat"), section(&retained, "chat"));
    let down = f.ctl(&["ui", "chat-wheel", "32768"]);
    assert!(follows(&down));
    assert_eq!(section(&down, "tasks"), scrolled_tasks);
    let previous_max = section(&down, "chat")[1];
    f.incoming(&swarm, "FOLLOWING-AGAIN");
    let followed = f.wait_frame(
        "wheel to bottom resumes incoming-message following",
        |frame| {
            section(frame, "chat")[1] > previous_max + 1.
                && frame.to_string().contains("FOLLOWING-AGAIN")
                && follows(frame)
        },
    );
    assert_eq!(section(&followed, "tasks"), scrolled_tasks);

    // The third handle gives Tasks space while leaving the outer navigation split and editor intact.
    let outer = rect(&followed, "named:conversation");
    let body = rect(&followed, "named:composer/body-viewport");
    let task_height = rect(&followed, "named:conversation/task-section")[3];
    let chat_height = rect(&followed, "named:conversation/chat-section")[3];
    let resized = f.drag("sidebar-tasks-resize", -52.);
    let larger_tasks = rect(&resized, "named:conversation/task-section")[3];
    assert!(larger_tasks > task_height + 40.);
    assert!(rect(&resized, "named:conversation/chat-section")[3] < chat_height - 40.);
    assert_eq!(rect(&resized, "named:conversation"), outer);
    assert!(near(
        rect(&resized, "named:composer/body-viewport")[3],
        body[3]
    ));
    assert!(follows(&resized));
    f.capture("dark-tasks-expanded", &resized);
    let config = f.root.join("account/config.toml");
    let saved = fs::read_to_string(&config).unwrap();
    let doc: toml_edit::DocumentMut = saved.parse().unwrap();
    assert!(near(
        doc["sidebar"]["tasks_height"].as_float().unwrap(),
        larger_tasks
    ));
    let process = f.process(&seat);
    let owners = f.owners();
    let mut old = f.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    f.start("light");
    f.ctl(&["app", "resize", "180x72"]);
    let restored = f.wait_frame("third Tasks height restores after UI restart", |frame| {
        near(
            rect(frame, "named:conversation/task-section")[3],
            larger_tasks,
        ) && section(frame, "tasks")[1] > 100.
            && frame.to_string().contains("FOLLOWING-AGAIN")
            && follows(frame)
    });
    assert_eq!(rect(&restored, "named:conversation"), outer);
    assert_eq!(fs::read_to_string(config).unwrap(), saved);
    assert_eq!(f.owners(), owners);
    assert_eq!(f.process(&seat), process);
    f.capture("light-restored", &restored);
    f.ctl(&["app", "stop"]);
    assert!(f.app.as_mut().unwrap().wait().unwrap().success());
}
