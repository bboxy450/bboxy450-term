use bbt_ui::{
    self as ui, components as c,
    modules::{link::LinkState, settings::Settings},
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
fn settings_accepts_live_link_records_and_legacy_four_field_cloud_records() {
    for cloud in [
        serde_json::json!(["nebula", "east", true, "#2E6BFF"]),
        serde_json::json!(["nebula","east",true,"#2E6BFF",{"Degraded":{"rtt_ms":380}}]),
    ] {
        let view: Settings = serde_json::from_value(
            serde_json::json!({"section":"Cloud","values":{"cloud":[cloud]}}),
        )
        .unwrap();
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let layout = view
            .element(ctx, &Metrics, 1216., 638.)
            .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
            .unwrap();
        assert!(labels(&layout).contains("nebula"));
    }
}

#[test]
fn every_link_state_roundtrips_and_renders_in_both_settings_records() {
    for state in [
        LinkState::Offline,
        LinkState::Connecting,
        LinkState::Connected { rtt_ms: 4 },
        LinkState::Degraded { rtt_ms: 380 },
        LinkState::Disconnected {
            reason: "link dropped".into(),
        },
    ] {
        let value = serde_json::to_value(&state).unwrap();
        for section in ["Cloud", "Machines"] {
            let view:Settings=serde_json::from_value(serde_json::json!({"section":section,"values":{"cloud":[["nebula","east",true,"#2E6BFF",value]],"machines":[["rig","agent@rig","M4","⌘1",value]]}})).unwrap();
            let restored: Settings =
                serde_json::from_value(serde_json::to_value(view).unwrap()).unwrap();
            let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
            let layout = restored
                .element(ctx, &Metrics, 1216., 638.)
                .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
                .unwrap();
            assert!(
                labels(&layout).contains(&state.label()),
                "{section}: {state:?}"
            );
        }
    }
    let _:Settings=serde_json::from_value(serde_json::json!({"section":"Machines","values":{"machines":[["old","agent@old","M2","⌘2",true]]}})).unwrap();
}
