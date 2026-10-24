//! StreamDock fixture host. Browser child diagnostics never enter native layout.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{
    self as ui, components as c,
    modules::stream_dock::{Entry, Message, StreamDock},
};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::{Rect, TextMeasure, TextStyle},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const DOCK_STATES: &[&str] = &[
    "swarm stream",
    "epic groups",
    "epic collapsed",
    "external lane",
    "messages",
    "audit lines",
    "subchannels",
    "subchannel unread · more",
    "subchannel muted",
    "subgroup message",
    "rejected progress",
    "filtered to one participant",
    "empty",
];
pub const MESSAGE_STATES: &[&str] = &[
    "oracle",
    "executor",
    "creativity",
    "unsigned",
    "reply nested",
    "stream",
    "evidence",
    "evidence climbing",
    "to a subagent",
    "relayed via main",
    "reply from a subagent",
    "agent ended (held)",
    "arrival ripple",
    "priority and issuer",
    "receipts",
];
pub const COMPOSE_STATES: &[&str] = &[
    "idle",
    "to a seat",
    "to a role",
    "to a channel",
    "to a subagent",
    "priority",
    "attachments",
    "sent",
    "kind · chooser",
    "send · pending",
    "send · sent",
    "send · held",
    "send · refused",
    "delegate · pick task",
    "delegate · pick seat",
    "delegate · claimed",
    "delegate · report back",
    "delegate · accepted",
    "delegate · reject reason",
    "delegate · reject typed",
    "delegate · reject refused",
    "delegate · rejected",
    "attach · picker",
    "attach · typed",
];
pub fn cases() -> Vec<Case> {
    let mut all = vec![];
    for (component, page, states) in [
        ("stream", "stream.html", DOCK_STATES),
        ("message-row", "message-row.html", MESSAGE_STATES),
        ("compose", "compose.html", COMPOSE_STATES),
    ] {
        for state in states {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    all.push(Case {
                        component: component.into(),
                        state: (*state).into(),
                        slug: state
                            .chars()
                            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
                            .collect::<String>()
                            .replace(' ', "-"),
                        density: gallery::density_name(density).into(),
                        theme: gallery::theme_name(theme).into(),
                        page: page.into(),
                        selector: ".card > .body".into(),
                        width: 1246,
                        height: 500,
                        checker: false,
                        fixture: Some(json!({"layer":"streams"})),
                    });
                }
            }
        }
    }
    all
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| {
            c.component == "stream_dock"
                || (c.component == "message-row"
                    && [
                        "oracle",
                        "executor",
                        "creativity",
                        "unsigned",
                        "reply nested",
                        "stream",
                        "arrival ripple",
                    ]
                    .contains(&c.state.as_str()))
        })
        .map(|mut c| {
            c.component.push_str("-v2");
            c.fixture = Some(json!({"layer":"streams","reference":"baseline"}));
            c
        })
        .collect()
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("Stream fixture missing")?;
    let ctx = crate::components::context(case);
    let crop = &f["backdrop"]["crop"];
    f["frames"]
        .as_array()
        .ok_or("Stream frames missing")?
        .iter()
        .map(|frame| {
            let mut model = frame["view"].clone();
            let entries = model["entries"]
                .as_array()
                .ok_or("Stream entries missing")?
                .clone();
            model["entries"] = json!([]);
            let view: StreamDock = serde_json::from_value(model)?;
            let width = seat_rows::number(&frame["host"], "w")?;
            let content_width = if view.dock {
                width - (bbt_ui_model::STYLE.spacing.s2 + bbt_ui_model::STYLE.spacing.s3) as f32
            } else {
                width
            };
            let mut items = vec![];
            for (i, entry) in entries.into_iter().enumerate() {
                if entry["kind"] == "composer" {
                    let composer: ui::modules::composer::Composer =
                        serde_json::from_value(entry["data"].clone())?;
                    items.push(
                        composer
                            .element_named(
                                &format!("e{i}"),
                                ctx,
                                content_width,
                                &crate::components::Metrics,
                            )?
                            .into(),
                    );
                } else {
                    let one = StreamDock {
                        entries: vec![serde_json::from_value(entry)?],
                        ..view.clone()
                    };
                    items.extend(one.items_at(ctx, i, &crate::components::Metrics));
                }
            }
            let host = &frame["host"];
            view.frame(ctx, items)
                .layout(
                    Rect::new(
                        seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
                        seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                        seat_rows::number(host, "w")?,
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
    let f = case.fixture.as_ref().ok_or("Stream fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Stream backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    for layout in layout_case(case)? {
        pixels = crate::components::execute(
            renderer,
            c::paint_with_draw(&layout, DrawCtx::new(ctx.theme, ctx.density)),
            pixels,
            [w, h],
            ctx,
        )?;
    }
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Stream crop outside backdrop".into());
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
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "streams"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let mut parts = vec![];
        for (frame, layout) in f["frames"]
            .as_array()
            .ok_or("Stream frames missing")?
            .iter()
            .zip(layout_case(case)?)
        {
            let mut names = BTreeMap::new();
            seat_rows::named(&layout, &mut names);
            for p in frame["parts"]
                .as_array()
                .ok_or("Stream diagnostics missing")?
            {
                let key = p["key"].as_str().ok_or("Stream part key missing")?;
                let native = if let Some(prefix) = p["native_prefix"].as_str() {
                    let mut rects = names
                        .iter()
                        .filter(|(k, _)| k.starts_with(prefix))
                        .map(|(_, n)| n.rect);
                    rects.next().map(|mut r| {
                        for other in rects {
                            let right = r.right().max(other.right());
                            let bottom = r.bottom().max(other.bottom());
                            r.x = r.x.min(other.x);
                            r.y = r.y.min(other.y);
                            r.w = right - r.x;
                            r.h = bottom - r.y;
                        }
                        json!([r.x - offset[0], r.y - offset[1], r.w, r.h])
                    })
                } else if p["native_inline"] == true {
                    fn text(n: &c::Layout) -> Option<&c::Layout> {
                        if matches!(n.kind, c::LayoutKind::Text { .. }) {
                            Some(n)
                        } else {
                            n.children.iter().find_map(text)
                        }
                    }
                    names.get(key).and_then(|n| text(n)).and_then(|n| {
                        let c::LayoutKind::Text { runs, .. } = &n.kind else {
                            return None;
                        };
                        let run = runs.first()?;
                        let font = TextStyle {
                            size: run.style.size,
                            weight: run.style.weight,
                            mono: run.style.mono,
                            italic: run.style.italic,
                            tracking_em: run.style.tracking_em,
                        };
                        let [a, d] = c::Measure::font_metrics(&crate::components::Metrics, &font)
                            .map(f32::round);
                        Some(json!([
                            run.rect.x - offset[0],
                            n.rect.y + n.baseline - a - offset[1],
                            crate::components::Metrics.width(&run.text, &font),
                            a + d
                        ]))
                    })
                } else {
                    names.get(key).map(|n| {
                        json!([
                            n.rect.x - offset[0],
                            n.rect.y - offset[1],
                            n.rect.w,
                            n.rect.h
                        ])
                    })
                };
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
    let mut all = cases();
    all.extend(legacy_cases());
    crate::materials::render_gallery(out, &all)
}
pub fn paint_legacy(r: Rect, state: &str, draw: &DrawCtx, out: &mut DrawList) {
    let roles = if state == "stream" {
        vec!["oracle", "executor", "creativity", "unsigned"]
    } else {
        vec![state, state]
    };
    let view = StreamDock {
        dock: false,
        framed: true,
        entries: roles
            .into_iter()
            .map(|role| Entry::Message {
                data: Box::new(Message {
                    from: if role == "oracle" {
                        "Vesper Greywind".into()
                    } else {
                        "Nyx Copperwing".into()
                    },
                    role: role.into(),
                    time: "12:07".into(),
                    text: "B claimed. File:line pointers checked. Report is ready.".into(),
                    unread: true,
                    ..Default::default()
                }),
                show_channel: None,
            })
            .collect(),
    };
    let ctx = ui::Context::new(draw.theme, draw.density);
    let layout = view
        .element(ctx, &crate::components::Metrics)
        .layout(
            Rect::new(r.x, r.y, r.w.min(700.), 0.),
            ctx,
            &crate::components::Metrics,
        )
        .expect("Stream composition");
    crate::components::append_draw(c::paint_with_draw(&layout, *draw), out)
        .expect("Stream flat paint");
}
