// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// The exporter includes these very declarations: no handwritten parallel model.
#![cfg(all(feature = "model", feature = "macros"))]
#![allow(missing_docs)]

bbt_yarn::protocol! {
    pub mod LatestResult {
        domain Id { A, B }
        state {
            desired: Option<Id> = None;
            visible: Option<Id> = None;
            submitted: Set<Id> = Set::empty();
            pending: Set<Id> = Set::empty();
        }
        events { Submit(id: Id); Complete(id: Id); Cancel; }
        effects { Fetch(id: Id); }
        on Submit(id) {
            case fresh when !submitted.contains(id) => {
                set desired = Some(id);
                set visible = None;
                set submitted = submitted.with(id);
                set pending = pending.with(id);
                emit Fetch(id);
            }
            case reused when submitted.contains(id) => {
                preserve desired, visible, submitted, pending;
                reject AlreadySubmitted;
            }
        }
        on Complete(id) {
            case current when submitted.contains(id) && pending.contains(id) && desired == Some(id) => {
                preserve desired, submitted;
                set visible = Some(id);
                set pending = pending.without(id);
            }
            case stale when !(submitted.contains(id) && pending.contains(id) && desired == Some(id)) => {
                preserve desired, visible, submitted, pending;
                ignore Stale;
            }
        }
        on Cancel {
            case clear when true => {
                set desired = None;
                set visible = None;
                preserve submitted, pending;
            }
        }
        #[invariant(id = "RESULT-001")]
        fn result_is_current() { visible.is_none() || visible == desired }
    }
}

bbt_yarn::protocol! {
    pub mod BrokenLatestResult {
        domain Id { A, B }
        state {
            desired: Option<Id> = None;
            visible: Option<Id> = None;
            submitted: Set<Id> = Set::empty();
            pending: Set<Id> = Set::empty();
        }
        events { Submit(id: Id); Complete(id: Id); Cancel; }
        effects { Fetch(id: Id); }
        on Submit(id) {
            case fresh when !submitted.contains(id) => {
                set desired = Some(id);
                set visible = None;
                set submitted = submitted.with(id);
                set pending = pending.with(id);
                emit Fetch(id);
            }
            case reused when submitted.contains(id) => {
                preserve desired, visible, submitted, pending;
                reject AlreadySubmitted;
            }
        }
        on Complete(id) {
            // The control removes exactly the pending/current-desire freshness fence.
            case current when submitted.contains(id) => {
                preserve desired, submitted;
                set visible = Some(id);
                set pending = pending.without(id);
            }
            case stale when !submitted.contains(id) => {
                preserve desired, visible, submitted, pending;
                ignore Stale;
            }
        }
        on Cancel {
            case clear when true => {
                set desired = None;
                set visible = None;
                preserve submitted, pending;
            }
        }
        #[invariant(id = "RESULT-001")]
        fn result_is_current() { visible.is_none() || visible == desired }
    }
}

bbt_yarn::requirements! {
    pub ResultContract {
        "RESULT-001" => LatestResult::result_is_current;
    }
}

bbt_yarn::verify! {
    #[test]
    pub fn latest_result_contract() {
        contract: ResultContract;
        protocol: LatestResult;
        exploration: exhaustive;
        require_reachable: [Submit::fresh, Submit::reused, Complete::current, Complete::stale, Cancel::clear];
        limits { states: 100_000; on_exhaustion: inconclusive; }
    }
}

#[cfg(test)]
mod checks {
    use super::{BrokenLatestResult as broken, LatestResult as good};
    use bbt_yarn::verify::{Outcome, Requirement};
    use std::collections::VecDeque;

    bbt_yarn::protocol! {
        mod DormantCase {
            state { enabled: bool = false; }
            events { Tick; }
            on Tick {
                case active when !enabled => { preserve enabled; }
                case dormant when enabled => { preserve enabled; }
            }
            #[invariant(id = "REACH-001")]
            fn remains_dormant() { !enabled }
        }
    }
    bbt_yarn::requirements! {
        ReachContract { "REACH-001" => DormantCase::remains_dormant; }
    }
    bbt_yarn::verify! {
        #[test]
        #[should_panic(expected = "refuted")]
        fn unreachable_case_fails_the_generated_test() {
            contract: ReachContract;
            protocol: DormantCase;
            exploration: exhaustive;
            require_reachable: [Tick::dormant];
            limits { states: 100; on_exhaustion: inconclusive; }
        }
    }

