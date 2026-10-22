//! L5 gallery and isolated comparisons with the web glyph/icon pages.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_render::chrome::{
    ChromeRenderer,
    materials::{self, Backdrop, Scene},
};
use bbt_ui::atoms::{self, Atom, AtomCtx};
use bbt_ui_model::{
    Activity, Density, Grade, SeatColor, Theme, WaitKind,
    draw::{Color, DrawCtx, DrawList, TextStyle, TypeScale},
    layout::{Rect, TextMeasure, TextStyle as MeasureStyle},
};
use std::{fs, path::Path};

pub const KINDS: [&str; 9] = [
    "hairline",
    "glyph",
    "icon",
    "badge",
    "chip",
    "pill",
    "swatch",
    "spinner",
    "checkerboard",
];
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &MeasureStyle) -> f32 {
        bbt_render::chrome::text::rasterize(
            text,
            TextStyle {
                scale: TypeScale::Custom(style.size),
                weight: style.weight,
                mono: style.mono,
                tracking_em: style.tracking_em,
                ..Default::default()
            },
            None,
            1.,
        )
        .expect("native atom gallery requires CoreText font metrics")
        .advance
    }
}
fn draw_context<'a>(draw: DrawCtx, measure: &'a dyn TextMeasure) -> AtomCtx<'a> {
    AtomCtx {
        draw,
        measure,
        scale: 1.,
    }
}
pub fn cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| matches!(c.component.as_str(), "glyph" | "harness-icon"))
        .map(|mut c| {
            c.fixture = Some(serde_json::json!({"layer":"atoms"}));
            c
        })
        .collect()
}
pub fn preview(kind: &str, theme: Theme, density: Density) -> Result<DrawList> {
    let mut out = DrawList::default();
    let draw = DrawCtx::new(theme, density);
    let ctx = draw_context(draw, &Metrics);
    out.rect(
        Rect::new(0., 0., 640., 160.),
        0.,
        Color::hex(theme.palette().bg),
    );
    out.text(
        Rect::new(18., 12., 600., 24.),
        format!(
            "{kind} · {} · {}",
            gallery::density_name(density),
            gallery::theme_name(theme)
        ),
        TextStyle {
            scale: TypeScale::Caption,
            ..Default::default()
        },
        draw.muted(),
    );
    let rect = Rect::new(24., 64., 560., 48.);
    let g = density.tokens().glyph as f32;
    let color = Color::hex(theme.palette().accent);
    match kind {
        "hairline" => atoms::Hairline.paint(rect, &ctx, &mut out),
        "glyph" => {
            let mut x = rect.x;
            for grade in [Grade::Reported, Grade::Shim, Grade::Inferred] {
                for activity in [
                    Activity::Active,
                    Activity::Idle,
                    Activity::Waiting(WaitKind::Question),
                    Activity::Errored,
                ] {
                    atoms::Glyph {
                        activity,
                        grade,
                        seat_color: color,
                    }
                    .paint(Rect::new(x, 72., g, g), &ctx, &mut out);
                    x += 32.;
                }
                x += 16.;
            }
        }
        "icon" => {
            for (i, name) in ["claude", "codex", "pi", "omp", "zsh", "gear", "sidebarfox"]
                .into_iter()
                .enumerate()
            {
                let size = density.tokens().icon as f32;
                atoms::Icon { name, color }.paint(
                    Rect::new(rect.x + i as f32 * 64., 72., size, size),
                    &ctx,
                    &mut out,
                );
            }
        }
        "badge" => {
            for (i, label) in ["3", "⌘1", "12"].into_iter().enumerate() {
                atoms::Badge {
                    label,
                    selected: i == 1,
                }
                .paint(
                    Rect::new(rect.x + i as f32 * 64., 72., 38., 24.),
                    &ctx,
                    &mut out,
                );
            }
        }
        "chip" => {
            for (i, (label, outlined)) in [("TASK", false), ("inv signatures", true)]
                .into_iter()
                .enumerate()
            {
                atoms::Chip {
                    label,
                    color,
                    outlined,
                }
                .paint(
                    Rect::new(rect.x + i as f32 * 160., 72., 130., 22.),
                    &ctx,
                    &mut out,
                );
            }
        }
        "pill" => atoms::Pill {
            items: &[
                ("Launch", "↵"),
                ("Split right", "⌘↵"),
                ("Split down", "⌘⇧↵"),
            ],
        }
        .paint(Rect::new(24., 64., 520., 34.), &ctx, &mut out),
        "swatch" => {
            for (i, hovered) in [false, true].into_iter().enumerate() {
                atoms::Swatch { color, hovered }.paint(
                    Rect::new(24. + i as f32 * 48., 72., 12., 12.),
                    &ctx,
                    &mut out,
                );
            }
        }
        "spinner" => {
            for (i, phase) in [0., 0.25, 0.5, 0.75].into_iter().enumerate() {
                atoms::Spinner { phase }.paint(
                    Rect::new(24. + i as f32 * 48., 64., 24., 24.),
                    &ctx,
                    &mut out,
                );
            }
        }
        "checkerboard" => atoms::Checkerboard.paint(Rect::new(24., 52., 560., 80.), &ctx, &mut out),
        _ => return Err("unknown atom gallery entry".into()),
    }
    out.validate()?;
    Ok(out)
}
pub fn render(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let mut renderer = ChromeRenderer::new(crate::images()?)?;
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bboxybook L5 atoms</title><style>body{font:14px system-ui;background:#11131c;color:white;margin:24px}a{color:#9bc3ff}img{max-width:640px;width:100%}section{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:12px}figure{margin:0}figcaption{padding:6px}</style><h1>L5 atoms</h1><p>Nine stateless painters × four densities × two themes. Explicit font metrics and spinner phase; no layout tree, clock or input.</p><p><a href=parity/index.html>Glyph and harness-icon parity, worst first</a></p><section>",
    );
    for kind in KINDS {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                let key = format!(
                    "atoms/{kind}-{}-{}.png",
                    gallery::density_name(density),
                    gallery::theme_name(theme)
                );
                let path = out.join(&key);
                fs::create_dir_all(path.parent().unwrap())?;
                let image =
                    renderer.render_to_rgba(&preview(kind, theme, density)?, 640, 160, 1.)?;
                bbt_render::write_png(&path, &image, 640, 160)?;
                html.push_str(&format!("<figure><figcaption>{key}</figcaption><a href='{key}'><img src='{key}'></a></figure>"));
            }
        }
    }
    html.push_str("</section>");
    fs::write(out.join("index.html"), html)?;
    crate::materials::render_gallery(&out.join("parity"), &cases())?;
    Ok(())
}
fn number(v: &serde_json::Value, key: &str) -> Result<f32> {
    v[key]
        .as_f64()
        .filter(|x| x.is_finite())
        .map(|x| x as f32)
        .ok_or_else(|| format!("invalid atom fixture {key}").into())
}
fn paint_atom(
    atom: &serde_json::Value,
    rect: Rect,
    draw: DrawCtx,
    foreground: &mut DrawList,
) -> Result<()> {
    let ctx = draw_context(draw, &Metrics);
    let rgba: Vec<f32> = serde_json::from_value(atom["color"].clone())?;
    let rgba: [f32; 4] = rgba.try_into().map_err(|_| "invalid atom colour")?;
    if atom["kind"] == "icon" {
        atoms::Icon {
            name: atom["name"].as_str().ok_or("icon name missing")?,
            color: Color(rgba),
        }
        .paint(rect, &ctx, foreground);
    } else if atom["kind"] == "glyph" {
        let activity = match atom["activity"].as_str() {
            Some("active") => Activity::Active,
            Some("idle") => Activity::Idle,
            Some("errored") => Activity::Errored,
            Some("waiting") => Activity::Waiting(match atom["wait_kind"].as_str() {
                Some("permission") => WaitKind::Permission,
                Some("auth") => WaitKind::Auth,
                Some("question") => WaitKind::Question,
                _ => return Err("waiting kind missing".into()),
            }),
            _ => return Err("unknown glyph state".into()),
        };
        let grade = match atom["grade"].as_str() {
            Some("reported") => Grade::Reported,
            Some("shim") => Grade::Shim,
            Some("inferred") => Grade::Inferred,
            _ => return Err("glyph grade missing".into()),
        };
        // Identity is shared data; the atom owns state colours, grade and kind geometry.
        let c = SeatColor::Derived.resolve("demo", draw.theme);
        let seat_color = Color::hsl(c.hue, c.saturation / 100., c.lightness / 100.);
        atoms::Glyph {
            activity,
            grade,
            seat_color,
        }
        .paint(rect, &ctx, foreground);
    } else {
        return Err("unknown captured atom kind".into());
    }
    Ok(())
}
pub fn render_case(renderer: &mut ChromeRenderer, case: &Case) -> Result<Vec<u8>> {
    let fix = case.fixture.as_ref().ok_or("missing atom fixture")?;
    let bg = &fix["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("atom backdrop missing")?,
    ))?;
    let x = number(&bg["crop"], "x")?;
    let y = number(&bg["crop"], "y")?;
    let draw = DrawCtx::new(gallery::theme(&case.theme), gallery::density(&case.density));
    let mut foreground = DrawList::default();
    let records = fix["atoms"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("atom reference is empty")?;
    for atom in records {
        let r = &atom["rect"];
        let rect = Rect::new(
            number(r, "x")? + x,
            number(r, "y")? + y,
            number(r, "w")?,
            number(r, "h")?,
        );
        paint_atom(atom, rect, draw, &mut foreground)?;
    }
    let image = materials::render_composed_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::Rgba(&pixels),
            items: &[],
            width: w,
            height: h,
            scale: 1.,
            time_ms: 0.,
            theme: draw.theme,
        },
        &foreground,
    )?;
    let (x, y) = (x.round() as u32, y.round() as u32);
    if x + case.width > w || y + case.height > h {
        return Err("atom crop outside backdrop".into());
    }
    let mut cropped = Vec::with_capacity((case.width * case.height * 4) as usize);
    for row in y..y + case.height {
        let start = ((row * w + x) * 4) as usize;
        cropped.extend_from_slice(&image[start..start + case.width as usize * 4]);
    }
    Ok(cropped)
}

