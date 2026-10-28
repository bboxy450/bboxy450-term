// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! F5 perf HUD host. Only the meter record each C.perfHud received and its outer
//! position enter layout; the width is the module's own (the spark's min-content).
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{components as c, modules::perf_hud::PerfHud};
use bbt_ui_model::{draw::DrawCtx, layout::Rect};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

/// perf.html states, less `shapes` (the divergence sketches, not the module).
pub const STATES: &[&str] = &[
    "steady · 120 Hz",
    "hitch · layout",
    "hitch · present",
    "idle",
    "60 Hz display",
    "hud alone",
];
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "perf".into(),
                    state: (*state).into(),
                    slug: gallery::slug(state),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "perf.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 480,
                    checker: false,
                    fixture: Some(json!({"layer":"perf"})),
                });
            }
        }
    }
    out
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("Perf fixture missing")?;
    let ctx = crate::components::context(case);
    let crop = &f["backdrop"]["crop"];
    f["frames"]
        .as_array()
        .ok_or("Perf frames missing")?
        .iter()
        .enumerate()
        .map(|(h, frame)| {
            let hud: PerfHud = serde_json::from_value(frame["record"].clone())?;
            let host = &frame["host"];
            hud.element(ctx, &format!("hud{h}"))
                .layout(
                    Rect::new(
                        seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
                        seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                        PerfHud::width(ctx),
                        0.,
                    ),
                    ctx,
                    &crate::components::Metrics,
                )
                .map_err(Into::into)
        })
        .collect()
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("Perf fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Perf backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    for layout in layout_case(case)? {
        pixels = crate::components::execute(
            renderer,
            c::paint_with_ancestor_clips(&layout, DrawCtx::new(ctx.theme, ctx.density), &[]),
            pixels,
            [w, h],
            ctx,
        )?;
    }
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Perf crop outside backdrop".into());
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
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "perf"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let mut parts = vec![];
        for (frame, layout) in f["frames"]
            .as_array()
            .ok_or("Perf frames missing")?
            .iter()
            .zip(layout_case(case)?)
        {
            let mut names = BTreeMap::new();
            seat_rows::named(&layout, &mut names);
            for p in frame["parts"]
                .as_array()
                .ok_or("Perf diagnostics missing")?
            {
                let key = p["key"].as_str().ok_or("Perf part key missing")?;
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
