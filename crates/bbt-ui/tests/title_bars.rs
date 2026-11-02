// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c, modules::title_bar::TitleBar};
use bbt_ui_model::{
    STYLE,
    layout::{Rect, TextMeasure, TextStyle},
};

struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, value: &str, style: &TextStyle) -> f32 {
        value.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn context() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn find<'a>(node: &'a c::Layout, key: &str) -> &'a c::Layout {
    fn walk<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind,c::LayoutKind::Named(k) if k==key) {
            return Some(node);
        }
        node.children.iter().find_map(|n| walk(n, key))
    }
    walk(node, key).unwrap()
}
fn layout(bar: TitleBar, width: f32) -> c::Layout {
    bar.element(context())
        .layout(Rect::new(3., 7., width, 0.), context(), &Metrics)
        .unwrap()
}

#[test]
fn usage_strip_renders_snapshot_hides_absent_accounts_and_retains_error_reason() {
    use bbt_ui_model::usage::{UsageMetric, UsageStrip};
    let metric = |label: &str, percent| UsageMetric {
        label: label.into(),
        percent,
        projected: None,
        windows: Vec::new(),
        stale: false,
        reason: None,
    };
    let mut strip = UsageStrip {
        machine: vec![
            metric("CPU", Some(42)),
            metric("MEM", Some(80)),
            metric("DISK", Some(95)),
        ],
        providers: vec![
            // 40% used but on pace to run out: Tachyon lifts green to yellow.
            UsageMetric {
                projected: Some(120),
                ..metric("claude", Some(40))
            },
            UsageMetric {
                label: "codex".into(),
                percent: None,
                projected: None,
                windows: Vec::new(),
                stale: false,
                reason: Some("Run `codex login`".into()),
            },
            UsageMetric {
                stale: true,
                ..metric("grok", Some(91))
            },
        ],
    };
    assert!(
        strip
            .accessible_label()
            .contains("Codex —: Run `codex login`")
    );
    assert!(
        strip
            .accessible_label()
            .contains("Claude 40% used, at this pace, limit before reset")
    );
    let solve = |strip: &UsageStrip| {
        TitleBar::default()
            .element_with_usage(context(), strip)
            .layout(Rect::new(3., 7., 1600., 0.), context(), &Metrics)
            .unwrap()
    };
    let wide = solve(&strip);
    let debug = format!("{wide:?}");
    for text in [
        "CPU 42%",
        "MEM 80%",
        "DISK 95%",
        "Claude 40%",
        "Codex —",
        "Grok 91% used",
        "Run `codex login`",
    ] {
        assert!(debug.contains(text), "{text}");
    }
    assert!(find(&wide, "bar/usage").rect.right() <= find(&wide, "bar/tbtn1").rect.x);
    fn color(n: &c::Layout) -> [f32; 4] {
        if let c::LayoutKind::Text { runs, .. } = &n.kind {
            return runs[0].style.color;
        }
        color(&n.children[0])
    }
    let waiting = bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.waiting).0;
    let errored = bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.errored).0;
    assert_eq!(color(find(&wide, "bar/usage/MEM")), waiting);
    assert_eq!(color(find(&wide, "bar/usage/DISK")), errored);
    // Tachyon bands color the rings: grok 91% red; claude 40% lifted to yellow by pace.
    use bbt_ui::modules::title_bar::{band_color, usage_breakdown};
    let rgba = |h: &str| Some(bbt_ui::Color::Rgba(bbt_ui_model::draw::Color::hex(h).0));
    assert_eq!(band_color(&strip.providers[2]), rgba("#FF453A"));
    assert_eq!(band_color(&strip.providers[0]), rgba("#FFD60A"));
    for ring in ["bar/usage/claude/ring", "bar/usage/grok/ring"] {
        assert!(debug.contains(ring), "{ring}");
    }
    assert!(!debug.contains(" left"));
    strip.providers.retain(|m| m.label != "grok");
    let layout = solve(&strip);
    assert!(!format!("{layout:?}").contains("bar/usage/grok"));

    // The hover breakdown lists each window with its reset, bar, used and pace.
    strip.providers[0].windows = vec![bbt_ui_model::usage::UsageWindow {
        label: "Weekly".into(),
        percent: 40,
        projected: Some(91),
        resets: Some("Resets Fri 8:00 AM".into()),
    }];
    let breakdown = usage_breakdown(&strip, 300.)
        .layout(Rect::new(0., 0., 300., 0.), context(), &Metrics)
        .unwrap();
    let text = format!("{breakdown:?}");
    for want in ["Claude", "Weekly", "Resets Fri 8:00 AM", "40% Used", "At this pace, ~91% by reset", "Click to refresh accounts"] {
        assert!(text.contains(want), "{want}");
    }
}
// keeps: spec gui-components §1.3
#[test]
fn narrow_bar_wraps_title_and_ellipsizes_crumb_without_shrinking_controls() {
    let make = || TitleBar {
        crumb: "· term · Vesper Greywind".into(),
        ..Default::default()
    };
    let wide = layout(make(), 1000.);
    let narrow = layout(make(), 330.);
    assert_eq!(narrow.rect.h, STYLE.spacing.title_bar as f32);
    for key in ["bar/win", "bar/tbtn", "bar/logo", "bar/tbtn1"] {
        assert_eq!(find(&wide, key).rect.w, find(&narrow, key).rect.w);
        assert!(find(&narrow, key).rect.right() <= narrow.rect.right());
    }
    assert!(find(&narrow, "bar/ttl").rect.h > find(&wide, "bar/ttl").rect.h);
    assert_eq!(
        find(&narrow, "bar/crumb").rect.h,
        find(&wide, "bar/crumb").rect.h
    );
    let crumb = find(&narrow, "bar/crumb");
    assert!(crumb.rect.w < find(&wide, "bar/crumb").rect.w);
    // Ellipsis is a renderer property; retained runs preserve the source text.
    assert!(matches!(
        &crumb.children[0].kind,
        c::LayoutKind::Text { ellipsis: true, .. }
    ));
}
// keeps: spec gui-components §1.3
#[test]
fn blur_filters_only_identity_and_dots_while_sidebar_state_changes_its_toggle() {
    let a = layout(TitleBar::default(), 800.);
    let b = layout(
        TitleBar {
            sidebar_open: false,
            blurred: true,
            ..Default::default()
        },
        800.,
    );
    assert_eq!(find(&a, "bar/logo").rect, find(&b, "bar/logo").rect);
    let icon = |n: &c::Layout, key| {
        let n = find(n, key);
        if let c::LayoutKind::Icon { name, .. } = &n.children[0].kind {
            name.clone()
        } else {
            panic!("icon")
        }
    };
    assert_eq!(icon(&a, "bar/tbtn/hicon"), "sidebarfoxopen");
    assert_eq!(icon(&b, "bar/tbtn/hicon"), "sidebarfox");
    let logo = find(&b, "bar/logo");
    assert!(matches!(
        &logo.children[0].kind,
        c::LayoutKind::Filter {
            saturation: 0.4,
            opacity: 0.8
        }
    ));
    let title_colour = |n: &c::Layout| {
        if let c::LayoutKind::Text { runs, .. } = &find(n, "bar/ttl").children[0].kind {
            runs[0].style.color
        } else {
            panic!("title")
        }
    };
    assert_eq!(title_colour(&a), title_colour(&b));
    let fill = |n: &c::Layout| {
        if let c::LayoutKind::Box { style, .. } = &find(n, "bar/win/i").children[0].kind {
            style.fill
        } else {
            panic!("dot")
        }
    };
    assert_eq!(fill(&a)[3], 1.);
    assert_eq!(fill(&b)[3], 0.6);
}
// keeps: spec gui-components §1.3
#[test]
fn old_titlebar_is_a_component_delegate() {
    let source = include_str!("../../bboxybook/src/modules.rs");
    let body = source
        .split("pub(crate) fn titlebar(")
        .nth(1)
        .unwrap()
        .split("\nfn showcase(")
        .next()
        .unwrap();
    assert!(body.contains("crate::title_bars::paint_legacy"));
    for forbidden in ["out.rect", "out.line", "Prim::", "text(", "icon("] {
        assert!(!body.contains(forbidden));
    }
}
