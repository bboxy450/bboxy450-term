// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use super::*;
use crate::knowledge::{WriteContext, data_source as d};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatId};
use serde_json::{Value, json};
use std::sync::{Condvar, Mutex};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{
    UnixStream,
    unix::{OwnedReadHalf, OwnedWriteHalf},
};

#[derive(Default)]
struct Held {
    entered: tokio::sync::Notify,
    calls: std::sync::atomic::AtomicUsize,
    active: std::sync::atomic::AtomicUsize,
    max_active: std::sync::atomic::AtomicUsize,
    released: Mutex<bool>,
    wake: Condvar,
}
impl Held {
    async fn entered(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.calls.load(Ordering::Acquire) < count {
                self.entered.notified().await;
            }
        })
        .await
        .unwrap();
    }
    fn block(&self) {
        self.max_active.fetch_max(
            self.active.fetch_add(1, Ordering::AcqRel) + 1,
            Ordering::AcqRel,
        );
        self.calls.fetch_add(1, Ordering::Release);
        self.entered.notify_one();
        let guard = self.released.lock().unwrap();
        drop(self.wake.wait_while(guard, |released| !*released).unwrap());
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
struct Release(Arc<Held>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
impl DataSource for Held {
    fn list(&self, request: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        if request.limit != Some(2) {
            self.block();
        }
        Ok(d::Page {
            items: vec![],
            next: None,
        })
    }
    fn resolve(&self, _: d::ResolveRequest) -> d::Result<d::RecordRef> {
        unreachable!()
    }
    fn fetch(&self, _: d::FetchRequest) -> d::Result<d::Record> {
        unreachable!()
    }
    fn search(&self, _: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        unreachable!()
    }
    fn links(&self, _: d::FetchRequest) -> d::Result<d::Links> {
        unreachable!()
    }
    fn graph(&self, _: d::GraphRequest) -> d::Result<d::GraphPage> {
        unreachable!()
    }
    fn write(&self, _: d::WriteRequest, _: &WriteContext<'_>) -> d::Result<d::WriteResult> {
        unreachable!()
    }
    fn changes_since(&self, _: d::ChangesRequest) -> d::Result<d::Changes> {
        unreachable!()
    }
}

struct Gui {
    read: BufReader<OwnedReadHalf>,
    write: OwnedWriteHalf,
    capability: [u8; 32],
}
impl Gui {
    async fn connect(endpoint: &EndpointInfo) -> Self {
        let (read, write) = UnixStream::connect(&endpoint.path)
            .await
            .unwrap()
            .into_split();
        let mut gui = Self {
            read: BufReader::new(read),
            write,
            capability: endpoint.capability,
        };
        gui.send(GuiAction::ListSeats).await;
        while !matches!(gui.next().await, GuiReply::Seats(_)) {}
        gui
    }
    async fn send(&mut self, action: GuiAction) {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        self.write.write_all(&bytes).await.unwrap();
    }
    async fn next(&mut self) -> GuiReply {
        let bytes = crate::seat_socket::read_line(&mut self.read)
            .await
            .unwrap()
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn held_read(same_session: bool, invalidate: bool) {
    let root = tempfile::tempdir().unwrap();
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.path().join("supervisor-journal")),
        ..Default::default()
    })
    .unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.path().join("run"),
            repository: root.path().join("repo"),
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[37; 32]),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let seat = Ulid::new();
    let swarm = Ulid::new();
    let mut launch = LaunchSpec::new("/bin/zsh", root.path());
    launch.seat = seat;
    launch.args = vec![
        "-f".into(),
        "-c".into(),
        "while IFS= read -r line; do print -r -- HELD-ECHO:$line; done".into(),
    ];
    logic
        .client
        .spawn(SeatRecord::new(seat, swarm, "held read", "zsh"), launch)
        .await
        .unwrap();
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let mut gui = Gui::connect(&endpoint).await;
    let mut other = if same_session {
        None
    } else {
        Some(Gui::connect(&endpoint).await)
    };
    let held = Arc::new(Held::default());
    let release = Release(held.clone());
    let (tx, rx) = oneshot::channel();
    logic
        .client
        .tx
        .send(Command::KnowledgeFixture(held.clone(), tx))
        .await
        .unwrap();
    rx.await.unwrap();
    let read_id = Ulid::new();
    gui.send(GuiAction::Knowledge {
        id: read_id,
        method: "list".into(),
        args: json!({"limit":1}),
        scope: invalidate.then_some(bbt_proto::gui::SwarmScope { swarm, revision: 0 }),
    })
    .await;
    held.entered(1).await;
    if invalidate {
        let next = root.path().join("next-project");
        std::fs::create_dir(&next).unwrap();
        let registry = Registry::new(
            SqliteStore::open(&object_dir(&root.path().join("run"), "account").unwrap()).unwrap(),
        );
        registry
            .update_swarm_workdir(swarm, Some(next.to_str().unwrap()))
            .unwrap();
    }
    let control = other.as_mut().unwrap_or(&mut gui);
    let input_id = Ulid::new();
    let seat_id = SeatId::from_u128(seat.0);
    let started = Instant::now();
    control
        .send(GuiAction::Input {
            seat: seat_id,
            generation: 1,
            id: input_id,
            bytes: b"responsive\n".to_vec(),
        })
        .await;
    control
        .send(GuiAction::Resize {
            seat: seat_id,
            generation: 1,
            cols: 66,
            rows: 22,
        })
        .await;
    let mut replies = 0;
    let mut control_latencies = vec![];
    let responsive = tokio::time::timeout(Duration::from_millis(500), async {
        while replies < 2 {
            match control.next().await {
                GuiReply::Ok { op } => {
                    control_latencies.push((
                        if op == input_id { "input" } else { "resize" },
                        started.elapsed(),
                    ));
                    replies += 1;
                }
                GuiReply::Error { reason, .. } => panic!("control refused: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .is_ok();
    let latency = started.elapsed();
    let second_id = Ulid::new();
    if responsive {
        // Check the actual PTY output/grid while the provider is still held.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let snapshot = logic.client.snapshot(seat, 1).await.unwrap();
                let text: String = snapshot
                    .screen
                    .cells
                    .iter()
                    .flatten()
                    .map(|cell| cell.c)
                    .collect();
                if (snapshot.screen.cols, snapshot.screen.rows) == (66, 22)
                    && text.contains("HELD-ECHO:responsive")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        if !invalidate && same_session {
            gui.send(GuiAction::Knowledge {
                scope: None,
                id: second_id,
                method: "list".into(),
                args: json!({}),
            })
            .await;
            held.entered(2).await;
            let refused = Ulid::new();
            gui.send(GuiAction::Knowledge {
                scope: None,
                id: refused,
                method: "list".into(),
                args: json!({}),
            })
            .await;
            let GuiReply::Knowledge { op, response } =
                tokio::time::timeout(Duration::from_millis(500), gui.next())
                    .await
                    .unwrap()
            else {
                panic!("expected bounded read refusal")
            };
            assert_eq!(op, refused);
            assert_eq!(
                serde_json::from_value::<d::Result<Value>>(response)
                    .unwrap()
                    .unwrap_err()
                    .code,
                "provider_busy"
            );
        } else if !invalidate {
            let client = logic.client.clone();
            let second =
                tokio::spawn(async move { client.gui_knowledge("list".into(), json!({})).await });
            held.entered(2).await;
            second.abort();
            assert!(second.await.unwrap_err().is_cancelled());
            // Cancelling the waiter must not open a slot for a still-running blocking read.
            let busy = tokio::time::timeout(
                Duration::from_millis(500),
                logic.client.gui_knowledge("list".into(), json!({})),
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
            assert_eq!(busy.code, "provider_busy");
            assert!(busy.retry_after_ms.is_some());
        }
        assert_eq!(
            held.max_active.load(Ordering::Acquire),
            if invalidate { 1 } else { 2 }
        );
    }
    held.release();
    // Drain the read before teardown even when the pre-fix responsiveness assertion fails.
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut pending = if same_session && responsive && !invalidate {
            vec![read_id, second_id]
        } else {
            vec![read_id]
        };
        while !pending.is_empty() {
            if let GuiReply::Knowledge { op, response } = gui.next().await {
                let at = pending
                    .iter()
                    .position(|id| *id == op)
                    .expect("original operation ID retained");
                pending.remove(at);
                let response = serde_json::from_value::<d::Result<Value>>(response).unwrap();
                if invalidate {
                    assert_eq!(response.unwrap_err().code, "swarm_root_changed");
                } else {
                    assert!(response.is_ok());
                }
            }
        }
    })
    .await
    .unwrap();
    if !responsive {
        let control = other.as_mut().unwrap_or(&mut gui);
        tokio::time::timeout(Duration::from_secs(5), async {
            while replies < 2 {
                if matches!(control.next().await, GuiReply::Ok { .. }) {
                    replies += 1;
                }
            }
        })
        .await
        .unwrap();
    }
    let snapshot = logic.client.snapshot(seat, 1).await.unwrap();
    assert_eq!((snapshot.screen.cols, snapshot.screen.rows), (66, 22));
    drop(release);
    drop(other);
    drop(gui);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
    eprintln!(
        "held provider: same_session={same_session} Input+Resize={latency:?} controls={control_latencies:?} completed_while_held={responsive}; max_provider_reads={}",
        held.max_active.load(Ordering::Acquire)
    );
    assert!(
        responsive,
        "Input/Resize stalled behind a held provider: same_session={same_session}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_read_does_not_stall_other_gui_seat_input_or_resize() {
    held_read(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_read_does_not_stall_same_gui_seat_input_or_resize() {
    held_read(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_read_completion_is_fenced_while_input_and_resize_remain_live() {
    held_read(true, true).await;
}

// keeps: one slow root cannot hold the operator's terminal/chat or another root.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_held_scan_keeps_input_chat_and_other_root_live_and_excludes_writes() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    for (path, label) in [(&first, "first"), (&second, "second")] {
        std::fs::create_dir_all(path.join(".bboxy/notes")).unwrap();
        std::fs::write(
            path.join(".bboxy/notes/same.md"),
            format!(
                "---\nid: \"{}\"\nnumber: 1\ntype: note\ntitle: Same\n---\n{label} content\n",
                Ulid(1)
            ),
        )
        .unwrap();
    }
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.path().join("supervisor")),
        ..Default::default()
    })
    .unwrap();
    let run = root.path().join("run");
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: first.clone(),
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[47; 32]),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let seat = Ulid::new();
    let swarm = Ulid::new();
    let other = Ulid::new();
    let mut launch = LaunchSpec::new("/bin/zsh", root.path());
    launch.seat = seat;
    launch.args = vec![
        "-f".into(),
        "-c".into(),
        "while IFS= read -r line; do print -r -- SCAN-ECHO:$line; done".into(),
    ];
    logic
        .client
        .spawn(SeatRecord::new(seat, swarm, "scan fixture", "zsh"), launch)
        .await
        .unwrap();
    let registry = Registry::new(SqliteStore::open(&object_dir(&run, "account").unwrap()).unwrap());
    let second_scope = registry
        .update_swarm_workdir(other, Some(second.to_str().unwrap()))
        .unwrap();
    let held = Arc::new(Held::default());
    let release = Release(held.clone());
    let first_id =
        crate::knowledge::roots::root_identity(&first.join(".bboxy").canonicalize().unwrap())
            .unwrap();
    let hold = held.clone();
    let selected = first_id.clone();
    let hook = Arc::new(move |root: &str| {
        if root == selected {
            hold.block();
        }
    });
    let (tx, rx) = oneshot::channel();
    logic
        .client
        .tx
        .send(Command::WorkspaceScanFixture(
            Some(bbt_proto::gui::SwarmScope { swarm, revision: 0 }),
            hook,
            tx,
        ))
        .await
        .unwrap();
    assert_eq!(rx.await.unwrap().unwrap().root_id, first_id);
    held.entered(1).await;
    let mut gui = Gui::connect(&logic.client.gui_endpoint().await.unwrap()).await;
    let input = Ulid::new();
    gui.send(GuiAction::Input {
        seat: SeatId::from_u128(seat.0),
        generation: 1,
        id: input,
        bytes: b"live\n".to_vec(),
    })
    .await;
    gui.send(GuiAction::Resize {
        seat: SeatId::from_u128(seat.0),
        generation: 1,
        cols: 72,
        rows: 24,
    })
    .await;
    tokio::time::timeout(Duration::from_millis(500), async {
        let mut replies = 0;
        while replies < 2 {
            if matches!(gui.next().await, GuiReply::Ok { .. }) {
                replies += 1;
            }
        }
    })
    .await
    .unwrap();
    let message = crate::messaging::api::Request::Send {
        message: crate::messaging::wire::SendRequest::text(
            format!("seats/{seat}/inbox"),
            "chat during scan",
        ),
    };
    tokio::time::timeout(
        Duration::from_millis(500),
        logic.client.messaging(seat, 1, message),
    )
    .await
    .unwrap()
    .unwrap();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match logic
                .client
                .gui_knowledge_scoped(
                    "resolve".into(),
                    json!({"locator":"#1"}),
                    Some(bbt_proto::gui::SwarmScope {
                        swarm: other,
                        revision: second_scope.revision,
                    }),
                )
                .await
                .unwrap()
            {
                Ok(value) => break value,
                Err(error) if error.code == "workspace_pending" => {
                    tokio::time::sleep(Duration::from_millis(20)).await
                }
                Err(error) => panic!("{error}"),
            }
        }
    })
    .await
    .unwrap();
    assert_ne!(read["knowledge_root_id"], first_id);
    let reference: d::RecordRef = serde_json::from_value(read).unwrap();
    let data = logic
        .client
        .gui_knowledge_scoped(
            "fetch".into(),
            json!({"ref":reference}),
            Some(bbt_proto::gui::SwarmScope {
                swarm: other,
                revision: second_scope.revision,
            }),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(data["body"], "second content\n");
    let call = crate::mcp::Call {
        op_id: Ulid::new(),
        tool: "knowledge_write".into(),
        args: json!({"path":"notes/after-scan.md","frontmatter":{"type":"note","title":"After scan"},"body":"only first root"}),
    };
    let refused = tokio::time::timeout(
        Duration::from_millis(500),
        logic.client.control_call(seat, 1, call.clone()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "workspace_pending"
    );
    assert!(!first.join(".bboxy/notes/after-scan.md").exists());
    let store = SqliteStore::open(&object_dir(&run, &format!("seat-{seat}")).unwrap()).unwrap();
    assert!(
        store
            .get(&format!("control/op/{}", call.op_id))
            .unwrap()
            .is_none(),
        "transient scan refusal admitted effect"
    );
    held.release();
    let applied = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let value = logic
                .client
                .control_call(seat, 1, call.clone())
                .await
                .unwrap();
            if value["structuredContent"]["error"]["code"] == "workspace_pending" {
                tokio::time::sleep(Duration::from_millis(20)).await;
                continue;
            }
            break value;
        }
    })
    .await
    .unwrap();
    assert_ne!(applied["isError"], true, "{applied}");
    assert!(first.join(".bboxy/notes/after-scan.md").is_file());
    assert!(!second.join(".bboxy/notes/after-scan.md").exists());
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let snapshot = logic.client.snapshot(seat, 1).await.unwrap();
            let text: String = snapshot
                .screen
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.c)
                .collect();
            if text.contains("SCAN-ECHO:live") && snapshot.screen.cols == 72 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    drop(release);
    drop(gui);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

