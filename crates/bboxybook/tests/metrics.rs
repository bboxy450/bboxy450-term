use bboxybook::components::{CachedMetrics, Metrics};
use bbt_ui::components::Measure;
use bbt_ui_model::layout::{TextMeasure, TextStyle};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Default)]
struct Probe {
    widths: Rc<Cell<usize>>,
    fonts: Rc<Cell<usize>>,
    x_heights: Rc<Cell<usize>>,
}
impl TextMeasure for Probe {
    fn width(&self, _: &str, _: &TextStyle) -> f32 {
        let n = self.widths.get();
        self.widths.set(n + 1);
        // Values whose exact bits expose a conversion or a recomputation.
        f32::from_bits([0x8000_0000, 0x7fc0_0042, 0x3f80_0001][n % 3])
    }
}
impl Measure for Probe {
    fn font_metrics(&self, _: &TextStyle) -> [f32; 2] {
        let n = self.fonts.get();
        self.fonts.set(n + 1);
        [f32::from_bits(0x4130_0001 + n as u32), -0.]
    }
    fn x_height(&self, _: &TextStyle) -> f32 {
        self.x_heights.set(self.x_heights.get() + 1);
        5.25
    }
}

// keeps: spec renderer.md §4.4a
#[test]
fn cache_hits_preserve_native_float_bits_and_instance_lifetime() {
    let raw = Probe::default();
    let cached = CachedMetrics::new(raw.clone());
    let style = TextStyle::for_density(bbt_ui::Density::Cozy);
    for text in ["A", "A ", "a"] {
        let bits = cached.width(text, &style).to_bits();
        for _ in 0..3 {
            assert_eq!(cached.width(text, &style).to_bits(), bits);
        }
    }
    let bits = cached.font_metrics(&style).map(f32::to_bits);
    assert_eq!(cached.font_metrics(&style).map(f32::to_bits), bits);
    assert_eq!(raw.widths.get(), 3);
    assert_eq!(raw.fonts.get(), 1);
    let stats = cached.stats();
    assert_eq!((stats.width_queries, stats.width_misses), (12, 3));
    assert_eq!((stats.font_queries, stats.font_misses), (2, 1));
    assert_eq!((stats.width_entries, stats.font_entries), (3, 1));

    cached.reset_stats();
    cached.width("A", &style);
    assert_eq!(
        (cached.stats().width_queries, cached.stats().width_misses),
        (1, 0)
    );
    assert_eq!(cached.stats().width_entries, 3);
    cached.clear();
    assert_eq!(cached.stats().width_entries, 0);
    assert_eq!(cached.stats().font_entries, 0);
    cached.width("A", &style);
    assert_eq!(raw.widths.get(), 4);

    let other = CachedMetrics::new(raw.clone());
    other.width("A", &style);
    assert_eq!(raw.widths.get(), 5, "a new owner starts cold");
    assert_eq!(cached.x_height(&style), 5.25);
    assert_eq!(cached.x_height(&style), 5.25);
    assert_eq!(
        raw.x_heights.get(),
        2,
        "x-height stays delegated, as before"
    );
}

// keeps: spec renderer.md §4.4a
#[test]
fn every_app_font_key_field_including_float_bits_is_distinct() {
    let raw = Probe::default();
    let cached = CachedMetrics::new(raw.clone());
    let base = TextStyle::for_density(bbt_ui::Density::Cozy);
    let mut styles = vec![base];
    let mut s = base;
    s.size = f32::from_bits(s.size.to_bits() + 1);
    styles.push(s);
    let mut s = base;
    s.weight += 1;
    styles.push(s);
    let mut s = base;
    s.mono = !s.mono;
    styles.push(s);
    let mut s = base;
    s.italic = !s.italic;
    styles.push(s);
    let mut s = base;
    s.tracking_em = 0.16;
    styles.push(s);
    let mut s = base;
    s.tracking_em = -0.;
    styles.push(s);
    for style in &styles {
        let width = cached.width("same label", style).to_bits();
        let font = cached.font_metrics(style).map(f32::to_bits);
        assert_eq!(cached.width("same label", style).to_bits(), width);
        assert_eq!(cached.font_metrics(style).map(f32::to_bits), font);
    }
    assert_eq!(raw.widths.get(), styles.len());
    assert_eq!(raw.fonts.get(), styles.len());
    assert_eq!(cached.stats().width_entries, styles.len());
    assert_eq!(cached.stats().font_entries, styles.len());
}

#[cfg(target_os = "macos")]
// keeps: spec renderer.md §4.4a
#[test]
fn cached_coretext_values_match_the_raw_host_bit_for_bit() {
    let cached = CachedMetrics::default();
    for (text, size, weight, mono, italic, tracking_em) in [
        ("Vesper Greywind", 12., 600, false, false, 0.),
        ("ffi AV — café", 13.5, 400, false, false, 0.),
        ("via main", 9.5, 500, false, true, 0.),
        ("CODE", 11., 600, true, false, 0.16),
        ("⊘ permission", 9., 400, true, true, 0.),
    ] {
        let style = TextStyle {
            size,
            weight,
            mono,
            italic,
            tracking_em,
        };
        let width = Metrics.width(text, &style).to_bits();
        let metrics = Metrics.font_metrics(&style).map(f32::to_bits);
        for _ in 0..2 {
            assert_eq!(cached.width(text, &style).to_bits(), width);
            assert_eq!(cached.font_metrics(&style).map(f32::to_bits), metrics);
        }
    }
}
