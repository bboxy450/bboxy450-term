// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(feature = "macros")]
#![allow(missing_docs)]

bbt_yarn::protocol! {
    pub mod ProtocolLanguage {
        domain Mode { A, B }
        state {
            left: bool = false;
            right: bool = true;
            mode: Mode = Mode::A;
            selected: Option<Mode> = Some(Mode::B);
            members: Set<Mode> = Set::empty().with(Mode::A);
        }
        events { Swap; Load(id: Mode, active: bool, choice: Option<Mode>, incoming: Set<Mode>); }
        effects { Snapshot(a: bool, b: bool, choice: Option<Mode>, members: Set<Mode>); }
        on Swap {
            case swap when true => {
                set left = right;
                set right = left;
                preserve mode, selected, members;
                emit Snapshot(left, right, selected, members);
                emit Snapshot(right, left, None, members.without(mode));
            }
        }
        on Load(id, active, choice, incoming) {
            case add when active => {
                set left = active;
                set right = !active;
                set mode = id;
                set selected = choice;
                set members = incoming.with(id);
            }
            case remove when !active => {
                set left = active;
                set right = !active;
                set mode = id;
                set selected = choice;
                set members = incoming.without(id);
            }
        }
        #[invariant(id = "BOOL-001")]
        fn complementary() { left != right && (left || right) && !(left && right) }
        #[invariant(id = "SET-001")]
        fn set_algebra() { members.with(mode).contains(mode) && !members.without(mode).contains(mode) && members.with(mode) == members.without(mode).with(mode) }
        #[invariant(id = "OPTION-001")]
        fn option_universe() { selected.is_none() || selected == Some(Mode::A) || selected == Some(Mode::B) }
    }
}

#[cfg(test)]
mod checks {
    use super::ProtocolLanguage as p;

    #[test]
    fn initialization_simultaneous_updates_and_effects_read_the_pre_state() {
        let state = p::State::default();
        assert!(!state.left());
        assert!(state.right());
        assert_eq!(state.selected(), Some(p::Mode::B));
        let p::Decision::Accepted { next, effects } = p::step(&state, p::Event::Swap);
        assert!(next.left());
        assert!(!next.right());
        assert!(!state.left());
        assert_eq!(
            effects,
            [
                p::Effect::Snapshot(false, true, Some(p::Mode::B), state.members()),
                p::Effect::Snapshot(true, false, None, state.members().without(p::Mode::A))
            ]
        );
        let p::Decision::Accepted { next, .. } = p::step(&next, p::Event::Swap);
        assert_eq!(next, state);
    }

    #[cfg(feature = "model")]
    #[test]
    fn all_language_values_and_edges_match_symbolic_lowering() {
        let model = p::model();
        let graph = model.check(100_000).unwrap();
        assert!(graph.violations.is_empty(), "{:?}", graph.violations);
        let mut states = vec![None; graph.states.len()];
        states[0] = Some(p::State::default());
        let mut queue = std::collections::VecDeque::from([0]);
        let mut checked_edges = 0;
        while let Some(i) = queue.pop_front() {
            let state = states[i].unwrap();
            assert_eq!(state.model_values(), graph.states[i]);
            assert!(
                p::complementary(&state) && p::set_algebra(&state) && p::option_universe(&state)
            );
            assert_eq!(graph.edges[i].len(), 49);
            for edge in &graph.edges[i] {
                let p::Decision::Accepted { next, .. } =
                    p::step(&state, p::action_event(edge.action).unwrap());
                assert_eq!(
                    next.model_values(),
                    graph.states[edge.to],
                    "{}",
                    model.actions[edge.action].name
                );
                if let Some(prior) = states[edge.to] {
                    assert_eq!(next, prior);
                } else {
                    states[edge.to] = Some(next);
                    queue.push_back(edge.to);
                }
                checked_edges += 1;
            }
        }
        assert!(states.iter().all(Option::is_some));
        assert_eq!(checked_edges, graph.states.len() * 49);
    }
}
