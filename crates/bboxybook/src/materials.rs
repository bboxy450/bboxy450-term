//! The L2 gallery intentionally contains no components or screen compositions.
use crate::Result;
use bbt_render::chrome::{ChromeRenderer, materials::*};
use bbt_ui_model::{
    STYLE, TOKENS, Theme,
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::Rect,
};
use std::{fs, path::Path};
pub const SIZE: u32 = 320;
pub fn material(kind: Kind, theme: Theme) -> Material {
    let p = theme.palette();
    let g = TOKENS.glow;
    match kind {
        Kind::Flat => Material::Flat(Color::hex(p.surface_2)),
        Kind::Gloss => Material::Gloss(Gloss {
            color: Color::hex(TOKENS.color.role.oracle),
            top_white: 0.12,
            bottom_black: 0.18,
            highlight: 0.18,
            bottom_shade: 0.25,
        }),
        Kind::Frosted => Material::Frosted(Frosted {
            tint: Color::TRANSPARENT,
            blur_radius: STYLE.material.panel_blur_px as f32,
            saturation: STYLE.material.panel_saturate as f32,
        }),
        Kind::Glow => Material::Glow(Glow {
            color: Color::hex(p.accent),
            waiting: Color::hex(TOKENS.color.state.waiting),
            errored: Color::hex(TOKENS.color.state.errored),
            activity: Activity::Active,
            focus: 1.,
            focused_boost: g.focused_boost as f32,
            radius: g.pane_radius as f32,
            seed: 7.,
            halo_alpha: 0.45,
            idle_alpha: g.idle_alpha as f32,
            active_alpha: [g.active_alpha_min as f32, g.active_alpha_max as f32],
            waiting_alpha: [g.waiting_alpha_min as f32, g.waiting_alpha_max as f32],
            errored_alpha: g.errored_alpha as f32,
            active_period_ms: g.active_pulse_ms as f32,
            waiting_period_ms: TOKENS.motion.pulse_ms as f32,
            particles: g.particles_active as u32,
            speed: g.particle_speed as f32,
            particle_size: g.particle_size as f32,
        }),
        Kind::Rim => Material::Rim(Rim {
            color: Color::hex(p.accent),
            waiting: Color::hex(TOKENS.color.state.waiting),
            errored: Color::hex(TOKENS.color.state.errored),
            activity: Activity::Active,
            focus: 1.,
            depth: TOKENS.glow.depth as f32,
            size: TOKENS.glow.rim as f32,
            phase_ms: 0.,
        }),
    }
}
pub fn backdrop(kind: Kind, theme: Theme) -> DrawList {
    let p = theme.palette();
    let mut list = DrawList::default();
    list.rect(
        Rect::new(0., 0., SIZE as f32, SIZE as f32),
        0.,
        Color::hex(p.bg),
    );
    if kind == Kind::Frosted {
        for y in (16..304).step_by(16) {
            for x in (16..304).step_by(16) {
                let c = if (x / 16 + y / 16) % 2 == 0 {
                    p.accent
                } else {
                    p.fg
                };
                list.rect(
                    Rect::new(x as f32, y as f32, 16., 16.),
                    0.,
                    Color::hex(c).alpha(0.65),
                );
            }
        }
        list.text(
            Rect::new(24., 148., 272., 28.),
            "Backdrop · 0123456789",
            TextStyle {
                scale: TypeScale::Custom(18.),
                weight: 700,
                ..Default::default()
            },
            Color::hex(p.fg),
        );
    }
    list
}
pub fn surface(kind: Kind, theme: Theme) -> Surface {
    Surface {
        rect: Rect::new(64., 64., 192., 192.),
        radii: [STYLE.radius.card as f32; 4],
        clips: vec![],
        material: material(kind, theme),
    }
}
pub fn render(out: &Path) -> Result<()> {
    render_gallery(out, &cases())
}
pub(crate) fn render_gallery(out: &Path, requested: &[crate::gallery::Case]) -> Result<()> {
    fs::create_dir_all(out)?;
    fs::write(
        out.join("requested.json"),
        serde_json::to_vec_pretty(requested)?,
    )?;
    let status = std::process::Command::new("node")
        .arg("crates/bboxybook/capture.mjs")
        .arg(out)
        .arg(out.join("requested.json"))
        .status()?;
    if !status.success() {
        return Err("layer web capture failed; inspect capture.json".into());
    }
    let capture: serde_json::Value = serde_json::from_slice(&fs::read(out.join("capture.json"))?)?;
    if capture["status"] != "captured" {
        return Err("layer references unproven: Chromium required for this gallery".into());
    }
    let cases: Vec<crate::gallery::Case> =
        serde_json::from_slice(&fs::read(out.join("cases.json"))?)?;
    crate::parity::render(&out.join("native"), &cases)?;
    crate::parity::compare(out, &cases)?;
    Ok(())
}
pub fn cases() -> Vec<crate::gallery::Case> {
    let mut cases = Vec::new();
    for kind in Kind::ALL {
        for density in crate::gallery::DENSITIES {
            for theme in crate::gallery::THEMES {
                cases.push(crate::gallery::Case {
                    component: "materials".into(),
                    state: kind.name().into(),
                    slug: kind.name().into(),
                    density: crate::gallery::density_name(density).into(),
                    theme: crate::gallery::theme_name(theme).into(),
                    page: "materials.html".into(),
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
pub fn render_case(renderer: &mut ChromeRenderer, case: &crate::gallery::Case) -> Result<Vec<u8>> {
    let fixture = case
        .fixture
        .as_ref()
        .ok_or("materials case needs captured inputs")?;
    let bg = &fixture["backdrop"];
    let (width, height, pixels) =
        crate::read_png(Path::new(bg["path"].as_str().ok_or("missing L2 backdrop")?))?;
    let crop = &bg["crop"];
    let x = crop["x"].as_f64().unwrap_or(0.);
    let y = crop["y"].as_f64().unwrap_or(0.);
    let theme = crate::gallery::theme(&case.theme);
    let mut items = Vec::new();
    let number = |v: &serde_json::Value, key: &str, default: f32| {
        v[key].as_f64().unwrap_or(default as f64) as f32
    };
    let color = |v: &serde_json::Value| -> Result<Color> {
        let a = v.as_array().ok_or("missing material colour")?;
        if a.len() != 4 {
            return Err("invalid material colour".into());
        }
        Ok(Color([
            a[0].as_f64().ok_or("colour")? as f32,
            a[1].as_f64().ok_or("colour")? as f32,
            a[2].as_f64().ok_or("colour")? as f32,
            a[3].as_f64().ok_or("colour")? as f32,
        ]))
    };
    for value in fixture["materials"]
        .as_array()
        .ok_or("missing L2 surfaces")?
    {
        let kind = match value["kind"].as_str().unwrap_or("") {
            "flat" => Kind::Flat,
            "gloss" => Kind::Gloss,
            "frosted" => Kind::Frosted,
            "glow" => Kind::Glow,
            _ => return Err("unknown L2 material".into()),
        };
        let params = &value["params"];
        let material = match kind {
            Kind::Rim => {
                return Err("the rim has no materials-page reference; see the pane cases".into());
            }
            Kind::Flat => Material::Flat(color(&value["color"])?),
            Kind::Gloss => Material::Gloss(Gloss {
                color: color(&value["color"])?,
                top_white: number(params, "top_white", 0.12),
                bottom_black: number(params, "bottom_black", 0.18),
                highlight: number(params, "highlight", 0.18),
                bottom_shade: number(params, "bottom_shade", 0.25),
            }),
            Kind::Frosted => Material::Frosted(Frosted {
                tint: color(&value["tint"])?,
                blur_radius: number(params, "blur_radius", 30.),
                saturation: number(params, "saturation", 1.4),
            }),
            Kind::Glow => {
                let Material::Glow(mut g) = material(kind, theme) else {
                    unreachable!()
                };
                g.color = color(&value["color"])?;
                g.seed = value["glow"]["seed"].as_f64().unwrap_or(g.seed as f64) as f32;
                g.radius = number(params, "radius", g.radius);
                g.particles = number(params, "particles", g.particles as f32) as u32;
                g.speed = number(params, "speed", g.speed);
                g.particle_size = number(params, "particle_size", g.particle_size);
                g.focused_boost = number(params, "focused_boost", g.focused_boost);
                Material::Glow(g)
            }
        };
        let r = &value["rect"];
        let radii = value["radii"].as_array().ok_or("missing L2 radii")?;
        if radii.len() != 4 {
            return Err("invalid radii".into());
        }
        items.push(Surface {
            rect: Rect::new(
                number(r, "x", 0.) + x as f32,
                number(r, "y", 0.) + y as f32,
                number(r, "w", 0.),
                number(r, "h", 0.),
            ),
            radii: [
                radii[0].as_f64().unwrap_or(0.) as f32,
                radii[1].as_f64().unwrap_or(0.) as f32,
                radii[2].as_f64().unwrap_or(0.) as f32,
                radii[3].as_f64().unwrap_or(0.) as f32,
            ],
            clips: vec![],
            material,
        });
    }
    if items.len() != 4 {
        return Err("expected four L2 radius samples".into());
    }
    let image = render_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::Rgba(&pixels),
            items: &items,
            width,
            height,
            scale: 1.,
            time_ms: 450.,
            theme,
        },
    )?;
    // Chromium snaps a screenshot clip's fractional origin to device pixels.
    let x = x.round() as u32;
    let y = y.round() as u32;
    if x + case.width > width || y + case.height > height {
        return Err("invalid material capture crop".into());
    }
    let mut out = Vec::with_capacity((case.width * case.height * 4) as usize);
    for row in y..y + case.height {
        let start = ((row * width + x) * 4) as usize;
        out.extend_from_slice(&image[start..start + case.width as usize * 4]);
    }
    Ok(out)
}
