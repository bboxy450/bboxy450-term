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
    fn list(&self, _: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        self.max_active.fetch_max(
            self.active.fetch_add(1, Ordering::AcqRel) + 1,
            Ordering::AcqRel,
        );
        self.calls.fetch_add(1, Ordering::Release);
        self.entered.notify_one();
        let guard = self.released.lock().unwrap();
        drop(self.wake.wait_while(guard, |released| !*released).unwrap());
        self.active.fetch_sub(1, Ordering::AcqRel);
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

async fn held_read(same_session: bool) {
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
    let mut launch = LaunchSpec::new("/bin/zsh", root.path());
    launch.seat = seat;
    launch.args = vec![
        "-f".into(),
        "-c".into(),
        "while IFS= read -r line; do print -r -- HELD-ECHO:$line; done".into(),
    ];
    logic
        .client
        .spawn(
            SeatRecord::new(seat, Ulid::new(), "held read", "zsh"),
            launch,
        )
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
        scope: None,
        id: read_id,
        method: "list".into(),
        args: json!({"limit":1}),
    })
    .await;
    held.entered(1).await;
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
        if same_session {
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
        } else {
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
        assert_eq!(held.max_active.load(Ordering::Acquire), 2);
    }
    held.release();
    // Drain the read before teardown even when the pre-fix responsiveness assertion fails.
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut pending = if same_session && responsive {
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
                assert!(
                    serde_json::from_value::<d::Result<Value>>(response)
                        .unwrap()
                        .is_ok()
                );
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
    held_read(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_read_does_not_stall_same_gui_seat_input_or_resize() {
    held_read(true).await;
}
