// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Event resume across the remote supervisor link (ICD §5). The remote keeps a
//! per-session event log for its current epoch; the local side shows events in
//! order by cursor. A link drop keeps both; a remote restart starts a new epoch
//! whose log has lost the old tail, so the remote sends `reset` before any
//! new-epoch event and the local view clears before it shows them. Ev={A,B}
//! are the first two events of an epoch (A before B). `crate::resume` is the
//! runtime; `tests/serve.rs` drives it through drop, replay and restart.

bbt_yarn::protocol! {
    pub mod RemoteResume {
        domain Ev { A, B }
        state {
            link_up: bool = true;
            produced: Set<Ev> = Set::empty();
            seen: Set<Ev> = Set::empty();
            reset_pending: bool = false;
        }
        events { Produce(e: Ev); Deliver(e: Ev); Drop; Reconnect; RemoteRestart; ApplyReset; }
        effects { Show(e: Ev); }
        on Produce(e) {
            case first when e == Ev::A && !produced.contains(Ev::A) => {
                set produced = produced.with(Ev::A);
                preserve link_up, seen, reset_pending;
            }
            case second when e == Ev::B && produced.contains(Ev::A) && !produced.contains(Ev::B) => {
                set produced = produced.with(Ev::B);
                preserve link_up, seen, reset_pending;
            }
            case done when !(e == Ev::A && !produced.contains(Ev::A)) && !(e == Ev::B && produced.contains(Ev::A) && !produced.contains(Ev::B)) => {
                preserve link_up, produced, seen, reset_pending;
                ignore Produced;
            }
        }
        on Deliver(e) {
            case next when link_up && !reset_pending && produced.contains(e) && !seen.contains(e) && (e == Ev::A || seen.contains(Ev::A)) => {
                set seen = seen.with(e);
                preserve link_up, produced, reset_pending;
                emit Show(e);
            }
            case skip when !(link_up && !reset_pending && produced.contains(e) && !seen.contains(e) && (e == Ev::A || seen.contains(Ev::A))) => {
                preserve link_up, produced, seen, reset_pending;
                ignore NotNext;
            }
        }
        on Drop {
            case drop when link_up => {
                set link_up = false;
                preserve produced, seen, reset_pending;
            }
            case down when !link_up => {
                preserve link_up, produced, seen, reset_pending;
                ignore AlreadyDown;
            }
        }
        on Reconnect {
            case up when !link_up => {
                set link_up = true;
                preserve produced, seen, reset_pending;
            }
            case live when link_up => {
                preserve link_up, produced, seen, reset_pending;
                ignore AlreadyUp;
            }
        }
        on RemoteRestart {
            case restart when true => {
                set link_up = false;
                set produced = Set::empty();
                set reset_pending = true;
                preserve seen;
            }
        }
        on ApplyReset {
            case reset when link_up && reset_pending => {
                set seen = Set::empty();
                set reset_pending = false;
                preserve link_up, produced;
            }
            case none when !(link_up && reset_pending) => {
                preserve link_up, produced, seen, reset_pending;
                ignore NoReset;
            }
        }
        #[invariant(id = "REMOTE-RESUME-001")]
        fn seen_is_in_current_log() {
            (!seen.contains(Ev::A) || produced.contains(Ev::A) || reset_pending) && (!seen.contains(Ev::B) || produced.contains(Ev::B) || reset_pending)
        }
        #[invariant(id = "REMOTE-RESUME-002")]
        fn shown_in_order() { !seen.contains(Ev::B) || seen.contains(Ev::A) }
    }
}

// Broken control: a restarted remote that resumes without `reset` leaves the
// local view holding events its new log does not have. Must violate RESUME-001.
bbt_yarn::protocol! {
    pub mod BrokenRemoteResume {
        domain Ev { A, B }
        state {
            link_up: bool = true;
            produced: Set<Ev> = Set::empty();
            seen: Set<Ev> = Set::empty();
            reset_pending: bool = false;
        }
        events { Produce(e: Ev); Deliver(e: Ev); Drop; Reconnect; RemoteRestart; ApplyReset; }
        effects { Show(e: Ev); }
        on Produce(e) {
            case first when e == Ev::A && !produced.contains(Ev::A) => {
                set produced = produced.with(Ev::A);
                preserve link_up, seen, reset_pending;
            }
            case second when e == Ev::B && produced.contains(Ev::A) && !produced.contains(Ev::B) => {
                set produced = produced.with(Ev::B);
                preserve link_up, seen, reset_pending;
            }
            case done when !(e == Ev::A && !produced.contains(Ev::A)) && !(e == Ev::B && produced.contains(Ev::A) && !produced.contains(Ev::B)) => {
                preserve link_up, produced, seen, reset_pending;
                ignore Produced;
            }
        }
        on Deliver(e) {
            case next when link_up && !reset_pending && produced.contains(e) && !seen.contains(e) && (e == Ev::A || seen.contains(Ev::A)) => {
                set seen = seen.with(e);
                preserve link_up, produced, reset_pending;
                emit Show(e);
            }
            case skip when !(link_up && !reset_pending && produced.contains(e) && !seen.contains(e) && (e == Ev::A || seen.contains(Ev::A))) => {
                preserve link_up, produced, seen, reset_pending;
                ignore NotNext;
            }
        }
        on Drop {
            case drop when link_up => {
                set link_up = false;
                preserve produced, seen, reset_pending;
            }
            case down when !link_up => {
                preserve link_up, produced, seen, reset_pending;
                ignore AlreadyDown;
            }
        }
        on Reconnect {
            case up when !link_up => {
                set link_up = true;
                preserve produced, seen, reset_pending;
            }
            case live when link_up => {
                preserve link_up, produced, seen, reset_pending;
                ignore AlreadyUp;
            }
        }
        on RemoteRestart {
            case restart when true => {
                set link_up = false;
                set produced = Set::empty();
                preserve reset_pending;
                preserve seen;
            }
        }
        on ApplyReset {
            case reset when link_up && reset_pending => {
                set seen = Set::empty();
                set reset_pending = false;
                preserve link_up, produced;
            }
            case none when !(link_up && reset_pending) => {
                preserve link_up, produced, seen, reset_pending;
                ignore NoReset;
            }
        }
        #[invariant(id = "REMOTE-RESUME-001")]
        fn seen_is_in_current_log() {
            (!seen.contains(Ev::A) || produced.contains(Ev::A) || reset_pending) && (!seen.contains(Ev::B) || produced.contains(Ev::B) || reset_pending)
        }
        #[invariant(id = "REMOTE-RESUME-002")]
        fn shown_in_order() { !seen.contains(Ev::B) || seen.contains(Ev::A) }
    }
}
