//! G3: the border-lane shader gallery, an operator checkpoint (pick the effect and L).
//! Candidates x activity x theme x L in {4, 6, 8} x t in {0, 900, 1800, 2700} ms, on
//! Wren's "split" card: two panes, each inset by exactly L, the gutter between them
//! one shared lane. Native only: the lab page runs WebGL live (lab/web/shaders.html).
use crate::Result;
use bbt_render::chrome::{ChromeRenderer, materials::*};
use bbt_ui_model::{
    STYLE, TOKENS, Theme,
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::Rect,
};
use std::{fmt::Write as _, fs, path::Path};

/// Card size in logical px (the lab's split card) and the device scale (retina).
pub const W: f32 = 760.;
pub const H: f32 = 300.;
pub const SCALE: f32 = 2.;
pub const LANES: [f32; 3] = [4., 6., 8.];
pub const TIMES: [f32; 4] = [0., 900., 1800., 2700.];
pub const ACTIVITIES: [Activity; 4] = [
    Activity::Idle,
    Activity::Active,
    Activity::Waiting,
    Activity::Errored,
];
/// The second pane runs this far ahead in time, as in the lab.
const PHASE_MS: f32 = 1700.;

pub fn activity_name(a: Activity) -> &'static str {
    match a {
        Activity::Idle => "idle",
        Activity::Active => "active",
        Activity::Waiting => "waiting",
        Activity::Errored => "errored",
    }
}

pub fn lane(
    fx: LaneFx,
    color: Color,
    activity: Activity,
    lane: f32,
    focus: f32,
    phase_ms: f32,
) -> Lane {
    Lane {
        color,
        waiting: Color::hex(TOKENS.color.state.waiting),
        errored: Color::hex(TOKENS.color.state.errored),
        activity,
        focus,
        lane,
        effect: fx,
        phase_ms,
    }
}

/// The two panes of the split card for lane `l`: outer edges keep their own lane, the gutter is one lane.
pub fn panes(l: f32) -> [Rect; 2] {
    let w = (W - 3. * l) / 2.;
    [
        Rect::new(l, l, w, H - 2. * l),
        Rect::new(2. * l + w, l, w, H - 2. * l),
    ]
}

pub fn radius() -> f32 {
    STYLE.radius.row as f32
}

fn seat_colors() -> [Color; 2] {
    [
        Color::hex(TOKENS.color.role.oracle),
        Color::hex(TOKENS.color.role.executor),
    ]
}

/// Background, the two panes with a constant 1 px seat-coloured border (G1), and a title line.
fn backdrop(theme: Theme, l: f32) -> DrawList {
    let p = theme.palette();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., W, H), 0., Color::hex(p.bg));
    let r = radius();
    for ((rect, seat), title) in panes(l)
        .into_iter()
        .zip(seat_colors())
        .zip(["Vesper Greywind · swarm control", "Nyx Copperwing · 7.10"])
    {
        list.rect(rect, r, seat);
        list.rect(
            Rect::new(rect.x + 1., rect.y + 1., rect.w - 2., rect.h - 2.),
            (r - 1.).max(0.),
            Color::hex(p.bg),
        );
        list.text(
            Rect::new(rect.x + 12., rect.y + 10., rect.w - 24., 18.),
            title,
            TextStyle {
                scale: TypeScale::Custom(12.),
                weight: 600,
                ..Default::default()
            },
            Color::hex(p.fg_dim),
        );
    }
    list
}

pub fn render_card(
    renderer: &mut ChromeRenderer,
    fx: LaneFx,
    activity: Activity,
    theme: Theme,
    l: f32,
    time_ms: f32,
) -> Result<Vec<u8>> {
    let backdrop = backdrop(theme, l);
    let r = radius();
    let items: Vec<Surface> = panes(l)
        .into_iter()
        .zip(seat_colors())
        .enumerate()
        .map(|(i, (rect, color))| Surface {
            rect,
            radii: [r; 4],
            clips: vec![],
            material: Material::Lane(lane(
                fx,
                color,
                activity,
                l,
                if i == 0 { 1. } else { 0. },
                i as f32 * PHASE_MS,
            )),
        })
        .collect();
    Ok(render_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::DrawList(&backdrop),
            items: &items,
            width: (W * SCALE) as u32,
            height: (H * SCALE) as u32,
            scale: SCALE,
            time_ms,
            theme,
        },
    )?)
}

pub fn file_name(fx: LaneFx, theme: Theme, l: f32, activity: Activity, t: f32) -> String {
    format!(
        "{}/{}-L{}-{}-t{}.png",
        fx.name(),
        crate::gallery::theme_name(theme),
        l,
        activity_name(activity),
        t
    )
}

/// Renders every frame and an index.html contact sheet into `out`.
pub fn render(out: &Path) -> Result<()> {
    let mut renderer = ChromeRenderer::new(crate::images()?)?;
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>border lane · shader gallery</title>\
<style>body{background:#111;color:#ddd;font:13px -apple-system,system-ui,sans-serif;margin:24px}\
h2{margin:32px 0 6px}table{border-collapse:collapse}td,th{padding:4px 6px;vertical-align:top;text-align:left}\
img{width:380px;display:block}.note{color:#999;max-width:1200px}</style>\
<h1>Border lane: shader gallery (G3)</h1><p class=note>Native render of Wren's lab/web/shaders.html candidates \
(silk is the pick; swell, tide; breathe is today's look as the control). Two panes, each inset by exactly L, \
sharing one gutter lane; the 1 px pane border never changes. Images are 2x (retina), shown at half size \
(click for full). Rows: activity; columns: t in ms. The operator picks L.</p>",
    );
    for theme in [Theme::Dark, Theme::Light] {
        for fx in LaneFx::ALL {
            for l in LANES {
                let _ = write!(
                    html,
                    "<h2>{} · L = {l} px · {}</h2><table><tr><th></th>",
                    fx.name(),
                    crate::gallery::theme_name(theme)
                );
                for t in TIMES {
                    let _ = write!(html, "<th>t = {t} ms</th>");
                }
                html.push_str("</tr>");
                for activity in ACTIVITIES {
                    let _ = write!(html, "<tr><th>{}</th>", activity_name(activity));
                    for t in TIMES {
                        let name = file_name(fx, theme, l, activity, t);
                        let path = out.join(&name);
                        fs::create_dir_all(path.parent().unwrap())?;
                        let pixels = render_card(&mut renderer, fx, activity, theme, l, t)?;
                        bbt_render::headless::write_png(
                            &path,
                            &pixels,
                            (W * SCALE) as u32,
                            (H * SCALE) as u32,
                        )?;
                        let _ = write!(
                            html,
                            "<td><a href='{name}'><img src='{name}' loading=lazy></a></td>"
                        );
                    }
                    html.push_str("</tr>");
                }
                html.push_str("</table>");
            }
        }
    }
    fs::write(out.join("index.html"), html)?;
    Ok(())
}
