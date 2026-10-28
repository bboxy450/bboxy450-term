// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public Path constructor records in caller-owned gallery slots. SVG path
//! internals and browser geometry do not enter tessellation.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows::number,
};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::{Value, json};
use std::path::Path;

pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in ["cubic", "dashed", "weights"] {
        for scale in [1, 2] {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    out.push(Case {
                        component: "paths".into(),
                        state: state.into(),
                        slug: format!("{state}-{scale}x"),
                        density: gallery::density_name(density).into(),
                        theme: gallery::theme_name(theme).into(),
                        page: format!("paths.html?scale={scale}"),
                        selector: ".card > .body".into(),
                        width: 1246,
                        height: 600,
                        checker: false,
                        fixture: Some(json!({"layer":"paths"})),
                    });
                }
            }
        }
    }
    out
}
pub fn element(input: &Value, scale: f32) -> Result<c::Element> {
    let pt = |v: &Value| -> Result<[f32; 2]> {
        Ok([
            v[0].as_f64().ok_or("path x missing")? as f32 * scale,
            v[1].as_f64().ok_or("path y missing")? as f32 * scale,
        ])
    };
    let points = input["points"]
        .as_array()
        .ok_or("Path points missing")?
        .iter()
        .map(|p| {
            Ok(if !p["move"].is_null() {
                c::PathCommand::Move(pt(&p["move"])?)
            } else if !p["line"].is_null() {
                c::PathCommand::Line(pt(&p["line"])?)
            } else {
                c::PathCommand::Cubic([
                    pt(&p["cubic"][0])?,
                    pt(&p["cubic"][1])?,
                    pt(&p["cubic"][2])?,
                ])
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let s = &input["stroke"];
    let color = match s["color"].as_str().unwrap_or("fg") {
        "fg" => ui::Color::Foreground,
        "fg_dim" => ui::Color::Muted,
        "accent" => ui::Color::Accent,
        "line" => ui::Color::Line,
        v if v.starts_with('#') => ui::Color::Rgba(Color::hex(v).0),
        _ => return Err("unsupported Path colour".into()),
    };
    Ok(c::Element::Path(c::Path {
        size: pt(&input["size"])?,
        points: points.into(),
        fill: None,
        paint: ui::Paint::solid(color),
        opacity: s["opacity"].as_f64().unwrap_or(1.) as f32,
        stroke: c::PathStroke {
            width: s["width"].as_f64().unwrap_or(1.) as f32 * scale,
            dash: s["dash"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| {
                            v.as_f64()
                                .map(|n| n as f32 * scale)
                                .ok_or("invalid path dash")
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default()
                .into(),
            cap: match s["cap"].as_str() {
                None | Some("butt") => c::LineCap::Butt,
                Some("round") => c::LineCap::Round,
                Some("square") => c::LineCap::Square,
                _ => return Err("invalid Path cap".into()),
            },
            join: match s["join"].as_str() {
                None | Some("miter") => c::LineJoin::Miter,
                Some("round") => c::LineJoin::Round,
                Some("bevel") => c::LineJoin::Bevel,
                _ => return Err("invalid Path join".into()),
            },
        },
    }))
}
pub fn draw(case: &Case) -> Result<DrawList> {
    let f = case.fixture.as_ref().ok_or("path fixture missing")?;
    let crop = &f["backdrop"]["crop"];
    let ctx = crate::components::context(case);
    let mut list = DrawList::default();
    for path in f["paths"].as_array().ok_or("Path constructors missing")? {
        let host = &path["host"];
        let element = element(
            &path["options"],
            path["scale"].as_f64().ok_or("Path scale missing")? as f32,
        )?;
        let layout = element.layout(
            Rect::new(
                number(host, "x")? + number(crop, "x")?,
                number(host, "y")? + number(crop, "y")?,
                element
                    .natural(f32::INFINITY, ctx, &crate::components::Metrics)?
                    .0[0],
                0.,
            ),
            ctx,
            &crate::components::Metrics,
        )?;
        crate::components::append_draw(
            c::paint_with_draw(&layout, DrawCtx::new(ctx.theme, ctx.density)),
            &mut list,
        )?;
    }
    Ok(list)
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    use bbt_ui_model::draw::{Prim, path_geometry};
    let mut all = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "paths"))
    {
        let f = case.fixture.as_ref().unwrap();
        let native = draw(case)?;
        let strokes: Vec<_> = native
            .primitives
            .iter()
            .filter_map(|p| {
                if let Prim::Path { points, .. } = p {
                    Some(points)
                } else {
                    None
                }
            })
            .collect();
        let mut parts = vec![];
        for (i, p) in f["paths"]
            .as_array()
            .ok_or("path geometry missing")?
            .iter()
            .enumerate()
        {
            let points = strokes.get(i).ok_or("native path missing")?;
            let (bounds, endpoints) = path_geometry(points);
            parts.push(json!({"kind":"box","key":format!("p{i}/bounds"),"native":bounds,"web":p["geometry"]["bounds"],"children":[]}));
            let web = p["geometry"]["endpoints"]
                .as_array()
                .ok_or("web endpoints missing")?;
            if endpoints.len() != web.len() {
                return Err("path endpoint count differs".into());
            }
            for (j, (n, w)) in endpoints.iter().zip(web).enumerate() {
                parts.push(json!({"kind":"box","key":format!("p{i}/endpoint{j}"),"native":[n[0],n[1],0.,0.],"web":[w[0],w[1],json!(0.),json!(0.)],"children":[]}));
            }
        }
        if strokes.len() != f["paths"].as_array().unwrap().len() {
            return Err("native path count differs".into());
        }
        all.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width,case.height],"web":[0.,0.,case.width,case.height],"children":parts}}));
    }
    Ok(all)
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    use bbt_render::chrome::materials::{self, Backdrop, Scene};
    let bg = &case.fixture.as_ref().ok_or("path fixture missing")?["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Path backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    let pixels = materials::render_composed_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::Rgba(&pixels),
            items: &[],
            width: w,
            height: h,
            scale: 1.,
            time_ms: 0.,
            theme: ctx.theme,
        },
        &draw(case)?,
    )?;
    let x = number(&bg["crop"], "x")?.round() as u32;
    let y = number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Path crop outside backdrop".into());
    }
    let mut out = vec![];
    for row in y..y + case.height {
        let at = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[at..at + case.width as usize * 4]);
    }
    Ok(out)
}

pub fn render(out:&Path)->Result<()> {crate::materials::render_gallery(out,&cases())}
