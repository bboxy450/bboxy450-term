// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real capture, with notifications held until the GUI's watch is admitted.
use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::ring::Playback;
use bbt_mux::streams::{Replica, Screen};
use bbt_mux::{Action, Client, Event, LaunchSpec, Reply, Supervisor, Ulid};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatId};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufReader};

#[derive(Default)]
struct State {
    release: bool,
    limit: usize,
    pages: Vec<Ulid>,
    notified: BTreeMap<Ulid, u64>,
}
struct HeldCapture {
    client: Client,
    order: Vec<Ulid>,
    shared: Arc<Mutex<State>>,
    latest: BTreeMap<Ulid, Event>,
    ready: VecDeque<Event>,
}
impl SupervisorConnection for HeldCapture {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        if let Action::ReadRingBatch { reads, max_bytes } = action {
            let available = {
                let state = self.shared.lock().unwrap();
                if state.release {
                    state.limit.saturating_sub(state.pages.len())
                } else {
                    usize::MAX
                }
            };
            let admitted = reads.len().min(available);
            let mut results = if admitted == 0 {
                vec![]
            } else {
                let Reply::RingHistoryBatch { results } =
                    self.client.request(Action::ReadRingBatch {
                        reads: reads[..admitted].to_vec(),
                        max_bytes,
                    })?
                else {
                    panic!("batch reply required")
                };
                results
            };
            results.extend(reads[admitted..].iter().map(|read| {
                bbt_supervisor::protocol::RingReadResult {
                    seat: read.attachment.seat,
                    generation: read.attachment.generation,
                    from: read.from,
                    to: read.to,
                    result: Err(bbt_mux::Failure {
                        code: bbt_mux::FailureCode::Busy,
                        message: "fixture replay page budget exhausted".into(),
                    }),
                }
            }));
            let mut state = self.shared.lock().unwrap();
            if state.release {
                state.pages.extend(
                    results
                        .iter()
                        .filter(|row| {
                            row.result
                                .as_ref()
                                .is_ok_and(|history| !history.records.is_empty())
                        })
                        .map(|row| row.seat),
                );
                assert!(
                    state.pages.len() <= state.limit,
                    "batch exceeded the leaf page budget"
                );
            }
            return Ok(Reply::RingHistoryBatch { results });
        }
        let seat = match &action {
            Action::ReadRing { attachment, .. } => Some(attachment.seat),
            _ => None,
        };
        if seat.is_some() {
            let state = self.shared.lock().unwrap();
            if state.release && state.pages.len() >= state.limit {
                return Err(Box::new(bbt_mux::Failure {
                    code: bbt_mux::FailureCode::Busy,
                    message: "fixture replay page budget exhausted".into(),
                }));
            }
        }
        let reply = self.client.request(action)?;
        if let Some(seat) = seat
            && matches!(&reply, Reply::RingHistory { history } if !history.records.is_empty())
        {
            let mut state = self.shared.lock().unwrap();
            if state.release {
                state.pages.push(seat);
            }
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if !self.shared.lock().unwrap().release {
            let mut next = SupervisorConnection::receive(&mut self.client, timeout)?;
            while let Some(event) = next {
                self.shared
                    .lock()
                    .unwrap()
                    .notified
                    .insert(event.seat(), event.sequence().unwrap());
                self.latest.insert(event.seat(), event);
                next = SupervisorConnection::receive(&mut self.client, Duration::ZERO)?;
            }
            return Ok(None);
        }
        if !self.latest.is_empty() {
            for seat in &self.order {
                if let Some(event) = self.latest.remove(seat) {
                    self.ready.push_back(event);
                }
            }
        }
        if let Some(event) = self.ready.pop_front() {
            return Ok(Some(event));
        }
        SupervisorConnection::receive(&mut self.client, timeout)
    }
}
// Release the artificial gate before Logic's drop, including on assertion failure.
struct Release(Arc<Mutex<State>>);
impl Drop for Release {
    fn drop(&mut self) {
        let mut state = self.0.lock().unwrap();
        state.release = true;
        state.limit = usize::MAX;
    }
}
async fn send(
    write: &mut tokio::net::unix::OwnedWriteHalf,
    capability: [u8; 32],
    action: GuiAction,
) {
    let mut bytes = serde_json::to_vec(&GuiRequest {
        version: bbt_proto::gui::VERSION,
        capability,
        action,
    })
    .unwrap();
    bytes.push(b'\n');
    write.write_all(&bytes).await.unwrap();
}
async fn apply(
    read: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    write: &mut tokio::net::unix::OwnedWriteHalf,
    capability: [u8; 32],
    replica: &mut Replica,
) {
    loop {
        let line = tokio::time::timeout(
            Duration::from_secs(60),
            bbt_mux::seat_socket::read_line(read),
        )
        .await
        .expect("GUI made no progress")
        .unwrap()
        .unwrap();
        match serde_json::from_slice::<GuiReply>(&line).unwrap() {
            GuiReply::Update { seat, update } => {
                replica.apply(update).unwrap();
                let snapshot = replica.snapshot.as_ref().unwrap();
                send(
                    write,
                    capability,
                    GuiAction::Applied {
                        seat,
                        view: snapshot.view,
                        sequence: snapshot.sequence,
                    },
                )
                .await;
                return;
            }
            GuiReply::Error { reason, .. } => panic!("GUI rejected fixture: {reason}"),
            _ => {}
        }
    }
}

