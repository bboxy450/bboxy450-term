use bbt_ui::{
    self as ui, components as c,
    modules::launcher::{Cap, Item, Launcher},
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
fn item(id: &str, label: &str) -> Item {
    Item {
        id: id.into(),
        label: label.into(),
        harness: "codex".into(),
        ..Default::default()
    }
}

#[test]
fn enter_picks_is_a_defaulted_public_constructor_input() {
    let ordinary: Item = serde_json::from_str(r#"{"id":"ssh","children":[]}"#).unwrap();
    assert!(!ordinary.enter_picks);
    let harness: Item =
        serde_json::from_str(r#"{"id":"codex","enterPicks":true,"children":[{"id":"codex:tab"}]}"#)
            .unwrap();
    assert!(harness.enter_picks);
    assert!(!harness.children.unwrap()[0].enter_picks);
}
fn launch() -> Launcher {
    let mut remote = item("ssh", "SSH");
    let mut machine = item("ssh:rig", "rig");
    let mut codex = item("ssh:rig:codex", "Codex");
    codex.children = Some(vec![
        item("codex:tab", "New tab"),
        item("codex:right", "Split right"),
        item("codex:bottom", "Split down"),
    ]);
    machine.children = Some(vec![codex]);
    remote.children = Some(vec![machine]);
    Launcher {
        items: vec![remote],
        query: String::new(),
        path: vec![],
        selected: None,
        placeholder: "pick".into(),
        name: "Fenn Calloway".into(),
        swatch: [0.4, 0.6, 0.8, 1.],
        actions: vec![("Open".into(), "↵".into())],
        cap: None,
        spotlight: true,
        edit: None,
    }
}
#[test]
fn typed_virtual_descent_and_explicit_paths_select_the_same_placement_level() {
    let mut a = launch();
    a.query = "ssh rig codex split".into();
    let av = a.view();
    assert_eq!(
        av.crumbs.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        ["ssh", "ssh:rig", "ssh:rig:codex"]
    );
    assert_eq!(
        av.rows.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        ["codex:right", "codex:bottom"]
    );
    let mut b = launch();
    b.path = vec!["ssh".into(), "ssh:rig".into(), "ssh:rig:codex".into()];
    b.query = "split".into();
    assert_eq!(
        b.view()
            .rows
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        av.rows.iter().map(|x| x.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        a.path,
        Vec::<String>::new(),
        "presentation does not mutate navigation state"
    );
}
#[test]
fn lexical_tiers_are_stable_and_every_word_must_match() {
    let mut a = launch();
    let mut facets = item("seat-a", "Nyx");
    facets.tags = vec!["codex".into(), "ssh".into()];
    let mut sub = item("seat-b", "Rook");
    sub.sub = "codex executor · rig".into();
    a.items = vec![
        facets.clone(),
        sub,
        item("co-1", "Codex B"),
        item("co-2", "Codex A"),
    ];
    a.query = "cod".into();
    assert_eq!(
        a.view()
            .rows
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        ["co-1", "co-2", "seat-a", "seat-b"]
    );
    a.query = "codex ssh".into();
    assert_eq!(a.view().rows.len(), 1);
    assert_eq!(a.view().rows[0].id, "seat-a");
    a.query = "nxyz".into();
    assert!(a.view().rows.is_empty());
}
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        t.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
#[test]
fn cap_keeps_navigation_available_and_dims_completed_launch_row_groups() {
    let mut a = launch();
    let mut go = item("goto", "Go to");
    go.navigation = true;
    go.children = Some(vec![]);
    a.items.push(go);
    a.cap = Some(Cap {
        scope: "swarm".into(),
        name: "term".into(),
        used: 8,
        limit: 8,
        counts: "6 running · 1 starting · 1 in grace".into(),
    });
    assert_eq!(a.view().selected, 1);
    assert_eq!(a.view().rows.len(), 2);
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let layout = a
        .element(ctx)
        .layout(Rect::new(0., 0., 760., 0.), ctx, &Metrics)
        .unwrap();
    fn filter(n: &c::Layout) -> usize {
        usize::from(matches!(
            &n.kind,
            c::LayoutKind::Filter { opacity: 0.38, .. }
        )) + n.children.iter().map(filter).sum::<usize>()
    }
    assert_eq!(filter(&layout), 1);
    assert!(
        c::paint(&layout, ctx)
            .iter()
            .any(|p| matches!(p, c::Paint::Group { opacity: 0.38, .. }))
    );
}
#[test]
fn legacy_launcher_delegates_without_hand_painting() {
    let old = include_str!("../../bboxybook/src/modules.rs");
    let body = old
        .split("fn launcher(")
        .nth(1)
        .unwrap()
        .split("\nfn settings(")
        .next()
        .unwrap();
    assert!(body.contains("crate::launchers::paint_legacy"));
    for bad in ["out.rect", "out.line", "Prim::", "text(", "IconTile"] {
        assert!(!body.contains(bad));
    }
}

#[test]
fn compact_selection_is_twenty_eight_percent_accent_over_surface() {
    use bbt_ui_model::draw::Color;
    let mut launcher = launch();
    launcher.spotlight = false;
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let root = launcher
            .element(ctx)
            .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
            .unwrap();
        fn find(n: &c::Layout) -> Option<&c::Layout> {
            if matches!(&n.kind,c::LayoutKind::Named(k) if k=="picker/div/opt") {
                return Some(n);
            }
            n.children.iter().find_map(find)
        }
        let c::LayoutKind::Box { style, .. } = &find(&root).unwrap().children[0].kind else {
            panic!("selected row")
        };
        let surface = Color::hex(theme.palette().surface).0;
        let accent = Color::hex(theme.palette().accent).0;
        for i in 0..4 {
            assert!((style.fill[i] - (surface[i] * 0.72 + accent[i] * 0.28)).abs() < 0.000001);
        }
    }
}
