use bboxybook::{gallery, panes};
use serde_json::json;
use std::collections::BTreeSet;
#[test]
fn pane_registry_pairs_every_current_and_historical_case_without_collisions() {
    let mut all = panes::cases();
    assert_eq!(all.len(), 136);
    let old = panes::legacy_cases();
    assert_eq!(old.len(), 80);
    assert!(
        old.iter()
            .all(|c| c.fixture.as_ref().unwrap()["reference"] == "baseline")
    );
    all.extend(old);
    assert_eq!(
        all.iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        216
    );
    assert_eq!(panes::STRIP_STATES.len(), 11);
    assert_eq!(panes::PANE_STATES.len(), 6);
}
#[test]
fn native_panes_ignore_child_web_bounds_and_font_diagnostics() {
    let mut case = panes::cases()
        .into_iter()
        .find(|c| c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture = Some(
        json!({"layer":"panes","backdrop":{"crop":{"x":17.,"y":25.}},"items":[{"host":{"x":10.,"y":20.,"w":400.},"args":[{"key":"a","name":"Nyx","harness":"codex","state":"active"},{"focused":true,"height":220}],"lines":[[{"key":"pane/term/run0","text":"output"}]],"parts":[{"key":"pane/strip","rect":{"x":4.,"y":4.,"w":20.,"h":9.},"css":{"font":"11px system-ui"}}]}]}),
    );
    let before = format!("{:?}", panes::layouts(&case).unwrap());
    let f = case.fixture.as_mut().unwrap();
    f["items"][0]["host"]["h"] = json!(9999.);
    f["items"][0]["parts"][0]["rect"] = json!({"x":8888.,"y":-999.,"w":1.,"h":500.});
    f["items"][0]["parts"][0]["css"]["font"] = json!("85px fantasy");
    assert_eq!(before, format!("{:?}", panes::layouts(&case).unwrap()));
    assert!(
        gallery::cases()
            .iter()
            .any(|c| c.component == "title_strip")
    );
}
#[test]
fn legacy_pane_calls_the_retained_composition_and_the_old_strip_painter_is_deleted() {
    let source = include_str!("../src/modules.rs");
    assert!(!source.contains("fn strip("));
    let body = source
        .split("pub(crate) fn pane(")
        .nth(1)
        .unwrap()
        .split("pub(crate) fn titlebar(")
        .next()
        .unwrap();
    assert!(body.contains("paint_legacy_pane"));
    for old in ["out.rect", "out.text", "Prim::", "StateGlyph"] {
        assert!(!body.contains(old));
    }
}

#[test]
fn legacy_pane_accepts_a_valid_hex_colour_after_its_hsl_round_trip() {
    use bbt_ui_model::{
        SwarmId,
        draw::{Color, DrawCtx, DrawList},
        layout::Rect,
    };
    let ctx = bbt_ui::Context::new(gallery::theme("dark"), gallery::density("touch"));
    let seat = bboxybook::fixtures::seat(
        &json!({
            "key":"sc0", "name":"Vesper Greywind", "harness":"claude", "role":"oracle",
            "title":"swarm control", "state":"active", "grade":"reported", "color":"#FFC53D"
        }),
        SwarmId::from_u128(1),
        ctx.theme,
    );
    let hsl = seat.color.resolve("sc0", ctx.theme);
    let rgba = Color::hsl(hsl.hue, hsl.saturation / 100., hsl.lightness / 100.).0;
    println!("valid hex #FFC53D -> HSL {hsl:?} -> RGBA {rgba:?}");
    bbt_ui::Paint::solid(bbt_ui::Color::Rgba(rgba))
        .resolve(ctx)
        .unwrap();
    let mut draw = DrawList::default();
    panes::paint_legacy_pane(
        &seat,
        Rect::new(14., 14., 700., 222.),
        false,
        &DrawCtx::new(ctx.theme, ctx.density),
        &mut draw,
    );
    draw.validate().unwrap();
}
