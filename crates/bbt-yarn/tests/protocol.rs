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

#[cfg(test)]
mod checks {
    use super::{BrokenLatestResult as broken, LatestResult as good};
    use std::collections::VecDeque;

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
