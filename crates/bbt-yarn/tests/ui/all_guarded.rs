use bbt_yarn::source::FixedQueue;

struct Sources {
    commands: FixedQueue<u8, 4>,
    telemetry: FixedQueue<u8, 4>,
}

struct App {
    accepts_commands: bool,
    accepts_telemetry: bool,
}

// Every arm carries `#[when]`. If both guards snapshot false in one
// arbitration, no source is polled and that arbitration registers no source
// wake. The macro rejects the directly visible zero-poll topology.
async fn run(app: &mut App, sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(commands)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "fixture exercises the all-guarded rejection")]
        #[when(app.accepts_commands)]
        _ = sources.commands => { Ok(bbt_yarn::reactor::Control::Continue) }

        #[source(telemetry)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "fixture exercises the all-guarded rejection")]
        #[when(app.accepts_telemetry)]
        _ = sources.telemetry => { Ok(bbt_yarn::reactor::Control::Stop(())) }
    }
}

fn main() {}
