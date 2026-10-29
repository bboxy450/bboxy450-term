// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app/PTY fixture, never the operator's runtime"
)]
use bbt_mux::{
    Ulid,
    registry::{Lifecycle, Registry, SeatRecord},
    store::{object_dir, sqlite::SqliteStore},
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
const TASK: &str = "GUI retained membership task";
const MESSAGE: &str = "GUI retained membership message";

// Adapted from this project's swarm_workdir.rs private headless fixture. Keep
// HOME, file custody, endpoint, logs and backend-owner cleanup inside TempDir.
// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct Fixture {
    directory: Option<tempfile::TempDir>,
    root: PathBuf,
    app: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("gui-events-app-")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("run"))
            .unwrap();
        fs::create_dir(root.join("home")).unwrap();
        fs::write(root.join("home/.zshrc"), "PROMPT='%# '\n").unwrap();
        fs::create_dir_all(root.join("repository/.bboxy/tasks")).unwrap();
        fs::write(
            root.join("repository/.bboxy/tasks/gui-events.md"),
            format!(
                "---\nid: {}\ntype: task\ntitle: {TASK}\nstatus: open\nverify: 'true'\n---\nPrivate GUI events fixture.\n",
                Ulid::new()
            ),
        )
        .unwrap();
        let registry = Registry::new(
            SqliteStore::open(&object_dir(&root.join("run"), "account").unwrap()).unwrap(),
        );
        for index in 0..43 {
            let mut seat = SeatRecord::new(
                Ulid(1000 + index),
                Ulid(2000 + index % 7),
                format!("retained fixture {index}"),
                "zsh",
            );
            // Historical registry membership is real; no current process,
            // harness session or signing identity is claimed for these rows.
            seat.lifecycle = Lifecycle::ClosedRetained;
            registry.insert(&seat).unwrap();
        }
        let retained = registry.list().unwrap();
        let swarms: BTreeSet<_> = retained.iter().map(|seat| seat.swarm).collect();
        assert_eq!((retained.len(), swarms.len()), (43, 7));
        assert!(retained.iter().all(|seat| {
            seat.lifecycle == Lifecycle::ClosedRetained && seat.harness_session_id.is_none()
        }));
        // Production operator_subjects: two operator sinks, four root folders,
        // one folder per swarm, one link per retained seat; GUI adds nine.
        // This is membership data, never a fabricated consumer/filter vector.
        assert_eq!(2 + 4 + swarms.len() + retained.len() + 9, 65);
        Self {
            directory: Some(directory),
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
            .current_dir(self.root.join("repository"))
            .stdin(Stdio::null());
        command
    }
    fn raw_ctl(&self, args: &[&str]) -> Output {
        self.command()
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"))
            .output()
            .unwrap()
    }
    fn ctl(&self, args: &[&str]) -> Value {
        let output = self.raw_ctl(args);
        let mut log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("ctl.jsonl"))
            .unwrap();
        writeln!(log, "{}", serde_json::json!({"args":args,"exit":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)})).unwrap();
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
    fn start(&mut self) {
        let log = fs::File::create(self.root.join("app.log")).unwrap();
        self.app = Some(
            self.command()
                .args(["--headless", "--ctl", "--unattended", "--test-account"])
                .arg(self.root.join("test-account"))
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
            if self.root.join("run/gui.endpoint").exists() {
                let output = self.raw_ctl(&["app", "status"]);
                if output.status.success() {
                    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
                    if status["data"]["gui_ready"] == true
                        && status["data"]["backend_ready"] == true
                        && status["data"]["events_ready"] == true
                    {
                        assert_eq!(status["data"]["mode"], "headless");
                        assert_eq!(status["data"]["windows"], 0);
                        assert_eq!(status["data"]["custody_backend"], "file");
                        break;
                    }
                }
            }
            assert!(
                Instant::now() < deadline,
                "65-subject GUI lane never ready: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let owners: Vec<u32> = ["supervisor", "logic"]
            .into_iter()
            .filter_map(|name| {
                let value: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).ok()?,
                )
                .ok()?;
                u32::try_from(value["process"]["pid"].as_u64()?).ok()
            })
            .collect();
        let _ = self.raw_ctl(&["app", "stop"]);
        if let Some(mut child) = self.app.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        for pid in owners {
            // Only backend owners recorded in this fixture's private run directory.
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
        if std::thread::panicking() || std::env::var_os("BBOXY_GUI_EVENTS_EVIDENCE").is_some() {
            eprintln!(
                "private GUI events evidence: {}",
                self.directory.take().unwrap().keep().display()
            );
        }
    }
}

fn find<'a>(node: &'a Value, predicate: &impl Fn(&Value) -> bool) -> Option<&'a Value> {
    if predicate(node) {
        return Some(node);
    }
    ["tree", "children"].into_iter().find_map(|field| {
        node[field]
            .as_array()?
            .iter()
            .find_map(|child| find(child, predicate))
    })
}
fn rect(node: &Value) -> [f64; 4] {
    std::array::from_fn(|i| node["rect"][i].as_f64().unwrap())
}
fn visible_in(child: [f64; 4], parent: [f64; 4]) -> bool {
    child.into_iter().chain(parent).all(f64::is_finite)
        && child[2] > 0.
        && child[3] > 0.
        && parent[2] > 0.
        && parent[3] > 0.
        && child[0].max(parent[0]) < (child[0] + child[2]).min(parent[0] + parent[2])
        && child[1].max(parent[1]) < (child[1] + child[3]).min(parent[1] + parent[3])
}

