use bbt_yarn::reactor::Control;
use bbt_yarn::source::{self, FixedQueue};

struct Sources {
    stream: FixedQueue<u8, 4>,
    completion: source::OneShot<core::future::Ready<()>>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[yields_to(completion, when = buffered)]
        _ = sources.stream => { Ok(Control::Continue) }

        #[source(completion)]
        #[readiness(quiescent)]
        _ = sources.completion => { Ok(Control::Continue) }
    }
}

fn main() {}
