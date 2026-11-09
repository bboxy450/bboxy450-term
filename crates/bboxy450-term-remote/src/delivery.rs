// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exactly-once delivery across the remote supervisor link (ICD
//! `.bboxy/interfaces/remote-supervisor-link.md` §4). The local mux owns an
//! outbox of accepted messages; the remote supervisor owns a durable applied
//! set. The link may drop, duplicate and replay frames, and the remote may
//! crash; a message is applied to its harness session at most once and is
//! acknowledged only after it was applied. The Yarn checker exports these exact
//! declarations to TLA+ (Rust TLC and Java TLC side by side); `crate::link`
//! is the runtime and `tests/delivery_witness.rs` drives both with the same steps.
//! RemoteCrash keeps `applied`: the runtime journals ids durably (fsync).

bbt_yarn::protocol! {
    pub mod RemoteDelivery {
        domain Msg { A, B }
        state {
            link_up: bool = true;
            unacked: Set<Msg> = Set::empty();
            inflight: Set<Msg> = Set::empty();
            applied: Set<Msg> = Set::empty();
            acked: Set<Msg> = Set::empty();
            ever: Set<Msg> = Set::empty();
            doubled: bool = false;
        }
        events { Submit(m: Msg); Send(m: Msg); Arrive(m: Msg); AckArrive(m: Msg); Drop; Reconnect; RemoteCrash; }
        effects { Apply(m: Msg); }
        on Submit(m) {
            case accept when !unacked.contains(m) && !acked.contains(m) => {
                set unacked = unacked.with(m);
                preserve link_up, inflight, applied, acked, ever, doubled;
            }
            case known when unacked.contains(m) || acked.contains(m) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                reject AlreadySubmitted;
            }
        }
        on Send(m) {
            case send when link_up && unacked.contains(m) && !inflight.contains(m) => {
                set inflight = inflight.with(m);
                preserve link_up, unacked, applied, acked, ever, doubled;
            }
            case hold when !(link_up && unacked.contains(m) && !inflight.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NotSendable;
            }
        }
        on Arrive(m) {
            case fresh when link_up && inflight.contains(m) && !applied.contains(m) => {
                set applied = applied.with(m);
                set ever = ever.with(m);
                set doubled = doubled || ever.contains(m);
                preserve link_up, unacked, inflight, acked;
                emit Apply(m);
            }
            case duplicate when link_up && inflight.contains(m) && applied.contains(m) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyApplied;
            }
            case lost when !(link_up && inflight.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NotInFlight;
            }
        }
        on AckArrive(m) {
            case ack when link_up && inflight.contains(m) && applied.contains(m) => {
                set unacked = unacked.without(m);
                set inflight = inflight.without(m);
                set acked = acked.with(m);
                preserve link_up, applied, ever, doubled;
            }
            case stale when !(link_up && inflight.contains(m) && applied.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NoAck;
            }
        }
        on Drop {
            case drop when link_up => {
                set link_up = false;
                set inflight = Set::empty();
                preserve unacked, applied, acked, ever, doubled;
            }
            case down when !link_up => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyDown;
            }
        }
        on Reconnect {
            case up when !link_up => {
                set link_up = true;
                preserve unacked, inflight, applied, acked, ever, doubled;
            }
            case live when link_up => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyUp;
            }
        }
        on RemoteCrash {
            case crash when true => {
                set link_up = false;
                set inflight = Set::empty();
                preserve unacked, applied, acked, ever, doubled;
            }
        }
        #[invariant(id = "REMOTE-DELIVERY-001")]
        fn applied_at_most_once() { !doubled }
        #[invariant(id = "REMOTE-DELIVERY-002")]
        fn ack_implies_applied() {
            (!acked.contains(Msg::A) || applied.contains(Msg::A)) && (!acked.contains(Msg::B) || applied.contains(Msg::B))
        }
        #[invariant(id = "REMOTE-DELIVERY-003")]
        fn acked_leaves_outbox() {
            (!acked.contains(Msg::A) || !unacked.contains(Msg::A)) && (!acked.contains(Msg::B) || !unacked.contains(Msg::B))
        }
    }
}

// Broken control: a remote that keeps its applied set only in memory
// re-applies a message replayed after a crash. Must violate REMOTE-DELIVERY-001.
bbt_yarn::protocol! {
    pub mod BrokenRemoteDelivery {
        domain Msg { A, B }
        state {
            link_up: bool = true;
            unacked: Set<Msg> = Set::empty();
            inflight: Set<Msg> = Set::empty();
            applied: Set<Msg> = Set::empty();
            acked: Set<Msg> = Set::empty();
            ever: Set<Msg> = Set::empty();
            doubled: bool = false;
        }
        events { Submit(m: Msg); Send(m: Msg); Arrive(m: Msg); AckArrive(m: Msg); Drop; Reconnect; RemoteCrash; }
        effects { Apply(m: Msg); }
        on Submit(m) {
            case accept when !unacked.contains(m) && !acked.contains(m) => {
                set unacked = unacked.with(m);
                preserve link_up, inflight, applied, acked, ever, doubled;
            }
            case known when unacked.contains(m) || acked.contains(m) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                reject AlreadySubmitted;
            }
        }
        on Send(m) {
            case send when link_up && unacked.contains(m) && !inflight.contains(m) => {
                set inflight = inflight.with(m);
                preserve link_up, unacked, applied, acked, ever, doubled;
            }
            case hold when !(link_up && unacked.contains(m) && !inflight.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NotSendable;
            }
        }
        on Arrive(m) {
            case fresh when link_up && inflight.contains(m) && !applied.contains(m) => {
                set applied = applied.with(m);
                set ever = ever.with(m);
                set doubled = doubled || ever.contains(m);
                preserve link_up, unacked, inflight, acked;
                emit Apply(m);
            }
            case duplicate when link_up && inflight.contains(m) && applied.contains(m) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyApplied;
            }
            case lost when !(link_up && inflight.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NotInFlight;
            }
        }
        on AckArrive(m) {
            case ack when link_up && inflight.contains(m) && applied.contains(m) => {
                set unacked = unacked.without(m);
                set inflight = inflight.without(m);
                set acked = acked.with(m);
                preserve link_up, applied, ever, doubled;
            }
            case stale when !(link_up && inflight.contains(m) && applied.contains(m)) => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore NoAck;
            }
        }
        on Drop {
            case drop when link_up => {
                set link_up = false;
                set inflight = Set::empty();
                preserve unacked, applied, acked, ever, doubled;
            }
            case down when !link_up => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyDown;
            }
        }
        on Reconnect {
            case up when !link_up => {
                set link_up = true;
                preserve unacked, inflight, applied, acked, ever, doubled;
            }
            case live when link_up => {
                preserve link_up, unacked, inflight, applied, acked, ever, doubled;
                ignore AlreadyUp;
            }
        }
        on RemoteCrash {
            case crash when true => {
                set link_up = false;
                set inflight = Set::empty();
                set applied = Set::empty();
                preserve unacked, acked, ever, doubled;
            }
        }
        #[invariant(id = "REMOTE-DELIVERY-001")]
        fn applied_at_most_once() { !doubled }
    }
}
