// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{ChromeRenderer, ChromeTarget};
use crate::Result;
use bbt_ui_model::{
    draw::{Color, DrawList, Prim, RetainedImage, TextStyle},
    layout::Rect,
};

/// The natural placement is in logical units. The region is the intersection
/// with the current viewport and active clips, in physical pixels.
#[derive(Clone, Copy)]
pub struct ImagePlacement {
    pub bounds: Rect,
    pub region: ImageRegion,
    pub opacity: f32,
}
#[derive(Clone, Copy)]
pub struct ImageRegion {
    pub pixels: [u32; 4],
    pub(super) clip: [f32; 4],
    pub(super) radii: [f32; 4],
}

pub(super) fn intersect_clip(outer: Rect, inner: Rect) -> Rect {
    let x = inner.x.max(outer.x);
    let y = inner.y.max(outer.y);
    Rect::new(
        x,
        y,
        (inner.right().min(outer.right()) - x).max(0.),
        (inner.bottom().min(outer.bottom()) - y).max(0.),
    )
}

type Painter = dyn for<'a> Fn(
        &mut ChromeRenderer,
        ChromeTarget<'a>,
        &mut wgpu::CommandEncoder,
        ImagePlacement,
    ) -> Result<()>
    + Send
    + Sync;
struct NativePicture(Box<Painter>);

pub fn retained_picture(
    size: [f32; 2],
    paint: impl for<'a> Fn(
        &mut ChromeRenderer,
        ChromeTarget<'a>,
        &mut wgpu::CommandEncoder,
        ImagePlacement,
    ) -> Result<()>
    + Send
    + Sync
    + 'static,
) -> RetainedImage {
    RetainedImage::new(size, NativePicture(Box::new(paint)))
}

fn region(bounds: Rect, clips: &[Prim], target: ChromeTarget<'_>) -> Option<ImageRegion> {
    let s = target.scale;
    let mut clip = Rect::new(0., 0., target.width as f32 / s, target.height as f32 / s);
    let mut radii = [0.; 4];
    for rule in clips {
        if let Prim::Clip {
            rect,
            radii: corners,
        } = rule
        {
            clip = intersect_clip(clip, *rect);
            radii = *corners;
        }
    }
    let visible = intersect_clip(clip, bounds);
    if visible.w == 0. || visible.h == 0. {
        return None;
    }
    let [x, y] = [visible.x, visible.y].map(|n| (n * s).floor() as u32);
    let right = ((visible.right() * s).ceil() as u32).min(target.width);
    let bottom = ((visible.bottom() * s).ceil() as u32).min(target.height);
    Some(ImageRegion {
        pixels: [x, y, right - x, bottom - y],
        clip: [clip.x, clip.y, clip.w, clip.h].map(|n| n * s),
        radii: radii.map(|n| n * s),
    })
}
impl ChromeRenderer {
    pub(super) fn draw_retained(
        &mut self,
        picture: &RetainedImage,
        bounds: Rect,
        opacity: f32,
        clips: &[Prim],
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(region) = region(bounds, clips, target) else {
            return Ok(());
        };
        let placement = ImagePlacement {
            bounds,
            region,
            opacity,
        };
        if let Some(paint) = picture.payload::<NativePicture>()
            && (paint.0)(self, target, encoder, placement).is_ok()
        {
            return Ok(());
        }

        let [x, y, w, h] = region.pixels;
        let s = target.scale;
        let clipped = Rect::new(x as f32 / s, y as f32 / s, w as f32 / s, h as f32 / s);
        let mut placeholder = DrawList {
            primitives: clips.to_vec(),
            ..Default::default()
        };
        placeholder.clip(clipped, [0.; 4]);
        placeholder.text(
            clipped,
            "Image unavailable",
            TextStyle::default(),
            Color::WHITE.alpha(opacity),
        );
        placeholder.unclip();
        for _ in clips {
            placeholder.unclip();
        }
        self.draw_flat_with_load(&placeholder, target, encoder, wgpu::LoadOp::Load)
    }
}
