//! L3 review fixture: the same low-level box painted with token-resolved styles.
//! This is not the retained L6 Box/Text component, layout or a module rebase.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_render::chrome::{
    ChromeRenderer,
    materials::{self, Backdrop, Kind, Scene, Surface},
};
use bbt_ui::{Context, Material, Preset, ResolvedBorder, ResolvedStyle, Style};
use bbt_ui_model::{
    TOKENS, Theme,
    draw::{Color, DrawList, Prim, TextStyle, TypeScale},
    layout::Rect,
};
use std::{fs, path::Path};

pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for preset in Preset::ALL {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                cases.push(Case {
                    component: "style".into(),
                    state: preset.name().into(),
                    slug: preset.name().into(),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "style.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 400,
                    checker: false,
                    fixture: None,
                });
            }
        }
    }
    cases
}
pub fn context(case: &Case) -> Context {
    let mut ctx = Context::new(gallery::theme(&case.theme), gallery::density(&case.density));
    ctx.seat = Color::hex(TOKENS.color.ui.ok).0; // The web Style fixture's SEAT constant.
    ctx.selected = case.state == "row";
    ctx
}
pub fn style(case: &Case) -> Result<Style> {
    let preset: Preset = serde_json::from_value(serde_json::Value::String(case.state.clone()))?;
    let mut style = Style::new(preset);
    // style.html sets a tall outer fixture for the pane-glow comparison.
    if preset == Preset::PaneGlow {
        style.overrides.height = Some(bbt_ui::Length::Px(120.));
    }
    Ok(style)
}
pub fn render(out: &Path) -> Result<()> {
    let requested = cases();
    fs::create_dir_all(out)?;
    let records=requested.iter().map(|case| -> Result<_> {
        let style=style(case)?;
        Ok(serde_json::json!({"key":case.relative().trim_end_matches(".png"),"style":style,"resolved":style.resolve(context(case))?}))
    }).collect::<Result<Vec<_>>>()?;
    fs::write(
        out.join("styles.json"),
        serde_json::to_vec_pretty(&records)?,
    )?;
    crate::materials::render_gallery(out, &requested)?;
    Ok(())
}

pub fn render_case(renderer: &mut ChromeRenderer, case: &Case) -> Result<Vec<u8>> {
    let fixture = case
        .fixture
        .as_ref()
        .ok_or("Style case needs captured outer fixture")?;
    let boxes = fixture["styles"]
        .as_array()
        .ok_or("missing Style fixture")?;
    if boxes.len() != 1 {
        return Err("Style case must contain exactly one box".into());
    }
    let item = &boxes[0];
    let bg = &fixture["backdrop"];
    let (w, h, pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("missing Style backdrop")?,
    ))?;
    let x = bg["crop"]["x"].as_f64().ok_or("crop x")? as f32;
    let y = bg["crop"]["y"].as_f64().ok_or("crop y")? as f32;
    let r = &item["rect"];
    let n = |key| {
        r[key]
            .as_f64()
            .map(|n| n as f32)
            .ok_or("invalid Style fixture rect")
    };
    let resolved = style(case)?.resolve(context(case))?;
    let inset = resolved
        .border
        .as_ref()
        .filter(|b| !b.outside)
        .map_or(0., |b| b.width);
    let height = resolved.height.unwrap_or(
        resolved.text.size * resolved.text.line_height
            + resolved.padding[0]
            + resolved.padding[2]
            + inset * 2.,
    );
    // Only the external fixture width/origin is shared. Height and padding are
    // resolved by the Style; the L6 layout engine is not involved yet.
    let rect = Rect::new(n("x")? + x, n("y")? + y, n("w")?, height);
    let image = paint_box(
        renderer,
        &pixels,
        [w, h],
        rect,
        &resolved,
        context(case),
        item["glow"]["seed"].as_f64().unwrap_or(7.) as f32,
        item["text"]["value"]
            .as_str()
            .ok_or("missing Style fixture text")?,
    )?;
    let (x, y) = (x.round() as u32, y.round() as u32);
    if x + case.width > w || y + case.height > h {
        return Err("invalid Style crop".into());
    }
    let mut crop = Vec::with_capacity((case.width * case.height * 4) as usize);
    for row in y..y + case.height {
        let start = ((row * w + x) * 4) as usize;
        crop.extend_from_slice(&image[start..start + case.width as usize * 4]);
    }
    Ok(crop)
}

fn border(list: &mut DrawList, rect: Rect, radii: [f32; 4], b: &ResolvedBorder, opacity: f32) {
    if b.width <= 0. {
        return;
    }
    let (rect, radii) = if b.outside {
        (
            Rect::new(
                rect.x - b.width,
                rect.y - b.width,
                rect.w + 2. * b.width,
                rect.h + 2. * b.width,
            ),
            radii.map(|r| r + b.width),
        )
    } else {
        (rect, radii)
    };
    list.push(Prim::Rect {
        rect,
        radii,
        fill: Color::TRANSPARENT,
        stroke: Some((b.width, Color(b.color).alpha(opacity))),
        inner_highlight: None,
        shadow: None,
    });
}

