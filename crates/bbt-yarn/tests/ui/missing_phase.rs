use bbt_yarn::reactor::Control;
use bbt_yarn::source::Latched;

struct Sources {
    event: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: [after_event]; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => { Ok(Control::Continue) }
    }
}

fn main() {}
