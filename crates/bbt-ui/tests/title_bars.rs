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
