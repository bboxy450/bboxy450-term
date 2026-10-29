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
    bus::Broker,
    registry::{Lifecycle, Registry, SeatRecord},
    store::{Events, Store, events::EventLog, object_dir, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;
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
                "---\nid: {}\nnumber: 1\ntype: task\ntitle: {TASK}\nstatus: open\nverify: 'true'\n---\nPrivate GUI events fixture.\n",
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
        // one folder per swarm, one link and canonical inbox per retained seat;
        // GUI adds nine.
        // This is membership data, never a fabricated consumer/filter vector.
        assert_eq!(2 + 4 + swarms.len() + 2 * retained.len() + 9, 108);
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
        if !output.status.success() && args.starts_with(&["ui", "compose"]) {
            // Read-only failure witness; never retry an unknown send.
            let layout = self.raw_ctl(&["ui", "layout"]);
            fs::write(
                self.root.join("failed-produced-layout.json"),
                &layout.stdout,
            )
            .unwrap();
        }
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
    fn retain_receipt_backlog(&mut self, swarm: &str, message: &str) -> (u64, u64) {
        // Only this fixture's UI/logic owners stop. Its supervisor and PTY stay
        // alive; no runtime reset, cursor write or operator identity is used.
        let owner: Value =
            serde_json::from_slice(&fs::read(self.root.join("run/logic.owner.json")).unwrap())
                .unwrap();
        let pid = u32::try_from(owner["process"]["pid"].as_u64().unwrap()).unwrap();
        let start = bbt_mux::adapters::native::process_start(pid).unwrap();
        let mut app = self.app.take().unwrap();
        app.kill().unwrap();
        app.wait().unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGTERM) }, 0);
        let deadline = Instant::now() + Duration::from_secs(10);
        while bbt_mux::adapters::native::process_start(pid).is_ok_and(|now| now == start) {
            assert!(
                Instant::now() < deadline,
                "private logic owner did not stop"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let stream = format!("swarm-{swarm}");
        let directory = self.root.join("run").join(&stream);
        let log = EventLog::open(&directory).unwrap();
        let original = log
            .after(0, 128)
            .unwrap()
            .into_iter()
            .find(|event| {
                event.event.subject.ends_with("/receipt")
                    && event.event.payload["message"] == message
            })
            .expect("real seed receipt on its canonical stream");
        let before = log.last_sequence().unwrap();
        // A labelled legacy workload, not fabricated model acknowledgement:
        // clone the private seed receipt's shape and append fixture-signed
        // no-effect/pending history. Production retry behavior is independently
        // exercised through the real expired-readiness guard in messaging_flow.
        // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        let mut broker =
            Broker::open(&self.root.join("run"), SigningKey::from_bytes(&[73; 32])).unwrap();
        for i in 0..16_384 {
            let mut draft = original.event.clone();
            draft.id = Ulid::new();
            draft.issued_by = None; // Fixture signer claims no app/account attribution.
            draft.payload["evidence"] = "recipient-stored".into();
            draft.payload["note"] = if i % 2 == 0 {
                "attempt journaled before native/PTY effects; result pending"
            } else {
                "readiness indication expired or replaced"
            }
            .into();
            draft.payload["fixture"] = "retained-legacy-no-effect-workload".into();
            broker.publish(&stream, draft).unwrap();
        }
        drop(broker);
        let high = log.last_sequence().unwrap();
        assert_eq!(high - before, 16_384);
        (before, high)
    }
    fn start(&mut self) {
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("app.log"))
            .unwrap();
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
            // Receipts expose the latest phase; a real delivery can advance
            // after durable recipient storage before this read arrives.
            use bbt_proto::messaging::Evidence;
            let durable = matches!(
                serde_json::from_value::<Evidence>(row["receipt"]["evidence"].clone()),
                Ok(Evidence::RecipientStored
                    | Evidence::NativeQueued
                    | Evidence::NativeWritten
                    | Evidence::ApplicationAcknowledged)
            );
            row["receipt"]["recipient"] == seat && durable && row["receipt"]["held"].is_null()
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

// keeps: #73 a retained unsuccessful-receipt lane must not hide a new composer
// write or monopolize terminal controls. All evidence is private produced data.
#[test]
#[ignore = "private retained app/PTY fixture; make test-chat-backlog-frame runs this unattended"]
fn retained_receipt_backlog_keeps_fresh_composer_and_terminal_live() {
    let mut fixture = Fixture::new();
    fixture.start();
    let spawned = fixture.ctl(&["seat", "spawn", "zsh", "--name", "retained receipt fixture"]);
    let seat = spawned["seat"].as_str().unwrap().to_owned();
    fixture.ctl(&["ui", "focus", &seat]);
    let seed = fixture.ctl(&[
        "ui",
        "compose",
        "--to",
        &seat,
        "--text",
        "private retained seed",
    ]);
    let seed = seed["composed"]["messages"][0].as_str().unwrap().to_owned();
    let rows = fixture.ctl(&["seat", "ls"]);
    let swarm = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == seat)
        .unwrap()["record"]["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let (before, high) = fixture.retain_receipt_backlog(&swarm, &seed);
    let account = SqliteStore::open(&fixture.root.join("run/account")).unwrap();
    let read_key = format!("read/swarm-{swarm}");
    let read_before = account.get(&read_key).unwrap();
    fixture.start();
    fixture.ctl(&["ui", "focus", &seat]);
    let started = Instant::now();
    let frame = fixture.ctl(&[
        "ui",
        "compose",
        "--to",
        &seat,
        "--text",
        "fresh message through retained receipts",
    ]);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "existing control budget"
    );
    let fresh = frame["composed"]["messages"][0].as_str().unwrap();
    let chat = find(&frame, &|n| n["kind"] == "named:conversation/rows").unwrap();
    assert!(visible_in(rect(&frame["composed"]), rect(chat)));
    assert_eq!(
        account.get(&read_key).unwrap(),
        read_before,
        "live recovery must not mark history read"
    );
    let receipts = fixture.ctl(&["message", "receipts", fresh]);
    assert!(
        receipts
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["receipt"]["recipient"] == seat
                && matches!(
                    serde_json::from_value::<bbt_proto::messaging::Evidence>(
                        row["receipt"]["evidence"].clone()
                    ),
                    Ok(bbt_proto::messaging::Evidence::RecipientStored
                        | bbt_proto::messaging::Evidence::NativeQueued
                        | bbt_proto::messaging::Evidence::NativeWritten
                        | bbt_proto::messaging::Evidence::NativeUnknown)
                ))
    );
    let compose_ms = started.elapsed().as_millis();
    let input_at = Instant::now();
    fixture.ctl(&[
        "seat",
        "type",
        &seat,
        "printf 'RETAINED-%s-FAIR\\n' INPUT\n",
    ]);
    fixture.ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "RETAINED-INPUT-FAIR",
        "--timeout",
        "5000",
    ]);
    assert!(input_at.elapsed() < Duration::from_secs(5));
    let log = EventLog::open(&fixture.root.join("run").join(format!("swarm-{swarm}"))).unwrap();
    assert!(log.last_sequence().unwrap() >= high);
    assert_eq!(
        log.at(before + 1).unwrap().unwrap().event.payload["fixture"],
        "retained-legacy-no-effect-workload"
    );
    fs::write(
        fixture.root.join("backlog-produced-frame.json"),
        serde_json::to_vec_pretty(&frame).unwrap(),
    )
    .unwrap();
    fs::write(fixture.root.join("backlog-witness.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "before":before,"retained_high":high,"fresh_message":fresh,"receipt":receipts,"frame_seq":frame["frame_seq"],"compose_ms":compose_ms,"input_ms":input_at.elapsed().as_millis(),"read_mark_unchanged":true
    })).unwrap()).unwrap();
    let capture = fixture.ctl(&[
        "seat",
        "capture",
        "window",
        fixture.root.join("backlog-produced.png").to_str().unwrap(),
    ]);
    assert_eq!(capture["offscreen"], true);
    assert_eq!(capture["stale"], false);
}
