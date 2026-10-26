#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bbt_render::chrome::{ChromeRenderer, materials::*};
use bbt_ui_model::{
    Theme,
    draw::{Color, DrawList},
    layout::Rect,
};

fn background(theme: Theme) -> DrawList {
    let mut out = DrawList::default();
    for y in (0..80).step_by(8) {
        for x in (0..96).step_by(8) {
            out.rect(
                Rect::new(x as f32, y as f32, 8., 8.),
                0.,
                Color::hex(if (x + y) % 16 == 0 {
                    theme.palette().bg
                } else {
                    theme.palette().surface_2
                }),
            );
        }
    }
    out
}
fn render(renderer: &mut ChromeRenderer, theme: Theme, scale: u32, items: &[Surface]) -> Vec<u8> {
    render_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::DrawList(&background(theme)),
            items,
            width: 96 * scale,
            height: 80 * scale,
            scale: scale as f32,
            time_ms: 450.,
            theme,
        },
    )
    .unwrap()
}
fn pixel(data: &[u8], scale: u32, x: u32, y: u32) -> [u8; 4] {
    let i = (((y * scale) * (96 * scale) + x * scale) * 4) as usize;
    data[i..i + 4].try_into().unwrap()
}
// keeps: spec renderer.md §4.5
#[test]
fn every_material_obeys_each_rounded_ancestor_at_one_and_two_x() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        for scale in [1, 2] {
            let base = render(&mut renderer, theme, scale, &[]);
            for kind in [Kind::Flat, Kind::Gloss, Kind::Frosted, Kind::Glow] {
                let mut material = bboxybook::materials::material(kind, theme);
                if kind == Kind::Flat {
                    material = Material::Flat(Color::hex("#4080C0"));
                }
                if let Material::Frosted(ref mut f) = material {
                    f.tint = Color::hex("#7D47EE").alpha(0.4);
                }
                if let Material::Glow(ref mut g) = material {
                    g.radius = 32.;
                    g.particles = 0;
                }
                let shape = Surface {
                    rect: if kind == Kind::Glow {
                        Rect::new(40., 32., 16., 16.)
                    } else {
                        Rect::new(0., 0., 96., 80.)
                    },
                    radii: [0.; 4],
                    material,
                    clips: vec![],
                };
                let unmasked = render(&mut renderer, theme, scale, std::slice::from_ref(&shape));
                let mut clipped = shape.clone();
                clipped.clips = vec![
                    Clip {
                        rect: Rect::new(8., 8., 64., 64.),
                        radii: [20.; 4],
                    },
                    Clip {
                        rect: Rect::new(24., 12., 64., 56.),
                        radii: [12.; 4],
                    },
                ];
                let masked = render(&mut renderer, theme, scale, std::slice::from_ref(&clipped));
                // Inside both bounding rectangles, but outside a different ancestor's rounded corner.
                for (x, y) in [(25, 13), (70, 65)] {
                    assert_ne!(
                        pixel(&unmasked, scale, x, y),
                        pixel(&base, scale, x, y),
                        "{kind:?} must visibly cover the test point before clipping"
                    );
                    assert_eq!(
                        pixel(&masked, scale, x, y),
                        pixel(&base, scale, x, y),
                        "{kind:?} leaked through an ancestor at {x},{y}"
                    );
                }
                assert_ne!(
                    pixel(&masked, scale, 50, 25),
                    pixel(&base, scale, 50, 25),
                    "interior incorrectly clipped: {kind:?}"
                );
                clipped.clips.push(Clip {
                    rect: Rect::new(1., 1., 0., 20.),
                    radii: [0.; 4],
                });
                assert_eq!(
                    render(&mut renderer, theme, scale, &[clipped]),
                    base,
                    "empty clip must preserve destination, {kind:?}"
                );
            }
        }
    }
}
// keeps: spec renderer.md §4.5
#[test]
fn different_surfaces_use_their_own_clip_ranges() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let items = [
        Surface {
            rect: Rect::new(0., 0., 96., 80.),
            radii: [0.; 4],
            material: Material::Flat(Color::hex("#FF0000")),
            clips: vec![Clip {
                rect: Rect::new(0., 0., 44., 80.),
                radii: [10.; 4],
            }],
        },
        Surface {
            rect: Rect::new(0., 0., 96., 80.),
            radii: [0.; 4],
            material: Material::Flat(Color::hex("#00FF00")),
            clips: vec![Clip {
                rect: Rect::new(48., 0., 48., 80.),
                radii: [16.; 4],
            }],
        },
    ];
    let p = render(&mut renderer, Theme::Dark, 1, &items);
    assert_eq!(pixel(&p, 1, 20, 40), [255, 0, 0, 255]);
    assert_eq!(pixel(&p, 1, 75, 40), [0, 255, 0, 255]);
}

// keeps: spec renderer.md §4.5
#[test]
fn retained_material_keeps_the_outer_rounding_when_an_inner_clip_is_square() {
    use bbt_ui::{self as ui, components as c};
    let ctx = ui::Context::new(Theme::Dark, ui::Density::Cozy);
    fn box_style(radius: f32, material: ui::Material) -> ui::Style {
        let mut s = ui::Style::new(ui::Preset::Card);
        s.overrides.width = Some(ui::Length::Px(64.));
        s.overrides.height = Some(ui::Length::Px(64.));
        s.overrides.padding = Some([ui::Length::Px(0.); 4]);
        s.overrides.radii = Some([ui::Length::Px(radius); 4]);
        s.overrides.material = Some(material);
        s.overrides.fill = Some(ui::Paint::solid(if material == ui::Material::Gloss {
            ui::Color::Accent
        } else {
            ui::Color::Transparent
        }));
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Px(0.),
            paint: ui::Paint::solid(ui::Color::Transparent),
            outside: false,
        });
        s
    }
    let child = c::Element::Box(c::Box::new(box_style(0., ui::Material::Gloss), vec![]));
    let mut middle = c::Box::new(box_style(0., ui::Material::Flat), vec![child.into()]);
    middle.clip = true;
    let mut outer = c::Box::new(
        box_style(20., ui::Material::Flat),
        vec![c::Element::Box(middle).into()],
    );
    outer.clip = true;
    let layout = c::Element::Box(outer)
        .layout(
            Rect::new(8., 8., 64., 0.),
            ctx,
            &bboxybook::components::Metrics,
        )
        .unwrap();
    let ops = c::paint(&layout, ctx);
    assert!(ops.iter().any(|p|matches!(p,c::Paint::Material{clips,..} if clips.len()==2 && clips[0].radii==[20.;4] && clips[1].radii==[0.;4])));
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let base = render(&mut renderer, Theme::Dark, 1, &[]);
    let result =
        bboxybook::components::execute(&mut renderer, ops, base.clone(), [96, 80], ctx).unwrap();
    assert_eq!(pixel(&result, 1, 8, 8), pixel(&base, 1, 8, 8));
    assert_ne!(pixel(&result, 1, 40, 40), pixel(&base, 1, 40, 40));
}
