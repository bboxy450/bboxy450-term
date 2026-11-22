// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Fit-to-content capture through the C0 UI module and the suite headless GPU.
use super::{d, error};
use bboxy450_canvas::{Bounds, Camera, Canvas, FIT_PADDING};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{DrawList, TextStyle, TypeScale},
    layout::{Rect, TextMeasure, TextStyle as MeasureStyle},
};
use std::cell::RefCell;
pub(super) const WIDTH: u32 = 960;
pub(super) const HEIGHT: u32 = 640;
const MAX_WIDTH: u32 = 1920;
const MAX_HEIGHT: u32 = 1440;

/// The PNG's pixel size: the fitted extent at scale one plus the fit padding,
/// between the default and the maximum, so a wide drawing is not shrunk to
/// the default. The extent is the canvas's own (`Canvas::extent`) or the frame's.
pub(super) fn image_size(canvas: &Canvas, frame: Option<Bounds>) -> [u32; 2] {
    let extent = frame.or_else(|| canvas.extent());
    let axis = |i: usize, min: u32, max: u32| {
        extent.map_or(min, |e| {
            (e.size[i] + FIT_PADDING * 2.).clamp(f64::from(min), f64::from(max)) as u32
        })
    };
    [axis(0, WIDTH, MAX_WIDTH), axis(1, HEIGHT, MAX_HEIGHT)]
}

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
    let [width, height] = image_size(canvas, frame);
    Camera::fit(
        frame.or_else(|| canvas.extent()),
        [f64::from(width), f64::from(height)],
        FIT_PADDING,
    )
    .map_err(|s| error("canvas_render", s))
}
pub(super) fn png(
    canvas: &Canvas,
    frame: Option<Bounds>,
    assets: &std::collections::BTreeMap<String, bbt_ui_model::draw::ImageAsset>,
    rasters: Vec<std::sync::Arc<bbt_render::chrome::icons::Image>>,
) -> d::Result<(Vec<u8>, [u32; 2])> {
    let map = |e: &dyn std::fmt::Display| error("canvas_render", e.to_string());
    let [width, height] = image_size(canvas, frame);
    let viewport = Rect::new(0., 0., width as f32, height as f32);
    let element = ui::modules::canvas::render_with_images(
        canvas,
        &fit(canvas, frame)?,
        viewport,
        ui::Theme::Dark,
        assets,
    )
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
    for raster in rasters {
        renderer.add_image(&raster).map_err(|e| map(&e))?;
    }
    let pixels = renderer
        .render_to_rgba(&list, width, height, 1.)
        .map_err(|e| map(&e))?;
    let mut bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| map(&e))?
            .write_image_data(&pixels)
            .map_err(|e| map(&e))?;
    }
    Ok((bytes, [width, height]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_images::{GeometryCache, Images, Prepared, Resolved, object_path};
    use bboxy450_canvas::Action;
    use bbt_supervisor::store::{Blobs, blobs::LocalBlobs};
    use bbt_ui_model::draw::Prim;
    use serde_json::json;

    #[test]
    fn canvas_artifact_image_paint_stays_inside_bounds() {
        let id = "01M828WMTZP8TQYR4AYF6K2N6H";
        let add: Action = serde_json::from_value(json!({
            "op":"add","element":{"id":"image","tags":[],"kind":null,
            "content":{"type":"image","bounds":{"origin":[0,0],"size":[240,120]},"artifact":id}}
        }))
        .unwrap();
        let mut canvas = Canvas::default();
        canvas.apply_set(&[add]).unwrap();

        let directory = tempfile::tempdir().unwrap();
        let root = "0".repeat(64);
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[255; 8])
                .unwrap();
        }
        let store = LocalBlobs::open(&object_path(directory.path(), &root)).unwrap();
        let resolved = Resolved {
            root,
            id: id.parse().unwrap(),
            revision: 1,
            digest: store.put(&png).unwrap(),
            media_type: "image/png".into(),
            bytes: png.len(),
        };
        let mut images = Images::default();
        let cache = GeometryCache::default();
        images.canvas(&canvas, |_, remaining| {
            Prepared::artifact(&resolved, directory.path(), &cache, remaining)
        });
        let viewport = Rect::new(0., 0., 600., 360.);
        let camera = Camera::default();
        let cx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let canvas = ui::modules::canvas::render_with_images(
            &canvas,
            &camera,
            viewport,
            ui::Theme::Dark,
            &images.assets(),
        )
        .unwrap();
        let layout = canvas.layout(viewport, cx, &Metrics::default()).unwrap();
        let rects = c::paint(&layout, cx)
            .into_iter()
            .flat_map(|paint| match paint {
                c::Paint::Draw(draw) => draw.primitives,
                _ => vec![],
            })
            .filter_map(|primitive| match primitive {
                Prim::Image { rect, .. } => Some(rect),
                Prim::Text { text, .. } => panic!("unexpected canvas caption: {text}"),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rects, [Rect::new(300., 180., 240., 120.)]);
    }
}
