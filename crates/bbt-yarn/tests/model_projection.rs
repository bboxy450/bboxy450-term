// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(all(feature = "model", feature = "macros", feature = "tokio"))]
use bbt_yarn::reactor::Control;
use bbt_yarn::source::{FixedQueue, Latched};
use bbt_yarn::{reactor, reactor_model};

// One declaration feeds executable Rust and its formal projection: no copied topology.
macro_rules! topology {
    ($backend:ident, $sources:ident) => {
        $backend! {
            policy { selection: biased; required_phases: []; }
            #[source(stop)] #[readiness(quiescent)] #[shutdown]
            _ = $sources.stop => { Ok::<u8,core::convert::Infallible>(1) }
            #[source(output)] #[readiness(may_remain_ready)] #[yields_to(input, when=buffered)] #[drain(max=2)]
            _ = $sources.output => { Ok(Control::Stop(2)) }
            #[source(input)] #[readiness(may_remain_ready)] #[last]
            _ = $sources.input => { Ok(Control::Stop(3)) }
        }
    };
}
struct Sources {
    stop: Latched<()>,
    output: FixedQueue<(), 2>,
    input: FixedQueue<(), 2>,
}
// A private test runtime is below bbt-rt in the dependency layering.
#[allow(clippy::disallowed_methods)]
#[tokio::test]
async fn every_nonempty_readiness_mask_has_the_same_first_selection_in_model_and_runtime() {
    for mask in 1..8 {
        let mut sources = Sources {
            stop: Latched::new(),
            output: FixedQueue::new(),
            input: FixedQueue::new(),
        };
        if mask & 1 != 0 {
            sources.stop.arm(()).unwrap();
        }
        if mask & 2 != 0 {
            sources.output.push(()).unwrap();
        }
        if mask & 4 != 0 {
            sources.input.push(()).unwrap();
        }
        let actual = topology!(reactor, sources).unwrap();
        let mut model = topology!(reactor_model, sources);
        for i in 0..3 {
            model.variables[4 + i].initial = i64::from(mask & (1 << i) != 0);
        }
        let graph = model.check(10_000).unwrap();
        let choices: Vec<_> = graph.edges[0]
            .iter()
            .filter_map(|edge| {
                let state = &graph.states[edge.to];
                (state[0] == 1).then_some(state[1])
            })
            .collect();
        assert_eq!(choices, vec![i64::from(actual)], "readiness mask {mask}");
    }
}
