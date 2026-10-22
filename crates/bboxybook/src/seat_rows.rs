//! L7 SeatRow fixture host. Only constructor arguments and external flow bounds
//! enter native composition. DOM parts are read by snapshots, never by layout.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_ui::{self as ui, components as c, modules::seat_row as row};
use bbt_ui_model::{
    Activity, Grade, WaitKind,
    layout::{Axis, Rect},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const STATES: &[&str] = &[
    "active",
    "idle",
    "waiting",
    "errored",
    "focused × state",
    "unread badge",
    "subtitle opt-in",
    "zsh seat",
    "long names",
    "muted · held",
    "unmuted · still held",
    "subagents · claude",
    "subagents · codex",
    "subagents · 12",
    "restarted",
    "listeners",
    "row menu · operator",
    "row menu · role flyout",
    "row menu · worker",
    "subagent menu",
    "authority · expired",
    "authority · revoked",
    "authority · forked",
    "bug rows",
    "invariant chips",
    "noise hint",
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
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "seat-row".into(),
                    state: (*state).into(),
                    slug: slug(state),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "seat-row.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 600,
                    checker: false,
                    fixture: Some(json!({"layer":"seat-row"})),
                });
            }
        }
    }
    for state in ["task-states", "artifacts"] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "seat-row".into(),
                    state: state.into(),
                    slug: state.into(),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "@component".into(),
                    selector: ".card > .body".into(),
                    width: 700,
                    height: 500,
                    checker: false,
                    fixture: Some(json!({"layer":"seat-row"})),
                });
            }
        }
    }
    out
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| c.component == "seat_row")
        .map(|mut c| {
            c.fixture = Some(json!({"layer":"seat-row","reference":"baseline"}));
            c
        })
        .collect()
}
fn string(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().into()
}
fn number(v: &Value, k: &str) -> Result<f32> {
    v[k].as_f64()
        .map(|v| v as f32)
        .ok_or_else(|| format!("SeatRow missing {k}").into())
}
fn color(v: &Value) -> Option<[f32; 4]> {
    let s = v.as_str()?;
    if s.starts_with('#') {
        Some(bbt_ui_model::draw::Color::hex(s).0)
    } else if let Some(hsl) = s.strip_prefix("hsl(").and_then(|s| s.strip_suffix(')')) {
        let n: Vec<f32> = hsl
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('%').parse())
            .collect::<std::result::Result<_, _>>()
            .ok()?;
        (n.len() == 3).then(|| bbt_ui_model::draw::Color::hsl(n[0], n[1] / 100., n[2] / 100.).0)
    } else {
        None
    }
}
fn activity(v: &Value) -> Activity {
    match v["state"].as_str() {
        Some("active") => Activity::Active,
        Some("waiting") => Activity::Waiting(match v["waitKind"].as_str().or(v["kind"].as_str()) {
            Some("permission") => WaitKind::Permission,
            Some("auth") => WaitKind::Auth,
            _ => WaitKind::Question,
        }),
        Some("errored") => Activity::Errored,
        _ => Activity::Idle,
    }
}
pub fn seat(v: &Value, o: &Value, height: Option<f32>) -> row::SeatRow {
    let mut s = row::SeatRow::new(string(v, "key"), string(v, "name"), string(v, "harness"));
    s.role = v["role"].as_str().map(Into::into);
    s.title = string(v, "title");
    s.activity = activity(v);
    s.grade = match v["grade"].as_str() {
        Some("shim") => Grade::Shim,
        Some("inferred") => Grade::Inferred,
        _ => Grade::Reported,
    };
    s.color = color(&v["color"]);
    s.where_ = v["where"]
        .as_str()
        .filter(|x| *x != "local")
        .map(Into::into);
    s.focused = o["focused"] == true;
    s.muted = v["muted"] == true;
    s.authority = v["authority"].as_str().map(Into::into);
    s.held = v["held"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| row::Held {
            id: string(h, "id"),
            blocked: h["blocked"].as_str().map(Into::into),
        })
        .collect();
    s.listeners = v["listeners"].as_u64().unwrap_or(0) as u32;
    s.touched = v["touched"].as_u64().unwrap_or(0) as u32;
    let unread = if !o["unread"].is_null() {
        &o["unread"]
    } else {
        &v["unread"]
    };
    if unread.as_u64().is_some_and(|n| n > 0) || unread.as_str().is_some_and(|s| !s.is_empty()) {
        s.unread = Some((
            unread
                .as_str()
                .map(Into::into)
                .unwrap_or_else(|| unread.to_string()),
            o["unreadRole"]
                .as_str()
                .or(v["unreadRole"].as_str())
                .unwrap_or("executor")
                .into(),
        ));
    }
    s.shortcut = o["shortcut"].as_str().map(Into::into);
    s.subtitle = o["subtitle"].as_str().map(Into::into);
    s.restarted = v["restarted"].as_bool().unwrap_or(false) || v["restarted"].as_f64().is_some();
    s.interrupt_sent =
        v["interruptSent"].as_bool().unwrap_or(false) || v["interruptSent"].as_f64().is_some();
    s.noise = v["noise"].as_f64().map(|n| n as f32);
    s.row_height = height;
    s
}
fn element(v: &Value, height: Option<f32>, ctx: ui::Context) -> Result<c::Element> {
    let args = &v["args"];
    Ok(match v["kind"].as_str() {
        Some("seatRow") => seat(&args[0], &args[1], height).element(ctx),
        Some("subagentRows") => {
            let s = seat(&args[0], &Value::Null, None);
            let agents = args[0]["subagents"]
                .as_array()
                .ok_or("missing subagents")?
                .iter()
                .map(|a| row::Subagent {
                    id: string(a, "id"),
                    description: string(a, "description"),
                    detail: a["detail"].as_str().map(Into::into),
                    activity: activity(a),
                })
                .collect::<Vec<_>>();
            s.subagent_rows(&agents, ctx)
                .into_iter()
                .nth(v["index"].as_u64().ok_or("missing subagent row index")? as usize)
                .ok_or("subagent row not running")?
        }
        Some("taskRow") => {
            let t = &args[0];
            row::TaskRow {
                title: string(t, "title"),
                state: string(t, "state"),
                bug: t["type"] == "bug",
                violates: t["violates"].as_str().map(Into::into),
                constrains: t["constrains"].as_str().map(Into::into),
                waits: t["waits"].as_str().map(Into::into),
                may_accept: !args[2]["onAccept"].is_null(),
            }
            .element(ctx)
        }
        Some("artifactRow") => {
            let a = &args[0];
            row::ArtifactRow {
                kind: a["kind"].as_str().unwrap_or("file").into(),
                reference: string(a, "ref"),
                digest: a["digest"].as_str().map(Into::into),
                responsible: a["responsible"].as_str().map(Into::into),
                responsible_color: color(&a["responsibleColor"]),
            }
            .element(ctx)
        }
        _ => return Err("unsupported SeatRow fixture constructor".into()),
    })
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("missing SeatRow fixture")?;
    let ctx = crate::components::context(case);
    let crop = &f["backdrop"]["crop"];
    f["groups"]
        .as_array()
        .ok_or("missing SeatRow groups")?
        .iter()
        .map(|g| {
            let height = g["row_height"].as_f64().map(|h| h as f32);
            let rows = g["rows"]
                .as_array()
                .ok_or("missing SeatRow inputs")?
                .iter()
                .map(|v| Ok(element(v, height, ctx)?.into()))
                .collect::<Result<Vec<_>>>()?;
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(number(g, "gap")?),
                align: c::Alignment::Stretch,
                children: rows,
            })
            .layout(
                Rect::new(
                    number(&g["host"], "x")? + number(crop, "x")?,
                    number(&g["host"], "y")? + number(crop, "y")?,
                    number(&g["host"], "w")?,
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
    let f = case.fixture.as_ref().ok_or("SeatRow fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("SeatRow backdrop missing")?,
    ))?;
    // This module is entirely flat. Preserve paint order in one retained draw
    // list instead of reading the GPU back after each label and icon.
    let mut draw = bbt_ui_model::draw::DrawList::default();
    for layout in layout_case(case)? {
        for op in c::paint(&layout, crate::components::context(case)) {
            match op {
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    draw.primitives.extend(list.primitives)
                }
                c::Paint::Material { .. } => {
                    return Err("SeatRow unexpectedly requires a material pass".into());
                }
            }
        }
    }
    pixels = crate::components::execute(
        renderer,
        vec![c::Paint::Draw(draw)],
        pixels,
        [w, h],
        crate::components::context(case),
    )?;
    let x = number(&bg["crop"], "x")?.round() as u32;
    let y = number(&bg["crop"], "y")?.round() as u32;
    let mut crop = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        crop.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(crop)
}
fn named<'a>(node: &'a c::Layout, out: &mut BTreeMap<&'a str, &'a c::Layout>) {
    if let c::LayoutKind::Named(name) = &node.kind {
        out.insert(name, node);
    }
    for ch in &node.children {
        named(ch, out);
    }
}
fn text_baseline(node: &c::Layout) -> Option<f32> {
    if matches!(node.kind, c::LayoutKind::Text { .. }) {
        Some(node.rect.y + node.baseline)
    } else {
        node.children.iter().find_map(text_baseline)
    }
}
pub(crate) fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut out = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "seat-row"))
    {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [number(crop, "x")?, number(crop, "y")?];
        let mut parts = vec![];
        for (layout, group) in layout_case(case)?
            .iter()
            .zip(f["groups"].as_array().ok_or("groups missing")?)
        {
            for (layout, row) in layout
                .children
                .iter()
                .zip(group["rows"].as_array().ok_or("rows missing")?)
            {
                let mut names = BTreeMap::new();
                named(layout, &mut names);
                for p in row["parts"].as_array().ok_or("parts missing")? {
                    let key = p["key"].as_str().ok_or("part name missing")?;
                    let reference = &p["rect"];
                    let web = [
                        number(reference, "x")?,
                        number(reference, "y")?,
                        number(reference, "w")?,
                        number(reference, "h")?,
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
                    parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text {json!({"native":names.get(key).and_then(|n|text_baseline(n)).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})}else{Value::Null},"children":[]}));
                }
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

/// Compatibility adapter for existing sidebar/showcase fixtures. The old
/// hand-positioned SeatRow painter has been removed from modules.rs.
pub fn paint_model(
    s: &bbt_ui_model::Seat,
    r: Rect,
    focused: bool,
    shortcut: Option<usize>,
    draw: &bbt_ui_model::draw::DrawCtx,
    out: &mut bbt_ui_model::draw::DrawList,
) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    let mut view = row::SeatRow::new(format!("{:?}", s.key), &s.name, &s.harness.0);
    view.role = s.role.as_ref().map(|r| r.name.clone());
    view.title = s.title.clone();
    view.activity = s.activity;
    view.grade = s.grade;
    view.focused = focused;
    view.muted = s.muted;
    let color = s.color.resolve(&format!("{:?}", s.key), draw.theme);
    view.color = Some(
        bbt_ui_model::draw::Color::hsl(color.hue, color.saturation / 100., color.lightness / 100.)
            .0,
    );
    view.where_ = match s.where_ {
        bbt_ui_model::Where::Local => None,
        bbt_ui_model::Where::Ssh { .. } => Some("ssh".into()),
        bbt_ui_model::Where::Cloud { .. } => Some("cloud".into()),
    };
    view.shortcut = shortcut.map(|n| format!("⌘{n}"));
    view.unread = s
        .unread
        .as_ref()
        .map(|(n, r)| (n.to_string(), r.name.clone()));
    view.row_height = Some(r.h);
    let layout = view
        .element(ctx)
        .layout(r, ctx, &crate::components::Metrics)
        .expect("SeatRow composition layout");
    for op in c::paint_with_draw(&layout, *draw) {
        match op {
            c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                out.primitives.extend(list.primitives)
            }
            c::Paint::Material { .. } => unreachable!("SeatRow is a flat composition"),
        }
    }
}
