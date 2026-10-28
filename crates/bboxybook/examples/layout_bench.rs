// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
use bbt_ui::components::{self as c, Measure};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
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

fn full_layout(e: &mut c::Element) {
    match e {
        c::Element::ScrollView(s) => {
            if let c::Element::FixedHeight { content, .. } = s.content.as_mut() {
                *s.content = std::mem::replace(content.as_mut(), c::Element::Spacer);
            }
        }
        c::Element::FixedHeight { content, .. }
        | c::Element::Named { content, .. }
        | c::Element::Filter { content, .. } => full_layout(content),
        c::Element::Box(b) => {
            for item in &mut b.children {
                full_layout(&mut item.element);
            }
        }
        c::Element::Stack(s) => {
            for item in &mut s.children {
                full_layout(&mut item.element);
            }
        }
        _ => {}
    }
}
fn verify_windowing(
    root: &c::Element,
    bounds: Rect,
    ctx: bbt_ui::Context,
    measure: &dyn Measure,
) -> bboxybook::Result<serde_json::Value> {
    use std::collections::BTreeMap;
    fn collect<'a>(
        n: &'a c::Layout,
        clip: Option<Rect>,
        names: &mut BTreeMap<String, (&'a c::Layout, bool)>,
        scrolls: &mut Vec<String>,
    ) {
        if let c::LayoutKind::Named(key) = &n.kind {
            let visible = clip.is_some_and(|c| {
                n.rect.x < c.right()
                    && n.rect.right() > c.x
                    && n.rect.y < c.bottom()
                    && n.rect.bottom() > c.y
            });
            names.insert(key.clone(), (n, visible));
        }
        let clip = if let c::LayoutKind::ScrollView { content, offset } = n.kind {
            scrolls.push(format!(
                "{:?}:{:?}:{:?}",
                n.rect,
                content.map(f32::to_bits),
                offset.map(f32::to_bits)
            ));
            Some(n.rect)
        } else {
            clip
        };
        for child in &n.children {
            collect(child, clip, names, scrolls);
        }
    }
    let mut full = root.clone();
    full_layout(&mut full);
    let full = full.layout(bounds, ctx, measure)?;
    let windowed = root.layout(bounds, ctx, measure)?;
    let (mut a, mut b, mut sa, mut sb) = (BTreeMap::new(), BTreeMap::new(), vec![], vec![]);
    collect(&full, None, &mut a, &mut sa);
    collect(&windowed, None, &mut b, &mut sb);
    if sa != sb {
        return Err("windowed viewport, offset or total extent differs".into());
    }
    for (key, (n, visible)) in &a {
        let other = b.get(key);
        if *visible && other.is_none() {
            return Err(format!("visible part omitted: {key}").into());
        }
        if let Some((other, _)) = other {
            let bits = |n: &c::Layout| {
                [n.rect.x, n.rect.y, n.rect.w, n.rect.h, n.baseline].map(f32::to_bits)
            };
            if bits(n) != bits(other) || format!("{:?}", n.kind) != format!("{:?}", other.kind) {
                return Err(format!("part geometry differs: {key}").into());
            }
        }
    }
    Ok(
        serde_json::json!({"full_named":a.len(),"windowed_named":b.len(),"visible_named":a.values().filter(|(_,v)|*v).count(),"extents":sa,"visible_geometry_bit_identical":true}),
    )
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
    let build = Instant::now();
    let (mut root, bounds) = if case.component == "launcher" || case.component == "picker" {
        launchers::element_case(case)?
    } else {
        swarms::element_case(case)?
    };
    let construction_ms = build.elapsed().as_secs_f64() * 1000.;
    if args.iter().any(|s| s == "--full") {
        full_layout(&mut root);
    }
    let ctx = components::context(case);
    let measure = Counted::new(mode);
    let verification = if args.iter().any(|s| s == "--verify-windowing") {
        Some(verify_windowing(&root, bounds, ctx, &measure)?)
    } else {
        None
    };
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
            "full_layout":args.iter().any(|s|s=="--full"),"construction_ms":construction_ms,
            "windowing_verification":verification,
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
