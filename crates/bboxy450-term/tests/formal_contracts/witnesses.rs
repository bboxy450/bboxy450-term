// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real API witnesses supplement contract checking; they are not refinement proofs.
use bboxy450_desktop_notifications::{
    Authorization, Error, Notification, NotificationId, Notifier,
};
use bboxy450_term::{
    attention::{Desired, EpisodeKey, Episodes, Notice},
    notifications::{Event, Worker},
};
use bbt_mux::Ulid;
use bbt_proto::{
    GuiAction, GuiReply,
    gui::{WindowSnapshot, WindowState},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

struct CountWake(AtomicUsize);
impl bbt_rt::Waker for CountWake {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

// Enumerate every short sequential history. Atomic interleavings are checked by DirtyWake.
#[test]
fn immediate_dirty_sources_preserve_all_published_bits_and_coalesce_wakes() {
    for history in 0..3_usize.pow(6) {
        let waker = Arc::new(CountWake(AtomicUsize::new(0)));
        let set = bbt_rt::DirtySet::new(2, waker.clone());
        let sources = [set.source(0), set.source(1)];
        let mut pending = [false; 2];
        let mut posted = false;
        let mut wakes = 0;
        let mut encoded = history;
        for _ in 0..6 {
            let step = encoded % 3;
            encoded /= 3;
            if step < 2 {
                sources[step].mark();
                pending[step] = true;
                if !posted {
                    wakes += 1;
                    posted = true;
                }
            } else {
                assert_eq!(
                    set.drain(),
                    (0..2).filter(|i| pending[*i]).collect::<Vec<_>>()
                );
                pending = [false; 2];
                posted = false;
            }
            assert_eq!(waker.0.load(Ordering::SeqCst), wakes);
        }
        assert_eq!(
            set.drain(),
            (0..2).filter(|i| pending[*i]).collect::<Vec<_>>()
        );
    }
}

fn state(width: f32) -> WindowState {
    // Adapted from window_writes.rs fixture in bboxy450-term (same project).
    serde_json::from_value(serde_json::json!({"version":1,"size":[width,720.0],"sidebar_width":300.0,"sidebar_scroll":0.0,"sidebar_shown":true,"open_swarms":[],"swarms":[],"stage":{"swarm":null,"tab":null,"focused":null},"scroll":[]})).unwrap()
}
#[test]
fn only_the_matching_durable_revision_releases_each_frame_reply() {
    use bboxy450_term::window_writes::Writes;
    let old = WindowSnapshot {
        epoch: Ulid::from(1_u128),
        state: None,
    };
    let new = WindowSnapshot {
        epoch: Ulid::from(2_u128),
        state: None,
    };
    let mut writes = Writes::new(Some(&old));
    writes.observe(state(1000.));
    let GuiAction::WindowSave { id: old_op, .. } = writes.next(Some(&old)).unwrap() else {
        panic!()
    };
    writes.observe(state(1200.));
    let reply = Ulid::from(3_u128);
    assert!(
        writes
            .fence(GuiAction::Focused {
                id: reply,
                result: Err("fixture".into())
            })
            .is_none()
    );
    let GuiAction::WindowSave { id: current_op, .. } = writes.next(Some(&new)).unwrap() else {
        panic!()
    };
    assert!(
        writes
            .completion(&GuiReply::WindowSaved {
                op: old_op,
                error: None
            })
            .unwrap()
            .is_empty()
    );
    let ready = writes
        .completion(&GuiReply::WindowSaved {
            op: current_op,
            error: None,
        })
        .unwrap();
    assert!(matches!(&ready[..],[GuiAction::Focused{id,..}] if *id==reply));
    assert!(
        writes
            .completion(&GuiReply::WindowSaved {
                op: current_op,
                error: None
            })
            .unwrap()
            .is_empty()
    );
}

#[test]
fn checkpoint_restores_partial_parser_state_without_reemitting_old_effects() {
    let mut term = bbt_core::Term::new(20, 4, 0);
    term.advance(b"\x07\x1b[6n\x1b]2;par");
    assert!(!term.take_events().is_empty());
    let checkpoint = term.checkpoint();
    term.advance(b"tial\x07");
    let events = term.take_events();
    assert!(!events.is_empty());
    let expected = format!("{events:?}");
    // Restore clears any effects generated since capture, too.
    term.advance(b"\x07");
    term.restore(checkpoint);
    assert!(term.take_events().is_empty());
    term.advance(b"tial\x07");
    assert_eq!(format!("{:?}", term.take_events()), expected);
}

// The authorization barrier gives a deterministic actual worker interleaving without
// native permissions, sleeps, desktop input, or dependence on the operator's identity.
struct SlowAuthorization {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    posts: Arc<AtomicUsize>,
}
impl Notifier for SlowAuthorization {
    fn authorize(&self) -> Result<Authorization, Error> {
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(Authorization::Granted)
    }
    fn post(&self, _: &Notification) -> Result<(), Error> {
        self.posts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn remove(&self, _: &NotificationId) -> Result<(), Error> {
        Ok(())
    }
}
#[test]
fn a_queued_mute_replaces_the_notice_after_slow_authorization() {
    let (entered, at_authorization) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let (ready, events) = mpsc::channel();
    let posts = Arc::new(AtomicUsize::new(0));
    let mut worker = Worker::start(
        Box::new(SlowAuthorization {
            entered,
            release: Mutex::new(blocked),
            posts: posts.clone(),
        }),
        Arc::new(move |event| {
            if matches!(event, Event::Ready) {
                let _ = ready.send(());
            }
        }),
    )
    .unwrap();
    let id = NotificationId("formal-private-seat".into());
    let desired: Desired = [(
        id.clone(),
        Notice {
            episode: EpisodeKey {
                epoch: Ulid::from(1_u128),
                generation: 1,
                ordinal: 1,
            },
            notification: Notification {
                id,
                title: "fixture".into(),
                body: "fixture".into(),
            },
        },
    )]
    .into();
    worker.submit(desired, Default::default(), true);
    at_authorization
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    worker.submit(Desired::new(), Default::default(), false);
    release.send(()).unwrap();
    while worker.progress().1 < 2 {
        events.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    assert_eq!(worker.progress(), (2, 2));
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    worker.join().unwrap();
}

#[test]
fn captured_episode_identity_survives_rebuild_but_not_a_to_b_to_a() {
    use bbt_proto::{
        Activity, Grade, GuiSeat, ProcessStatus, SeatId, SeatState, WaitKind, registry::SeatRecord,
    };
    use std::time::Instant;
    let mut row = GuiSeat {
        record: SeatRecord::new(Ulid::from(7_u128), Ulid::from(8_u128), "A", "claude"),
        state: SeatState {
            generation: 1,
            process: ProcessStatus::Running { pid: 9 },
            activity: Activity::Waiting(WaitKind::Question),
            grade: Grade::Shim,
            title: String::new(),
            detail: Some("Answer?".into()),
            turn_open: true,
            observed_ms: 0,
            subagents: vec![],
        },
        attention: None,
        notification_episode: Some(EpisodeKey {
            epoch: Ulid::from(1_u128),
            generation: 1,
            ordinal: 1,
        }),
    };
    let id = SeatId::from_u128(7);
    let now = Instant::now();
    let captured_a = row.notification_episode.unwrap();
    let mut episodes = Episodes::default();
    episodes.observe(&row, now);
    assert!(episodes.handle(id, captured_a));
    // The daemon/UI may independently rebuild from the same shared key. Exact
    // handled context remains applicable; an unchanged projection never resets it.
    let mut rebuilt = Episodes::default();
    rebuilt.observe(&row, now);
    assert!(rebuilt.handle(id, captured_a));
    rebuilt.observe(&row, now);
    assert!(!rebuilt.handle(id, captured_a));
    assert!(
        rebuilt
            .notifications(&Default::default(), false, None)
            .is_empty()
    );
    for incarnation in [2_u128, 3] {
        row.notification_episode.as_mut().unwrap().epoch = Ulid::from(incarnation);
        rebuilt.observe(&row, now);
        assert!(!rebuilt.handle(id, captured_a));
        assert_eq!(
            rebuilt
                .notifications(&Default::default(), false, None)
                .len(),
            1
        );
    }
    assert_eq!(rebuilt.key(id), row.notification_episode);
    assert!(rebuilt.handle(id, row.notification_episode.unwrap()));
}

#[test]
fn a_replaced_focus_timeout_cannot_complete_the_current_operation() {
    use bboxy450_term::focus_ack::{Ask, Pending};
    let mut pending = Pending::default();
    let seat = bbt_proto::SeatId::from_u128(1);
    let first = Ulid::from(1_u128);
    let second = Ulid::from(2_u128);
    pending.set(Ask {
        id: first,
        seat,
        prior_spawns: vec![],
    });
    assert_eq!(
        pending
            .set(Ask {
                id: second,
                seat,
                prior_spawns: vec![]
            })
            .unwrap()
            .id,
        first
    );
    assert!(pending.expire(first).is_none());
    assert_eq!(pending.expire(second).unwrap().id, second);
    assert!(pending.expire(second).is_none());
}
