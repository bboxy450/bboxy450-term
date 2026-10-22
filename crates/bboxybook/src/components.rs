//! L6 host: converts captured public UI inputs into retained native components.
//! Child DOM rectangles are diagnostics only; native layout never consumes them.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_render::chrome::{
    ChromeRenderer,
    materials::{self, Backdrop, Kind, Scene, Surface},
};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList, TextStyle, TypeScale},
    layout::{Axis, Rect, TextMeasure, TextStyle as MeasureStyle},
};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub struct Metrics;
fn font(style: &MeasureStyle) -> TextStyle {
    TextStyle {
        scale: TypeScale::Custom(style.size),
        weight: style.weight,
        mono: style.mono,
        italic: style.italic,
        tracking_em: style.tracking_em,
        ..Default::default()
    }
}
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &MeasureStyle) -> f32 {
        bbt_render::chrome::text::rasterize(text, font(style), None, 1.)
            .expect("native component font")
            .advance
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &MeasureStyle) -> [f32; 2] {
        let m = bbt_render::chrome::text::rasterize("Hg", font(style), None, 1.)
            .expect("native component font metrics");
        [m.ascent, m.descent]
    }
}

pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in ["box", "text", "stack"] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "components".into(),
                    state: state.into(),
                    slug: state.into(),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "components.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 600,
                    checker: false,
                    fixture: Some(json!({"layer":"components"})),
                });
            }
        }
    }
    out
}
pub fn context(case: &Case) -> ui::Context {
    ui::Context::new(gallery::theme(&case.theme), gallery::density(&case.density))
}
fn items(v: &Value, ctx: ui::Context) -> Result<Vec<c::Item>> {
    v.as_array()
        .ok_or("component children missing")?
        .iter()
        .map(|v| Ok(element(v, ctx)?.into()))
        .collect()
}
pub fn element(v: &Value, ctx: ui::Context) -> Result<c::Element> {
    Ok(match v["kind"].as_str().ok_or("component kind missing")? {
        "text" => {
            let o = &v["options"];
            let mut s = ui::style::Text {
                step: serde_json::from_value(Value::String(
                    o["step"].as_str().unwrap_or("body").into(),
                ))?,
                weight: o["weight"].as_u64().unwrap_or(400) as u16,
                mono: o["mono"].as_bool().unwrap_or(false),
                italic: o["italic"].as_bool().unwrap_or(false),
                uppercase: o["caps"].as_bool().unwrap_or(false),
                ..Default::default()
            };
            if s.uppercase {
                s.tracking_em = o["track"].as_f64().unwrap_or(0.22) as f32;
            }
            s.paint = ui::Paint::solid(if o["dim"] == true {
                ui::Color::Muted
            } else {
                match o["color"].as_str() {
                    Some("var(--on-accent)") => ui::Color::OnAccent,
                    Some("var(--fg-dim)") => ui::Color::Muted,
                    Some("var(--accent)") => ui::Color::Accent,
                    None => ui::Color::Foreground,
                    Some(_) => return Err("unsupported component Text colour".into()),
                }
            });
            c::Element::Text(c::Text::new(
                v["value"].as_str().ok_or("component text missing")?,
                s,
            ))
        }
        "spacer" => c::Element::Spacer,
        "stack" => {
            let o = &v["options"];
            c::Element::Stack(c::Stack {
                direction: if o["dir"] == "row" {
                    Axis::Row
                } else {
                    Axis::Column
                },
                gap: ui::Length::Px(o["gap"].as_f64().unwrap_or(ctx.density.tokens().gap) as f32),
                align: match o["align"].as_str() {
                    None | Some("stretch") => c::Alignment::Stretch,
                    Some("start") | Some("flex-start") => c::Alignment::Start,
                    Some("end") | Some("flex-end") => c::Alignment::End,
                    Some("center") => c::Alignment::Center,
                    Some("baseline") => c::Alignment::Baseline,
                    _ => return Err("unsupported component alignment".into()),
                },
                children: items(&v["children"], ctx)?,
            })
        }
        "box" => {
            let input = &v["style"];
            let style = if let Some(name) = input.as_str() {
                ui::Style::new(serde_json::from_value(Value::String(name.into()))?)
            } else if input["material"] == "gloss" {
                let mut s = ui::Style::new(ui::Preset::Band);
                if let Some(color) = input["swarm"].as_str() {
                    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(Color::hex(color).0)));
                }
                if let Some(r) = input["radius"].as_f64() {
                    s.overrides.radii = Some([ui::Length::Px(r as f32); 4]);
                }
                if let Some(h) = input["height"].as_f64() {
                    s.overrides.height = Some(ui::Length::Px(h as f32));
                }
                s
            } else if input["fill"] == "transparent" {
                ui::Style::new(ui::Preset::Row)
            } else {
                return Err("unsupported custom Box input".into());
            };
            c::Element::Box(c::Box::new(style, items(&v["children"], ctx)?))
        }
        "overlay" | "modal" => return crate::overlays::element(v, ctx),
        _ => return crate::widgets::element(v, ctx),
    })
}
fn num(v: &Value, k: &str) -> Result<f32> {
    v[k].as_f64()
        .filter(|x| x.is_finite())
        .map(|x| x as f32)
        .ok_or_else(|| format!("invalid component {k}").into())
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let fix = case.fixture.as_ref().ok_or("missing component fixture")?;
    let host = &fix["component"]["host"];
    let crop = &fix["backdrop"]["crop"];
    let root = element(&fix["component"]["tree"], context(case))?;
    let layout = root.layout(
        Rect::new(
            num(host, "x")? + num(crop, "x")?,
            num(host, "y")? + num(crop, "y")?,
            num(host, "w")?,
            0.,
        ),
        context(case),
        &Metrics,
    )?;
    validate_wrapping(&layout, &fix["component"]["tree"])?;
    Ok(layout)
}
fn validate_wrapping(layout: &c::Layout, reference: &Value) -> Result<()> {
    if reference["options"]["wrap"] == true
        && layout
            .children
            .iter()
            .any(|c| c.rect.right() > layout.rect.right() + 0.01)
    {
        return Err("overflowing wrapping row needs a flow-layout pass".into());
    }
    if let Some(children) = reference["children"].as_array() {
        for (n, r) in layout.children.iter().zip(children) {
            validate_wrapping(n, r)?;
        }
    }
    Ok(())
}
fn scene<'a>(
    pixels: &'a [u8],
    size: [u32; 2],
    items: &'a [Surface],
    ctx: ui::Context,
    time_ms: f32,
) -> Scene<'a> {
    Scene {
        backdrop: Backdrop::Rgba(pixels),
        items,
        width: size[0],
        height: size[1],
        scale: 1.,
        time_ms,
        theme: ctx.theme,
    }
}
pub fn execute(
    renderer: &mut ChromeRenderer,
    ops: Vec<c::Paint>,
    pixels: Vec<u8>,
    size: [u32; 2],
    ctx: ui::Context,
) -> Result<Vec<u8>> {
    execute_at(renderer, ops, pixels, size, ctx, 450.)
}
pub fn execute_at(
    renderer: &mut ChromeRenderer,
    ops: Vec<c::Paint>,
    mut pixels: Vec<u8>,
    size: [u32; 2],
    ctx: ui::Context,
    time_ms: f32,
) -> Result<Vec<u8>> {
    let mut underlay = DrawList::default();
    for op in ops {
        match op {
            c::Paint::Group {
                ops,
                opacity,
                saturation,
            } => {
                if !underlay.primitives.is_empty() {
                    pixels = materials::render_composed_to_rgba(
                        renderer,
                        scene(&pixels, size, &[], ctx, time_ms),
                        &underlay,
                    )?;
                    underlay = DrawList::default();
                }
                let layer = renderer.with_group_layer(|renderer| {
                    execute_at(renderer, ops, vec![0; pixels.len()], size, ctx, time_ms)
                })?;
                pixels = renderer.composite_group_rgba(
                    &layer,
                    &pixels,
                    size,
                    bbt_render::chrome::group::Filter {
                        opacity,
                        saturation,
                    },
                )?;
            }
            c::Paint::Underlay(list) => underlay.primitives.extend(list.primitives),
            c::Paint::Draw(list) => {
                let mut combined = std::mem::take(&mut underlay);
                combined.primitives.extend(list.primitives);
                pixels = materials::render_composed_to_rgba(
                    renderer,
                    scene(&pixels, size, &[], ctx, time_ms),
                    &combined,
                )?;
            }
            c::Paint::Material {
                rect,
                radii,
                kind,
                color,
                blur,
                glow,
                clips,
            } => {
                let material = match kind {
                    ui::Material::Gloss => {
                        let materials::Material::Gloss(mut g) =
                            crate::materials::material(Kind::Gloss, ctx.theme)
                        else {
                            unreachable!()
                        };
                        g.color = color;
                        materials::Material::Gloss(g)
                    }
                    ui::Material::Frosted => {
                        let materials::Material::Frosted(mut f) =
                            crate::materials::material(Kind::Frosted, ctx.theme)
                        else {
                            unreachable!()
                        };
                        if let Some(b) = blur {
                            f.blur_radius = b.radius;
                            f.saturation = b.saturation;
                        }
                        materials::Material::Frosted(f)
                    }
                    ui::Material::Glow => {
                        let materials::Material::Glow(mut g) =
                            crate::materials::material(Kind::Glow, ctx.theme)
                        else {
                            unreachable!()
                        };
                        configure_glow(&mut g, ctx, glow);
                        materials::Material::Glow(g)
                    }
                    ui::Material::Flat => materials::Material::Flat(color),
                };
                let items = [Surface {
                    rect,
                    radii,
                    clips: clips
                        .into_iter()
                        .map(|c| materials::Clip {
                            rect: c.rect,
                            radii: c.radii,
                        })
                        .collect(),
                    material,
                }];
                pixels = materials::render_layered_to_rgba(
                    renderer,
                    scene(&pixels, size, &items, ctx, time_ms),
                    &underlay,
                    &DrawList::default(),
                )?;
                underlay = DrawList::default();
            }
        }
    }
    if !underlay.primitives.is_empty() {
        pixels = materials::render_composed_to_rgba(
            renderer,
            scene(&pixels, size, &[], ctx, time_ms),
            &underlay,
        )?;
    }
    Ok(pixels)
}

