use bbt_yarn::reactor::Control;

struct VendorWaiter;
struct Sources {
    touch_irq: VendorWaiter,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    bbt_yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(touch_irq)]
        #[readiness(quiescent)]
        _ = sources.touch_irq => { Ok(Control::Continue) }
    }
}

fn main() {}
