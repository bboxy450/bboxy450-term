//! GUI lifecycle/history against owned private processes, never user custody.
#![allow(
    clippy::disallowed_methods,
    reason = "headless integration fixture owns its runtime"
)]
use bboxy450_term::cockpit::{Config, Session};
use bbt_mux::{Ulid, store::now_ms};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, GuiSeat, ProcessStatus, SeatId, SpawnSpec, SwarmId, Where,
};
use bbt_proto::{
    gui::HistoryRecord,
    registry::{Lifecycle, Position},
};
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

async fn start(root: &Path) -> Session {
    tokio::fs::create_dir_all(root.join("repo")).await.unwrap();
    tokio::fs::create_dir_all(root.join("zsh")).await.unwrap();
    let executable = root.join("app");
    tokio::fs::write(
        &executable,
        format!(
            "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory exec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_bboxy450-term")
        ),
    )
    .await
    .unwrap();
    tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
    Session::start(Config {
        executable,
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository: root.join("repo"),
        shell_config: Some(root.join("zsh")),
        ctl: false,
        test_account: None,
    })
    .await
    .unwrap()
}
fn seat_id(seat: &GuiSeat) -> SeatId {
    SeatId::from_u128(seat.record.id.0)
}
fn pid(seat: &GuiSeat) -> u32 {
    match seat.state.process {
        ProcessStatus::Running { pid } => pid,
        _ => panic!("not running: {seat:?}"),
    }
}
fn position(seat: &GuiSeat) -> Position {
    Position {
        swarm: seat.record.swarm,
        tab: Ulid::new(),
        split_path: vec![1, 0],
        ratios: vec![0.35, 0.65],
        layout_revision: 7,
    }
}
async fn spawn(session: &mut Session, root: &Path) -> GuiSeat {
    let operation = Ulid::new();
    session
        .send(GuiAction::Spawn {
            id: operation,
            spec: SpawnSpec {
                args: Vec::new(),
                env: Vec::new(),
                command: None,
                swarm: SwarmId::from_u128(77),
                harness_kind: "zsh".into(),
                name: Some("lifecycle".into()),
                role: None,
                cwd: root.to_string_lossy().into(),
                location: Where::Local,
                resume: None,
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Spawned { op, seat } if op == operation => {
                    return session
                        .list()
                        .await
                        .unwrap()
                        .into_iter()
                        .find(|item| seat_id(item) == seat)
                        .unwrap();
                }
                GuiReply::Error { op, reason } if op == operation => panic!("spawn: {reason}"),
                _ => (),
            }
        }
    })
    .await
    .unwrap()
}
async fn current(session: &mut Session, seat: SeatId) -> GuiSeat {
    session
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| seat_id(s) == seat)
        .unwrap()
}
async fn error(session: &mut Session, operation: Ulid) -> String {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let GuiReply::Error { op, reason } = session.receive().await.unwrap() {
                assert_eq!(op, operation);
                return reason;
            }
        }
    })
    .await
    .unwrap()
}
async fn input(session: &mut Session, seat: &GuiSeat, bytes: Vec<u8>) {
    let id = Ulid::new();
    session
        .send(GuiAction::Input {
            seat: seat_id(seat),
            generation: seat.record.generation,
            id,
            bytes,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Ok { op } if op == id => break,
                GuiReply::Error { op, reason } if op == id => panic!("input: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
}
async fn history(
    session: &mut Session,
    seat: SeatId,
    from: u64,
    to: u64,
) -> (Vec<HistoryRecord>, Option<u64>) {
    session
        .send(GuiAction::GetHistory { seat, from, to })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::History {
                    seat: got,
                    records,
                    next,
                } => {
                    assert_eq!(got, seat);
                    return (records, next);
                }
                GuiReply::Error { reason, .. } => panic!("history: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}
struct Observer {
    read: BufReader<OwnedReadHalf>,
    write: OwnedWriteHalf,
}
impl Observer {
    async fn connect(session: &Session) -> Self {
        let stream = UnixStream::connect(session.endpoint.root.join("gui.sock"))
            .await
            .unwrap();
        let (read, mut write) = stream.into_split();
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: session.endpoint.capability,
            action: GuiAction::ListSeats,
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        let mut this = Self {
            read: BufReader::new(read),
            write,
        };
        loop {
            if matches!(this.reply().await, GuiReply::Seats(_)) {
                break;
            }
        }
        // Commands use the Session connection; this observer is the one
        // device-bound state drain, not a second consumer beside it.
        let id = Ulid::new();
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: session.endpoint.capability,
            action: GuiAction::AttachEvents { id },
        })
        .unwrap();
        bytes.push(b'\n');
        this.write.write_all(&bytes).await.unwrap();
        loop {
            match this.reply().await {
                GuiReply::Ok { op } if op == id => return this,
                GuiReply::Error { op, reason } if op == id => panic!("observer attach: {reason}"),
                _ => (),
            }
        }
    }
    async fn reply(&mut self) -> GuiReply {
        loop {
            let bytes = tokio::time::timeout(
                Duration::from_secs(10),
                bbt_mux::seat_socket::read_line(&mut self.read),
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap();
            match serde_json::from_slice(&bytes).unwrap() {
                GuiReply::Event { envelope, .. } => match envelope.payload {
                    bbt_proto::events::GuiEventPayload::Seat { seat } => {
                        return GuiReply::Seat(*seat);
                    }
                    _ => continue,
                },
                reply => return reply,
            }
        }
    }
    async fn state(&mut self, seat: SeatId, lifecycle: Lifecycle) -> GuiSeat {
        loop {
            match self.reply().await {
                GuiReply::Seat(s) if seat_id(&s) == seat && s.record.lifecycle == lifecycle => {
                    return s;
                }
                GuiReply::History { .. } => panic!("unrequested history push"),
                _ => {}
            }
        }
    }
}

async fn gui_close_retry_and_undo_publish_unwatched_state_and_keep_the_pty_and_position() {
    let root = tempfile::Builder::new()
        .prefix("c1c-close-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let original = spawn(&mut session, root.path()).await;
    let seat = seat_id(&original);
    let old_pid = pid(&original);
    let mut observer = Observer::connect(&session).await;
    let pos = position(&original);
    let operation = Ulid::new();
    let before = now_ms();
    session
        .send(GuiAction::ScheduleClose {
            seat,
            id: operation,
            position: pos.clone(),
        })
        .await
        .unwrap();
    let closed = current(&mut session, seat).await;
    assert_eq!(closed.record.lifecycle, Lifecycle::ClosedGrace);
    let close = closed.record.close.clone().unwrap();
    assert_eq!(close.operation, operation);
    assert_eq!(close.position, pos);
    assert!((before + 59_000..=now_ms() + 60_000).contains(&close.deadline_ms));
    assert!(
        matches!(closed.state.process,ProcessStatus::ClosedGrace{until_ms} if until_ms==close.deadline_ms)
    );
    assert_eq!(
        observer
            .state(seat, Lifecycle::ClosedGrace)
            .await
            .record
            .close,
        Some(close.clone())
    );
    session
        .send(GuiAction::ScheduleClose {
            seat,
            id: operation,
            position: pos.clone(),
        })
        .await
        .unwrap();
    assert_eq!(
        current(&mut session, seat).await.record.close,
        Some(close.clone()),
        "close retry reset the grace"
    );
    input(
        &mut session,
        &original,
        b"printf '\\nclosed-capture\\n'\n".to_vec(),
    )
    .await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let (entries, _) = history(&mut session, seat, 1, u64::MAX).await;
            let bytes: Vec<u8> = entries
                .into_iter()
                .filter_map(|e| match e {
                    HistoryRecord::Bytes { bytes, .. } => Some(bytes),
                    _ => None,
                })
                .flatten()
                .collect();
            if bytes
                .windows(b"\r\nclosed-capture\r\n".len())
                .any(|w| w == b"\r\nclosed-capture\r\n")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let wrong = Ulid::new();
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: wrong,
            layout_revision: 7,
        })
        .await
        .unwrap();
    assert!(
        error(&mut session, wrong)
            .await
            .contains("stale close operation")
    );
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: operation,
            layout_revision: 8,
        })
        .await
        .unwrap();
    assert!(
        error(&mut session, operation)
            .await
            .contains("layout/close revision conflict")
    );
    assert_eq!(
        current(&mut session, seat).await.record.lifecycle,
        Lifecycle::ClosedGrace
    );
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: operation,
            layout_revision: 7,
        })
        .await
        .unwrap();
    let restored = current(&mut session, seat).await;
    assert_eq!(restored.record.lifecycle, Lifecycle::Open);
    assert_eq!(pid(&restored), old_pid);
    assert_eq!(restored.record.generation, original.record.generation);
    assert_eq!(restored.record.close.as_ref().unwrap().position, pos);
    assert_eq!(
        observer
            .state(seat, Lifecycle::Open)
            .await
            .record
            .lifecycle_revision,
        restored.record.lifecycle_revision
    );
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: operation,
            layout_revision: 7,
        })
        .await
        .unwrap();
    let retry = current(&mut session, seat).await;
    assert_eq!(
        retry.record.lifecycle_revision,
        restored.record.lifecycle_revision
    );
    assert_eq!(pid(&retry), old_pid);
    observer.write.shutdown().await.unwrap();
    drop(observer);
    // The previously accepted undo is idempotent only for that process. A
    // supervisor replacement must not reuse its restoration handle.
    session.supervisor.start_kill().unwrap();
    let replacement = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let GuiReply::Seats(seats) = session.receive().await.unwrap() {
                break seats.into_iter().find(|s| seat_id(s) == seat).unwrap();
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        replacement.record.generation,
        original.record.generation + 1
    );
    assert_ne!(pid(&replacement), old_pid);
    assert!(replacement.record.close.is_none());
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: operation,
            layout_revision: 7,
        })
        .await
        .unwrap();
    assert!(
        error(&mut session, operation)
            .await
            .contains("stale close operation")
    );
    assert_eq!(pid(&current(&mut session, seat).await), pid(&replacement));
    session.shutdown().await.unwrap();
    println!(
        "close/undo: default60s, retry stable deadline, unwatched state publication, capture during grace, exact Position, stale/layout conflicts, same PID {old_pid} and generation for idempotent undo; replaced-generation handle rejected"
    );
}

