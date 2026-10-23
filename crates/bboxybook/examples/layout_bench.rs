#![allow(
    clippy::disallowed_methods,
    reason = "headless layout benchmark, not the app UI thread"
)]
//! Time Element::layout on captured inputs, without construction, GPU or IO.
//! layout_bench <cases.json> <relative-case.png> <iterations> [profile-seconds]
//!              [--mode uncached|cold|warm]
//! Warm (default) retains the app-equivalent measurement cache after 3 warmups.
//! Cold empties it before each layout; uncached preserves the R7/R8 method.
use bboxybook::{components, gallery::Case, launchers, swarms};
use bbt_ui::components::Measure;
use bbt_ui_model::layout::{TextMeasure, TextStyle};
use std::cell::Cell;
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Uncached,
    Cold,
    Warm,
}
struct Counted {
    mode: Mode,
    cache: components::CachedMetrics,
    widths: Cell<usize>,
    metrics: Cell<usize>,
    x_heights: Cell<usize>,
}
impl Counted {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            cache: components::CachedMetrics::default(),
            widths: Cell::default(),
            metrics: Cell::default(),
            x_heights: Cell::default(),
        }
    }
    fn prepare(&self) {
        if self.mode == Mode::Cold {
            self.cache.clear();
        } else {
            self.cache.reset_stats();
        }
        self.widths.set(0);
        self.metrics.set(0);
        self.x_heights.set(0);
    }
}
impl TextMeasure for Counted {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        self.widths.set(self.widths.get() + 1);
        if self.mode == Mode::Uncached {
            components::Metrics.width(text, style)
        } else {
            self.cache.width(text, style)
        }
    }
}
impl Measure for Counted {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        self.metrics.set(self.metrics.get() + 1);
        if self.mode == Mode::Uncached {
            components::Metrics.font_metrics(style)
        } else {
            self.cache.font_metrics(style)
        }
    }
    fn x_height(&self, style: &TextStyle) -> f32 {
        self.x_heights.set(self.x_heights.get() + 1);
        if self.mode == Mode::Uncached {
            components::Metrics.x_height(style)
        } else {
            self.cache.x_height(style)
        }
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
    let mode_name = if let Some(i) = args.iter().position(|a| a == "--mode") {
        args.get(i + 1).ok_or("--mode requires a value")?.as_str()
    } else {
        "warm"
    };
    let mode = match mode_name {
        "uncached" => Mode::Uncached,
        "cold" => Mode::Cold,
        "warm" => Mode::Warm,
        _ => return Err("--mode must be uncached, cold or warm".into()),
    };
    let (root, bounds) = if case.component == "launcher" || case.component == "picker" {
        launchers::element_case(case)?
    } else {
        swarms::element_case(case)?
    };
    let ctx = components::context(case);
    let measure = Counted::new(mode);
    for _ in 0..3 {
        measure.prepare();
        black_box(root.layout(bounds, ctx, &measure)?);
    }
    let mut micros = Vec::with_capacity(count);
    let (mut widths, mut fonts, mut x_heights, mut width_misses, mut font_misses) = (0, 0, 0, 0, 0);
    for _ in 0..count {
        // Cache eviction/counter resets and Layout destruction stay outside
        // the timed section, as tree construction did in the R7/R8 method.
        measure.prepare();
        let start = Instant::now();
        let layout = root.layout(bounds, ctx, &measure)?;
        micros.push(start.elapsed().as_secs_f64() * 1e6);
        black_box(layout);
        widths += measure.widths.get();
        fonts += measure.metrics.get();
        x_heights += measure.x_heights.get();
        if mode == Mode::Uncached {
            width_misses += measure.widths.get();
            font_misses += measure.metrics.get();
        } else {
            let stats = measure.cache.stats();
            width_misses += stats.width_misses;
            font_misses += stats.font_misses;
        }
    }
    micros.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        micros[((count as f64 * p).ceil() as usize)
            .saturating_sub(1)
            .min(count - 1)]
    };
    let stats = measure.cache.stats();
    println!(
        "{}",
        serde_json::json!({
            "case":key,"mode":mode_name,"iterations":count,"warmups":3,
            "build":if cfg!(debug_assertions){"debug"}else{"release"},
            "p50_us":percentile(0.5),"p99_us":percentile(0.99),
            "p50_ms":percentile(0.5)/1000.,"p99_ms":percentile(0.99)/1000.,
            "frame_budget_ms":8.33,"layout_budget_ms":2.,
            "p50_layout_budget_multiple":percentile(0.5)/2000.,
            "p99_layout_budget_multiple":percentile(0.99)/2000.,
            "width_calls_per_layout":widths/count,"font_metrics_calls_per_layout":fonts/count,
            "x_height_calls_per_layout":x_heights/count,
            "width_misses_per_layout":width_misses/count,"font_misses_per_layout":font_misses/count,
            "width_hits_per_layout":(widths-width_misses)/count,"font_hits_per_layout":(fonts-font_misses)/count,
            "native_shape_calls_per_layout":(width_misses+font_misses)/count,
            "width_cache_entries":stats.width_entries,"font_cache_entries":stats.font_entries,
            "timed":"Element::layout; tree construction, cache clearing and Layout destruction excluded",
            "scope":"retained tree, instrumented app-equivalent width/font cache; not a complete app frame"
        })
    );
    if let Some(seconds) = args.get(4).filter(|a| !a.starts_with("--")) {
        let until = Instant::now() + Duration::from_secs_f64(seconds.parse()?);
        let mut iterations = 0;
        while Instant::now() < until {
            measure.prepare();
            black_box(root.layout(bounds, ctx, &measure)?);
            iterations += 1;
        }
        eprintln!("profile_iterations={iterations}");
    }
    Ok(())
}
