use bbt_yarn::reactor::Control;
use bbt_yarn::source::Latched;

struct Sources {
    voice: Latched<()>,
    telemetry: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(voice)]
        #[readiness(quiescent)]
        #[starvation(allowed, reason = "voice remains intentionally best effort")]
        #[last]
        _ = sources.voice => { Ok(Control::Continue) }

        #[source(telemetry)]
        #[readiness(quiescent)]
        #[starvation(allowed, reason = "telemetry remains intentionally best effort")]
        _ = sources.telemetry => { Ok(Control::Continue) }
    }
}

fn main() {}