/// Atom slots are caller-owned fixture bounds. Child text positions, advances
/// and baselines come from the same emitted primitives and fonts as paint.
pub(crate) fn snapshots(cases: &[Case]) -> Result<Vec<serde_json::Value>> {
    use bbt_ui_model::draw::Prim;
    use serde_json::{Value, json};
    let mut snapshots = vec![];
    for case in cases
        .iter()
        .filter(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "atoms"))
    {
        let f = case.fixture.as_ref().ok_or("missing atom fixture")?;
        let draw = DrawCtx::new(gallery::theme(&case.theme), gallery::density(&case.density));
        let records = f["atoms"].as_array().ok_or("missing atom records")?;
        let web_rect = |v: &Value| -> Result<Value> {
            Ok(json!([
                number(v, "x")?,
                number(v, "y")?,
                number(v, "w")?,
                number(v, "h")?
            ]))
        };
        let mut children = vec![];
        for (index, atom) in records.iter().enumerate() {
            let r = &atom["rect"];
            let rect = Rect::new(
                number(r, "x")?,
                number(r, "y")?,
                number(r, "w")?,
                number(r, "h")?,
            );
            let mut list = DrawList::default();
            paint_atom(atom, rect, draw, &mut list)?;
            let mut parts = vec![];
            let mut text_index = 0;
            for prim in &list.primitives {
                match prim {
                    Prim::Text {
                        origin,
                        text,
                        style,
                        max_width,
                        ..
                    } => {
                        let diagnostic = &atom[if text_index == 0 { "text" } else { "kind_text" }];
                        let key = if text_index == 0 {
                            "symbol"
                        } else {
                            "waiting-kind-text"
                        };
                        let mask =
                            bbt_render::chrome::text::rasterize(text, *style, *max_width, 1.)?;
                        let metrics = bbt_render::chrome::text::rasterize("Hg", *style, None, 1.)?;
                        // The glyph and kind use CSS line-height:1. The painter
                        // centres its 1.4em primitive in that same logical line.
                        let height = (style.pixels() * 64.).floor() / 64.;
                        let y = origin[1] + (style.pixels() * 1.4 - height) * 0.5;
                        let baseline = metrics.ascent.round()
                            + ((height - metrics.ascent.round() - metrics.descent.round()) * 0.5)
                                .floor();
                        parts.push(json!({"key":key,"kind":"text",
                            "native":[origin[0],y,mask.advance,height],
                            "web":web_rect(&diagnostic["rect"])?,
                            "baseline":{"native":y+baseline,"web":number(&diagnostic["rect"],"y")?
                                + number(&diagnostic["font_diagnostics"],"baseline")?},"children":[]}));
                        text_index += 1;
                    }
                    Prim::Rect {
                        rect, stroke: None, ..
                    } => {
                        parts.push(json!({"key":"waiting-kind-box","kind":"box",
                            "native":[rect.x,rect.y,rect.w,rect.h],"web":web_rect(&atom["kind_rect"])?,"children":[]}));
                    }
                    _ => {}
                }
            }
            children.push(json!({"key":format!("atom{index}"),"kind":"box",
                "native":[rect.x,rect.y,rect.w,rect.h],"web":web_rect(r)?,"children":parts}));
        }
        snapshots.push(json!({"key":case.relative(),"scope":"caller-owned atom slots; child glyph/kind geometry independently measured",
            "tree":{"kind":"box","native":[0,0,case.width,case.height],"web":[0,0,case.width,case.height],"children":children}}));
    }
    Ok(snapshots)
}

