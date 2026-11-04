// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by TLA+ response/fairness concepts, Leslie Lamport:
// https://lamport.azurewebsites.net/tla/high-level-view.html. No source copied.
// The exporter includes these declarations, including the three broken controls.
#![cfg(feature = "macros")]
#![allow(missing_docs)]

// One frame, two nonreused generations, one owner, arbitrary completion order.
// Fairness is on the presentation case, never on ignored completion events.
macro_rules! presentation {
    ($name:ident, $generation_guard:expr, [$($fair:tt)*]) => {
        bbt_yarn::protocol! {
            pub mod $name {
                domain Frame { F }
                domain Generation { First, Second }
                state {
                    generation: Generation = Generation::First;
                    selected: bool = false;
                    issued: Set<Generation> = Set::empty();
                    queued: Set<Generation> = Set::empty();
                    admitted: Set<Generation> = Set::empty();
                    fetched: Set<Generation> = Set::empty();
                    visible: Option<Generation> = None;
                }
                events { Select(next_generation: Generation); Present(frame: Frame, expected_generation: Generation); Cancel; }
                effects { Fetch(frame: Frame, expected_generation: Generation) completes {
                    Succeeded, FailedBeforeEffect, UnknownOutcome, CancelledBeforeStart
                }; }
                on Select(next_generation) {
                    case fresh when !issued.contains(next_generation) => {
                        set generation = next_generation;
                        set selected = true;
                        set issued = issued.with(next_generation);
                        set queued = queued.with(next_generation);
                        set visible = None;
                        preserve admitted, fetched;
                        emit Fetch(Frame::F, next_generation);
                    }
                    case reused when issued.contains(next_generation) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        reject GenerationReused;
                    }
                }
                on FetchAdmitted(frame, expected_generation) {
                    #[fair(strong)]
                    case admit when queued.contains(expected_generation) && !admitted.contains(expected_generation) => {
                        set admitted = admitted.with(expected_generation);
                        preserve generation, selected, issued, queued, fetched, visible;
                    }
                    case stale when !(queued.contains(expected_generation) && !admitted.contains(expected_generation)) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore NotQueued;
                    }
                }
                on FetchSucceeded(frame, expected_generation) {
                    case complete when admitted.contains(expected_generation) => {
                        set fetched = fetched.with(expected_generation);
                        set admitted = admitted.without(expected_generation);
                        set queued = queued.without(expected_generation);
                        preserve generation, selected, issued, visible;
                    }
                    case stale when !admitted.contains(expected_generation) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore NotAdmitted;
                    }
                }
                on FetchFailedBeforeEffect(frame, expected_generation) {
                    case failed when admitted.contains(expected_generation) => {
                        set admitted = admitted.without(expected_generation);
                        set queued = queued.without(expected_generation);
                        preserve generation, selected, issued, fetched, visible;
                    }
                    case stale when !admitted.contains(expected_generation) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore NotAdmitted;
                    }
                }
                on FetchUnknownOutcome(frame, expected_generation) {
                    case unknown when admitted.contains(expected_generation) => {
                        set admitted = admitted.without(expected_generation);
                        set queued = queued.without(expected_generation);
                        preserve generation, selected, issued, fetched, visible;
                    }
                    case stale when !admitted.contains(expected_generation) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore NotAdmitted;
                    }
                }
                on FetchCancelledBeforeStart(frame, expected_generation) {
                    case cancelled when queued.contains(expected_generation) && !admitted.contains(expected_generation) => {
                        set queued = queued.without(expected_generation);
                        preserve generation, selected, issued, admitted, fetched, visible;
                    }
                    case stale when !(queued.contains(expected_generation) && !admitted.contains(expected_generation)) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore AlreadyStarted;
                    }
                }
                on Present(frame, expected_generation) {
                    $($fair)*
                    case current when selected && fetched.contains(expected_generation) && $generation_guard => {
                        set visible = Some(expected_generation);
                        preserve generation, selected, issued, queued, admitted, fetched;
                    }
                    case stale when !(selected && fetched.contains(expected_generation) && $generation_guard) => {
                        preserve generation, selected, issued, queued, admitted, fetched, visible;
                        ignore StaleGeneration;
                    }
                }
                on Cancel {
                    case clear when true => {
                        set selected = false;
                        set visible = None;
                        preserve generation, issued, queued, admitted, fetched;
                    }
                }
                #[invariant(id = "PRESENT-001")]
                fn current_generation_only() { visible.is_none() || (selected && visible == Some(generation)) }
                #[predicate]
                fn first_ready() { selected && generation == Generation::First && fetched.contains(Generation::First) }
                #[predicate]
                fn first_resolved() { !selected || generation != Generation::First || visible == Some(Generation::First) }
                #[predicate]
                fn second_ready() { selected && generation == Generation::Second && fetched.contains(Generation::Second) }
                #[predicate]
                fn second_resolved() { !selected || generation != Generation::Second || visible == Some(Generation::Second) }
                #[quiescent]
                fn idle() { !selected && queued == Set::empty() && admitted == Set::empty() }
                #[terminal]
                fn finished() { !selected && queued == Set::empty() && admitted == Set::empty() && issued.contains(Generation::First) && issued.contains(Generation::Second) }
                #[property(id = "PRESENT-LIVE-First", leads_to(first_ready, first_resolved))];
                #[property(id = "PRESENT-LIVE-Second", leads_to(second_ready, second_resolved))];
            }
        }
    };
}
presentation!(PresentGeneration, expected_generation == generation, [#[fair(weak)]]);
presentation!(BrokenPresentGeneration, true, [#[fair(weak)]]);
presentation!(PresentProgress, expected_generation == generation, [#[fair(weak)]]);
presentation!(BrokenPresentProgress, expected_generation == generation, []);

// UnknownOutcome includes the possible external commit. A retained claim fences
// another attempt with the same identity; known pre-effect failure releases it.
macro_rules! retry {
    ($name:ident, $dedup:expr) => {
        bbt_yarn::protocol! {
            pub mod $name {
                domain Id { Only }
                state {
                    started: bool = false;
                    intent: bool = false;
                    admitted: bool = false;
                    claimed: bool = false;
                    unknown: bool = false;
                    committed: bool = false;
                    duplicate: bool = false;
                }
                events { Begin; Retry; }
                effects { Write(id: Id) completes { Succeeded, FailedBeforeEffect, UnknownOutcome, CancelledBeforeStart }; }
                on Begin {
                    case fresh when !started => {
                        set started = true;
                        set intent = true;
                        preserve admitted, claimed, unknown, committed, duplicate;
                        emit Write(Id::Only);
                    }
                    case reused when started => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        reject AlreadyStarted;
                    }
                }
                on Retry {
                    case retry when started && !intent && !admitted && $dedup => {
                        set intent = true;
                        set unknown = false;
                        preserve started, admitted, claimed, committed, duplicate;
                        emit Write(Id::Only);
                    }
                    case fenced when !(started && !intent && !admitted && $dedup) => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        reject ClaimedOrBusy;
                    }
                }
                on WriteAdmitted(id) {
                    case admit when intent && !admitted => {
                        set admitted = true;
                        set claimed = true;
                        preserve started, intent, unknown, committed, duplicate;
                    }
                    case stale when !(intent && !admitted) => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        ignore NotQueued;
                    }
                }
                on WriteSucceeded(id) {
                    case complete when admitted => {
                        set committed = true;
                        set duplicate = committed || duplicate;
                        set admitted = false;
                        set intent = false;
                        preserve started, claimed, unknown;
                    }
                    case stale when !admitted => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        ignore NotAdmitted;
                    }
                }
                on WriteUnknownOutcome(id) {
                    case uncertain when admitted => {
                        set committed = true;
                        set duplicate = committed || duplicate;
                        set admitted = false;
                        set intent = false;
                        set unknown = true;
                        preserve started, claimed;
                    }
                    case stale when !admitted => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        ignore NotAdmitted;
                    }
                }
                on WriteFailedBeforeEffect(id) {
                    case failed when admitted => {
                        set admitted = false;
                        set intent = false;
                        set claimed = false;
                        preserve started, unknown, committed, duplicate;
                    }
                    case stale when !admitted => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        ignore NotAdmitted;
                    }
                }
                on WriteCancelledBeforeStart(id) {
                    case cancelled when intent && !admitted => {
                        set intent = false;
                        preserve started, admitted, claimed, unknown, committed, duplicate;
                    }
                    case stale when !(intent && !admitted) => {
                        preserve started, intent, admitted, claimed, unknown, committed, duplicate;
                        ignore AlreadyStarted;
                    }
                }
                #[invariant(id = "EFFECT-ONCE-001")]
                fn at_most_once() { !duplicate }
                #[quiescent]
                fn idle() { !intent && !admitted }
                #[terminal]
                fn done() { committed && !intent && !admitted && !unknown }
            }
        }
    };
}
retry!(RetryUnknown, !claimed);
retry!(BrokenRetryUnknown, !claimed || unknown);