pub fn watched_tail_with_background_backlogs() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(crate::common::capture_config(temp.path())).unwrap();
    let seats: Vec<_> = (0..4).map(|_| Ulid::new()).collect();
    let watched = seats[0];
    let shared = Arc::new(Mutex::new(State {
        limit: usize::MAX,
        ..Default::default()
    }));
    let logic = Logic::start(
        HeldCapture {
            client: supervisor.connect().unwrap(),
            order: seats.clone(),
            shared: shared.clone(),
            latest: BTreeMap::new(),
            ready: VecDeque::new(),
        },
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[93; 32]),
        runtime.handle(),
    )
    .unwrap();
    let _release = Release(shared.clone());
    let swarm = Ulid::new();
    let mut byte_ends = BTreeMap::new();
    for (index, &seat) in seats.iter().enumerate() {
        // Equal-size streams still leave every background backlogged at the
        // watched tail; actual page counts below enforce the same quota proof.
        // Extra megabytes only repeat capture/parsing work after that proof.
        let count = 512 * 1024;
        let tail = format!("\r\nSEAT-{index}-VISIBLE-TAIL\r\n");
        byte_ends.insert(seat, count + tail.len());
        let mut spec = LaunchSpec::new("python3", temp.path());
        spec.seat = seat;
        spec.args = vec![
            "-c".into(),
            r#"import os,sys,time,tty
tty.setraw(1)
while not os.path.exists('go'): time.sleep(.002)
data=b'x'*int(sys.argv[1])+sys.argv[2].encode()
while data:
 n=os.write(1,data);data=data[n:]
while True: time.sleep(1)
"#
            .into(),
            count.to_string(),
            tail,
        ];
        let info = runtime
            .block_on(logic.client.spawn(
                SeatRecord::new(seat, swarm, format!("priority {index}"), "python"),
                spec,
            ))
            .unwrap();
        println!("owned priority seat={seat} pid={}", info.pid);
    }
    let endpoint = runtime.block_on(logic.client.gui_endpoint()).unwrap();
    let (read, mut write) = runtime
        .block_on(tokio::net::UnixStream::connect(endpoint.path))
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    let mut replica = Replica::default();
    let initial = runtime.block_on(logic.client.snapshot(watched, 1)).unwrap();
    runtime.block_on(send(
        &mut write,
        endpoint.capability,
        GuiAction::Watch {
            seat: SeatId::from_u128(watched.0),
            view: initial.view,
        },
    ));
    runtime.block_on(apply(
        &mut read,
        &mut write,
        endpoint.capability,
        &mut replica,
    ));
    std::fs::write(temp.path().join("go"), []).unwrap();
    let mut tails = BTreeMap::new();
    let mut progress = Instant::now();
    let mut captured = BTreeMap::new();
    while tails.len() < seats.len() {
        for &seat in &seats {
            let status = runtime
                .block_on(logic.client.history(seat, 1, u64::MAX, u64::MAX))
                .unwrap()
                .status;
            let observed = (status.byte_end, status.captured_sequence);
            if captured.insert(seat, observed) != Some(observed) {
                progress = Instant::now();
            }
            if status.byte_end >= byte_ends[&seat] as u64
                && shared
                    .lock()
                    .unwrap()
                    .notified
                    .get(&seat)
                    .is_some_and(|&seq| seq >= status.captured_sequence)
            {
                tails.insert(seat, status.captured_sequence);
            }
        }
        assert!(
            progress.elapsed() < Duration::from_secs(60),
            "finite PTY capture made no progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    // Count actual pages on this captured fixture; no payload/page-size assumption.
    // These read-only probes happen before replay is released and are not counted.
    let mut reference = BTreeMap::new();
    let mut watched_pages = 0;
    for &seat in &seats {
        let mut playback = Playback::new(seat, 1, Default::default());
        let mut pages = 0;
        while playback.sequence < tails[&seat] {
            let history = runtime
                .block_on(
                    logic
                        .client
                        .history(seat, 1, playback.sequence + 1, tails[&seat]),
                )
                .unwrap();
            assert!(history.gap.is_none());
            assert!(!history.records.is_empty());
            for record in history.records {
                playback.apply(&record.event).unwrap();
            }
            pages += 1;
        }
        if seat == watched {
            watched_pages = pages;
        }
        reference.insert(
            seat,
            Screen::from_term(&playback.term, playback.title, playback.cwd, playback.gap),
        );
    }
    assert!(
        watched_pages >= 16,
        "fixture needs enough pages to distinguish quotas"
    );
    let budget = 2 * watched_pages + 16;
    {
        let mut state = shared.lock().unwrap();
        state.limit = budget;
        state.release = true;
    }
    progress = Instant::now();
    let mut previous = 0;
    loop {
        let count = shared.lock().unwrap().pages.len();
        if count == budget {
            break;
        }
        if count != previous {
            previous = count;
            progress = Instant::now();
        }
        assert!(
            progress.elapsed() < Duration::from_secs(60),
            "replay made no progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let snapshot = runtime.block_on(logic.client.snapshot(watched, 1)).unwrap();
    let pages = shared.lock().unwrap().pages.clone();
    println!(
        "watched source pages={watched_pages}, fixed budget={budget}, watched replay pages={}",
        pages.iter().filter(|&&seat| seat == watched).count()
    );
    // Until its final page, the watched lane and all three backgrounds are
    // runnable. Each sliding eight-page window must satisfy the policy floor.
    let final_watched = pages.iter().rposition(|&seat| seat == watched).unwrap();
    assert!(final_watched >= 8);
    for window in pages[..=final_watched].windows(8) {
        assert!(
            window.iter().filter(|&&seat| seat == watched).count() >= 4,
            "watched lane got less than half: {window:?}"
        );
    }
    assert_eq!(
        snapshot.sequence, tails[&watched],
        "watched tail starved behind background replay"
    );
    assert_eq!(snapshot.screen, reference[&watched]);
    for &seat in &seats[1..] {
        assert!(
            runtime
                .block_on(logic.client.snapshot(seat, 1))
                .unwrap()
                .sequence
                < tails[&seat],
            "background finished before the visible tail proof"
        );
    }
    shared.lock().unwrap().limit = usize::MAX;
    runtime.block_on(async {
        while replica.snapshot.as_ref().unwrap().sequence < tails[&watched] {
            apply(&mut read, &mut write, endpoint.capability, &mut replica).await;
        }
    });
    assert_eq!(
        replica.snapshot.as_ref().unwrap().screen,
        reference[&watched]
    );
    progress = Instant::now();
    let mut previous = BTreeMap::new();
    loop {
        let current: BTreeMap<_, _> = seats
            .iter()
            .map(|&seat| {
                (
                    seat,
                    runtime.block_on(logic.client.snapshot(seat, 1)).unwrap(),
                )
            })
            .collect();
        let cursors: BTreeMap<_, _> = current
            .iter()
            .map(|(&seat, snapshot)| (seat, snapshot.sequence))
            .collect();
        if cursors != previous {
            previous = cursors;
            progress = Instant::now();
        }
        if current
            .iter()
            .all(|(seat, snapshot)| snapshot.sequence == tails[seat])
        {
            for (&seat, snapshot) in &current {
                assert_eq!(snapshot.screen, reference[&seat]);
            }
            break;
        }
        assert!(
            progress.elapsed() < Duration::from_secs(60),
            "background replay made no progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(write);
    drop(read);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