/// Preserve group boundaries when a flat gallery coalesces adjacent chrome.
pub(crate) fn append_draw(ops: Vec<c::Paint>, out: &mut DrawList) -> Result<()> {
    for op in ops {
        match op {
            c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                out.primitives.extend(list.primitives)
            }
            c::Paint::Group {
                ops,
                opacity,
                saturation,
            } => {
                let mut inner = DrawList::default();
                append_draw(ops, &mut inner)?;
                out.push(bbt_ui_model::draw::Prim::Group {
                    primitives: inner.primitives,
                    opacity,
                    saturation,
                });
            }
            c::Paint::Material { .. } => {
                return Err("flat composition requires a material pass".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn configure_glow(g: &mut materials::Glow, ctx: ui::Context, state: Option<ui::Glow>) {
    g.color = Color(ctx.seat);
    g.focus = if ctx.focused { 1. } else { 0. };
    if let Some(state) = state {
        use ui::GlowActivity as A;
        g.activity = match state.activity {
            A::Idle => materials::Activity::Idle,
            A::Active => materials::Activity::Active,
            A::Waiting => materials::Activity::Waiting,
            A::Errored => materials::Activity::Errored,
        };
        let idle = bbt_ui_model::TOKENS.glow.particles_idle as u32;
        g.particles = match state.activity {
            A::Active => bbt_ui_model::TOKENS.glow.particles_active as u32,
            A::Waiting => idle * 2,
            A::Idle | A::Errored => idle,
        };
        if let Some(alpha) = state.halo_alpha {
            g.halo_alpha = alpha;
        }
        if let Some(n) = state.particles {
            g.particles = n;
        }
        g.seed = state.seed as f32;
        for color in [&mut g.color, &mut g.waiting, &mut g.errored] {
            *color = color.alpha(state.inferred_multiplier);
        }
    }
}
pub fn render_case(renderer: &mut ChromeRenderer, case: &Case) -> Result<Vec<u8>> {
    let fix = case.fixture.as_ref().ok_or("component fixture missing")?;
    let bg = &fix["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("component backdrop missing")?,
    ))?;
    let image = execute(
        renderer,
        c::paint(&layout_case(case)?, context(case)),
        pixels,
        [w, h],
        context(case),
    )?;
    let x = num(&bg["crop"], "x")?.round() as u32;
    let y = num(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("component crop outside backdrop".into());
    }
    let mut crop = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        crop.extend_from_slice(&image[i..i + case.width as usize * 4]);
    }
    Ok(crop)
}
fn snapshot(
    layout: &c::Layout,
    reference: &Value,
    crop: [f32; 2],
    worst: &mut f32,
) -> Result<Value> {
    let r = layout.rect;
    let native = [r.x - crop[0], r.y - crop[1], r.w, r.h];
    let web = [
        num(&reference["rect"], "x")?,
        num(&reference["rect"], "y")?,
        num(&reference["rect"], "w")?,
        num(&reference["rect"], "h")?,
    ];
    for (a, b) in native.into_iter().zip(web) {
        *worst = worst.max((a - b).abs());
    }
    let empty = vec![];
    let reference_children = reference["children"].as_array().unwrap_or(&empty);
    if reference_children.len() != layout.children.len() {
        return Err("native/reference component trees have different child counts".into());
    }
    let children = layout
        .children
        .iter()
        .zip(reference_children)
        .map(|(n, w)| snapshot(n, w, crop, worst))
        .collect::<Result<Vec<_>>>()?;
    let baseline = matches!(layout.kind, c::LayoutKind::Text { .. }).then(|| {
        json!({
            "native":layout.rect.y-crop[1]+layout.baseline,
            "web":reference["font_diagnostics"]["baseline"].as_f64().map(|b| b+f64::from(web[1]))
        })
    });
    Ok(
        json!({"kind":reference["kind"],"native":native,"web":web,"baseline":baseline,"children":children}),
    )
}
pub fn render(out: &Path) -> Result<()> {
    render_native_gallery(&out.join("gallery"))?;
    crate::materials::render_gallery(out, &cases())?;
    Ok(())
}
pub(crate) fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut records = vec![];
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "components")
    }) {
        let fix = case.fixture.as_ref().unwrap();
        let crop = &fix["backdrop"]["crop"];
        let mut worst = 0.;
        let tree = snapshot(
            &layout_case(case)?,
            &fix["component"]["tree"],
            [num(crop, "x")?, num(crop, "y")?],
            &mut worst,
        )?;
        records.push(json!({"key":case.relative(),"max_geometry_error":worst,"tree":tree}));
    }
    Ok(records)
}

