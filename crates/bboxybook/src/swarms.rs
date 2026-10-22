//! SwarmCard/Sidebar host. Constructor data and an external viewport are inputs;
//! every descendant rectangle and baseline remains an independent diagnostic.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows, tabs,
};
use bbt_ui::{
    self as ui, components as c,
    modules::{
        sidebar::Sidebar,
        swarm_card::{self, PendingSpawn, SwarmCard},
    },
};
use bbt_ui_model::layout::Rect;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for (page, states) in [
        (
            "space-header",
            vec![
                "collapsed small",
                "collapsed large",
                "collapsed with waiting",
                "expanded",
            ],
        ),
        (
            "sidebar",
            vec![
                "10 seats",
                "100 seats",
                "1000 seats",
                "phone sheet",
                "pending spawns",
            ],
        ),
    ] {
        for state in states {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    let slug = state.replace(' ', "-");
                    out.push(Case {
                        component: page.into(),
                        state: state.into(),
                        slug,
                        density: gallery::density_name(density).into(),
                        theme: gallery::theme_name(theme).into(),
                        page: format!("{page}.html"),
                        selector: ".card > .body".into(),
                        width: 1246,
                        height: 600,
                        checker: false,
                        fixture: Some(json!({"layer":"swarms"})),
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
        .filter(|c| matches!(c.component.as_str(), "band" | "sidebar"))
        .map(|mut c| {
            c.fixture = Some(json!({"layer":"swarms","reference":"baseline"}));
            if c.component == "sidebar" {
                c.component = "sidebar-v2".into();
            }
            c
        })
        .collect()
}
fn contains(v: &Value, s: &str) -> bool {
    v.as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(s)))
}
fn pending(v: &Value, dismiss: bool, ctx: ui::Context) -> c::Element {
    let state = v["status"].as_str().unwrap_or("accepted");
    let caption = match state {
        "accepted" => format!("spawning · by {}", v["by"].as_str().unwrap_or("the Oracle")),
        "rejected" => format!(
            "rejected · {}",
            v["reason"].as_str().unwrap_or("no reason given")
        ),
        _ => format!(
            "{state} · {}",
            v["outcome"].as_str().unwrap_or(if state == "unknown" {
                "no confirmation from the mux · never retried"
            } else {
                "some effects applied"
            })
        ),
    };
    swarm_card::pending_row(
        &PendingSpawn {
            name: v["name"].as_str().unwrap_or("").into(),
            harness: v["harness"].as_str().unwrap_or("").into(),
            status: state.into(),
            caption,
            dismiss,
        },
        ctx,
    )
}
fn card(
    v: &Value,
    opts: &Value,
    expanded: bool,
    standalone: bool,
    ctx: ui::Context,
) -> Result<SwarmCard> {
    let key = v["key"].as_str().unwrap_or("");
    let rows = v["tabs"].as_array().ok_or("swarm tabs missing")?;
    let mut all = vec![];
    let mut visible = vec![];
    for t in rows {
        let name = t["name"].as_str().unwrap_or("");
        let tab = tabs::tab(
            t,
            &json!({"expanded":contains(&opts["openTabs"],&format!("{key}/{name}")),"focusedKey":opts["focusedKey"]}),
            ctx,
        )?;
        all.extend(tab.seats.iter().map(|s| s.row.clone()));
        visible.push(tab);
    }
    Ok(SwarmCard {
        key: key.into(),
        name: v["name"].as_str().unwrap_or("").into(),
        color: match seat_rows::seat(v, &Value::Null, None).color(ctx) {
            ui::Color::Rgba(c) => c,
            _ => unreachable!(),
        },
        seats: all,
        expanded,
        closed: !standalone && !expanded,
        unfocused: false,
        tabs: visible,
        body: expanded,
        new_seat: !standalone,
        extra: v["pending"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| pending(p, !opts["onDismissPending"].is_null(), ctx))
            .collect(),
        shadows: v["participants"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| seat_rows::seat(p, &Value::Null, None))
            .collect(),
    })
}
pub fn sidebar(
    model: &Value,
    opts: &Value,
    size: [f32; 2],
    phone: bool,
    scroll: f32,
    ctx: ui::Context,
) -> Result<Sidebar> {
    let spaces = model["spaces"].as_array().ok_or("sidebar swarms missing")?;
    let focus = opts["focusedKey"].as_str().unwrap_or("");
    let owner = spaces.iter().find(|sp| {
        sp["tabs"].as_array().into_iter().flatten().any(|t| {
            t["seats"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|s| s["key"] == focus)
        })
    });
    let current = opts["currentSpace"]
        .as_str()
        .or_else(|| owner.and_then(|v| v["key"].as_str()))
        .or_else(|| spaces.first().and_then(|v| v["key"].as_str()));
    let mut cards = vec![];
    for v in spaces {
        let key = v["key"].as_str().unwrap_or("");
        let mut c = card(v, opts, contains(&opts["openSpaces"], key), false, ctx)?;
        c.unfocused = current.is_some_and(|s| s != key);
        cards.push(c)
    }
    Ok(Sidebar {
        cards,
        size,
        scroll,
        phone,
        new_swarm: !opts["onNewSpace"].is_null(),
    })
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let f = case.fixture.as_ref().ok_or("swarm fixture missing")?;
    let ctx = crate::components::context(case);
    let host = &f["host"];
    let crop = &f["backdrop"]["crop"];
    let w = seat_rows::number(host, "w")?;
    let e = if f["kind"] == "sidebar" {
        sidebar(
            &f["args"][0],
            &f["args"][1],
            [w, seat_rows::number(host, "h")?],
            f["phone"] == true,
            f["scroll"].as_f64().unwrap_or(0.) as f32,
            ctx,
        )?
        .element(ctx)
    } else {
        let mut c = card(
            &f["args"][0],
            &Value::Null,
            f["args"][1]["expanded"] == true,
            true,
            ctx,
        )?;
        c.tabs = f["body_tabs"]
            .as_array()
            .ok_or("card tab inputs missing")?
            .iter()
            .map(|a| tabs::tab(&a[0], &a[1], ctx))
            .collect::<Result<_>>()?;
        c.element(ctx)
    };
    e.layout(
        Rect::new(
            seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
            seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
            w,
            0.,
        ),
        ctx,
        &crate::components::Metrics,
    )
    .map_err(Into::into)
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("swarm fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("swarm backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    let layout = layout_case(case)?;
    let mut clips = vec![];
    if f["ancestor_clip"].is_object() {
        let external = &f["ancestor_clip"];
        let r = &external["rect"];
        let mut radii = [0.; 4];
        for (i, radius) in radii.iter_mut().enumerate() {
            *radius = external["radii"][i]
                .as_f64()
                .ok_or("external clip radius missing")? as f32;
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
        &layout,
        bbt_ui_model::draw::DrawCtx::new(ctx.theme, ctx.density),
        &clips,
    );
    let pixels = crate::components::execute(renderer, plan, pixels, [w, h], ctx)?;
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    let mut crop = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        crop.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(crop)
}
pub(crate) fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut out = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "swarms"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let layout = layout_case(case)?;
        let mut names = BTreeMap::new();
        seat_rows::named(&layout, &mut names);
        let mut parts = vec![];
        for p in f["parts"].as_array().ok_or("swarm diagnostics missing")? {
            let key = p["key"].as_str().ok_or("part name missing")?;
            let r = &p["rect"];
            let web = [
                seat_rows::number(r, "x")?,
                seat_rows::number(r, "y")?,
                seat_rows::number(r, "w")?,
                seat_rows::number(r, "h")?,
            ];
            let native = names.get(key).map(|n| {
                [
                    n.rect.x - offset[0],
                    n.rect.y - offset[1],
                    n.rect.w,
                    n.rect.h,
                ]
            });
            let text = p["kind"] == "text";
            parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text{json!({"native":names.get(key).and_then(|n|seat_rows::text_baseline(n)).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})}else{Value::Null},"children":[]}));
        }
        out.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width as f32,case.height as f32],"web":[0.,0.,case.width as f32,case.height as f32],"children":parts}}));
    }
    Ok(out)
}
pub fn render(out: &Path) -> Result<()> {
    let mut requested = cases();
    requested.extend(legacy_cases());
    crate::materials::render_gallery(out, &requested)
}