// keeps: two slow reads in one captured workspace cannot exhaust another's socket admission.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_shared_socket_two_held_reads_leave_other_scope_capacity_and_release_slots() {
    let root = tempfile::tempdir().unwrap();
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.path().join("supervisor")),
        ..Default::default()
    })
    .unwrap();
    let run = root.path().join("run");
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: root.path().join("first"),
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[48; 32]),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let first = bbt_proto::gui::SwarmScope {
        swarm: Ulid::new(),
        revision: 0,
    };
    let second_dir = root.path().join("second");
    std::fs::create_dir(&second_dir).unwrap();
    let registry = Registry::new(SqliteStore::open(&object_dir(&run, "account").unwrap()).unwrap());
    let second_record = registry
        .update_swarm_workdir(Ulid::new(), Some(second_dir.to_str().unwrap()))
        .unwrap();
    let second = bbt_proto::gui::SwarmScope {
        swarm: second_record.id,
        revision: second_record.revision,
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match logic
                .client
                .gui_knowledge_scoped("list".into(), json!({"limit":2}), Some(second))
                .await
                .unwrap()
            {
                Ok(_) => break,
                Err(error) if error.code == "workspace_pending" => {
                    tokio::time::sleep(Duration::from_millis(20)).await
                }
                Err(error) => panic!("{error}"),
            }
        }
    })
    .await
    .unwrap();
    let held = Arc::new(Held::default());
    let release = Release(held.clone());
    let (tx, rx) = oneshot::channel();
    logic
        .client
        .tx
        .send(Command::KnowledgeFixture(held.clone(), tx))
        .await
        .unwrap();
    rx.await.unwrap();
    let mut gui = Gui::connect(&logic.client.gui_endpoint().await.unwrap()).await;
    let a = Ulid::new();
    let b = Ulid::new();
    for id in [a, b] {
        gui.send(GuiAction::Knowledge {
            id,
            scope: Some(first),
            method: "list".into(),
            args: json!({"limit":1}),
        })
        .await;
    }
    held.entered(2).await;
    let refused = Ulid::new();
    gui.send(GuiAction::Knowledge {
        id: refused,
        scope: Some(first),
        method: "list".into(),
        args: json!({"limit":1}),
    })
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let GuiReply::Knowledge { op, response } = gui.next().await {
                assert_eq!(op, refused);
                let error = serde_json::from_value::<d::Result<Value>>(response)
                    .unwrap()
                    .unwrap_err();
                assert_eq!(error.code, "provider_busy");
                assert_eq!(error.retry_after_ms, Some(100));
                break;
            }
        }
    })
    .await
    .unwrap();
    let ready = Ulid::new();
    gui.send(GuiAction::Knowledge {
        id: ready,
        scope: Some(second),
        method: "list".into(),
        args: json!({"limit":2}),
    })
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let GuiReply::Knowledge { op, response } = gui.next().await {
                assert_eq!(op, ready);
                let value = serde_json::from_value::<d::Result<Value>>(response)
                    .unwrap()
                    .unwrap();
                assert_eq!(value["swarm_workspace_id"], json!(second.swarm));
                break;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        held.calls.load(Ordering::Acquire),
        2,
        "readyB waited for or entered heldA provider work"
    );
    assert_eq!(held.active.load(Ordering::Acquire), 2);
    held.release();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut pending = vec![a, b];
        while !pending.is_empty() {
            if let GuiReply::Knowledge { op, response } = gui.next().await {
                assert!(
                    serde_json::from_value::<d::Result<Value>>(response)
                        .unwrap()
                        .is_ok()
                );
                let at = pending.iter().position(|id| *id == op).unwrap();
                pending.remove(at);
            }
        }
    })
    .await
    .unwrap();
    let after = Ulid::new();
    gui.send(GuiAction::Knowledge {
        id: after,
        scope: Some(first),
        method: "list".into(),
        args: json!({"limit":1}),
    })
    .await;
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let GuiReply::Knowledge { op, response } = gui.next().await {
                assert_eq!(op, after);
                assert!(
                    serde_json::from_value::<d::Result<Value>>(response)
                        .unwrap()
                        .is_ok()
                );
                break;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(held.calls.load(Ordering::Acquire), 3);
    drop(release);
    drop(gui);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
