//! L6 control fixtures; public constructor arguments feed the native widgets.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{self as ui, components as c, widgets as w};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList, TextStyle, TypeScale},
    layout::Rect,
};
use serde_json::Value;
use std::{fs, path::Path};

fn string(v: &Value, key: &str) -> Result<String> {
    Ok(v[key]
        .as_str()
        .ok_or_else(|| format!("missing widget {key}"))?
        .into())
}
pub fn element(v: &Value, ctx: ui::Context) -> Result<c::Element> {
    let o = &v["options"];
    Ok(match v["kind"].as_str() {
        Some("button") => w::Button {
            label: string(v, "label")?,
            kind: match o["kind"].as_str() {
                Some("ghost") => w::ButtonKind::Ghost,
                Some("danger") => w::ButtonKind::Danger,
                _ => w::ButtonKind::Primary,
            },
            state: match o["state"].as_str() {
                Some("hover") => w::ButtonState::Hover,
                Some("pressed") => w::ButtonState::Pressed,
                Some("focus") => w::ButtonState::Focus,
                Some("disabled") => w::ButtonState::Disabled,
                _ => w::ButtonState::Idle,
            },
            icon: o["icon"].as_str().map(str::to_owned),
            key: o["kbd"].as_str().map(str::to_owned),
        }
        .element(ctx)?,
        Some("list") => {
            let items = v["items"]
                .as_array()
                .ok_or("missing List items")?
                .iter()
                .map(|v| {
                    Ok(w::ListItem {
                        label: string(v, "label")?,
                        subtitle: v["sub"].as_str().map(str::to_owned),
                        icon: v["icon"].as_str().map(str::to_owned),
                        key: v["kbd"].as_str().map(str::to_owned),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            w::List {
                items,
                selected: o["selected"].as_u64().map(|n| n as usize),
                more: o["more"].as_u64().unwrap_or(0) as usize,
                viewport: None,
            }
            .element(ctx, 0.)
        }
        Some("table") => {
            let columns = v["columns"]
                .as_array()
                .ok_or("missing Table columns")?
                .iter()
                .map(|v| {
                    Ok(w::Column {
                        title: string(v, "label")?,
                        weight: v["w"]
                            .as_str()
                            .unwrap_or("1fr")
                            .strip_suffix("fr")
                            .ok_or("only fractional Table columns are supported")?
                            .parse()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let rows = v["rows"]
                .as_array()
                .ok_or("missing Table rows")?
                .iter()
                .map(|r| {
                    r.as_array()
                        .ok_or_else(|| "missing Table cells".into())
                        .and_then(|a| {
                            a.iter()
                                .map(|v| {
                                    v.as_str()
                                        .map(str::to_owned)
                                        .ok_or_else(|| "non-text Table cell".into())
                                })
                                .collect::<Result<Vec<_>>>()
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            w::Table {
                columns,
                rows,
                selected: o["selected"].as_u64().map(|n| n as usize),
                zebra: o["zebra"] == true,
                viewport: None,
            }
            .element(ctx, 0.)?
        }
        Some("input") => w::Input {
            value: o["value"].as_str().unwrap_or("").into(),
            placeholder: o["placeholder"].as_str().unwrap_or("").into(),
            state: match o["state"].as_str() {
                Some("value") => w::InputState::Value,
                Some("focus") => w::InputState::Focus,
                Some("invalid") => w::InputState::Invalid,
                _ => w::InputState::Empty,
            },
            icon: o["icon"].as_str().map(str::to_owned),
            error: o["error"].as_str().map(str::to_owned),
            edit: serde_json::from_value(o["edit"].clone())?,
        }
        .element(ctx),
        _ => return Err("unsupported L6 widget".into()),
    })
}
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in [
        "button",
        "list",
        "table",
        "input",
        "input · focused empty",
        "input · caret",
        "input · selection",
        "input · preedit",
    ] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "components".into(),
                    state: state.into(),
                    slug: state.replace(" · ", "-").replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "components.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 600,
                    checker: false,
                    fixture: Some(serde_json::json!({"layer":"components"})),
                });
            }
        }
    }
    out
}
pub fn render(out: &Path) -> Result<()> {
    render_native_gallery(&out.join("gallery"))?;
    crate::materials::render_gallery(out, &cases())
}

/// Supplementary large-data fixtures: the native view allocates visible rows only.
pub fn virtualized(kind: &str, ctx: ui::Context, offset: f32) -> Result<c::Element> {
    let viewport = Some(w::Viewport {
        height: 160.,
        offset,
    });
    Ok(match kind {
        "list" => w::List {
            items: (0..1000)
                .map(|i| w::ListItem {
                    label: format!("seat {i:04}"),
                    subtitle: Some("executor".into()),
                    icon: Some("codex".into()),
                    key: None,
                })
                .collect(),
            selected: Some(500),
            more: 0,
            viewport,
        }
        .element(ctx, 592.),
        "table" => w::Table {
            columns: vec![
                w::Column {
                    title: "seat".into(),
                    weight: 1.4,
                },
                w::Column {
                    title: "role".into(),
                    weight: 1.,
                },
                w::Column {
                    title: "now".into(),
                    weight: 1.6,
                },
            ],
            rows: (0..1000)
                .map(|i| {
                    vec![
                        format!("seat {i:04}"),
                        "executor".into(),
                        "retained view".into(),
                    ]
                })
                .collect(),
            selected: Some(500),
            zebra: true,
            viewport,
        }
        .element(ctx, 592.)?,
        _ => return Err("unknown virtualized control".into()),
    })
}

pub fn preview(kind: &str, ctx: ui::Context, position: &str) -> Result<c::Layout> {
    let pitch = ctx.density.tokens().row_h as f32
        + if kind == "list" {
            bbt_ui_model::STYLE.spacing.s1 as f32
        } else {
            0.
        };
    let offset = match position {
        "top" => 0.,
        "middle" => pitch * 500. - 40.,
        "end" => 1_000_000.,
        _ => return Err("unknown virtualized control position".into()),
    };
    Ok(virtualized(kind, ctx, offset)?.layout(
        Rect::new(24., 64., 592., 0.),
        ctx,
        &crate::components::Metrics,
    )?)
}
fn node_count(layout: &c::Layout) -> usize {
    1 + layout.children.iter().map(node_count).sum::<usize>()
}
fn render_native_gallery(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut renderer = ChromeRenderer::new(crate::images()?)?;
    let mut records = vec![];
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bboxybook L6 controls</title><style>body{font:14px system-ui;background:#11131c;color:white;margin:24px}a{color:#9bc3ff}img{width:100%;max-width:640px}section{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:12px}figure{margin:0}figcaption{padding:6px}</style><h1>L6 List and Table: 1000 rows</h1><p>Top, middle and end offsets; only visible rows become layout nodes. Row 500 is selected; the Table header stays fixed. Viewport and state are caller-owned. These are native supplements, not web parity claims.</p><p><a href=../index.html>32 web/native control comparisons</a> · <a href=virtualized.json>Layout node counts and scroll extents</a></p><section>",
    );
    for kind in ["list", "table"] {
        for position in ["top", "middle", "end"] {
            for density in gallery::DENSITIES {
                for theme in gallery::THEMES {
                    let ctx = ui::Context::new(theme, density);
                    let layout = preview(kind, ctx, position)?;
                    let scroll = if kind == "list" {
                        &layout
                    } else {
                        &layout.children[1]
                    };
                    let c::LayoutKind::ScrollView { content, offset } = &scroll.kind else {
                        return Err("missing virtualized ScrollView".into());
                    };
                    let key = format!(
                        "{kind}-{position}-{}-{}.png",
                        gallery::density_name(density),
                        gallery::theme_name(theme)
                    );
                    let mut bg = DrawList::default();
                    bg.rect(
                        Rect::new(0., 0., 640., 280.),
                        0.,
                        Color::hex(theme.palette().bg),
                    );
                    bg.text(
                        Rect::new(18., 12., 610., 24.),
                        format!(
                            "{kind} · 1000 rows · {position} · {} · {}",
                            gallery::density_name(density),
                            gallery::theme_name(theme)
                        ),
                        TextStyle {
                            scale: TypeScale::Caption,
                            ..Default::default()
                        },
                        DrawCtx::new(theme, density).muted(),
                    );
                    let pixels = renderer.render_to_rgba(&bg, 640, 280, 1.)?;
                    let pixels = crate::components::execute(
                        &mut renderer,
                        c::paint(&layout, ctx),
                        pixels,
                        [640, 280],
                        ctx,
                    )?;
                    bbt_render::write_png(out.join(&key), &pixels, 640, 280)?;
                    records.push(serde_json::json!({"key":key,"rows":1000,"layout_nodes":node_count(&layout),"scroll_extent":content,"offset":offset}));
                    html.push_str(&format!("<figure><figcaption>{key}</figcaption><a href='{key}'><img loading=lazy src='{key}'></a></figure>"));
                }
            }
        }
    }
    html.push_str("</section>");
    fs::write(out.join("index.html"), html)?;
    fs::write(
        out.join("virtualized.json"),
        serde_json::to_vec_pretty(&records)?,
    )?;
    println!(
        "native virtualized gallery: {} PNGs; {}",
        records.len(),
        out.join("index.html").display()
    );
    Ok(())
}