fn scene<'a>(pixels: &'a [u8], size: [u32; 2], items: &'a [Surface], theme: Theme) -> Scene<'a> {
    Scene {
        backdrop: Backdrop::Rgba(pixels),
        items,
        width: size[0],
        height: size[1],
        scale: 1.,
        time_ms: 450.,
        theme,
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_box(
    renderer: &mut ChromeRenderer,
    pixels: &[u8],
    size: [u32; 2],
    rect: Rect,
    s: &ResolvedStyle,
    ctx: Context,
    seed: f32,
    label: &str,
) -> Result<Vec<u8>> {
    let mut underlay = DrawList::default();
    if let Some(shadow) = &s.shadow {
        underlay.push(Prim::Rect {
            rect: Rect::new(rect.x + shadow.offset[0], rect.y, rect.w, rect.h),
            radii: s.radii,
            fill: Color::TRANSPARENT,
            stroke: None,
            inner_highlight: None,
            shadow: Some((
                shadow.offset[1],
                // The chrome pass maps CSS blur radius to sigma = blur / 2:
                // https://www.w3.org/TR/css-backgrounds-3/#shadow-blur
                shadow.blur,
                Color(shadow.color).alpha(s.opacity),
            )),
        });
    }
    let fill = Color(s.fill).alpha(s.opacity);
    let mat = match s.material {
        Material::Flat | Material::Glow => materials::Material::Flat(fill),
        Material::Gloss => {
            let materials::Material::Gloss(mut g) =
                crate::materials::material(Kind::Gloss, ctx.theme)
            else {
                unreachable!()
            };
            g.color = fill;
            materials::Material::Gloss(g)
        }
        Material::Frosted => crate::materials::material(Kind::Frosted, ctx.theme),
    };
    let items = [Surface {
        rect,
        radii: s.radii,
        clip: None,
        material: mat,
    }];
    let mut foreground = DrawList::default();
    // Blur is L2. Its tint is an independent Style-owned fill, followed by the
    // Style's ring/highlight. Native Glow likewise never owns this fill/border.
    if s.material == Material::Frosted {
        foreground.push(Prim::Rect {
            rect,
            radii: s.radii,
            fill,
            stroke: None,
            inner_highlight: s.inner_highlight.map(Color),
            shadow: None,
        });
    }
    if let Some(b) = &s.border {
        border(&mut foreground, rect, s.radii, b, s.opacity);
    }
    if let Some(b) = &s.focus_ring {
        border(&mut foreground, rect, s.radii, b, s.opacity);
    }
    let inset = s
        .border
        .as_ref()
        .filter(|b| !b.outside)
        .map_or(0., |b| b.width);
    foreground.clip(rect, s.radii);
    foreground.text(
        Rect::new(
            rect.x + s.padding[3] + inset,
            rect.y + (rect.h - s.text.size * 1.4) * 0.5,
            (rect.w - s.padding[1] - s.padding[3] - inset * 2.).max(0.),
            s.text.size * 1.4,
        ),
        label,
        TextStyle {
            scale: TypeScale::Custom(s.text.size),
            weight: s.text.weight,
            mono: s.text.mono,
            tracking_em: s.text.tracking_em,
            uppercase: s.text.uppercase,
        },
        Color(s.text.color).alpha(s.opacity),
    );
    foreground.unclip();
    let mut pixels = materials::render_layered_to_rgba(
        renderer,
        scene(pixels, size, &items, ctx.theme),
        &underlay,
        &foreground,
    )?;
    if s.material == Material::Glow {
        let materials::Material::Glow(mut g) = crate::materials::material(Kind::Glow, ctx.theme)
        else {
            unreachable!()
        };
        g.color = Color(ctx.seat).alpha(s.opacity);
        g.seed = seed;
        let items = [Surface {
            rect,
            radii: s.radii,
            clip: None,
            material: materials::Material::Glow(g),
        }];
        pixels = materials::render_to_rgba(renderer, scene(&pixels, size, &items, ctx.theme))?;
    }
    Ok(pixels)
}

/// Fixed outer fixture for the headless Style test; no retained component or
/// layout API is introduced before L6. The paired gallery uses measured bounds.
pub fn preview(renderer: &mut ChromeRenderer, style: &Style, ctx: Context) -> Result<Vec<u8>> {
    let s = style.resolve(ctx)?;
    let mut bg = DrawList::default();
    bg.rect(
        Rect::new(0., 0., 360., 180.),
        0.,
        Color::hex(ctx.theme.palette().bg),
    );
    let pixels = renderer.render_to_rgba(&bg, 360, 180, 1.)?;
    paint_box(
        renderer,
        &pixels,
        [360, 180],
        Rect::new(40., 40., 280., s.height.unwrap_or(80.)),
        &s,
        ctx,
        7.,
        "Style",
    )
}
