use bbt_ui::{
    self as ui, components as c,
    modules::{
        launcher::{Item, Launcher},
        link::LinkState,
    },
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, t: &TextStyle) -> f32 {
        s.chars().count() as f32 * t.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, t: &TextStyle) -> [f32; 2] {
        [t.size * 0.8, t.size * 0.2]
    }
}
fn labels(l: &c::Layout) -> String {
    let mut s = if let c::LayoutKind::Text { runs, .. } = &l.kind {
        runs.iter().map(|r| r.text.as_str()).collect()
    } else {
        String::new()
    };
    for child in &l.children {
        s += &labels(child)
    }
    s
}
#[test]
fn all_link_states_render_and_unavailable_launcher_rows_cannot_activate() {
    for (state, text, down) in [
        (LinkState::Offline, "offline", true),
        (LinkState::Connecting, "connecting…", false),
        (LinkState::Connected { rtt_ms: 4 }, "4 ms", false),
        (LinkState::Degraded { rtt_ms: 380 }, "slow · 380 ms", false),
        (
            LinkState::Disconnected {
                reason: "link dropped".into(),
            },
            "link dropped",
            true,
        ),
    ] {
        let view = Launcher {
            items: vec![Item {
                label: "host".into(),
                link: Some(state),
                ..Default::default()
            }],
            query: String::new(),
            path: vec![],
            selected: Some(0),
            placeholder: String::new(),
            name: "zsh".into(),
            swatch: [0.; 4],
            actions: vec![],
            cap: None,
            spotlight: true,
            edit: None,
        };
        assert_eq!(view.activation().is_none(), down);
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let layout = view
            .element(ctx)
            .layout(Rect::new(0., 0., 760., 0.), ctx, &Metrics)
            .unwrap();
        assert!(labels(&layout).contains(text));
    }
}
