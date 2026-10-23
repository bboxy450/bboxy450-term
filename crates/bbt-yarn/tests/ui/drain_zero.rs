use bbt_yarn::reactor::Control;
use bbt_yarn::source::FixedQueue;

struct Sources {
    stream: FixedQueue<u8, 4>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "single source drain mutation fixture")]
        #[drain(max = 0)]
        _ = sources.stream => { Ok(Control::Continue) }
    }
}

fn main() {}
