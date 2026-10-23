#![allow(
    clippy::disallowed_methods,
    reason = "headless layout benchmark, not the app UI thread"
)]
//! R7: time only Element::layout on captured public inputs, without GPU or IO.
//! layout_bench <cases.json> <relative-case.png> <iterations> [profile-seconds]
use bboxybook::{components, gallery::Case, launchers, swarms};
use bbt_ui::components::Measure;
use bbt_ui_model::layout::{TextMeasure, TextStyle};
use std::cell::Cell;
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Counted {
    widths: Cell<usize>,
    metrics: Cell<usize>,
}
impl TextMeasure for Counted {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        self.widths.set(self.widths.get() + 1);
        components::Metrics.width(text, style)
    }
}
impl Measure for Counted {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        self.metrics.set(self.metrics.get() + 1);
        components::Metrics.font_metrics(style)
    }
    fn x_height(&self, style: &TextStyle) -> f32 {
        components::Metrics.x_height(style)
    }
}

fn main() -> bboxybook::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(args.get(1).ok_or("cases.json required")?)?)?;
    let key = args.get(2).ok_or("relative case required")?;
    let case = cases
        .iter()
        .find(|c| &c.relative() == key)
        .ok_or("case absent")?;
    let count: usize = args.get(3).ok_or("iteration count required")?.parse()?;
    if count == 0 {
        return Err("iterations must be positive".into());
    }
    let (root, bounds) = if case.component == "launcher" || case.component == "picker" {
        launchers::element_case(case)?
    } else {
        swarms::element_case(case)?
    };
    let ctx = components::context(case);
    let measure = Counted::default();
    for _ in 0..3 {
        black_box(root.layout(bounds, ctx, &measure)?);
    }
    measure.widths.set(0);
    measure.metrics.set(0);
    let mut micros = Vec::with_capacity(count);
    for _ in 0..count {
        let start = Instant::now();
        let layout = root.layout(bounds, ctx, &measure)?;
        micros.push(start.elapsed().as_secs_f64() * 1e6);
        black_box(layout);
    }
    micros.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        micros[((count as f64 * p).ceil() as usize)
            .saturating_sub(1)
            .min(count - 1)]
    };
    println!(
        "{}",
        serde_json::json!({"case":key,"iterations":count,"p50_us":percentile(0.5),"p99_us":percentile(0.99),"width_calls_per_layout":measure.widths.get()/count,"font_metrics_calls_per_layout":measure.metrics.get()/count,"timed":"Element::layout; tree construction and Layout destruction excluded"})
    );
    if let Some(seconds) = args.get(4) {
        let until = Instant::now() + Duration::from_secs_f64(seconds.parse()?);
        let mut iterations = 0;
        while Instant::now() < until {
            black_box(root.layout(bounds, ctx, &measure)?);
            iterations += 1;
        }
        eprintln!("profile_iterations={iterations}");
    }
    Ok(())
}