    bbt_yarn::verify! {
        #[test]
        #[should_panic(expected = "inconclusive")]
        fn exhaustion_fails_the_generated_test() {
            contract: super::ResultContract;
            protocol: super::LatestResult;
            exploration: exhaustive;
            require_reachable: [Submit::fresh];
            limits { states: 1; on_exhaustion: inconclusive; }
        }
    }

    bbt_yarn::requirements! { EmptyContract {} }
    mod caller_macro_hygiene {
        // A caller's ordinary macro must not erase the generated test verdict.
        #[allow(unused_macros)]
        macro_rules! assert {
            ($($tokens:tt)*) => {};
        }
        bbt_yarn::verify! {
            #[test]
            #[should_panic(expected = "invalid_input")]
            fn caller_assert_macro_cannot_hide_a_failure() {
                contract: super::EmptyContract;
                protocol: super::super::LatestResult;
                exploration: exhaustive;
                require_reachable: [];
                limits { states: 100; on_exhaustion: inconclusive; }
            }
        }
    }
    bbt_yarn::verify! {
        #[test]
        #[should_panic(expected = "invalid_input")]
        fn silently_dropped_requirement_fails_the_generated_test() {
            contract: EmptyContract;
            protocol: super::LatestResult;
            exploration: exhaustive;
            require_reachable: [];
            limits { states: 100_000; on_exhaustion: inconclusive; }
        }
    }

    bbt_yarn::requirements! {
        MissingContract { "RESULT-002" => super::LatestResult::result_is_current; }
    }
    bbt_yarn::verify! {
        #[test]
        #[should_panic(expected = "invalid_input")]
        fn missing_protocol_claim_fails_the_generated_test() {
            contract: MissingContract;
            protocol: super::LatestResult;
            exploration: exhaustive;
            require_reachable: [];
            limits { states: 100_000; on_exhaustion: inconclusive; }
        }
    }

    bbt_yarn::requirements! {
        BrokenContract { "RESULT-001" => super::BrokenLatestResult::result_is_current; }
    }
    bbt_yarn::verify! {
        #[test]
        #[should_panic(expected = "refuted")]
        fn counterexample_fails_the_generated_test() {
            contract: BrokenContract;
            protocol: super::BrokenLatestResult;
            exploration: exhaustive;
            require_reachable: [Submit::fresh, Complete::current];
            limits { states: 100_000; on_exhaustion: inconclusive; }
        }
    }