// keeps: GUI lane admission must survive legitimate retained membership above64;
// real composed-message receipts and task geometry remain visible, without OS input.
#[test]
#[ignore = "private real app/PTY fixture; make test-gui-events runs this unattended"]
fn retained_membership_over_64_keeps_events_chat_and_tasks_live() {
    let mut fixture = Fixture::new();
    fixture.start();
    let spawned = fixture.ctl(&["seat", "spawn", "zsh", "--name", "GUI events live fixture"]);
    let seat = spawned["seat"].as_str().unwrap();
    fixture.ctl(&["ui", "focus", seat]);
    let seats = fixture.ctl(&["seat", "ls"]);
    let seats = seats.as_array().unwrap();
    // ListSeats projects current GUI seats; historical membership remains in
    // the registry and still contributes to the exact event catalogue.
    let retained = Registry::new(
        SqliteStore::open(&object_dir(&fixture.root.join("run"), "account").unwrap()).unwrap(),
    )
    .list()
    .unwrap();
    assert_eq!(
        retained
            .iter()
            .filter(|s| s.lifecycle == Lifecycle::ClosedRetained)
            .count(),
        43
    );
    assert_eq!(seats.len(), 1);
    assert_eq!(
        seats
            .iter()
            .filter(|s| s["record"]["lifecycle"] == "open")
            .count(),
        1
    );
    fixture.ctl(&["seat", "attention", seat, "direct", "conversation", "inbox"]);
    let cards = fixture.ctl(&["task", "list", "--unassigned"]);
    assert_eq!(cards.as_array().unwrap().len(), 1, "{cards}");
    assert_eq!(cards[0]["title"], TASK);
    let composed = fixture.ctl(&["ui", "compose", "--to", seat, "--text", MESSAGE]);
    assert_eq!(composed["composed"]["text"], MESSAGE);
    let messages = composed["composed"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let message = messages[0].as_str().unwrap();
    let message_rect = rect(&composed["composed"]);
    let size = &composed["size"];
    assert!(visible_in(
        message_rect,
        [0., 0., size[0].as_f64().unwrap(), size[1].as_f64().unwrap()]
    ));
    assert!(
        find(&composed, &|node| node["text"]
            .as_str()
            .is_some_and(|s| s.contains(MESSAGE)))
        .is_some()
    );
    let chat = find(&composed, &|n| n["kind"] == "named:conversation/rows").unwrap();
    assert!(visible_in(message_rect, rect(chat)));
    let receipts = fixture.ctl(&["message", "receipts", message]);
    assert!(
        receipts.as_array().unwrap().iter().any(|row| {
            row["receipt"]["recipient"] == seat
                && row["receipt"]["evidence"] == "recipient-stored"
                && row["receipt"]["held"].is_null()
        }),
        "recipient lacks a real stored receipt: {receipts}"
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let shown = loop {
        let layout = fixture.ctl(&["ui", "layout"]);
        let header = find(&layout, &|n| n["kind"] == "named:conversation/tasks-header");
        let viewport = find(&layout, &|n| n["kind"] == "named:conversation/tasks");
        let task = find(&layout, &|n| {
            n["text"].as_str().is_some_and(|s| s.contains(TASK))
        });
        if let (Some(header), Some(viewport), Some(task)) = (header, viewport, task)
            && rect(header)[2] > 0.
            && rect(header)[3] > 0.
            && visible_in(rect(task), rect(viewport))
        {
            break layout;
        }
        assert!(
            Instant::now() < deadline,
            "task never visible in produced Tasks viewport: {layout}"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(shown["frame_seq"].as_u64().unwrap() >= composed["frame_seq"].as_u64().unwrap());
    assert!(find(&shown, &|n| n["kind"] == "named:conversation/chat-header").is_some());
    let section = find(&shown, &|n| n["kind"] == "named:conversation/task-section").unwrap();
    let section = rect(section);
    assert!(section[0] >= 0. && section[1] >= 0. && section[2] > 0. && section[3] > 0.);
    assert!(section[0] + section[2] <= shown["size"][0].as_f64().unwrap() + 0.01);
    assert!(section[1] + section[3] <= shown["size"][1].as_f64().unwrap() + 0.01);
    assert_eq!(fixture.ctl(&["app", "status"])["events_ready"], true);
    fs::write(
        fixture.root.join("produced-layout.json"),
        serde_json::to_vec_pretty(&shown).unwrap(),
    )
    .unwrap();
    let capture = fixture.ctl(&[
        "seat",
        "capture",
        "window",
        fixture.root.join("produced.png").to_str().unwrap(),
    ]);
    assert_eq!(capture["offscreen"], true);
    assert_eq!(capture["stale"], false);
    assert!(capture["frame_seq"].as_u64().unwrap() >= shown["frame_seq"].as_u64().unwrap());
}
