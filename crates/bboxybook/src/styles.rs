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
    let resolved = style(case)?.resolve(context(case))?;
    let rect = box_frame(case, &resolved)?;
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

fn box_frame(case: &Case, resolved: &ResolvedStyle) -> Result<Rect> {
    let f = case.fixture.as_ref().ok_or("missing Style fixture")?;
    let item = &f["styles"][0];
    let x = f["backdrop"]["crop"]["x"].as_f64().ok_or("crop x")? as f32;
    let y = f["backdrop"]["crop"]["y"].as_f64().ok_or("crop y")? as f32;
    let r = &item["rect"];
    let n = |key| {
        r[key]
            .as_f64()
            .map(|n| n as f32)
            .ok_or("invalid Style fixture rect")
    };

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
    Ok(Rect::new(n("x")? + x, n("y")? + y, n("w")?, height))
}
fn text_frame(rect: Rect, s: &ResolvedStyle) -> Rect {
    let inset = s
        .border
        .as_ref()
        .filter(|b| !b.outside)
        .map_or(0., |b| b.width);
    Rect::new(
        rect.x + s.padding[3] + inset,
        rect.y
            + s.padding[0]
            + inset
            + (rect.h - s.padding[0] - s.padding[2] - inset * 2. - s.text.size * 1.4) * 0.5,
        (rect.w - s.padding[1] - s.padding[3] - inset * 2.).max(0.),
        s.text.size * 1.4,
    )
}
pub(crate) fn snapshots(cases: &[Case]) -> Result<Vec<serde_json::Value>> {
    use serde_json::json;
    let mut out = vec![];
    for case in cases.iter().filter(|c| c.component == "style") {
        let f = case.fixture.as_ref().ok_or("missing Style fixture")?;
        let item = &f["styles"][0];
        let s = style(case)?.resolve(context(case))?;
        let outer = box_frame(case, &s)?;
        let line = text_frame(outer, &s);
        let x = f["backdrop"]["crop"]["x"].as_f64().ok_or("crop x")? as f32;
        let y = f["backdrop"]["crop"]["y"].as_f64().ok_or("crop y")? as f32;
        let font = TextStyle {
            scale: TypeScale::Custom(s.text.size),
            weight: s.text.weight,
            mono: s.text.mono,
            italic: s.text.italic,
            tracking_em: s.text.tracking_em,
            uppercase: s.text.uppercase,
        };
        let mask = bbt_render::chrome::text::rasterize(
            item["text"]["value"].as_str().ok_or("missing Style text")?,
            font,
            Some(line.w),
            1.,
        )?;
        let metrics = bbt_render::chrome::text::rasterize("Hg", font, None, 1.)?;
        let height = s.text.size * s.text.line_height;
        let text_y = line.y + (line.h - height) * 0.5;
        let baseline = metrics.ascent.round()
            + ((height - metrics.ascent.round() - metrics.descent.round()) * 0.5).floor();
        let web = |r: &serde_json::Value| {
            [
                r["x"].as_f64(),
                r["y"].as_f64(),
                r["w"].as_f64(),
                r["h"].as_f64(),
            ]
        };
        let text = json!({"kind":"text","native":[line.x-x,text_y-y,mask.advance,height],"web":web(&item["text"]["rect"]),"baseline":{"native":text_y-y+baseline,"web":item["text"]["font_diagnostics"]["baseline"].as_f64().zip(item["text"]["rect"]["y"].as_f64()).map(|(b,y)|b+y)},"children":[]});
        out.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[outer.x-x,outer.y-y,outer.w,outer.h],"web":web(&item["rect"]),"children":[text]}}));
    }
    Ok(out)
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
        Material::Flat | Material::Glow | Material::Rim => materials::Material::Flat(fill),
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
        clips: vec![],
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
    foreground.clip(rect, s.radii);
    foreground.text(
        text_frame(rect, s),
        label,
        TextStyle {
            scale: TypeScale::Custom(s.text.size),
            weight: s.text.weight,
            mono: s.text.mono,
            italic: s.text.italic,
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
        if s.rim.is_some() {
            crate::components::configure_glow(&mut g, ctx, s.rim);
            g.color = g.color.alpha(s.opacity);
            g.waiting = g.waiting.alpha(s.opacity);
            g.errored = g.errored.alpha(s.opacity);
        }
        let items = [Surface {
            rect,
            radii: s.radii,
            clips: vec![],
            material: materials::Material::Glow(g),
        }];
        pixels = materials::render_to_rgba(renderer, scene(&pixels, size, &items, ctx.theme))?;
    }
    if s.material == Material::Rim {
        // style.html explicitly shows an active, focused pane-glow fixture.
        let state = s.rim.unwrap_or(bbt_ui::Rim {
            focused: true,
            ..Default::default()
        });
        let mut rim = crate::components::rim(ctx, Some(state));
        for color in [&mut rim.color, &mut rim.waiting, &mut rim.errored] {
            *color = color.alpha(s.opacity);
        }
        let items = [Surface {
            rect,
            radii: s.radii,
            clips: vec![],
            material: materials::Material::Rim(rim),
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

#[cfg(all(test, target_os = "macos"))]
mod geometry_tests {
    use super::*;
    // keeps: spec renderer.md §4.4a
    #[test]
    fn snapshot_geometry_uses_the_paint_frame_and_fonts_not_child_dom_bounds() -> Result<()> {
        let mut case = cases()
            .into_iter()
            .find(|c| c.state == "card" && c.density == "cozy" && c.theme == "dark")
            .unwrap();
        case.fixture = Some(
            serde_json::json!({"styles":[{"rect":{"x":3,"y":7,"w":300,"h":1000},"text":{"value":"Vesper","rect":{"x":999,"y":999,"w":1,"h":1},"font_diagnostics":{"baseline":99}}}],"backdrop":{"crop":{"x":13,"y":17}}}),
        );
        let data = snapshots(&[case.clone()])?;
        let tree = &data[0]["tree"];
        assert_eq!(tree["native"][0], 3.);
        assert_eq!(tree["native"][1], 7.);
        assert_eq!(tree["native"][2], 300.);
        assert!(tree["native"][3].as_f64().unwrap() < 50.);
        let text = &tree["children"][0];
        // Card's outside ring contributes no padding or layout inset.
        assert_eq!(text["native"][0], 17.);
        assert_eq!(text["native"][1], 19.);
        assert!(text["native"][2].as_f64().unwrap() > 20.);
        assert_eq!(text["baseline"]["web"], 1098.);
        assert!(text["baseline"]["native"].as_f64().unwrap() < 100.);
        case.fixture.as_mut().unwrap()["styles"][0]["rect"]["h"] = serde_json::json!(12);
        assert_eq!(snapshots(&[case])?[0]["tree"]["native"], tree["native"]);
        Ok(())
    }
}
