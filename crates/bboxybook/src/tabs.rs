//! L7 TabRow/MiniGrid fixture host. Public arguments and external flow bounds
//! feed the compositions; child DOM rectangles only feed the diagnostics.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{
    self as ui, components as c,
    modules::{
        mini_grid::{MiniGrid, Node},
        tab_row::{TabRow, TabSeat},
    },
};
use bbt_ui_model::{
    SeatId, SplitNode,
    draw::{DrawCtx, DrawList},
    layout::{Axis, Rect},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const GRID_STATES: &[&str] = &[
    "single",
    "V",
    "H",
    "V(a, H(b,c))",
    "H(V(a,b), c)",
    "depth 3",
    "focused cell",
    "waiting + errored cells",
];
pub const TAB_STATES: &[&str] = &[
    "1 seat collapsed",
    "3 seats collapsed",
    "8 seats collapsed",
    "expanded",
    "expanded with focus",
];
fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
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
    for (id, states) in [("mini-grid", GRID_STATES), ("tab-row", TAB_STATES)] {
        for state in states {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    out.push(Case {
                        component: id.into(),
                        state: (*state).into(),
                        slug: slug(state),
                        density: gallery::density_name(density).into(),
                        theme: gallery::theme_name(theme).into(),
                        page: format!("{id}.html"),
                        selector: ".card > .body".into(),
                        width: 1246,
                        height: 600,
                        checker: id == "mini-grid",
                        fixture: Some(json!({"layer":"tabs"})),
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
        .filter(|c| matches!(c.component.as_str(), "tab_row" | "mini_grid"))
        .map(|mut c| {
            c.fixture = Some(json!({"layer":"tabs","reference":"baseline"}));
            c
        })
        .collect()
}
pub fn node(v: &Value) -> Result<Node> {
    if let Some(key) = v["key"].as_str() {
        Ok(Node::Cell {
            key: key.into(),
            name: v["name"].as_str().unwrap_or("").into(),
            activity: seat_rows::activity(v),
        })
    } else {
        let ratio = v["ratio"].as_f64().filter(|r| *r != 0.).unwrap_or(0.5) as f32;
        Ok(Node::Split {
            axis: match v["dir"].as_str() {
                Some("v") => Axis::Row,
                Some("h") => Axis::Column,
                _ => return Err("MiniGrid missing split direction".into()),
            },
            ratio,
            a: Box::new(node(&v["a"])?),
            b: Box::new(node(&v["b"])?),
        })
    }
}
fn leaves(v: &Value) -> Vec<&Value> {
    if v["key"].is_string() {
        vec![v]
    } else {
        [leaves(&v["a"]), leaves(&v["b"])].concat()
    }
}
pub fn tab(v: &Value, options: &Value, ctx: ui::Context) -> Result<TabRow> {
    let mut seats = vec![];
    for v in v["seats"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_else(|| leaves(&v["tree"]))
    {
        let row = seat_rows::seat(v, &Value::Null, None);
        let agents = v["subagents"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| ui::modules::seat_row::Subagent {
                id: a["id"].as_str().unwrap_or("").into(),
                description: a["description"].as_str().unwrap_or("").into(),
                detail: a["detail"].as_str().map(Into::into),
                activity: seat_rows::activity(a),
            })
            .collect::<Vec<_>>();
        let mut children = row.subagent_rows(&agents, ctx);
        for task in v["tasks"].as_array().into_iter().flatten() {
            children.push(seat_rows::element(
                &json!({"kind":"taskRow","args":[task,v,{}]}),
                None,
                ctx,
            )?);
            for a in task["artifacts"].as_array().into_iter().flatten() {
                children.push(seat_rows::element(
                    &json!({"kind":"artifactRow","args":[a]}),
                    None,
                    ctx,
                )?);
            }
        }
        seats.push(TabSeat { row, children });
    }
    Ok(TabRow {
        name: v["name"].as_str().unwrap_or("").into(),
        shortcut: v["shortcut"].as_str().map(Into::into),
        tree: node(&v["tree"])?,
        seats,
        expanded: options["expanded"] == true,
        focused_key: options["focusedKey"].as_str().map(Into::into),
    })
}
fn element(f: &Value, ctx: ui::Context) -> Result<c::Element> {
    let items = f["items"]
        .as_array()
        .ok_or("TabRow/MiniGrid inputs missing")?;
    if f["kind"] == "tab-row" {
        let args = &items.first().ok_or("TabRow input missing")?["args"];
        Ok(tab(&args[0], &args[1], ctx)?.element(ctx))
    } else {
        let grids = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let args = &item["args"];
                Ok(MiniGrid {
                    tree: node(&args[0])?,
                    focused_key: args[1].as_str().map(Into::into),
                    size: [
                        item["size"][0].as_f64().ok_or("MiniGrid width missing")? as f32,
                        item["size"][1].as_f64().ok_or("MiniGrid height missing")? as f32,
                    ],
                    big: args[2] == true,
                }
                .element_named(ctx, &format!("grid{i}"))
                .into())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(seat_rows::number(f, "gap")?),
            align: c::Alignment::Center,
            children: grids,
        }))
    }
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let f = case
        .fixture
        .as_ref()
        .ok_or("TabRow/MiniGrid fixture missing")?;
    let crop = &f["backdrop"]["crop"];
    let host = &f["host"];
    let ctx = crate::components::context(case);
    element(f, ctx)?
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
}
fn append(layout: &c::Layout, draw: DrawCtx, out: &mut DrawList) {
    for op in c::paint_with_draw(layout, draw) {
        match op {
            c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                out.primitives.extend(list.primitives)
            }
            c::Paint::Material { .. } => unreachable!("TabRow/MiniGrid are flat compositions"),
        }
    }
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case
        .fixture
        .as_ref()
        .ok_or("TabRow/MiniGrid fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"]
            .as_str()
            .ok_or("TabRow/MiniGrid backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    let mut draw = DrawList::default();
    append(
        &layout_case(case)?,
        DrawCtx::new(ctx.theme, ctx.density),
        &mut draw,
    );
    let pixels =
        crate::components::execute(renderer, vec![c::Paint::Draw(draw)], pixels, [w, h], ctx)?;
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
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "tabs"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let layout = layout_case(case)?;
        let mut names = BTreeMap::new();
        seat_rows::named(&layout, &mut names);
        let mut parts = vec![];
        for item in f["items"]
            .as_array()
            .ok_or("TabRow/MiniGrid inputs missing")?
        {
            for p in item["parts"]
                .as_array()
                .ok_or("TabRow/MiniGrid diagnostics missing")?
            {
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

/// Retained composition adapters for modules still awaiting their own L7 step.
pub fn paint_mini_model(
    v: &Value,
    r: Rect,
    focus: &str,
    big: bool,
    draw: &DrawCtx,
    out: &mut DrawList,
) -> Result<()> {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let grid = MiniGrid {
        tree: node(v)?,
        focused_key: (!focus.is_empty()).then(|| focus.into()),
        size: [r.w, r.h],
        big,
    };
    let layout = grid
        .element(ctx)
        .layout(r, ctx, &crate::components::Metrics)?;
    append(&layout, *draw, out);
    Ok(())
}
pub fn paint_tab_model(
    v: &Value,
    r: Rect,
    expanded: bool,
    focus: &str,
    draw: &DrawCtx,
    out: &mut DrawList,
) -> Result<()> {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let layout = tab(v, &json!({"expanded":expanded,"focusedKey":focus}), ctx)?
        .element(ctx)
        .layout(
            Rect::new(r.x, r.y, r.w, 0.),
            ctx,
            &crate::components::Metrics,
        )?;
    append(&layout, *draw, out);
    Ok(())
}
fn legacy_node(tree: &SplitNode) -> Node {
    match tree {
        SplitNode::Leaf(id) => {
            let s = crate::modules::seat(id.0.0 as usize - 1);
            Node::Cell {
                key: format!("{id:?}"),
                name: s.name,
                activity: s.activity,
            }
        }
        SplitNode::Split { dir, ratio, a, b } => Node::Split {
            axis: *dir,
            ratio: *ratio,
            a: Box::new(legacy_node(a)),
            b: Box::new(legacy_node(b)),
        },
    }
}
pub fn paint_legacy_mini(
    tree: &SplitNode,
    r: Rect,
    focused: Option<SeatId>,
    draw: &DrawCtx,
    out: &mut DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let grid = MiniGrid {
        tree: legacy_node(tree),
        focused_key: focused.map(|id| format!("{id:?}")),
        size: [r.w, r.h],
        big: false,
    };
    let layout = grid
        .element(ctx)
        .layout(r, ctx, &crate::components::Metrics)
        .expect("MiniGrid layout");
    append(&layout, *draw, out)
}
pub fn paint_legacy_tab(
    tree: &SplitNode,
    name: &str,
    r: Rect,
    expanded: bool,
    draw: &DrawCtx,
    out: &mut DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let view = TabRow {
        name: name.into(),
        shortcut: None,
        tree: legacy_node(tree),
        seats: tree
            .leaves()
            .into_iter()
            .map(|id| TabSeat {
                row: seat_rows::model_view(&crate::modules::seat(id.0.0 as usize - 1), draw),
                children: vec![],
            })
            .collect(),
        expanded,
        focused_key: expanded.then(|| format!("{:?}", crate::modules::sid(0))),
    };
    let layout = view
        .element(ctx)
        .layout(
            Rect::new(r.x, r.y, r.w, 0.),
            ctx,
            &crate::components::Metrics,
        )
        .expect("TabRow layout");
    // Legacy card hosts allocate a header slot and compose their seats separately.
    out.clip(r, [0.; 4]);
    append(&layout, *draw, out);
    out.unclip();
}
