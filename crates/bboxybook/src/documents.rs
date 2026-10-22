//! Document fixture host. Public document records and parent constraints only.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{components as c, modules::document_modal::DocumentModal};
use bbt_ui_model::{draw::DrawCtx, layout::Rect};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const STATES: &[&str] = &[
    "epic",
    "spec",
    "task",
    "bug",
    "note",
    "invariant",
    "artifact",
    "with a diagram",
    "signed",
    "unsigned",
    "superseded",
    "broken frontmatter",
    "as modal",
    "modal · plain markdown",
];
pub fn cases() -> Vec<Case> {
    let mut all = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                all.push(Case {
                    component: "documents".into(),
                    state: (*state).into(),
                    slug: state.replace(" · ", "-").replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "documents.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 900,
                    checker: false,
                    fixture: Some(json!({"layer":"documents"})),
                });
            }
        }
    }
    let source: Vec<_> = all
        .iter()
        .filter(|c| c.state == "as modal")
        .cloned()
        .map(|mut c| {
            c.slug = "as-modal-source".into();
            c.fixture = Some(json!({"layer":"documents","diagram_source":true}));
            c
        })
        .collect();
    all.extend(source);
    all
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| c.component == "document_modal")
        .map(|mut c| {
            c.component.push_str("-v2");
            c.fixture = Some(json!({"layer":"documents","reference":"baseline"}));
            c
        })
        .collect()
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("Document fixture missing")?;
    let ctx = crate::components::context(case);
    let metrics = crate::settings::Metrics::default();
    let crop = &f["backdrop"]["crop"];
    f["frames"]
        .as_array()
        .ok_or("Document frames missing")?
        .iter()
        .map(|frame| {
            let view: DocumentModal = serde_json::from_value(frame["view"].clone())?;
            let host = &frame["host"];
            let available = [seat_rows::number(host, "w")?, seat_rows::number(host, "h")?];
            let element = view.element(ctx, &metrics, available);
            let width = if view.modal {
                980_f32.min(view.viewport[0] * 0.94).min(available[0])
            } else {
                available[0]
            };
            element
                .layout(
                    Rect::new(
                        seat_rows::number(host, "x")?
                            + seat_rows::number(crop, "x")?
                            + if view.modal {
                                (available[0] - width) / 2.
                            } else {
                                0.
                            },
                        seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                        width,
                        if view.modal { available[1] } else { 0. },
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
    let f = case.fixture.as_ref().ok_or("Document fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Document backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    for (frame, layout) in f["frames"]
        .as_array()
        .ok_or("Document frames missing")?
        .iter()
        .zip(layout_case(case)?)
    {
        let mut clips = vec![];
        if !frame["clip"].is_null() {
            let r = &frame["clip"]["rect"];
            let origin = &bg["crop"];
            let radii: Vec<f32> = frame["clip"]["radii"]
                .as_array()
                .ok_or("Document clip radii missing")?
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
        return Err("Document crop outside backdrop".into());
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
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "documents")
    }) {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let mut parts = vec![];
        for (frame, layout) in f["frames"]
            .as_array()
            .ok_or("Document frames missing")?
            .iter()
            .zip(layout_case(case)?)
        {
            let mut names = BTreeMap::new();
            seat_rows::named(&layout, &mut names);
            for p in frame["parts"]
                .as_array()
                .ok_or("Document diagnostics missing")?
            {
                let key = p["key"].as_str().ok_or("Document part key missing")?;
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