fn append(
    layout: &c::Layout,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) {
    append_ops(c::paint_with_draw(layout, *draw), out);
}
fn append_ops(ops: Vec<c::Paint>, out: &mut bbt_ui_model::draw::DrawList) {
    use bbt_ui_model::draw::Prim;
    for op in ops {
        match op {
            c::Paint::Draw(d) | c::Paint::Underlay(d) => out.primitives.extend(d.primitives),
            c::Paint::Group {
                ops,
                opacity,
                saturation,
            } => {
                let mut inner = bbt_ui_model::draw::DrawList::default();
                append_ops(ops, &mut inner);
                out.push(Prim::Group {
                    primitives: inner.primitives,
                    opacity,
                    saturation,
                });
            }
            c::Paint::Material {
                rect,
                radii,
                kind: ui::Material::Gloss,
                color,
                clips,
                ..
            } => {
                for clip in &clips {
                    out.clip(clip.rect, clip.radii)
                }
                out.push(Prim::Gloss { rect, radii, color });
                for _ in clips {
                    out.unclip()
                }
            }
            c::Paint::Material { .. } => {
                unreachable!("SwarmCard lab chrome uses only flat and gloss")
            }
        }
    }
}
pub fn paint_sidebar_model(
    model: &Value,
    opts: &Value,
    r: Rect,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) -> Result<()> {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let e = sidebar(model, opts, [r.w, r.h], false, 0., ctx)?.element(ctx);
    let layout = e.layout(r, ctx, &crate::components::Metrics)?;
    append(&layout, draw, out);
    Ok(())
}
pub fn paint_header_model(
    v: &Value,
    opts: &Value,
    r: Rect,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) -> Result<()> {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let e = card(v, opts, opts["expanded"] == true, true, ctx)?.header_element(ctx);
    let layout = e.layout(r, ctx, &crate::components::Metrics)?;
    append(&layout, draw, out);
    Ok(())
}
fn legacy_card(
    name: &str,
    indices: &[usize],
    open: bool,
    focused: bool,
    draw: &bbt_ui_model::draw::DrawCtx,
) -> SwarmCard {
    use ui::modules::{
        mini_grid::Node,
        tab_row::{TabRow, TabSeat},
    };
    let ctx = ui::Context::new(draw.theme, draw.density);
    let key = format!("space-{name}");
    let color = match seat_rows::seat(&json!({"key":key}), &Value::Null, None).color(ctx) {
        ui::Color::Rgba(c) => c,
        _ => unreachable!(),
    };
    let seats = indices
        .iter()
        .map(|i| seat_rows::model_view(&crate::modules::seat(*i), draw))
        .collect::<Vec<_>>();
    let mut tabs = vec![];
    for (i, s) in seats.iter().enumerate() {
        tabs.push(TabRow {
            name: s.title.clone(),
            shortcut: (i < 9).then(|| format!("⌘{}", i + 1)),
            tree: Node::Cell {
                key: s.key.clone(),
                name: s.name.clone(),
                activity: s.activity,
            },
            seats: vec![TabSeat {
                row: s.clone(),
                children: vec![],
            }],
            expanded: true,
            focused_key: focused.then(|| seats[0].key.clone()),
        });
    }
    SwarmCard {
        key,
        name: name.into(),
        color,
        seats,
        expanded: open,
        closed: !open,
        unfocused: !focused,
        tabs,
        body: open,
        new_seat: true,
        extra: vec![],
        shadows: vec![],
    }
}
pub fn paint_legacy_card(
    r: Rect,
    name: &str,
    indices: &[usize],
    open: bool,
    focused: bool,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let e = legacy_card(name, indices, open, focused, draw).element(ctx);
    let layout = e
        .layout(r, ctx, &crate::components::Metrics)
        .expect("SwarmCard layout");
    append(&layout, draw, out)
}
pub fn paint_legacy_sidebar(
    r: Rect,
    count: usize,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let cards = if count == 15 {
        vec![legacy_card(
            "swarm",
            &(0..15).collect::<Vec<_>>(),
            true,
            true,
            draw,
        )]
    } else {
        vec![
            legacy_card("term", &[0, 1, 3, 2, 4], true, true, draw),
            legacy_card("winnow", &[5, 6, 7], false, false, draw),
        ]
    };
    let e = Sidebar {
        cards,
        size: [r.w, r.h],
        scroll: 0.,
        phone: false,
        new_swarm: true,
    }
    .element(ctx);
    let layout = e
        .layout(r, ctx, &crate::components::Metrics)
        .expect("Sidebar layout");
    append(&layout, draw, out)
}
