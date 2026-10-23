//! G3 geometry invariant: a lane effect paints only inside its lane, (0, L) px
//! outside the box, never inside the box and never beyond L, for every candidate,
//! activity, lane width and time. The box rect is an input the effect never changes.
use bboxybook::shaders::{ACTIVITIES, LANES, TIMES, lane, radius};
use bbt_render::chrome::{ChromeRenderer, materials::*};
use bbt_ui_model::{Theme, draw::Color, layout::Rect};

const SCALE: f32 = 2.;
const SIZE: u32 = 360;

/// Signed distance (logical px) from a uniform-radius rounded rect, positive outside.
fn sdf(px: f32, py: f32, r: Rect, radius: f32) -> f32 {
    let (cx, cy) = (r.x + r.w / 2., r.y + r.h / 2.);
    let rad = radius.min(r.w.min(r.h) / 2.);
    let qx = (px - cx).abs() - r.w / 2. + rad;
    let qy = (py - cy).abs() - r.h / 2. + rad;
    qx.max(qy).min(0.) + (qx.max(0.).powi(2) + qy.max(0.).powi(2)).sqrt() - rad
}

#[test]
fn lane_effects_never_leave_the_lane() {
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    let rect = Rect::new(30., 30., 120., 90.);
    let r = radius();
    for fx in LaneFx::ALL {
        for activity in ACTIVITIES {
            for l in LANES {
                for t in [TIMES[1], TIMES[3]] {
                    let items = [Surface {
                        rect,
                        radii: [r; 4],
                        clips: vec![],
                        material: Material::Lane(lane(
                            fx,
                            Color::hex("#2E6BFF"),
                            activity,
                            l,
                            1.,
                            0.,
                        )),
                    }];
                    // Opaque black: the lane keeps destination alpha (as the glow does), so
                    // effect pixels show as colour over it.
                    let mut black = bbt_ui_model::draw::DrawList::default();
                    black.rect(
                        Rect::new(0., 0., SIZE as f32 / SCALE, SIZE as f32 / SCALE),
                        0.,
                        Color::hex("#000000"),
                    );
                    let px = render_to_rgba(
                        &mut renderer,
                        Scene {
                            backdrop: Backdrop::DrawList(&black),
                            items: &items,
                            width: SIZE,
                            height: SIZE,
                            scale: SCALE,
                            time_ms: t,
                            theme: Theme::Dark,
                        },
                    )
                    .unwrap();
                    let mut lit = 0;
                    for y in 0..SIZE {
                        for x in 0..SIZE {
                            let i = ((y * SIZE + x) * 4) as usize;
                            let a = px[i].max(px[i + 1]).max(px[i + 2]);
                            if a == 0 {
                                continue;
                            }
                            lit += 1;
                            let d =
                                sdf((x as f32 + 0.5) / SCALE, (y as f32 + 0.5) / SCALE, rect, r);
                            assert!(
                                d > -1e-3 && d < l + 1e-3,
                                "{} {:?} L={l} t={t}: colour {a} at ({x},{y}), {d:.3} px from the box",
                                fx.name(),
                                activity
                            );
                        }
                    }
                    assert!(
                        lit > 0,
                        "{} {:?} L={l} t={t}: nothing painted",
                        fx.name(),
                        activity
                    );
                }
            }
        }
    }
}

#[test]
fn lane_quad_and_parameters_are_validated() {
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    let empty = bbt_ui_model::draw::DrawList::default();
    for bad in [0., 40., f32::NAN] {
        let items = [Surface {
            rect: Rect::new(10., 10., 50., 50.),
            radii: [4.; 4],
            clips: vec![],
            material: Material::Lane(lane(
                LaneFx::Silk,
                Color::hex("#2E6BFF"),
                Activity::Active,
                bad,
                1.,
                0.,
            )),
        }];
        let result = render_to_rgba(
            &mut renderer,
            Scene {
                backdrop: Backdrop::DrawList(&empty),
                items: &items,
                width: 80,
                height: 80,
                scale: 1.,
                time_ms: 0.,
                theme: Theme::Dark,
            },
        );
        assert!(result.is_err(), "lane {bad} accepted");
    }
}
