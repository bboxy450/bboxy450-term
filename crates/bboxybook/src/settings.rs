//! Settings fixture host. Browser child diagnostics never enter native layout.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{self as ui, components as c, modules::settings::Settings};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

// Per-layout memoization keeps repeated intrinsic probes from rasterizing the
// same label thousands of times. Values still come from the native font host.
type FontKey = (u32, u16, bool, bool, u32);
type WidthKey = (String, u32, u16, bool, bool, u32);
#[derive(Default)]
struct Metrics {
    widths: std::cell::RefCell<BTreeMap<WidthKey, f32>>,
    fonts: std::cell::RefCell<BTreeMap<FontKey, [f32; 2]>>,
}
fn metric_key(s: &bbt_ui_model::layout::TextStyle) -> (u32, u16, bool, bool, u32) {
    (
        s.size.to_bits(),
        s.weight,
        s.mono,
        s.italic,
        s.tracking_em.to_bits(),
    )
}
impl bbt_ui_model::layout::TextMeasure for Metrics {
    fn width(&self, v: &str, s: &bbt_ui_model::layout::TextStyle) -> f32 {
        let a = metric_key(s);
        let key = (v.into(), a.0, a.1, a.2, a.3, a.4);
        if let Some(v) = self.widths.borrow().get(&key) {
            return *v;
        }
        let v = bbt_ui_model::layout::TextMeasure::width(&crate::components::Metrics, v, s);
        self.widths.borrow_mut().insert(key, v);
        v
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &bbt_ui_model::layout::TextStyle) -> [f32; 2] {
        let key = metric_key(s);
        if let Some(v) = self.fonts.borrow().get(&key) {
            return *v;
        }
        let v = c::Measure::font_metrics(&crate::components::Metrics, s);
        self.fonts.borrow_mut().insert(key, v);
        v
    }
    fn x_height(&self, s: &bbt_ui_model::layout::TextStyle) -> f32 {
        c::Measure::x_height(&crate::components::Metrics, s)
    }
}
pub const STATES: &[&str] = &[
    "Account",
    "Cloud",
    "Machines",
    "Roles",
    "Harnesses",
    "new harness",
    "Appearance",
    "Shortcuts",
    "in a tab",
];
pub fn cases() -> Vec<Case> {
    let mut all = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                all.push(Case {
                    component: "settings".into(),
                    state: (*state).into(),
                    slug: state.to_lowercase().replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "settings.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 900,
                    checker: false,
                    fixture: Some(json!({"layer":"settings"})),
                });
            }
        }
    }
    all
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| c.component == "settings_tab")
        .map(|mut c| {
            c.component.push_str("-v2");
            c.fixture = Some(json!({"layer":"settings","reference":"baseline"}));
            c
        })
        .collect()
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("Settings fixture missing")?;
    let ctx = crate::components::context(case);
    let metrics = Metrics::default();
    let crop = &f["backdrop"]["crop"];
    f["frames"]
        .as_array()
        .ok_or("Settings frames missing")?
        .iter()
        .map(|frame| {
            let view: Settings = serde_json::from_value(frame["view"].clone())?;
            let host = &frame["host"];
            view.element(
                ctx,
                &metrics,
                seat_rows::number(host, "w")?,
                seat_rows::number(host, "h")?,
            )
            .layout(
                Rect::new(
                    seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
                    seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                    seat_rows::number(host, "w")?,
                    seat_rows::number(host, "h")?,
                ),
                ctx,
                &metrics,
            )
            .map_err(Into::into)
        })
        .collect()
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("Settings fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Settings backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    for (frame, layout) in f["frames"]
        .as_array()
        .ok_or("Settings frames missing")?
        .iter()
        .zip(layout_case(case)?)
    {
        let mut clips = vec![];
        if !frame["clip"].is_null() {
            let r = &frame["clip"]["rect"];
            let origin = &bg["crop"];
            let radii: Vec<f32> = frame["clip"]["radii"]
                .as_array()
                .ok_or("Settings clip radii missing")?
                .iter()
                .map(|n| n.as_f64().map(|n| n as f32).ok_or("invalid clip radius"))
                .collect::<std::result::Result<_, _>>()?;
            clips.push(c::Clip {
                rect: Rect::new(
                    seat_rows::number(r, "x")? + seat_rows::number(origin, "x")?,
                    seat_rows::number(r, "y")? + seat_rows::number(origin, "y")?,
                    seat_rows::number(r, "w")?,
                    seat_rows::number(r, "h")?,
                ),
                radii: radii.try_into().map_err(|_| "four clip radii required")?,
            });
        }
        pixels = crate::components::execute(
            renderer,
            c::paint_with_ancestor_clips(&layout, DrawCtx::new(ctx.theme, ctx.density), &clips),
            pixels,
            [w, h],
            ctx,
        )?;
    }
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Settings crop outside backdrop".into());
    }
    let mut out = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(out)
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut all = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "settings"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let mut parts = vec![];
        for (frame, layout) in f["frames"]
            .as_array()
            .ok_or("Settings frames missing")?
            .iter()
            .zip(layout_case(case)?)
        {
            let mut names = BTreeMap::new();
            seat_rows::named(&layout, &mut names);
            for p in frame["parts"]
                .as_array()
                .ok_or("Settings diagnostics missing")?
            {
                let key = p["key"].as_str().ok_or("Settings part key missing")?;
                let native = names.get(key).map(|n| {
                    json!([
                        n.rect.x - offset[0],
                        n.rect.y - offset[1],
                        n.rect.w,
                        n.rect.h
                    ])
                });
                let r = &p["rect"];
                let web = [
                    seat_rows::number(r, "x")?,
                    seat_rows::number(r, "y")?,
                    seat_rows::number(r, "w")?,
                    seat_rows::number(r, "h")?,
                ];
                let text = p["kind"] == "text";
                parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text{json!({"native":names.get(key).and_then(|n|seat_rows::text_baseline(n)).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1]as f64)})}else{Value::Null},"children":[]}));
            }
        }
        all.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width as f32,case.height as f32],"web":[0.,0.,case.width as f32,case.height as f32],"children":parts}}));
    }
    Ok(all)
}
pub fn render(out: &Path) -> Result<()> {
    crate::materials::render_gallery(out, &cases())
}
pub fn paint_legacy(r: Rect, state: &str, draw: &DrawCtx, out: &mut DrawList) {
    let view = Settings {
        section: state.into(),
        ..Default::default()
    };
    let ctx = ui::Context::new(draw.theme, draw.density);
    let layout = view
        .element(ctx, &crate::components::Metrics, r.w, r.h)
        .layout(r, ctx, &crate::components::Metrics)
        .expect("Settings composition");
    crate::components::append_draw(c::paint_with_draw(&layout, *draw), out)
        .expect("Settings paint");
}