    #[test]
    fn results_distinguish_limits_missing_claims_unreachable_cases_and_real_counterexamples() {
        let metadata = good::verification_metadata();
        assert_eq!(metadata.domains[0].variants, ["A", "B"]);
        assert_eq!(metadata.claims[0].id, "RESULT-001");
        assert!(metadata.span.line > 0);
        assert!(metadata.claims[0].span.line >= metadata.span.line);
        for (index, action) in good::model().actions.iter().enumerate() {
            let label = good::action_case(index).unwrap();
            assert!(metadata.cases.contains(&label));
            assert_eq!(
                action.name.rsplit_once("::").unwrap().1,
                label.rsplit_once("::").unwrap().1
            );
        }
        assert_eq!(good::action_case(usize::MAX), None);
        let unreachable = unreachable_case_fails_the_generated_test::check();
        assert_eq!(unreachable.outcome, Outcome::Refuted);
        assert_eq!(unreachable.states, Some(1));
        assert_eq!(
            unreachable.counterexample.unwrap().failed_condition,
            "reachable(Tick::dormant)"
        );
        let result = exhaustion_fails_the_generated_test::check();
        assert_eq!(result.outcome, Outcome::Inconclusive);
        assert_eq!(
            result.states, None,
            "a partial count must not masquerade as completion"
        );
        assert!(!result.is_established());
        let dropped = silently_dropped_requirement_fails_the_generated_test::check();
        assert_eq!(dropped.outcome, Outcome::InvalidInput);
        assert!(
            dropped
                .message
                .contains("unrequired protocol claims [\"RESULT-001\"]")
        );
        let missing = missing_protocol_claim_fails_the_generated_test::check();
        assert_eq!(missing.outcome, Outcome::InvalidInput);
        assert!(missing.message.contains("missing claims [\"RESULT-002\"]"));
        let broken = counterexample_fails_the_generated_test::check();
        let counterexample = broken.counterexample.unwrap();
        assert_eq!(counterexample.failed_condition, "RESULT-001");
        assert_eq!(
            counterexample.events,
            ["Submit(A)", "Submit(B)", "Complete(A)"]
        );
        assert!(counterexample.cycle.is_empty());

        // A declared case exists but its guard never enables in this control.
        let mut disabled = good::model();
        for action in &mut disabled.actions {
            if action.name.ends_with("::current") {
                action.guard = bbt_yarn::model::Expr::Bool(false);
            }
        }
        let result = bbt_yarn::verify::check(
            "ResultContract",
            super::ResultContract::REQUIREMENTS,
            &metadata,
            &disabled,
            &["Complete::current"],
            100_000,
            "disabled guard control",
        );
        assert_eq!(result.outcome, Outcome::Refuted);
        assert_eq!(
            result.counterexample.unwrap().failed_condition,
            "reachable(Complete::current)"
        );
        let wrong_predicate = [Requirement {
            id: "RESULT-001",
            protocol: "LatestResult",
            predicate: "another_predicate",
        }];
        let result = bbt_yarn::verify::check(
            "WrongPredicate",
            &wrong_predicate,
            &metadata,
            &good::model(),
            &[],
            100,
            "wrong predicate",
        );
        assert_eq!(result.outcome, Outcome::InvalidInput);

        let mut temporal = good::model();
        temporal.responses.push(bbt_yarn::model::Response {
            name: "LIVE-001".into(),
            request: bbt_yarn::model::Expr::Bool(true),
            response: bbt_yarn::model::Expr::Bool(true),
        });
        let mut required = super::ResultContract::REQUIREMENTS.to_vec();
        let extra = bbt_yarn::verify::check(
            "Temporal",
            &required,
            &metadata,
            &temporal,
            &[],
            100,
            "unrequired property control",
        );
        assert_eq!(extra.outcome, Outcome::InvalidInput);
        required.push(Requirement {
            id: "LIVE-001",
            protocol: "LatestResult",
            predicate: "eventually_visible",
        });
        let covered = bbt_yarn::verify::check(
            "Temporal",
            &required,
            &metadata,
            &temporal,
            &[],
            100,
            "required property control",
        );
        assert_eq!(covered.outcome, Outcome::Established);
        temporal.responses[0].response = bbt_yarn::model::Expr::Bool(false);
        let refuted = bbt_yarn::verify::check(
            "Temporal",
            &required,
            &metadata,
            &temporal,
            &[],
            100,
            "stuttering response control",
        );
        assert_eq!(refuted.outcome, Outcome::Refuted);
        assert!(!refuted.counterexample.unwrap().cycle.is_empty());
    }

