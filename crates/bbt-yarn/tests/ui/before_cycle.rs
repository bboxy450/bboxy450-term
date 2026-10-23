use bbt_yarn::reactor::Control;
use bbt_yarn::source::Latched;

struct Sources {
    a: Latched<()>,
    b: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(a)]
        #[readiness(quiescent)]
        #[before(b)]
        _ = sources.a => { Ok(Control::Continue) }

        #[source(b)]
        #[readiness(quiescent)]
        #[before(a)]
        _ = sources.b => { Ok(Control::Continue) }
    }
}

fn main() {}