async fn gui_history_is_lazy_bounded_capture_pages_with_ordered_gaps_and_control_cursors() {
    let root = tempfile::Builder::new()
        .prefix("c1c-history-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let original = spawn(&mut session, root.path()).await;
    let seat = seat_id(&original);
    session
        .send(GuiAction::Resize {
            seat,
            generation: original.record.generation,
            cols: 100,
            rows: 30,
        })
        .await
        .unwrap();
    input(&mut session,&original,b"printf '\\nPAGE-START\\n'; python3 -c 'import os; os.write(1,bytes(range(32,127))*2048)'; printf '\\nPAGE-END\\n'; exit\n".to_vec()).await;
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if matches!(
                current(&mut session, seat).await.state.process,
                ProcessStatus::Exited { .. }
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let mut from = 0;
    let mut sequence = 0;
    let mut pages = 0;
    let mut output = Vec::new();
    let mut control_cursors = 0;
    let mut prefix = false;
    let mut suffix = false;
    loop {
        let (entries, next) = history(&mut session, seat, from, u64::MAX).await;
        pages += 1;
        assert!(
            serde_json::to_vec(&GuiReply::History {
                seat,
                records: entries.clone(),
                next
            })
            .unwrap()
            .len()
                < 1024 * 1024
        );
        for entry in entries {
            match entry {
                HistoryRecord::Gap {
                    from: a,
                    to: b,
                    reason,
                    ..
                } => {
                    assert!(!reason.is_empty());
                    if a == 0 {
                        assert_eq!(b, 0);
                        assert!(!prefix);
                        assert_eq!(sequence, 0);
                        prefix = true;
                    } else {
                        assert!(next.is_none());
                        assert_eq!(a, sequence + 1);
                        assert_eq!(b, u64::MAX);
                        assert!(!suffix);
                        suffix = true;
                    }
                }
                HistoryRecord::Bytes { sequence: s, bytes } => {
                    assert!(!suffix);
                    assert_eq!(s, sequence + 1);
                    sequence = s;
                    if bytes.is_empty() {
                        control_cursors += 1;
                    }
                    output.extend(bytes);
                }
            }
        }
        match next {
            Some(n) => {
                assert!(n > from);
                assert_eq!(n, sequence + 1);
                assert!(!suffix);
                from = n;
            }
            None => break,
        }
    }
    assert!(pages > 1);
    assert!(prefix && suffix);
    assert_eq!(
        control_cursors, 2,
        "resize and exit must keep their capture cursor slots"
    );
    let begin = b"\r\nPAGE-START\r\n";
    let end = b"\r\nPAGE-END\r\n";
    let a = output
        .windows(begin.len())
        .position(|w| w == begin)
        .unwrap()
        + begin.len();
    let b = output[a..]
        .windows(end.len())
        .position(|w| w == end)
        .unwrap()
        + a;
    let expected: Vec<u8> = (32u8..127).cycle().take(95 * 2048).collect();
    assert_eq!(&output[a..b], expected);
    session
        .send(GuiAction::GetHistory {
            seat,
            from: 5,
            to: 4,
        })
        .await
        .unwrap();
    loop {
        if let GuiReply::Error { reason, .. } = session.receive().await.unwrap() {
            assert!(reason.contains("invalid history range"));
            break;
        }
    }
    assert_eq!(
        current(&mut session, seat).await.record.generation,
        original.record.generation
    );
    session.shutdown().await.unwrap();
    println!(
        "history: {pages} bounded pages; exact 194560 output bytes; capture sequences1..{sequence}, prefix/suffix gaps ordered, resize+exit cursor slots retained, invalid range named without disconnect"
    );
}

async fn gui_close_expires_without_logic_and_relaunch_does_not_reset_grace_or_resume_it() {
    let root = tempfile::Builder::new()
        .prefix("c1c-expiry-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let original = spawn(&mut session, root.path()).await;
    let seat = seat_id(&original);
    let old_pid = pid(&original);
    let operation = Ulid::new();
    session
        .send(GuiAction::ScheduleClose {
            seat,
            id: operation,
            position: position(&original),
        })
        .await
        .unwrap();
    let close = current(&mut session, seat).await.record.close.unwrap();
    let supervisor = session.supervisor.id();
    input(
        &mut session,
        &original,
        b"for i in 1 2 3; do printf 'grace-output-%s\\n' $i; sleep 1; done\n".to_vec(),
    )
    .await;
    session.logic.start_kill().unwrap();
    session.logic.wait().await.unwrap();
    let delay = close
        .deadline_ms
        .saturating_add(500)
        .saturating_sub(now_ms());
    tokio::time::sleep(Duration::from_millis(delay)).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let p = tokio::process::Command::new("/bin/ps")
                .args(["-p", &old_pid.to_string(), "-o", "pid="])
                .output()
                .await
                .unwrap();
            if !p.status.success() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let retained = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let GuiReply::Seats(seats) = session.receive().await.unwrap() {
                break seats.into_iter().find(|s| seat_id(s) == seat).unwrap();
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(session.supervisor.id(), supervisor);
    assert_eq!(retained.record.generation, original.record.generation);
    assert_eq!(retained.record.lifecycle, Lifecycle::ClosedRetained);
    assert_eq!(retained.record.close, Some(close));
    assert!(matches!(retained.state.process, ProcessStatus::Retained));
    session
        .send(GuiAction::UndoClose {
            seat,
            close_id: operation,
            layout_revision: 7,
        })
        .await
        .unwrap();
    let reason = error(&mut session, operation).await;
    assert!(
        reason.contains("exited") || reason.contains("expired"),
        "{reason}"
    );
    let (entries, _) = history(&mut session, seat, 1, u64::MAX).await;
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, HistoryRecord::Bytes { bytes, .. } if bytes.is_empty()))
    );
    let bytes: Vec<u8> = entries
        .into_iter()
        .filter_map(|e| match e {
            HistoryRecord::Bytes { bytes, .. } => Some(bytes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        bytes
            .windows(b"grace-output-3".len())
            .any(|w| w == b"grace-output-3")
    );
    session.shutdown().await.unwrap();
    println!(
        "expiry: original60s deadline survived logicSIGKILL, supervisor reaped PID {old_pid}, no new generation/process, retained history and explicit expired undo"
    );
}

// keeps: invariant restart-keeps-state (seat/close history boundary, not full RS1)
// keeps: spec c1-live-seats §4 (all original GUI lifecycle/history assertions)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "night-only real process boundary; scripts/night-test-boundaries.sh"]
async fn lifecycle_real_process_boundary() {
    gui_close_retry_and_undo_publish_unwatched_state_and_keep_the_pty_and_position().await;
    gui_history_is_lazy_bounded_capture_pages_with_ordered_gaps_and_control_cursors().await;
    gui_close_expires_without_logic_and_relaunch_does_not_reset_grace_or_resume_it().await;
}
