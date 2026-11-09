// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Witness: the runtime Outbox/Inbox and the Yarn model take the same steps,
//! including a duplicate, a link drop and a remote crash with replay, and agree
//! on what was applied and acknowledged. Then the exported model is checked
//! exhaustively and the broken control must fail.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::delivery::{BrokenRemoteDelivery, RemoteDelivery as M};
use bboxy450_term_remote::link::{Arrival, FileJournal, Inbox, Outbox};

fn step(s: &M::State, e: M::Event) -> (M::State, usize) {
    match M::step(s, e) {
        M::Decision::Accepted { next, effects } => (next, effects.len()),
        _ => (*s, 0),
    }
}

#[test]
fn runtime_and_model_agree_through_duplicate_drop_and_crash_replay() {
    let dir = std::env::temp_dir().join(format!("bboxy-remote-witness-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let journal = dir.join("applied.log");
    let mut out: Outbox<&str> = Outbox::new();
    let mut inbox = Inbox::new(FileJournal::open(&journal).unwrap());
    let mut model = M::State::default();
    let mut applies = (0, 0);
    let ids = [("a", M::Msg::A), ("b", M::Msg::B)];
    for (id, m) in ids {
        out.submit(id, "turn").unwrap();
        model = step(&model, M::Event::Submit(m)).0;
        assert!(out.submit(id, "turn").is_err(), "AlreadySubmitted");
    }
    // Send both; a arrives twice (duplicate frame), b arrives once.
    assert_eq!(out.sendable().len(), 2);
    for (_, m) in ids {
        model = step(&model, M::Event::Send(m)).0;
    }
    for (id, m) in [ids[0], ids[0], ids[1]] {
        let fresh = inbox.arrive(id).unwrap() == Arrival::Fresh;
        let (next, effects) = step(&model, M::Event::Arrive(m));
        model = next;
        assert_eq!(
            fresh,
            effects == 1,
            "{id}: runtime and model disagree on Apply"
        );
        applies.0 += usize::from(fresh);
        applies.1 += effects;
    }
    // a's Applied arrives; then the remote crashes before b's Applied.
    assert!(out.on_applied("a"));
    model = step(&model, M::Event::AckArrive(M::Msg::A)).0;
    out.drop_link();
    model = step(&model, M::Event::RemoteCrash).0;
    drop(inbox);
    let mut inbox = Inbox::new(FileJournal::open(&journal).unwrap());
    assert!(!out.on_applied("b"), "no ack on a dead link");
    out.reconnect();
    model = step(&model, M::Event::Reconnect).0;
    // Replay: only b is still unacked, and it is a duplicate after restart.
    let resend = out.sendable();
    assert_eq!(
        resend.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        ["b"]
    );
    model = step(&model, M::Event::Send(M::Msg::B)).0;
    assert_eq!(inbox.arrive("b").unwrap(), Arrival::Duplicate);
    let (next, effects) = step(&model, M::Event::Arrive(M::Msg::B));
    model = next;
    assert_eq!(effects, 0);
    assert!(out.on_applied("b"));
    model = step(&model, M::Event::AckArrive(M::Msg::B)).0;
    assert_eq!(applies, (2, 2), "each message applied exactly once");
    for (id, m) in ids {
        assert_eq!(out.is_acked(id), model.acked().contains(m));
        assert_eq!(out.is_unacked(id), model.unacked().contains(m));
        assert!(model.applied().contains(m));
    }
    assert!(M::applied_at_most_once(&model));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn exported_model_holds_and_volatile_dedup_control_fails() {
    let pass = M::model().check(100_000).expect("model check runs");
    assert!(pass.violations.is_empty(), "{:?}", pass.violations);
    let broken = BrokenRemoteDelivery::model()
        .check(100_000)
        .expect("control runs");
    assert!(
        broken
            .violations
            .iter()
            .any(|v| v.property.contains("applied_at_most_once")
                || v.property.contains("REMOTE-DELIVERY-001")),
        "volatile applied set must double-apply after crash replay: {:?}",
        broken.violations
    );
}

#[test]
fn resume_model_holds_and_resetless_restart_control_fails() {
    use bboxy450_term_remote::resume_model::{BrokenRemoteResume, RemoteResume};
    let pass = RemoteResume::model()
        .check(100_000)
        .expect("model check runs");
    assert!(pass.violations.is_empty(), "{:?}", pass.violations);
    let broken = BrokenRemoteResume::model()
        .check(100_000)
        .expect("control runs");
    assert!(
        !broken.violations.is_empty(),
        "a restart without reset must leave the view holding events the new log lacks"
    );
}