macro_rules! stuttering {
    ($name:ident, [$($transition:tt)*]) => {
        bbt_yarn::protocol! {
            pub mod $name {
                state { visible: bool = false; }
                events { Wait; Refuse; }
                on Wait {
                    case waiting when !visible => { $($transition)* }
                    case done when visible => { preserve visible; ignore Done; }
                }
                on Refuse { #[fair(strong)] case refused when true => { preserve visible; reject Refused; } }
                #[fair(weak, case = Wait::waiting)];
                #[quiescent] fn parked() { !visible }
                #[terminal] fn done() { visible }
                #[property(id = "STUTTER-LIVE", leads_to(parked, done))];
            }
        }
    };
}
stuttering!(ProtocolProgress, [set visible = true;]);
stuttering!(BrokenProtocolProgress, [preserve visible; ignore Waiting;]);

#[cfg(all(test, feature = "model"))]
mod checks {
    use super::*;
    use bbt_yarn::model::{Model, Report};
    use std::collections::VecDeque;

    // This checks every explicit labeled edge, including all negative models.
    macro_rules! replay {
        ($protocol:ident) => {{
            use super::$protocol as p;
            let model = p::model();
            let report = model.check(10_000).unwrap();
            let mut states = vec![None; report.states.len()];
            states[0] = Some(p::State::default());
            let mut queue = VecDeque::from([0]);
            let mut count = 0;
            while let Some(index) = queue.pop_front() {
                let state = states[index].unwrap();
                assert_eq!(state.model_values(), report.states[index]);
                let mut events = Vec::new();
                for edge in &report.edges[index] {
                    let event = p::action_event(edge.action).unwrap();
                    assert!(!events.contains(&event), "one case per concrete event");
                    events.push(event);
                    let next = match p::step(&state, event) {
                        p::Decision::Accepted { next, effects } => {
                            assert!(effects.len() <= 1);
                            next
                        }
                        p::Decision::Rejected { .. } | p::Decision::Ignored { .. } => state,
                    };
                    assert_eq!(state.model_values(), report.states[index]);
                    assert_eq!(
                        next.model_values(),
                        report.states[edge.to],
                        "{}",
                        model.actions[edge.action].name
                    );
                    if let Some(previous) = states[edge.to] {
                        assert_eq!(previous, next);
                    } else {
                        states[edge.to] = Some(next);
                        queue.push_back(edge.to);
                    }
                    count += 1;
                }
            }
            assert!(states.iter().all(Option::is_some));
            assert_eq!(count, report.edges.iter().map(Vec::len).sum::<usize>());
            (model, report, count)
        }};
    }

    macro_rules! trace {
        ($protocol:ident, $report:expr, $path:expr) => {{
            use super::$protocol as p;
            let report = $report;
            let path = $path;
            let mut state = p::State::default();
            assert_eq!(state.model_values(), report.states[path[0]]);
            let mut events = Vec::new();
            for pair in path.windows(2) {
                let edge = report.edges[pair[0]]
                    .iter()
                    .find(|e| e.to == pair[1])
                    .unwrap();
                let event = p::action_event(edge.action).unwrap();
                state = match p::step(&state, event) {
                    p::Decision::Accepted { next, .. } => next,
                    p::Decision::Rejected { .. } | p::Decision::Ignored { .. } => state,
                };
                assert_eq!(state.model_values(), report.states[pair[1]]);
                events.push(event);
            }
            (state, events)
        }};
    }

    #[test]
    fn every_model_edge_replays_and_bounds_are_exact() {
        fn check(
            model: &Model,
            report: &Report,
            edges: usize,
            bounds: (usize, usize),
            expected: &str,
        ) {
            assert_eq!((report.states.len(), edges), bounds, "{}", model.name);
            match expected {
                "pass" => assert!(report.violations.is_empty()),
                "safety" => assert!(report.violations.iter().any(|v| v.cycle.is_empty())),
                "liveness" => assert!(report.violations.iter().any(|v| !v.cycle.is_empty())),
                _ => unreachable!(),
            }
        }
        let (m, r, edges) = replay!(PresentGeneration);
        check(&m, &r, edges, (91, 1365), "pass");
        let (m, r, edges) = replay!(BrokenPresentGeneration);
        check(&m, &r, edges, (99, 1485), "safety");
        let (m, r, edges) = replay!(PresentProgress);
        check(&m, &r, edges, (91, 1365), "pass");
        let (m, r, edges) = replay!(BrokenPresentProgress);
        check(&m, &r, edges, (91, 1365), "liveness");
        let (m, r, edges) = replay!(RetryUnknown);
        check(&m, &r, edges, (6, 42), "pass");
        let (m, r, edges) = replay!(BrokenRetryUnknown);
        check(&m, &r, edges, (16, 112), "safety");
        let (m, r, edges) = replay!(ProtocolProgress);
        check(&m, &r, edges, (2, 4), "pass");
        let (m, r, edges) = replay!(BrokenProtocolProgress);
        check(&m, &r, edges, (1, 2), "liveness");
    }

    #[test]
    fn missing_generation_guard_has_a_replayed_safety_counterexample() {
        use BrokenPresentGeneration as p;
        let report = p::model().check(10_000).unwrap();
        let violation = report
            .violations
            .iter()
            .find(|v| v.property == "PRESENT-001")
            .unwrap();
        assert!(violation.cycle.is_empty());
        let (state, events) = trace!(BrokenPresentGeneration, &report, &violation.prefix);
        assert!(!p::current_generation_only(&state));
        assert_eq!(events.len(), 5);
        assert_ne!(state.visible(), Some(state.generation()));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, p::Event::FetchSucceeded(..)))
        );
        assert!(matches!(events.last(), Some(p::Event::Present(..))));
    }

    #[test]
    fn missing_fairness_has_a_replayed_lasso_despite_ignore_and_reject() {
        use BrokenPresentProgress as p;
        let model = p::model();
        assert!(model.weak_fair.is_empty());
        let report = model.check(10_000).unwrap();
        let fair = PresentProgress::model();
        assert_eq!(fair.weak_fair.len(), 1);
        assert_eq!(fair.weak_fair[0].len(), 2);
        assert_eq!(fair.strong_fair.len(), 1);
        assert_eq!(fair.strong_fair[0].len(), 2);
        for action in &fair.weak_fair[0] {
            let name = &fair.actions[*action].name;
            assert!(name.starts_with("Present("));
            assert!(name.ends_with("::current"));
        }
        let fair_report = fair.check(10_000).unwrap();
        assert!(fair_report.violations.is_empty());
        assert_eq!(fair_report.states, report.states);
        let edges = |r: &Report| {
            r.edges
                .iter()
                .map(|edges| edges.iter().map(|e| (e.action, e.to)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        assert_eq!(edges(&fair_report), edges(&report));
        let violation = report
            .violations
            .iter()
            .find(|v| v.property == "PRESENT-LIVE-First")
            .unwrap();
        assert!(!violation.cycle.is_empty());
        assert_eq!(violation.cycle.first(), violation.cycle.last());
        let mut path = violation.prefix.clone();
        path.extend(violation.cycle.iter().skip(1));
        let (state, _) = trace!(BrokenPresentProgress, &report, &path);
        assert!(p::first_ready(&state));
        assert!(!p::first_resolved(&state));
        assert!(matches!(
            p::step(&state, p::Event::Select(p::Generation::First)),
            p::Decision::Rejected { .. }
        ));
        assert!(matches!(
            p::step(
                &state,
                p::Event::Present(p::Frame::F, p::Generation::Second)
            ),
            p::Decision::Ignored { .. }
        ));
        assert!(!p::finished(&state));
        assert!(!p::idle(&state));
    }

    #[test]
    fn terminal_and_quiescent_labels_and_fair_stutters_do_not_discharge_response() {
        use BrokenProtocolProgress as p;
        let model = p::model();
        assert_eq!(model.weak_fair, [vec![0]]);
        assert_eq!(model.strong_fair, [vec![2]]);
        let report = model.check(2).unwrap();
        let violation = report
            .violations
            .iter()
            .find(|v| v.property == "STUTTER-LIVE")
            .unwrap();
        assert_eq!(violation.prefix, [0]);
        assert_eq!(violation.cycle, [0, 0]);
        let (state, _) = trace!(BrokenProtocolProgress, &report, &[0, 0]);
        assert!(p::parked(&state));
        assert!(!p::done(&state));
        assert!(matches!(
            p::step(&state, p::Event::Refuse),
            p::Decision::Rejected { .. }
        ));
        assert!(model.tla().unwrap().contains("WF_vars(A0)"));
        assert!(model.tla().unwrap().contains("SF_vars(A2)"));
    }

    #[test]
    fn unknown_outcome_retry_without_the_claim_guard_duplicates_the_commit() {
        use BrokenRetryUnknown as p;
        let report = p::model().check(10_000).unwrap();
        let violation = report
            .violations
            .iter()
            .find(|v| v.property == "EFFECT-ONCE-001")
            .unwrap();
        assert!(violation.cycle.is_empty());
        let (state, events) = trace!(BrokenRetryUnknown, &report, &violation.prefix);
        assert_eq!(
            events,
            [
                p::Event::Begin,
                p::Event::WriteAdmitted(p::Id::Only),
                p::Event::WriteUnknownOutcome(p::Id::Only),
                p::Event::Retry,
                p::Event::WriteAdmitted(p::Id::Only),
                p::Event::WriteSucceeded(p::Id::Only)
            ]
        );
        assert!(state.duplicate());
        assert!(!p::at_most_once(&state));
    }
}

#[cfg(test)]
mod runtime_checks {
    use super::*;

    #[test]
    fn intents_admission_completion_revalidation_and_cancellation_are_separate() {
        use PresentGeneration as p;
        fn accepted(state: &p::State, event: p::Event) -> p::State {
            let p::Decision::Accepted { next, effects } = p::step(state, event) else {
                panic!("expected accepted {event:?}")
            };
            if !matches!(event, p::Event::Select(_)) {
                assert!(effects.is_empty());
            }
            next
        }
        let first = p::Generation::First;
        let second = p::Generation::Second;
        let f = p::Frame::F;
        let initial = p::State::default();
        assert!(p::idle(&initial));
        assert!(!p::finished(&initial));
        assert!(matches!(
            p::step(&initial, p::Event::FetchSucceeded(f, first)),
            p::Decision::Ignored { .. }
        ));
        let p::Decision::Accepted {
            next: queued,
            effects,
        } = p::step(&initial, p::Event::Select(first))
        else {
            panic!()
        };
        assert_eq!(effects, [p::Effect::Fetch(f, first)]);
        assert!(!queued.admitted().contains(first));
        assert!(!queued.fetched().contains(first));
        assert!(queued.visible().is_none());
        assert!(matches!(
            p::step(&queued, p::Event::FetchSucceeded(f, first)),
            p::Decision::Ignored { .. }
        ));
        let admitted = accepted(&queued, p::Event::FetchAdmitted(f, first));
        assert!(matches!(
            p::step(&admitted, p::Event::FetchCancelledBeforeStart(f, first)),
            p::Decision::Ignored { .. }
        ));
        let fetched = accepted(&admitted, p::Event::FetchSucceeded(f, first));
        assert!(fetched.visible().is_none());
        let moved = accepted(&fetched, p::Event::Select(second));
        assert!(matches!(
            p::step(&moved, p::Event::Present(f, first)),
            p::Decision::Ignored { .. }
        ));
        assert!(p::current_generation_only(&moved));
        assert!(
            matches!(
                p::step(&moved, p::Event::Select(first)),
                p::Decision::Rejected { .. }
            ),
            "A → B → A generation reuse is rejected"
        );
        let cancelled = accepted(&moved, p::Event::FetchCancelledBeforeStart(f, second));
        let stopped = accepted(&cancelled, p::Event::Cancel);
        assert!(p::idle(&stopped));
        assert!(p::finished(&stopped));
    }

    #[test]
    fn unknown_retains_claim_but_known_pre_effect_failure_allows_retry() {
        use RetryUnknown as p;
        fn accepted(state: &p::State, event: p::Event) -> p::State {
            let p::Decision::Accepted { next, .. } = p::step(state, event) else {
                panic!("expected accepted {event:?}")
            };
            next
        }
        let queued = accepted(&p::State::default(), p::Event::Begin);
        let admitted = accepted(&queued, p::Event::WriteAdmitted(p::Id::Only));
        let unknown = accepted(&admitted, p::Event::WriteUnknownOutcome(p::Id::Only));
        assert!(unknown.unknown() && unknown.claimed() && unknown.committed());
        assert!(matches!(
            p::step(&unknown, p::Event::Retry),
            p::Decision::Rejected { .. }
        ));
        assert!(p::at_most_once(&unknown));
        let failed = accepted(&admitted, p::Event::WriteFailedBeforeEffect(p::Id::Only));
        assert!(!failed.claimed());
        let retried = accepted(&failed, p::Event::Retry);
        let admitted = accepted(&retried, p::Event::WriteAdmitted(p::Id::Only));
        let complete = accepted(&admitted, p::Event::WriteSucceeded(p::Id::Only));
        assert!(p::at_most_once(&complete));
        assert!(p::done(&complete));
        assert!(p::idle(&complete));
    }
}