    #[test]
    fn every_reachable_state_and_every_action_edge_replays_the_generated_step() {
        let model = good::model();
        let report = model.check(100_000).unwrap();
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(report.states.len(), 21);
        assert_eq!(report.edges.iter().map(Vec::len).sum::<usize>(), 105);
        let mut runtime = vec![None; report.states.len()];
        runtime[0] = Some(good::State::default());
        let mut queue = VecDeque::from([0]);
        let mut checked_edges = 0;
        while let Some(index) = queue.pop_front() {
            let state = runtime[index].unwrap();
            assert_eq!(state.model_values(), report.states[index]);
            assert!(good::result_is_current(&state));
            // Exactly one case per concrete event, including rejected/ignored self-loops.
            let mut events = Vec::new();
            for edge in &report.edges[index] {
                let event = good::action_event(edge.action).unwrap();
                assert!(!events.contains(&event));
                events.push(event);
                let next = match good::step(&state, event) {
                    good::Decision::Accepted { next, effects } => {
                        match event {
                            good::Event::Submit(id) => {
                                assert_eq!(effects, [good::Effect::Fetch(id)])
                            }
                            _ => assert!(effects.is_empty()),
                        }
                        next
                    }
                    good::Decision::Rejected { reason } => {
                        assert_eq!(reason, good::RejectReason::AlreadySubmitted);
                        assert!(matches!(event, good::Event::Submit(_)));
                        state
                    }
                    good::Decision::Ignored { reason } => {
                        assert_eq!(reason, good::IgnoreReason::Stale);
                        assert!(matches!(event, good::Event::Complete(_)));
                        state
                    }
                };
                assert_eq!(
                    state.model_values(),
                    report.states[index],
                    "input is immutable"
                );
                assert_eq!(
                    next.model_values(),
                    report.states[edge.to],
                    "{}",
                    model.actions[edge.action].name
                );
                if let Some(previous) = runtime[edge.to] {
                    assert_eq!(next, previous);
                } else {
                    runtime[edge.to] = Some(next);
                    queue.push_back(edge.to);
                }
                checked_edges += 1;
            }
            assert_eq!(events.len(), 5);
        }
        assert!(runtime.iter().all(Option::is_some));
        assert_eq!(checked_edges, 105);
    }

    #[test]
    fn broken_control_counterexample_replays_submit_a_submit_b_complete_a() {
        let model = broken::model();
        let report = model.check(100_000).unwrap();
        let violation = report
            .violations
            .iter()
            .find(|v| v.property == "RESULT-001")
            .unwrap();
        assert!(violation.cycle.is_empty());
        let mut state = broken::State::default();
        let mut actions = Vec::new();
        for pair in violation.prefix.windows(2) {
            let edge = report.edges[pair[0]]
                .iter()
                .find(|e| e.to == pair[1])
                .unwrap();
            let event = broken::action_event(edge.action).unwrap();
            actions.push(event);
            let broken::Decision::Accepted { next, .. } = broken::step(&state, event) else {
                panic!("counterexample must be accepted")
            };
            state = next;
            assert_eq!(state.model_values(), report.states[pair[1]]);
        }
        assert_eq!(
            actions,
            [
                broken::Event::Submit(broken::Id::A),
                broken::Event::Submit(broken::Id::B),
                broken::Event::Complete(broken::Id::A)
            ]
        );
        assert_eq!(state.desired(), Some(broken::Id::B));
        assert_eq!(state.visible(), Some(broken::Id::A));
        assert!(!broken::result_is_current(&state));
        assert_eq!(
            state.model_values(),
            report.states[*violation.prefix.last().unwrap()]
        );
    }

    #[test]
    fn cancellation_duplicates_and_identity_reuse_keep_the_current_state() {
        use good::{Decision, Event, Id};
        let initial = good::State::default();
        assert!(matches!(
            good::step(&initial, Event::Complete(Id::A)),
            Decision::Ignored { .. }
        ));
        let Decision::Accepted {
            next: submitted,
            effects,
        } = good::step(&initial, Event::Submit(Id::A))
        else {
            panic!()
        };
        assert_eq!(effects, [good::Effect::Fetch(Id::A)]);
        let Decision::Accepted {
            next: cancelled,
            effects,
        } = good::step(&submitted, Event::Cancel)
        else {
            panic!()
        };
        assert!(effects.is_empty());
        assert_eq!(cancelled.desired(), None);
        assert!(cancelled.pending().contains(Id::A));
        assert!(matches!(
            good::step(&cancelled, Event::Complete(Id::A)),
            Decision::Ignored { .. }
        ));
        assert!(matches!(
            good::step(&cancelled, Event::Submit(Id::A)),
            Decision::Rejected { .. }
        ));
        let Decision::Accepted { next: other, .. } = good::step(&cancelled, Event::Submit(Id::B))
        else {
            panic!()
        };
        assert!(other.submitted().contains(Id::A));
        assert!(
            matches!(
                good::step(&other, Event::Submit(Id::A)),
                Decision::Rejected { .. }
            ),
            "A → B → A is explicitly rejected within this finite identity universe"
        );
        assert!(good::result_is_current(&other));
    }
}
