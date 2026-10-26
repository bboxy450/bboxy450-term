//! L6 Overlay/Modal previews; module content is rebased in later briefs.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{STYLE, draw::Color, layout::Rect};
use serde_json::Value;
use std::{fs, path::Path};

pub fn element(v: &Value, ctx: ui::Context) -> Result<c::Element> {
    Ok(match v["kind"].as_str() {
        Some("overlay") => {
            let children = v["children"].as_array().ok_or("Overlay content missing")?;
            if children.len() != 1 {
                return Err("Overlay fixture requires one content subtree".into());
            }
            c::Element::Overlay(c::Overlay::new(crate::components::element(
                &children[0],
                ctx,
            )?))
        }
        Some("modal") => ui::overlays::Modal::new(
            v["title"].as_str().ok_or("Modal title missing")?,
            crate::components::element(&v["body"], ctx)?,
            v["actions"]
                .as_array()
                .ok_or("Modal actions missing")?
                .iter()
                .map(|a| crate::components::element(a, ctx))
                .collect::<Result<_>>()?,
        )
        .element(ctx),
        _ => return Err("unsupported L6 floating component".into()),
    })
}
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in ["overlay", "modal"] {
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
                    fixture: Some(serde_json::json!({"layer":"components"})),
                });
            }
        }
    }
    out
}
pub fn render(out: &Path) -> Result<()> {
    render_native_gallery(&out.join("gallery"))?;
    let mut requested = cases();
    requested.extend(surface_cases());
    crate::materials::render_gallery(out, &requested)?;
    Ok(())
}

pub(crate) fn surface_snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut records = vec![];
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "floating-surfaces")
    }) {
        let f = case.fixture.as_ref().unwrap();
        let layout = surface_layout(case)?;
        let crop = &f["backdrop"]["crop"];
        let panel = layout.children[0].rect;
        let native = [
            panel.x - number(crop, "x")?,
            panel.y - number(crop, "y")?,
            panel.w,
            panel.h,
        ];
        let web = &f["floating"]["panel"];
        let reference = [
            number(web, "x")?,
            number(web, "y")?,
            number(web, "w")?,
            number(web, "h")?,
        ];
        let error = native
            .into_iter()
            .zip(reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0_f32, f32::max);
        records.push(serde_json::json!({"key":case.relative(),"scope":"container surfaces only; module children hidden", "native":native,"web":reference,"max_rect_error":error,"module_slot_height_input":f["floating"]["slot_height"]}));
    }
    Ok(records)
}

