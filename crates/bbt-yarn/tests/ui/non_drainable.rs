use bbt_yarn::reactor::Control;
use bbt_yarn::source::Latched;

struct Sources {
    touch: Latched<u8>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(touch)]
        #[readiness(quiescent)]
        #[drain(max = 4)]
        _ = sources.touch => { Ok(Control::Continue) }
    }
}

fn main() {}
