//! F5 perf HUD: the perf.html case matrix and the module's own geometry.
use bboxybook::perf;
use bbt_ui::{
    Context, Density, Theme, components as c,
    modules::perf_hud::{PerfFrame, PerfHud, SPARK_W},
};
use bbt_ui_model::layout::Rect;
use std::collections::{BTreeMap, BTreeSet};

fn named<'a>(node: &'a c::Layout, out: &mut BTreeMap<&'a str, &'a c::Layout>) {
    if let c::LayoutKind::Named(name) = &node.kind {
        out.insert(name, node);
    }
    for ch in &node.children {
        named(ch, out);
    }
}

#[test]
fn perf_cases_cover_six_states_with_ascii_slugs() {
    let cases = perf::cases();
    assert_eq!(cases.len(), 48); // Six states, across four densities and two themes.
    let keys: BTreeSet<_> = cases.iter().map(|c| c.relative()).collect();
    assert_eq!(keys.len(), 48);
    assert!(keys.iter().all(|k| k.is_ascii()));
    assert!(keys.contains("perf/steady-120-hz-cozy-dark.png"));
}

#[test]
fn hud_width_is_the_spark_and_every_part_is_named() {
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut hud = PerfHud {
        hz: 120,
        interval_ms: 8.33,
        budget_ms: 8.33,
        frames: vec![PerfFrame::default(); 120],
        ..Default::default()
    };
    hud.frames[119].idle = true;
    let layout = hud
        .element(ctx, "hud0")
        .layout(
            Rect::new(0., 0., PerfHud::width(ctx), 0.),
            ctx,
            &bboxybook::components::Metrics,
        )
        .unwrap();
    let mut names = BTreeMap::new();
    named(&layout, &mut names);
    assert_eq!(names["hud0/spark"].rect.w, SPARK_W);
    assert_eq!(names["hud0"].rect.w, PerfHud::width(ctx));
    for key in [
        "hud0/head/b",
        "hud0/bar0",
        "hud0/bar119",
        "hud0/budget",
        "hud0/ph4/value",
        "hud0/foot2",
    ] {
        assert!(names.contains_key(key), "{key} missing");
    }
}
