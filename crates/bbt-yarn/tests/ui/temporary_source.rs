use bbt_yarn::reactor::Control;

async fn run() -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(packet)]
        #[readiness(quiescent)]
        _ = core::future::pending::<u8>() => { Ok(Control::Continue) }
    }
}

fn main() {}
