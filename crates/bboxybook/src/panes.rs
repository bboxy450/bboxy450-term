//! L7 Pane/Strip host. Terminal spans are fixture content; descendant rectangles
//! and font diagnostics are consumed only by the independent geometry report.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{
    self as ui, components as c,
    modules::{
        pane::{Pane, Resumed, Span},
        strip::Strip,
    },
};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
pub const STRIP_STATES: &[&str] = &[
    "idle",
    "active",
    "waiting",
    "errored",
    "focused vs unfocused",
    "row + pane breathe together",
    "three seats, one focused",
    "stop · operator",
    "interrupt sent",
    "worker view",
    "authority · revoked",
];
pub const PANE_STATES: &[&str] = &[
    "gap · retained",
    "gap · unavailable",
    "reconnecting",
    "reconnected",
    "⌘-hover path",
    "resumed",
];
fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').into()
}
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for (page, states) in [("title-strip", STRIP_STATES), ("pane", PANE_STATES)] {
        for state in states {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    out.push(Case {
                        component: page.into(),
                        state: (*state).into(),
                        slug: slug(state),
                        density: gallery::density_name(density).into(),
                        theme: gallery::theme_name(theme).into(),
                        page: format!("{page}.html"),
                        selector: ".card > .body".into(),
                        width: 1246,
                        height: 600,
                        checker: false,
                        fixture: Some(json!({"layer":"panes"})),
                    });
                }
            }
        }
    }
    out
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| matches!(c.component.as_str(), "title_strip" | "pane"))
        .map(|mut c| {
            if c.component == "pane" {
                c.component = "pane-v2".into();
            }
            c.fixture = Some(json!({"layer":"panes","reference":"baseline"}));
            c
        })
        .collect()
}
fn num(v: &Value, key: &str) -> Result<f32> {
    v[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .map(|n| n as f32)
        .ok_or_else(|| format!("invalid pane {key}").into())
}
fn view(item: &Value) -> Result<Pane> {
    let v = &item["args"][0];
    let o = &item["args"][1];
    let mut strip = Strip::new(seat_rows::seat(v, o, None));
    strip.waiting_description = v["waitKind"].as_str().map(Into::into);
    strip.may_interrupt = o["onInterrupt"]["$callback"] == true;
    strip.may_mute = o["onMute"]["$callback"] == true;
    let lines = item["lines"]
        .as_array()
        .ok_or("missing semantic terminal lines")?
        .iter()
        .map(|line| {
            line.as_array()
                .ok_or("invalid semantic terminal line")?
                .iter()
                .map(|p| {
                    let key = p["key"].as_str().ok_or("missing terminal span key")?.into();
                    Ok(if p["cursor"] == true {
                        Span::Cursor { key }
                    } else if let Some(parts) = p["hint"].as_array() {
                        Span::Hint {
                            key,
                            parts: parts
                                .iter()
                                .map(|p| {
                                    Ok((
                                        p["key"].as_str().ok_or("hint key")?.into(),
                                        p["text"].as_str().ok_or("hint text")?.into(),
                                        p["kbd"] == true,
                                    ))
                                })
                                .collect::<Result<_>>()?,
                        }
                    } else {
                        Span::Text {
                            key,
                            value: p["text"].as_str().ok_or("missing terminal text")?.into(),
                            foreground: p["foreground"] == true,
                            hover: p["hover"] == true,
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Pane {
        strip,
        lines,
        gap_from: (!v["gap"].is_null()).then(|| v["gap"]["from"].as_str().map(Into::into)),
        resumed: (!v["resumed"].is_null()).then(|| Resumed {
            generation: v["resumed"]["gen"].as_u64().unwrap_or(0),
            command: v["resumed"]["cmd"].as_str().map(Into::into),
        }),
        min_height: o["height"].as_f64().unwrap_or(140.) as f32,
        unfocused: o["unfocused"] == true,
        reconnecting: o["reconnecting"] == true,
        reconnected: o["reconnected"] == true,
        glow_seed: item["glow"]["seed"].as_u64().map(|n| n as u32),
        opacity: bbt_ui_model::TOKENS.window.opacity as f32,
    })
}
pub fn layouts(case: &Case) -> Result<Vec<(ui::Context, c::Layout)>> {
    let f = case.fixture.as_ref().ok_or("missing Pane fixture")?;
    let crop = &f["backdrop"]["crop"];
    f["items"]
        .as_array()
        .ok_or("missing Pane instances")?
        .iter()
        .map(|item| {
            let p = view(item)?;
            let mut ctx =
                ui::Context::new(gallery::theme(&case.theme), gallery::density(&case.density));
            ctx.focused = item["glow"]["focused"] == true;
            ctx.seat = ui::Paint::solid(p.strip.seat.color(ctx)).resolve(ctx)?;
            let h = &item["host"];
            let root = p.element(ctx);
            let layout = root.layout(
                Rect::new(
                    num(h, "x")? + num(crop, "x")?,
                    num(h, "y")? + num(crop, "y")?,
                    num(h, "w")?,
                    0.,
                ),
                ctx,
                &crate::components::Metrics,
            )?;
            Ok((ctx, layout))
        })
        .collect()
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("missing Pane fixture")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("missing Pane backdrop")?,
    ))?;
    for (ctx, layout) in layouts(case)? {
        pixels = crate::components::execute_at(
            renderer,
            c::paint(&layout, ctx),
            pixels,
            [w, h],
            ctx,
            0.,
        )?;
    }
    let x = num(&bg["crop"], "x")?.round() as u32;
    let y = num(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Pane crop outside backdrop".into());
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
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "panes"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [num(crop, "x")?, num(crop, "y")?];
        let mut parts = vec![];
        for ((_, layout), reference) in layouts(case)?.iter().zip(f["items"].as_array().unwrap()) {
            let mut names = BTreeMap::new();
            seat_rows::named(layout, &mut names);
            for p in reference["parts"]
                .as_array()
                .ok_or("missing Pane diagnostics")?
            {
                let key = p["key"].as_str().ok_or("Pane diagnostic key")?;
                let native = names.get(key).map(|n| {
                    json!([
                        n.rect.x - offset[0],
                        n.rect.y - offset[1],
                        n.rect.w,
                        n.rect.h
                    ])
                });
                let r = &p["rect"];
                let web = [num(r, "x")?, num(r, "y")?, num(r, "w")?, num(r, "h")?];
                let text = p["kind"] == "text";
                parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text{json!({"native":names.get(key).and_then(|n|seat_rows::text_baseline(n)).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})}else{Value::Null},"children":[]}));
            }
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

fn model_strip(s: &bbt_ui_model::Seat, focused: bool, ctx: ui::Context) -> Strip {
    let mut row = seat_rows::model_view(s, &DrawCtx::new(ctx.theme, ctx.density));
    row.focused = focused;
    row.shortcut = Some("⌘1".into());
    let mut strip = Strip::new(row);
    strip.may_mute = true;
    strip
}
fn append(layout: &c::Layout, draw: DrawCtx, out: &mut DrawList) {
    append_ops(c::paint_with_draw(layout, draw), out);
}
fn append_ops(ops: Vec<c::Paint>, out: &mut DrawList) {
    for op in ops {
        match op {
            c::Paint::Draw(d) | c::Paint::Underlay(d) => out.primitives.extend(d.primitives),
            c::Paint::Group {
                ops,
                opacity,
                saturation,
                mask,
            } => {
                let mut inner = DrawList::default();
                append_ops(ops, &mut inner);
                out.push(bbt_ui_model::draw::Prim::Group {
                    primitives: inner.primitives,
                    opacity,
                    saturation,
                    mask,
                });
            }
            c::Paint::Material { .. } => {}
        }
    }
}
pub fn paint_legacy_pane(
    s: &bbt_ui_model::Seat,
    r: Rect,
    focused: bool,
    draw: &DrawCtx,
    out: &mut DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let p = Pane {
        strip: model_strip(s, focused, ctx),
        lines: vec![],
        gap_from: None,
        resumed: None,
        min_height: r.h,
        unfocused: !focused,
        reconnecting: false,
        reconnected: false,
        glow_seed: None,
        opacity: bbt_ui_model::TOKENS.window.opacity as f32,
    };
    let layout = p
        .element(ctx)
        .layout(
            Rect::new(r.x, r.y, r.w, 0.),
            ctx,
            &crate::components::Metrics,
        )
        .expect("legacy Pane");
    append(&layout, *draw, out)
}