pub fn render_native_gallery(out: &Path) -> Result<()> {
    use bbt_ui_model::draw::{DrawList, TextStyle, TypeScale};
    fs::create_dir_all(out)?;
    let mut renderer = bbt_render::chrome::ChromeRenderer::new(crate::images()?)?;
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bboxybook floating surfaces</title><style>body{background:#11131c;color:white;font:14px system-ui;margin:24px}img{width:100%;max-width:640px}section{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:12px}figure{margin:0}a{color:#9bc3ff}</style><h1>L6 Overlay and Modal</h1><p>Backdrop stripes and text remain sharp outside the Overlay, blur under it, then receive a separate frosted Modal and sharp foreground. Visibility, focus, dismissal and input are caller-owned.</p><p><a href=../index.html>Primitive and surface-only comparisons</a> · <a href=../surface-layouts.json>Module shell geometry</a></p><section>",
    );
    for density in gallery::DENSITIES {
        for theme in gallery::THEMES {
            let ctx = ui::Context::new(theme, density);
            let mut background = DrawList::default();
            for x in (0..640).step_by(12) {
                background.rect(
                    Rect::new(x as f32, 0., 12., 340.),
                    0.,
                    Color::hex(if x % 24 == 0 {
                        theme.palette().accent
                    } else {
                        theme.palette().bg
                    }),
                );
            }
            for y in [0., 90., 180., 270.] {
                background.text(
                    Rect::new(4., y, 630., 30.),
                    "SHARP BACKGROUND · blur is behind the foreground",
                    TextStyle {
                        scale: TypeScale::Custom(23.),
                        weight: 600,
                        ..Default::default()
                    },
                    Color::hex(theme.palette().fg),
                );
            }
            let body = c::Element::Text(c::Text::new(
                "Frosted material over a dimmed, blurred backdrop. The title, actions and this text stay sharp.",
                ui::Text {
                    step: ui::TypeStep::Caption,
                    paint: ui::Paint::solid(ui::Color::Muted),
                    ..Default::default()
                },
            ));
            let actions = [
                ("Cancel", ui::widgets::ButtonKind::Ghost),
                ("Continue", ui::widgets::ButtonKind::Primary),
            ]
            .into_iter()
            .map(|(label, kind)| {
                ui::widgets::Button {
                    label: label.into(),
                    kind,
                    state: ui::widgets::ButtonState::Idle,
                    icon: None,
                    key: None,
                }
                .element(ctx)
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
            let modal = ui::overlays::Modal::new("Layered Modal", body, actions).element(ctx);
            let layout = c::Element::Overlay(c::Overlay::new(modal)).layout(
                Rect::new(24., 64., 592., 0.),
                ctx,
                &crate::components::Metrics,
            )?;
            let pixels = renderer.render_to_rgba(&background, 640, 340, 1.)?;
            let pixels = crate::components::execute(
                &mut renderer,
                c::paint(&layout, ctx),
                pixels,
                [640, 340],
                ctx,
            )?;
            let key = format!(
                "floating-{}-{}.png",
                gallery::density_name(density),
                gallery::theme_name(theme)
            );
            bbt_render::write_png(out.join(&key), &pixels, 640, 340)?;
            html.push_str(&format!(
                "<figure><figcaption>{key}</figcaption><img src='{key}' loading=lazy></figure>"
            ));
        }
    }
    html.push_str("</section>");
    fs::write(out.join("index.html"), html)?;
    println!(
        "native floating gallery: 8 PNGs; {}",
        out.join("index.html").display()
    );
    Ok(())
}

pub fn surface_cases() -> Vec<Case> {
    let mut cases = vec![];
    for (page, state, slug, kind) in [
        ("launcher", "root", "root", "launcher"),
        (
            "launcher",
            "root · over pattern",
            "root-over-pattern",
            "launcher",
        ),
        ("documents", "as modal", "as-modal", "document"),
    ] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                cases.push(Case {
                    component: page.into(),
                    state: state.into(),
                    slug: slug.into(),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: format!("{page}.html"),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 900,
                    checker: false,
                    fixture: Some(
                        serde_json::json!({"layer":"floating-surfaces","surface_kind":kind}),
                    ),
                });
            }
        }
    }
    cases
}
fn number(v: &Value, key: &str) -> Result<f32> {
    Ok(v[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("floating fixture {key} missing"))? as f32)
}
pub fn surface_layout(case: &Case) -> Result<c::Layout> {
    let f = case.fixture.as_ref().ok_or("floating fixture missing")?;
    let data = &f["floating"];
    let host = &data["host"];
    let crop = &f["backdrop"]["crop"];
    let ctx = crate::components::context(case);
    let launcher = data["kind"] == "launcher";
    // The unrendered L7 contents occupy an external fixed-height slot. Native
    // code computes container position, width, materials and decoration itself.
    let mut panel = ui::Style::new(ui::Preset::Modal);
    let viewport = data["viewport_width"].as_f64().ok_or("viewport width")? as f32;
    panel.overrides.width = Some(ui::Length::Px(if launcher {
        760_f32.min(viewport * 0.92)
    } else {
        980_f32.min(viewport * 0.94)
    }));
    panel.overrides.padding = Some([ui::Length::Px(0.); 4]);
    if launcher {
        // The launcher's panel adds an inside line to Modal's outside hair.
        panel.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(ui::Color::Line),
            outside: false,
        });
        panel.overrides.rings = Some(vec![ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: true,
        }]);
        panel.overrides.height = Some(ui::Length::Px(number(data, "slot_height")?));
    }
    let mut overlay = c::Overlay::new(c::Element::Box(c::Box::new(panel, vec![])));
    overlay.clip = false;
    // lab.js maps the legacy --r-pill name to STYLE.radius.card; --r-round
    // is the actual 999px pill. These page stages use the former.
    overlay.style.overrides.radii = Some([ui::Length::Radius(ui::Radius::Card); 4]);
    if launcher {
        overlay.min_height = ui::Length::Px(560.);
        overlay.style.overrides.padding = Some([
            ui::Length::Px(28.),
            ui::Length::Px(0.),
            ui::Length::Px(0.),
            ui::Length::Px(0.),
        ]);
        overlay.style.overrides.blur = Some(ui::Blur {
            radius: ui::Length::Px(STYLE.material.backdrop_blur_px as f32),
            saturation: 1.15,
        });
        overlay.vertical = c::Alignment::Start;
    } else {
        overlay.style.overrides.material = Some(ui::Material::Flat);
        overlay.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(
            Color::hex(ctx.theme.palette().navy).0,
        )));
        overlay.style.overrides.padding = Some([ui::Length::Px(28.); 4]);
        overlay.style.overrides.height = Some(ui::Length::Px(720.));
        overlay.vertical = c::Alignment::Stretch;
    }
    Ok(c::Element::Overlay(overlay).layout(
        Rect::new(
            number(host, "x")? + number(crop, "x")?,
            number(host, "y")? + number(crop, "y")?,
            number(host, "w")?,
            0.,
        ),
        ctx,
        &crate::components::Metrics,
    )?)
}
pub fn render_surface_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("floating fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("floating backdrop missing")?,
    ))?;
    let pixels = crate::components::execute(
        renderer,
        c::paint(&surface_layout(case)?, crate::components::context(case)),
        pixels,
        [w, h],
        crate::components::context(case),
    )?;
    let x = number(&bg["crop"], "x")?.round() as u32;
    let y = number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("floating crop outside backdrop".into());
    }
    let mut crop = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        crop.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(crop)
}
