//! T7's whole-window recipe. Public sidebar/split/pane inputs are shared;
//! native layout never consumes a descendant web rectangle.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{
    self as ui, components as c,
    modules::{closed_strip::ClosedStrip, seat_row::named, stream_dock::Message},
};
use bbt_ui_model::{
    STYLE, SeatId, SplitNode, TOKENS,
    draw::DrawCtx,
    layout::{Axis, Rect, stage_split},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in ["open", "in grace", "undone", "gone"] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "close".into(),
                    state: format!("T7 · {state}"),
                    slug: state.replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "close.html".into(),
                    selector: ".card > .body > div > div:first-child".into(),
                    width: 1246,
                    height: 560,
                    checker: false,
                    fixture: Some(json!({"layer":"close"})),
                });
            }
        }
    }
    out
}
fn num(v: &Value, k: &str) -> Result<f32> {
    seat_rows::number(v, k)
}
fn split(v: &Value, ids: &BTreeMap<String, SeatId>) -> Result<SplitNode> {
    if let Some(k) = v["key"].as_str() {
        return Ok(SplitNode::Leaf(*ids.get(k).ok_or("unknown T7 pane")?));
    }
    Ok(SplitNode::split(
        if v["dir"] == "v" {
            Axis::Row
        } else {
            Axis::Column
        },
        v["ratio"].as_f64().ok_or("missing split ratio")? as f32,
        split(&v["a"], ids)?,
        split(&v["b"], ids)?,
    ))
}
struct Node {
    ctx: ui::Context,
    layout: c::Layout,
}
fn layouts(case: &Case) -> Result<Vec<Node>> {
    let f = case.fixture.as_ref().ok_or("missing T7 fixture")?;
    let ctx = crate::components::context(case);
    let m = crate::components::CachedMetrics::default();
    let crop = &f["backdrop"]["crop"];
    let x = num(crop, "x")?;
    let y = num(crop, "y")?;
    let w = num(&f["host"], "w")?;
    let h = num(&f["host"], "h")?;
    let hair = STYLE.material.hairline_px as f32;
    let mut shell = ui::Style::new(ui::Preset::Card);
    shell.overrides.padding = Some([ui::Length::Px(0.); 4]);
    shell.overrides.height = Some(ui::Length::Px(h));
    shell.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    shell.overrides.radii = Some([ui::Length::Px(STYLE.radius.card as f32); 4]);
    shell.overrides.border = Some(ui::Border {
        width: ui::Length::Px(hair),
        paint: ui::Paint::solid(ui::Color::Line),
        outside: false,
    });
    let mut out = vec![Node {
        ctx,
        layout: named("frame", c::Element::Box(c::Box::new(shell, vec![]))).layout(
            Rect::new(x, y, w, h),
            ctx,
            &m,
        )?,
    }];
    let sbw = num(f, "sidebar_width")?;
    let mut sb = crate::swarms::sidebar(
        &f["model"],
        &f["options"],
        [sbw, h - 2. * hair],
        false,
        0.,
        ctx,
    )?;
    let card = ui::Style::new(ui::Preset::SwarmCard).resolve(ctx)?;
    let row = ui::Style::new(ui::Preset::CardRow).resolve(ctx)?;
    let sw = (sbw
        - 2. * STYLE.spacing.s3 as f32
        - hair
        - TOKENS.sidebar.edge as f32
        - card.padding[1]
        - card.padding[3]
        - row.margin[1]
        - row.margin[3])
        .max(0.);
    for (card, space) in sb
        .cards
        .iter_mut()
        .zip(f["model"]["spaces"].as_array().ok_or("missing spaces")?)
    {
        for closed in space["closed"].as_array().into_iter().flatten() {
            let s = &closed["seat"];
            card.extra.push(
                ClosedStrip {
                    key: s["key"].as_str().ok_or("missing closed key")?.into(),
                    name: s["name"].as_str().ok_or("missing closed name")?.into(),
                    harness: s["harness"]
                        .as_str()
                        .ok_or("missing closed harness")?
                        .into(),
                    remaining_ms: closed["remaining_ms"]
                        .as_f64()
                        .ok_or("missing grace pose")? as u64,
                    grace_ms: closed["graceMs"].as_u64().ok_or("missing grace duration")?,
                }
                .element(ctx, &m, sw),
            );
        }
    }
    let sidebar =
        sb.element(ctx)
            .layout(Rect::new(x + hair, y + hair, sbw, h - 2. * hair), ctx, &m)?;
    out.push(Node {
        ctx,
        layout: sidebar,
    });
    let stage = Rect::new(x + hair + sbw, y + hair, w - 2. * hair - sbw, h - 2. * hair);
    out.push(Node {
        ctx,
        layout: {
            let mut style = ui::Style::new(ui::Preset::Card);
            style.overrides.height = Some(ui::Length::Px(stage.h));
            style.overrides.padding = Some([ui::Length::Px(0.); 4]);
            named("stage", c::Element::Box(c::Box::new(style, vec![]))).layout(stage, ctx, &m)?
        },
    });
    let items = f["panes"].as_array().ok_or("missing T7 panes")?;
    let ids: BTreeMap<_, _> = items
        .iter()
        .enumerate()
        .map(|(i, p)| {
            Ok((
                p["args"][0]["key"]
                    .as_str()
                    .ok_or("missing pane key")?
                    .to_string(),
                SeatId::from_u128(i as u128),
            ))
        })
        .collect::<Result<_>>()?;
    let tree = split(&f["tree"], &ids)?;
    let solved = stage_split(&tree, stage, TOKENS.glow.rim as f32);
    for (id, r) in solved.panes {
        let item = &items[id.0.0 as usize];
        let mut pane = crate::panes::view(item)?;
        pane.min_height = r.h;
        let mut pctx = ctx;
        pctx.focused = item["args"][1]["focused"] == true;
        pctx.seat = ui::Paint::solid(pane.strip.seat.color(ctx)).resolve(ctx)?;
        out.push(Node {
            ctx: pctx,
            layout: pane.element(pctx).layout(r, pctx, &m)?,
        });
    }
    if f["notice"].is_object() {
        let msg: Message = serde_json::from_value(f["notice"].clone())?;
        let dock = ui::modules::stream_dock::StreamDock {
            dock: true,
            framed: false,
            ..Default::default()
        };
        let element = dock.frame(
            ctx,
            vec![
                named(
                    "notice",
                    msg.element(ctx, "notice/content", true, false, &m),
                )
                .into(),
            ],
        );
        let mw = sbw - hair - TOKENS.sidebar.edge as f32;
        let mh = element.natural(mw, ctx, &m)?.0[1];
        out.push(Node {
            ctx,
            layout: element.layout(
                Rect::new(
                    x + hair + TOKENS.sidebar.edge as f32,
                    y + h - hair - mh,
                    mw,
                    mh,
                ),
                ctx,
                &m,
            )?,
        });
    }
    Ok(out)
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("missing close fixture")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("missing close backdrop")?,
    ))?;
    let nodes = layouts(case)?;
    let frame = nodes[0].layout.rect;
    let hair = STYLE.material.hairline_px as f32;
    let clip = c::Clip {
        rect: Rect::new(
            frame.x + hair,
            frame.y + hair,
            frame.w - 2. * hair,
            frame.h - 2. * hair,
        ),
        radii: [(STYLE.radius.card as f32 - hair).max(0.); 4],
    };
    let mut rims = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if i == 2 {
            continue;
        }
        let plan = if i == 0 {
            c::paint(&node.layout, node.ctx)
        } else {
            c::paint_with_ancestor_clips(
                &node.layout,
                DrawCtx::new(node.ctx.theme, node.ctx.density),
                std::slice::from_ref(&clip),
            )
        };
        let (body, rim) = c::split_rims(plan);
        pixels = crate::components::execute_at(renderer, body, pixels, [w, h], node.ctx, 0.)?;
        rims.push((node.ctx, rim));
    }
    for (ctx, rim) in rims {
        pixels = crate::components::execute_at(renderer, rim, pixels, [w, h], ctx, 0.)?;
    }
    let x = num(&bg["crop"], "x")?.round() as u32;
    let y = num(&bg["crop"], "y")?.round() as u32;
    let mut out = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(out)
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut out = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "close"))
    {
        let f = case.fixture.as_ref().unwrap();
        let nodes = layouts(case)?;
        let crop = &f["backdrop"]["crop"];
        let offset = [num(crop, "x")?, num(crop, "y")?];
        let mut names = BTreeMap::new();
        for node in &nodes {
            seat_rows::named(&node.layout, &mut names);
        }
        let mut parts = vec![];
        for p in f["parts"].as_array().ok_or("missing close diagnostics")? {
            let key = p["key"].as_str().ok_or("missing close part")?;
            let n = if key == "frame" {
                Some(&nodes[0].layout)
            } else if key == "sidebar" {
                Some(&nodes[1].layout)
            } else {
                names.get(key).copied().or_else(|| {
                    names
                        .iter()
                        .find(|(name, _)| name.ends_with(key))
                        .map(|(_, n)| *n)
                })
            };
            parts.push(part(key, p, n, offset)?);
        }
        for (i, item) in f["panes"].as_array().unwrap().iter().enumerate() {
            let mut names = BTreeMap::new();
            seat_rows::named(&nodes[3 + i].layout, &mut names);
            for p in item["parts"].as_array().ok_or("missing pane geometry")? {
                let key = p["key"].as_str().ok_or("missing pane part")?;
                parts.push(part(
                    &format!("pane{i}/{key}"),
                    p,
                    names.get(key).copied(),
                    offset,
                )?);
            }
        }
        out.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width,case.height],"web":[0.,0.,case.width,case.height],"children":parts}}));
    }
    Ok(out)
}
fn part(key: &str, p: &Value, n: Option<&c::Layout>, offset: [f32; 2]) -> Result<Value> {
    let r = &p["rect"];
    let web = [num(r, "x")?, num(r, "y")?, num(r, "w")?, num(r, "h")?];
    let text = p["kind"] == "text";
    Ok(
        json!({"key":key,"kind":if text {"text"}else{"box"},"native":n.map(|n|[n.rect.x-offset[0],n.rect.y-offset[1],n.rect.w,n.rect.h]),"web":web,"baseline":if text {json!({"native":n.and_then(seat_rows::text_baseline).map(|b|b-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})} else {Value::Null},"children":[]}),
    )
}
