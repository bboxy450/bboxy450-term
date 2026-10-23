#![allow(dead_code)]

//! Doc comments above phases and source arms are accepted as source-side
//! rationale and intentionally not emitted into the expansion. Agents
//! following the context-reconstructible-source rule write rationale exactly
//! here; rejecting `///` would punish the documented practice.

use bbt_yarn::reactor::Control;
use bbt_yarn::source::{FixedQueue, Latched};

struct Sources {
    stop: Latched<()>,
    events: FixedQueue<u8, 4>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: [after_event]; }

        /// Shutdown stays first; the macro enforces the leading prefix.
        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => { Ok(()) }

        /// Telemetry is deliberately best effort in this fixture.
        #[source(events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "fixture telemetry is best effort")]
        _ = sources.events => { Ok(Control::Continue) }

        /// Post-event hook rationale is also legal here.
        after_event { Ok(()) }
    }
}

fn main() {}