#[cfg(all(test, target_os = "macos"))]
mod geometry_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn glyph_geometry_uses_native_primitives_and_fonts_not_child_diagnostics() -> Result<()> {
        let mut case = cases()
            .into_iter()
            .find(|c| {
                c.component == "glyph"
                    && c.state == "waiting"
                    && c.density == "cozy"
                    && c.theme == "dark"
            })
            .unwrap();
        let diagnostic =
            json!({"rect":{"x":999,"y":998,"w":1,"h":1},"font_diagnostics":{"baseline":99}});
        case.fixture = Some(
            json!({"layer":"atoms","atoms":[{"kind":"glyph","activity":"waiting","grade":"reported","wait_kind":"question","color":[1,0.5,0,1],"rect":{"x":10,"y":20,"w":12,"h":12},"text":diagnostic,"kind_text":diagnostic,"kind_rect":{"x":900,"y":901,"w":1,"h":1}}]}),
        );
        let first = snapshots(&[case.clone()])?;
        let parts = &first[0]["tree"]["children"][0]["children"];
        assert!(parts[0]["native"][0].as_f64().unwrap() < 30.);
        assert!(parts[0]["native"][2].as_f64().unwrap() > 5.);
        assert!(parts[0]["baseline"]["native"].as_f64().unwrap() < 50.);
        assert_eq!(parts[0]["web"][0], 999.);
        case.fixture.as_mut().unwrap()["atoms"][0]["text"]["rect"]["x"] = json!(-500);
        let changed = snapshots(&[case.clone()])?;
        assert_eq!(
            parts[0]["native"],
            changed[0]["tree"]["children"][0]["children"][0]["native"]
        );
        case.fixture.as_mut().unwrap()["atoms"][0]["rect"]["h"] = json!(16);
        let resized = snapshots(&[case])?;
        assert_eq!(
            resized[0]["tree"]["children"][0]["children"][0]["native"][3],
            16.
        );
        Ok(())
    }
}
