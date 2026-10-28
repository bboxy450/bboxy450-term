// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! L7 TitleBar host. Only public constructor data and outer bounds enter layout.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{self as ui, components as c, modules::title_bar::TitleBar};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const STATES: &[&str] = &[
    "default",
    "sidebar closed",
    "long crumb",
    "narrow window",
    "window unfocused",
];
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "titlebar".into(),
                    state: (*state).into(),
                    slug: state.replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "titlebar.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 300,
                    checker: false,
                    fixture: Some(json!({"layer":"titlebars"})),
                });
            }
        }
    }
    out
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| c.component == "title_bar")
        .map(|mut c| {
            c.fixture = Some(json!({"layer":"titlebars","reference":"baseline"}));
            c
        })
        .collect()
}
fn view(f: &Value) -> TitleBar {
    let o = &f["args"][0];
    TitleBar {
        title: o["title"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("bboxy450-term")
            .into(),
        crumb: o["crumb"].as_str().unwrap_or("").into(),
        sidebar_open: o["sidebarOpen"] != false,
        blurred: o["blurred"] == true,
    }
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let f = case.fixture.as_ref().ok_or("TitleBar fixture missing")?;
    let host = &f["host"];
    let crop = &f["backdrop"]["crop"];
    view(f)
        .element(crate::components::context(case))
        .layout(
            Rect::new(
                seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
                seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                seat_rows::number(host, "w")?,
                0.,
            ),
            crate::components::context(case),
            &crate::components::Metrics,
        )
        .map_err(Into::into)
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("TitleBar fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("TitleBar backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    let mut clips = vec![];
    if f["ancestor_clip"].is_object() {
        let external = &f["ancestor_clip"];
        let r = &external["rect"];
        let mut radii = [0.; 4];
        for (i, radius) in radii.iter_mut().enumerate() {
            *radius = external["radii"][i]
                .as_f64()
                .ok_or("TitleBar external clip radius missing")? as f32;
        }
        clips.push(c::Clip {
            rect: Rect::new(
                seat_rows::number(r, "x")? + seat_rows::number(&bg["crop"], "x")?,
                seat_rows::number(r, "y")? + seat_rows::number(&bg["crop"], "y")?,
                seat_rows::number(r, "w")?,
                seat_rows::number(r, "h")?,
            ),
            radii,
        });
    }
    let plan = c::paint_with_ancestor_clips(
        &layout_case(case)?,
        DrawCtx::new(ctx.theme, ctx.density),
        &clips,
    );
    let pixels = crate::components::execute(renderer, plan, pixels, [w, h], ctx)?;
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("TitleBar crop outside backdrop".into());
    }
    let mut out = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(out)
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut out = vec![];
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "titlebars")
    }) {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let layout = layout_case(case)?;
        let mut names = BTreeMap::new();
        seat_rows::named(&layout, &mut names);
        let mut parts = vec![];
        for p in f["parts"]
            .as_array()
            .ok_or("TitleBar diagnostics missing")?
        {
            let key = p["key"].as_str().ok_or("TitleBar diagnostic key missing")?;
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
            parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text{json!({"native":names.get(key).and_then(|n|seat_rows::text_baseline(n)).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})}else{Value::Null},"children":[]}));
        }
        out.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width as f32,case.height as f32],"web":[0.,0.,case.width as f32,case.height as f32],"children":parts}}));
    }
    Ok(out)
}
pub fn render(out: &Path) -> Result<()> {
    let mut all = cases();
    all.extend(legacy_cases());
    crate::materials::render_gallery(out, &all)
}
pub fn paint_legacy(r: Rect, draw: &DrawCtx, out: &mut DrawList) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let mut layout = TitleBar {
        crumb: "swarm  /  Vesper Greywind".into(),
        ..Default::default()
    }
    .element(ctx)
    .layout(
        Rect::new(r.x, r.y, r.w, 0.),
        ctx,
        &crate::components::Metrics,
    )
    .expect("legacy TitleBar");
    // The old flat gallery cannot sample backdrop materials. Its retained
    // Style fill remains; the isolated TitleBar gallery executes full Frosted.
    fn flat(node: &mut c::Layout) {
        if let c::LayoutKind::Box { style, .. } = &mut node.kind {
            style.material = ui::Material::Flat;
        }
        for child in &mut node.children {
            flat(child);
        }
    }
    flat(&mut layout);
    crate::components::append_draw(c::paint_with_draw(&layout, *draw), out)
        .expect("flat legacy TitleBar");
}
