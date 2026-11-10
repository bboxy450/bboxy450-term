// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Fit-to-content capture through the C0 UI module and the suite headless GPU.
use super::{d, error};
use bboxy450_canvas::{Bounds, Camera, Canvas};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{DrawList, TextStyle, TypeScale},
    layout::{Rect, TextMeasure, TextStyle as MeasureStyle},
};
use std::cell::RefCell;
pub(super) const WIDTH: u32 = 960;
pub(super) const HEIGHT: u32 = 640;

// Adapted from bboxy450-term src/scene.rs Metrics, without its frame cache.
// https://github.com/bboxy450/bboxy450-term — same native font measurement.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#[derive(Default)]
struct Metrics {
    failure: RefCell<Option<String>>,
}
impl Metrics {
    fn measure(&self, text: &str, s: &MeasureStyle) -> bbt_render::chrome::text::TextMetrics {
        let style = TextStyle {
            scale: TypeScale::Custom(s.size),
            weight: s.weight,
            mono: s.mono,
            italic: s.italic,
            tracking_em: s.tracking_em,
            ..Default::default()
        };
        bbt_render::chrome::text::measure(text, style, None).unwrap_or_else(|error| {
            *self.failure.borrow_mut() = Some(error.to_string());
            bbt_render::chrome::text::TextMetrics {
                advance: 0.,
                ascent: 0.,
                descent: 0.,
                leading: 0.,
            }
        })
    }
}
impl TextMeasure for Metrics {
    fn width(&self, text: &str, s: &MeasureStyle) -> f32 {
        self.measure(text, s).advance
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &MeasureStyle) -> [f32; 2] {
        let m = self.measure("Hg", s);
        [m.ascent, m.descent]
    }
}
fn fit(canvas: &Canvas, frame: Option<Bounds>) -> d::Result<Camera> {
    let bounds: Vec<_> = frame.map_or_else(
        || {
            canvas
                .elements()
                .filter_map(|e| e.content.bounds())
                .collect()
        },
        |b| vec![b],
    );
    if bounds.is_empty() {
        return Ok(Camera::default());
    }
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for b in bounds {
        for axis in 0..2 {
            min[axis] = min[axis].min(b.origin[axis]);
            max[axis] = max[axis].max(b.origin[axis] + b.size[axis]);
        }
    }
    let size = [max[0] - min[0], max[1] - min[1]];
    let camera = Camera {
        center: [min[0] + size[0] / 2., min[1] + size[1] / 2.],
        zoom: ((f64::from(WIDTH) - 64.) / size[0]).min((f64::from(HEIGHT) - 64.) / size[1]),
    };
    camera.validate().map_err(|s| error("canvas_render", s))?;
    Ok(camera)
}
pub(super) fn png(canvas: &Canvas, frame: Option<Bounds>) -> d::Result<Vec<u8>> {
    let map = |e: &dyn std::fmt::Display| error("canvas_render", e.to_string());
    let viewport = Rect::new(0., 0., WIDTH as f32, HEIGHT as f32);
    let element =
        ui::modules::canvas::render(canvas, &fit(canvas, frame)?, viewport, ui::Theme::Dark)
            .map_err(|e| map(&e))?;
    let cx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let metrics = Metrics::default();
    let layout = element
        .layout(viewport, cx, &metrics)
        .map_err(|e| map(&e))?;
    if let Some(reason) = metrics.failure.into_inner() {
        return Err(error("canvas_render", reason));
    }
    let mut list = DrawList::default();
    for paint in c::paint(&layout, cx) {
        match paint {
            c::Paint::Draw(draw) | c::Paint::Underlay(draw) => {
                list.primitives.extend(draw.primitives)
            }
            _ => return Err(error("canvas_render", "unexpected non-static canvas paint")),
        }
    }
    list.validate().map_err(|e| map(&e))?;
    let mut renderer =
        bbt_render::chrome::ChromeRenderer::new(bbt_render::chrome::icons::Assets::default())
            .map_err(|e| map(&e))?;
    let pixels = renderer
        .render_to_rgba(&list, WIDTH, HEIGHT, 1.)
        .map_err(|e| map(&e))?;
    let mut bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut bytes, WIDTH, HEIGHT);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| map(&e))?
            .write_image_data(&pixels)
            .map_err(|e| map(&e))?;
    }
    Ok(bytes)
}
