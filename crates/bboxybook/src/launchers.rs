// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! L7 Launcher host. Only public constructor data and outer bounds enter layout.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{
    self as ui, components as c,
    modules::launcher::{Item, Launcher},
};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const STATES: &[&str] = &[
    "root",
    "typed \"co\"",
    "typed \"ssh rig cod\"",
    "Claude ▸ placements",
    "Claude ▸ roles",
    "Claude ▸ roles · no Oracle yet",
    "Claude ▸ Architect ▸ placements",
    "Codex ▸ placements",
    "SSH ▸ machines",
    "SSH ▸ rig ▸ harnesses",
    "SSH ▸ rig ▸ Codex ▸ placements",
    "Cloud ▸ regions",
    "Go to ▸ seats and swarms",
    "Settings ▸ sections",
    "Resume ▸ recently closed",
    "preset · split right",
    "preset · split down",
    "cap reached",
    "query · focused empty",
    "query · caret",
    "query · selection",
    "query · preedit",
];
pub const PICKER_STATES: &[&str] = &[
    "open",
    "typed \"c\"",
    "typed \"co\"",
    "typed \"z\"",
    "no match",
    "rerolled",
    "cap · swarm",
    "cap · machine",
];
fn slug(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').into()
}
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for (page, states) in [("launcher", STATES), ("picker", PICKER_STATES)] {
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
                        fixture: Some(json!({"layer":"launchers"})),
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
        .filter(|c| c.component == "launcher")
        .map(|mut c| {
            c.component = "launcher-v2".into();
            c.fixture = Some(json!({"layer":"launchers","reference":"baseline"}));
            c
        })
        .collect()
}
fn view(f: &Value) -> Result<Launcher> {
    let a = &f["args"];
    let swatch: [f32; 4] = serde_json::from_value(a["swatch"].clone())?;
    Ok(Launcher {
        items: serde_json::from_value(a["items"].clone())?,
        query: a["query"].as_str().unwrap_or("").into(),
        path: serde_json::from_value(a["path"].clone())?,
        selected: None,
        placeholder: a["placeholder"].as_str().unwrap_or("").into(),
        name: a["name"].as_str().ok_or("Launcher name missing")?.into(),
        swatch,
        actions: serde_json::from_value(a["actions"].clone())?,
        cap: serde_json::from_value(a["cap"].clone())?,
        spotlight: f["spotlight"] == true,
        edit: serde_json::from_value(a["edit"].clone())?,
        params: None,
    })
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let (root, bounds) = element_case(case)?;
    root.layout(
        bounds,
        crate::components::context(case),
        &crate::components::CachedMetrics::default(),
    )
    .map_err(Into::into)
}
/// The retained constructor and host bounds, also used by the layout benchmark.
pub fn element_case(case: &Case) -> Result<(c::Element, Rect)> {
    let f = case.fixture.as_ref().ok_or("Launcher fixture missing")?;
    let ctx = crate::components::context(case);
    let launcher = view(f)?;
    let banner = &f["banner"];
    let host = if banner.is_object() {
        &banner["host"]
    } else {
        &f["host"]
    };
    let crop = &f["backdrop"]["crop"];
    let mut children = vec![];
    if banner.is_object() {
        let b = ui::modules::launcher_banner::Banner {
            tree: crate::tabs::node(&banner["tree"])?,
            focused_key: banner["focused_key"]
                .as_str()
                .ok_or("preview focus missing")?
                .into(),
            title: banner["title"]
                .as_str()
                .ok_or("preview title missing")?
                .into(),
            anchor: banner["tree"]["a"]["name"]
                .as_str()
                .ok_or("preview anchor missing")?
                .into(),
            anchor_color: serde_json::from_value(banner["anchor_color"].clone())?,
        };
        children.push(b.element(ctx).into());
    }
    children.push(launcher.element(ctx).into());
    let root = c::Element::Stack(c::Stack {
        direction: bbt_ui_model::layout::Axis::Column,
        gap: ui::Length::Spacing(7),
        align: c::Alignment::Stretch,
        children,
    });
    Ok((
        root,
        Rect::new(
            seat_rows::number(host, "x")? + seat_rows::number(crop, "x")?,
            seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
            seat_rows::number(host, "w")?,
            0.,
        ),
    ))
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("Launcher fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Launcher backdrop missing")?,
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
                .ok_or("Launcher external clip radius missing")? as f32;
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
        return Err("Launcher crop outside backdrop".into());
    }
    let mut out = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(out)
}
fn banner_range(names: &BTreeMap<&str, &c::Layout>, key: &str) -> Option<(Rect, f32)> {
    use c::Measure;
    let wanted = match key {
        "banner/txt/of/text" => 0,
        "banner/txt/of/b" => 1,
        "banner/txt/of/text1" => 2,
        _ => return None,
    };
    let node = *names.get("banner/txt/of")?;
    let c::LayoutKind::Text { runs, .. } = &node.children.first()?.kind else {
        return None;
    };
    let mut phase = 0;
    let mut selected = vec![];
    for run in runs {
        if run.style.weight == 600 {
            phase = 1;
        } else if phase == 1 {
            phase = 2;
        }
        if phase == wanted {
            selected.push(run);
        }
    }
    let first = selected.first()?;
    let [ascent, descent] = crate::components::Metrics
        .font_metrics(&bbt_ui_model::layout::TextStyle {
            size: first.style.size,
            weight: first.style.weight,
            mono: first.style.mono,
            italic: first.style.italic,
            tracking_em: first.style.tracking_em,
        })
        .map(f32::round);
    let baseline = seat_rows::text_baseline(node)?;
    let x = selected
        .iter()
        .map(|r| r.rect.x)
        .fold(f32::INFINITY, f32::min);
    let right = selected
        .iter()
        .map(|r| r.rect.right())
        .fold(f32::NEG_INFINITY, f32::max);
    Some((
        Rect::new(x, baseline - ascent, right - x, ascent + descent),
        baseline,
    ))
}
fn paint_bounds(node: &c::Layout, offset: [f32; 2], text: &mut Vec<Value>, icons: &mut Vec<Value>) {
    let rect = |r: Rect| json!([r.x - offset[0], r.y - offset[1], r.w, r.h]);
    match &node.kind {
        c::LayoutKind::Text { runs, .. } => {
            for run in runs {
                if run.style.color[3] > 0. && !run.text.trim().is_empty() {
                    text.push(rect(run.rect));
                }
            }
        }
        c::LayoutKind::Icon { color, .. } if color[3] > 0. => icons.push(rect(node.rect)),
        _ => {}
    }
    for child in &node.children {
        paint_bounds(child, offset, text, icons);
    }
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut out = vec![];
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "launchers")
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
            .ok_or("Launcher diagnostics missing")?
        {
            let key = p["key"].as_str().ok_or("Launcher diagnostic key missing")?;
            let range = banner_range(&names, key);
            let native = range
                .map(|(rect, _)| rect)
                .or_else(|| names.get(key).map(|n| n.rect))
                .map(|r| json!([r.x - offset[0], r.y - offset[1], r.w, r.h]));
            let r = &p["rect"];
            let web = [
                seat_rows::number(r, "x")?,
                seat_rows::number(r, "y")?,
                seat_rows::number(r, "w")?,
                seat_rows::number(r, "h")?,
            ];
            let text = p["kind"] == "text";
            parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"baseline":if text{json!({"native":range.map(|(_,baseline)|baseline).or_else(||names.get(key).and_then(|n|seat_rows::text_baseline(n))).map(|y|y-offset[1]),"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1] as f64)})}else{Value::Null},"children":[]}));
        }
        let mut text_bounds = vec![];
        let mut icon_bounds = vec![];
        paint_bounds(&layout, offset, &mut text_bounds, &mut icon_bounds);
        out.push(json!({"key":case.relative(),"paint_bounds":{"text":text_bounds,"icons":icon_bounds},"tree":{"kind":"box","native":[0.,0.,case.width as f32,case.height as f32],"web":[0.,0.,case.width as f32,case.height as f32],"children":parts}}));
    }
    Ok(out)
}
pub fn render(out: &Path) -> Result<()> {
    let mut all = cases();
    all.extend(legacy_cases());
    crate::materials::render_gallery(out, &all)
}
pub fn paint_legacy(r: Rect, state: &str, draw: &DrawCtx, out: &mut DrawList) {
    let ctx = ui::Context::new(draw.theme, draw.density);
    // Sample data only; current parity consumes the actual lab item tree.
    let items = [
        (
            "claude",
            "Claude",
            "oracle by default",
            "#D97757",
            "Suggestions",
        ),
        (
            "codex",
            "Codex",
            "executor by default",
            "#1F2937",
            "Suggestions",
        ),
        ("pi", "pi", "executor by default", "#7C3AED", "Harnesses"),
        ("omp", "omp", "executor by default", "#0EA5E9", "Harnesses"),
        ("zsh", "zsh", "~", "#111827", "Shells"),
    ]
    .into_iter()
    .map(|(id, label, sub, tile, section)| Item {
        id: id.into(),
        label: label.into(),
        sub: sub.into(),
        tile: Some(tile.into()),
        section: Some(section.into()),
        harness: id.into(),
        kind: if id == "zsh" {
            "Shell".into()
        } else {
            "Harness".into()
        },
        ..Default::default()
    })
    .collect();
    let launcher = Launcher {
        items,
        query: if state == "no match" {
            "zzqx".into()
        } else {
            state
                .strip_prefix("typed ")
                .unwrap_or("")
                .trim_matches('"')
                .into()
        },
        path: vec![],
        selected: None,
        placeholder: "launch or go to · type to search".into(),
        name: "Fenn Calloway".into(),
        swatch: [0.3, 0.7, 1., 1.],
        actions: vec![
            ("Open".into(), "↵".into()),
            ("Into".into(), "→".into()),
            ("Back".into(), "←".into()),
            ("Reroll".into(), "⌘R".into()),
        ],
        cap: None,
        spotlight: true,
        edit: None,
        params: None,
    };
    let mut layout = launcher
        .element(ctx)
        .layout(
            Rect::new(r.x, r.y, r.w.min(760.), 0.),
            ctx,
            &crate::components::CachedMetrics::default(),
        )
        .expect("legacy Launcher");
    fn flat(n: &mut c::Layout) {
        if let c::LayoutKind::Box { style, .. } = &mut n.kind {
            style.material = ui::Material::Flat;
        }
        for child in &mut n.children {
            flat(child);
        }
    }
    flat(&mut layout);
    crate::components::append_draw(c::paint_with_draw(&layout, *draw), out)
        .expect("flat legacy Launcher");
}