fn label(value: impl Into<String>, step: ui::TypeStep) -> c::Element {
    c::Element::Text(c::Text::new(
        value,
        ui::Text {
            step,
            ..Default::default()
        },
    ))
}
fn column(children: Vec<c::Item>, gap: ui::Length) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: Axis::Column,
        gap,
        align: c::Alignment::Stretch,
        children,
    })
}
pub fn preview(kind: &str, ctx: ui::Context) -> Result<c::Layout> {
    let root = if let Some(material) = kind.strip_prefix("box-") {
        let mut children = vec![];
        for radius in [0., 6., 12., 20.] {
            let preset = match material {
                "flat" => ui::Preset::Card,
                "gloss" => ui::Preset::Band,
                "frosted" => ui::Preset::Modal,
                "glow" => ui::Preset::PaneGlow,
                _ => return Err("unknown Box material".into()),
            };
            let mut style = ui::Style::new(preset);
            style.overrides.radii = Some([ui::Length::Px(radius); 4]);
            style.overrides.height = Some(ui::Length::Px(112.));
            style.overrides.padding = Some([ui::Length::Spacing(4); 4]);
            let mut b = c::Box::new(
                style,
                vec![label(format!("r {radius}"), ui::TypeStep::Body).into()],
            );
            b.direction = Axis::Column;
            b.align = c::Alignment::Center;
            children.push(c::Item {
                element: c::Element::Box(b),
                extent: c::Extent::Fill,
            });
        }
        c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Spacing(7),
            align: c::Alignment::Stretch,
            children,
        })
    } else if let Some(position) = kind.strip_prefix("scroll-") {
        let content = column(
            (0..12)
                .map(|i| {
                    let mut style = ui::Style::new(ui::Preset::Row);
                    if i % 2 == 0 {
                        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface2));
                    }
                    c::Element::Box(c::Box::new(
                        style,
                        vec![
                            label(
                                format!("row {:02} · retained content", i + 1),
                                ui::TypeStep::Row,
                            )
                            .into(),
                        ],
                    ))
                    .into()
                })
                .collect(),
            ui::Length::Spacing(2),
        );
        let offset = match position {
            "top" => 0.,
            "middle" => ctx.density.tokens().row_h as f32 * 4.,
            "end" => 10_000.,
            _ => return Err("unknown ScrollView offset".into()),
        };
        c::Element::ScrollView(c::ScrollView {
            viewport: [592., 160.],
            offset: [0., offset],
            content_width: None,
            content: std::boxed::Box::new(content),
        })
    } else if kind == "text-wrap" {
        let mut mixed = c::Text::new("Selectable text with ", ui::Text::default());
        mixed.runs.push(c::TextRun {
            value: "mixed styles".into(),
            style: ui::Text {
                step: ui::TypeStep::Subtitle,
                weight: 600,
                paint: ui::Paint::solid(ui::Color::Accent),
                ..Default::default()
            },
        });
        mixed.runs.push(c::TextRun {
            value: " wraps at words inside its allocated width. No hidden clock or input state."
                .into(),
            style: ui::Text::default(),
        });
        let mut ellipsis = c::Text::new(
            "A deliberately long seat title that ends in an ellipsis when it reaches the available width",
            ui::Text::default(),
        );
        ellipsis.wrap = false;
        ellipsis.ellipsis = true;
        column(
            vec![
                c::Element::Text(mixed).into(),
                c::Element::Text(ellipsis).into(),
            ],
            ui::Length::Spacing(7),
        )
    } else {
        return Err("unknown component preview".into());
    };
    let width = if kind == "text-wrap" { 300. } else { 592. };
    Ok(root.layout(Rect::new(24., 64., width, 0.), ctx, &Metrics)?)
}
fn render_native_gallery(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut renderer = ChromeRenderer::new(crate::images()?)?;
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bboxybook L6 components</title><style>body{font:14px system-ui;background:#11131c;color:white;margin:24px}a{color:#9bc3ff}img{width:100%;max-width:640px}section{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:12px}figure{margin:0}figcaption{padding:6px}</style><h1>L6 Box, Text, Stack, ScrollView</h1><p>Retained component trees; shared layout solver; Style-owned materials, fill, border and shadow. Four radii per Box image. ScrollView offsets top/middle/end. Mixed Text runs, word wrapping and ellipsis; selection is metadata until the input controller.</p><p><a href=../index.html>24 web/native comparisons</a> · <a href=../layouts.json>Child layout snapshots</a></p><section>",
    );
    for kind in [
        "box-flat",
        "box-gloss",
        "box-frosted",
        "box-glow",
        "scroll-top",
        "scroll-middle",
        "scroll-end",
        "text-wrap",
    ] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                let ctx = ui::Context::new(theme, density);
                let layout = preview(kind, ctx)?;
                let mut background = DrawList::default();
                background.rect(
                    Rect::new(0., 0., 640., 260.),
                    0.,
                    Color::hex(theme.palette().bg),
                );
                if kind == "box-frosted" {
                    use ui::atoms::Atom;
                    ui::atoms::Checkerboard.paint(
                        Rect::new(0., 48., 640., 160.),
                        &ui::atoms::AtomCtx {
                            draw: DrawCtx::new(theme, density),
                            measure: &Metrics,
                            scale: 1.,
                        },
                        &mut background,
                    );
                }
                background.text(
                    Rect::new(18., 12., 610., 24.),
                    format!(
                        "{kind} · {} · {}",
                        gallery::density_name(density),
                        gallery::theme_name(theme)
                    ),
                    TextStyle {
                        scale: TypeScale::Caption,
                        ..Default::default()
                    },
                    DrawCtx::new(theme, density).muted(),
                );
                let pixels = renderer.render_to_rgba(&background, 640, 260, 1.)?;
                let pixels = execute(
                    &mut renderer,
                    c::paint(&layout, ctx),
                    pixels,
                    [640, 260],
                    ctx,
                )?;
                let key = format!(
                    "{kind}-{}-{}.png",
                    gallery::density_name(density),
                    gallery::theme_name(theme)
                );
                bbt_render::write_png(out.join(&key), &pixels, 640, 260)?;
                html.push_str(&format!("<figure><figcaption>{key}</figcaption><a href='{key}'><img loading=lazy src='{key}'></a></figure>"));
            }
        }
    }
    html.push_str("</section>");
    fs::write(out.join("index.html"), html)?;
    println!(
        "native L6 gallery: 64 PNGs; {}",
        out.join("index.html").display()
    );
    Ok(())
}
