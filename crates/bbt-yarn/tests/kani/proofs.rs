// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Kani's induction and autoharness examples, implemented independently:
// https://model-checking.github.io/kani/reference/experimental/autoharness.html

use super::declarations::{BrokenLatestResult as broken, LatestResult as good};

fn event() -> good::Event {
    // All five concrete events, with no scheduling or reachability assumption.
    let choice: u8 = kani::any();
    kani::assume(choice < 5);
    match choice {
        0 => good::Event::Submit(good::Id::A),
        1 => good::Event::Submit(good::Id::B),
        2 => good::Event::Complete(good::Id::A),
        3 => good::Event::Complete(good::Id::B),
        _ => good::Event::Cancel,
    }
}

fn broken_event() -> broken::Event {
    match event() {
        good::Event::Submit(good::Id::A) => broken::Event::Submit(broken::Id::A),
        good::Event::Submit(good::Id::B) => broken::Event::Submit(broken::Id::B),
        good::Event::Complete(good::Id::A) => broken::Event::Complete(broken::Id::A),
        good::Event::Complete(good::Id::B) => broken::Event::Complete(broken::Id::B),
        good::Event::Cancel => broken::Event::Cancel,
    }
}

pub fn initial_invariant() {
    assert!(good::result_is_current(&good::State::default()));
}

// These argument-taking functions are selected by the pinned autoharness runner.
// Kani synthesizes arbitrary values of the generated private State, including
// Option discriminants and both u64-backed Set<Id> values. This overapproximates
// *every* public-API state, without a path-length bound or a layout/transmute hack.
// Unused high set bits do not affect guards or RESULT-001. No inferred/constructor
// invariants, function stubs, bounded arguments or reachability filters are used.
pub fn step_never_panics(state: good::State) {
    let _ = good::step(&state, event());
}

pub fn invariant_is_inductive(state: good::State) {
    kani::assume(good::result_is_current(&state));
    if let good::Decision::Accepted { next, .. } = good::step(&state, event()) {
        assert!(good::result_is_current(&next), "RESULT-001 inductive step");
    }
}

pub fn refusal_preserves_input(state: good::State) {
    let before = state;
    let decision = good::step(&state, event());
    assert!(state == before, "step input is immutable");
    // The public Decision contains no next state on rejection/ignore. The caller
    // retains the input; explicitly check the same interpretation as its witness.
    let retained = match decision {
        good::Decision::Accepted { .. } => return,
        good::Decision::Rejected { .. } | good::Decision::Ignored { .. } => state,
    };
    assert!(retained == before, "refusal retains the input state");
}

pub fn broken_invariant_is_inductive(state: broken::State) {
    kani::assume(broken::result_is_current(&state));
    if let broken::Decision::Accepted { next, .. } = broken::step(&state, broken_event()) {
        // No should_panic: the runner requires this exact assertion to fail and
        // rejects compiler/solver/unwinding failures as negative-control evidence.
        assert!(
            broken::result_is_current(&next),
            "RESULT-001 broken inductive step"
        );
    }
}
