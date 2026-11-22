// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Isolated offscreen layers on the chrome device, composed in CSS sRGB space.
use super::{ChromeRenderer, ChromeTarget};
use crate::{Result, atlas::AtlasRect, device::DeviceContext};
use bbt_ui_model::draw::LinearAlphaMask;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub opacity: f32,
    pub saturation: f32,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            opacity: 1.,
            saturation: 1.,
        }
    }
}
impl Filter {
    fn validate(self) -> Result<()> {
        if !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
            || !self.saturation.is_finite()
            || !(0.0..=4.0).contains(&self.saturation)
        {
            return Err("invalid group filter".into());
        }
        Ok(())
    }
}
pub(super) fn texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("isolated chrome group"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    })
}
fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("isolated group composite"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(include_str!("shapes.wgsl"), include_str!("group.wgsl")).into(),
        ),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sRGB group source-over"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
fn validate_layer(
    layer: &wgpu::TextureView,
    region: Option<super::ImageRegion>,
    filter: Filter,
    mask: Option<LinearAlphaMask>,
    target: &ChromeTarget<'_>,
) -> Result<()> {
    filter.validate()?;
    if let Some(mask) = mask {
        mask.validate()?;
    }
    let size = layer.texture().size();
    let origin = region.map_or([0, 0], |crop| [crop.pixels[0], crop.pixels[1]]);
    if region.is_none() && (size.width != target.width || size.height != target.height) {
        return Err("group layer size differs from target".into());
    }
    if origin[0].saturating_add(size.width) > target.width
        || origin[1].saturating_add(size.height) > target.height
    {
        return Err("group layer exceeds target bounds".into());
    }
    Ok(())
}

/// The group pass and its submission arena. A terminal raster worker needs
/// this pass without allocating a chrome image or glyph atlas.
pub struct Compositor {
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    pub(super) arenas: super::arena::Arenas,
}

impl Default for Compositor {
    fn default() -> Self {
        Self {
            pipelines: HashMap::new(),
            arenas: super::arena::Arenas::new("chrome draw"),
        }
    }
}

impl Compositor {
    pub fn begin_frame(&mut self) {
        self.arenas.begin_frame();
    }

    pub fn flush(&mut self, context: &DeviceContext) {
        self.arenas.flush(context);
    }

    pub fn composite_group(
        &mut self,
        context: &DeviceContext,
        layer: &wgpu::TextureView,
        filter: Filter,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        validate_layer(layer, None, filter, None, &target)?;
        self.draw(
            context,
            (layer, None),
            filter,
            None,
            None,
            (target, wgpu::LoadOp::Load),
            encoder,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one GPU pass carries its source, masks and destination"
    )]
    fn draw(
        &mut self,
        context: &DeviceContext,
        (layer, region): (&wgpu::TextureView, Option<super::ImageRegion>),
        filter: Filter,
        mask: Option<LinearAlphaMask>,
        coverage: Option<(super::text_cache::Part, AtlasRect, wgpu::TextureView)>,
        (target, load): (ChromeTarget<'_>, wgpu::LoadOp<wgpu::Color>),
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let size = layer.texture().size();
        let origin = region.map_or([0, 0], |crop| [crop.pixels[0], crop.pixels[1]]);
        let device = &context.device;
        let format = target.format.remove_srgb_suffix();
        let pipeline = self
            .pipelines
            .entry(format)
            .or_insert_with(|| pipeline(device, format));
        let mut values = [0.; 28];
        values[0] = filter.opacity;
        values[1] = filter.saturation;
        values[18..20].copy_from_slice(&origin.map(|n| n as f32));
        if let Some(region) = region {
            values[10] = 1.;
            values[20..24].copy_from_slice(&region.clip);
            values[24..28].copy_from_slice(&region.radii);
        }
        if let Some(mask) = mask {
            values[2] = 1.;
            values[4..8].copy_from_slice(&[
                mask.start[0] * target.scale,
                mask.start[1] * target.scale,
                mask.end[0] * target.scale,
                mask.end[1] * target.scale,
            ]);
            values[8] = mask.alpha0;
            values[9] = mask.alpha1;
        }
        if let Some((part, rect, _)) = &coverage {
            values[3] = 1.;
            values[12..16].copy_from_slice(&[
                part.offset[0] as f32,
                part.offset[1] as f32,
                f32::from(rect.x),
                f32::from(rect.y),
            ]);
            values[16] = part.size[0] as f32;
            values[17] = part.size[1] as f32;
        }
        let params = self.arenas.uniform(context, bytemuck::cast_slice(&values));
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("group composite bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(layer),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        coverage.as_ref().map_or(layer, |(_, _, view)| view),
                    ),
                },
            ],
        });
        let view = target
            .view
            .texture()
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(format),
                ..Default::default()
            });
        crate::passes::count(crate::passes::Kind::Group);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("isolated group source-over"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        if region.is_some() {
            pass.set_viewport(
                origin[0] as f32,
                origin[1] as f32,
                size.width as f32,
                size.height as f32,
                0.,
                1.,
            );
        }
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

impl ChromeRenderer {
    /// Retained consumers use the same pooled isolation layers as Prim::Group.
    pub fn paint_group(
        &mut self,
        filter: Filter,
        mask: Option<LinearAlphaMask>,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        paint: impl FnOnce(&mut Self, ChromeTarget<'_>, &mut wgpu::CommandEncoder) -> Result<()>,
    ) -> Result<()> {
        self.paint_group_inner(filter, mask, None, target, encoder, paint)
    }
    /// A worker-prepared arbitrary path clip is applied once to the whole group.
    pub fn paint_group_coverage(
        &mut self,
        filter: Filter,
        coverage: Option<&super::PreparedPath>,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        paint: impl FnOnce(&mut Self, ChromeTarget<'_>, &mut wgpu::CommandEncoder) -> Result<()>,
    ) -> Result<()> {
        self.paint_group_inner(filter, None, coverage, target, encoder, paint)
    }
    fn paint_group_inner(
        &mut self,
        filter: Filter,
        mask: Option<LinearAlphaMask>,
        coverage: Option<&super::PreparedPath>,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        paint: impl FnOnce(&mut Self, ChromeTarget<'_>, &mut wgpu::CommandEncoder) -> Result<()>,
    ) -> Result<()> {
        let layer = self.take_group_layer(target.width, target.height);
        let view = layer.create_view(&Default::default());
        let group_target = ChromeTarget {
            view: &view,
            format: wgpu::TextureFormat::Rgba8Unorm,
            ..target
        };
        let result = self.with_group_layer(|renderer| {
            renderer.draw(
                &bbt_ui_model::draw::DrawList::default(),
                group_target,
                encoder,
            )?;
            paint(renderer, group_target, encoder)
        });
        let result = result.and_then(|()| {
            self.composite_with_load(
                (&view, None),
                filter,
                mask,
                coverage,
                (target, wgpu::LoadOp::Load),
                encoder,
            )
        });
        self.group_layers.push(layer);
        result
    }
    /// Paint an offscreen group using gamma-premultiplied storage. Chrome text
    /// uses the same CSS source-over rule inside and outside the group.
    pub fn with_group_layer<T, E>(
        &mut self,
        paint: impl FnOnce(&mut Self) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        self.group_depth += 1;
        let result = paint(self);
        self.group_depth -= 1;
        result
    }
    /// Composite a completed premultiplied layer; the destination alpha survives
    /// normal source-over. Callers keep identity strokes outside the dim group.
    pub fn composite_group(
        &mut self,
        layer: &wgpu::TextureView,
        filter: Filter,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.composite_group_with_mask(layer, filter, None, target, encoder)
    }
    /// Optional ramp in draw-list logical pixels, applied once to group alpha.
    pub fn composite_group_with_mask(
        &mut self,
        layer: &wgpu::TextureView,
        filter: Filter,
        mask: Option<LinearAlphaMask>,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.composite_with_load(
            (layer, None),
            filter,
            mask,
            None,
            (target, wgpu::LoadOp::Load),
            encoder,
        )
    }
    /// As composite_group onto an uncleared target: the composite pass itself clears it to
    /// transparent, exactly what a separate clear pass followed by composite_group draws.
    pub fn composite_group_clearing(
        &mut self,
        layer: &wgpu::TextureView,
        filter: Filter,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        self.composite_with_load((layer, None), filter, None, None, (target, clear), encoder)
    }
    /// A native retained picture reuses group storage and source-over on only
    /// the intersecting physical rectangle. No device-sized scratch or readback.
    pub fn paint_image_region(
        &mut self,
        region: super::ImageRegion,
        opacity: f32,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        paint: impl FnOnce(&mut Self, ChromeTarget<'_>, &mut wgpu::CommandEncoder) -> Result<()>,
    ) -> Result<()> {
        let [x, y, width, height] = region.pixels;
        if width == 0
            || height == 0
            || width > 4096
            || height > 4096
            || u64::from(width) * u64::from(height) > 4_000_000
            || x.saturating_add(width) > target.width
            || y.saturating_add(height) > target.height
        {
            return Err("image exceeds its visible render budget".into());
        }
        let layer = self.take_group_layer(width, height);
        let view = layer.create_view(&Default::default());
        let own_target = ChromeTarget {
            width,
            height,
            view: &view,
            scale: 1.,
            format: wgpu::TextureFormat::Rgba8Unorm,
        };
        let result = self.with_group_layer(|renderer| {
            super::clear(&view, wgpu::Color::TRANSPARENT, encoder);
            paint(renderer, own_target, encoder)
        });
        let result = result.and_then(|()| {
            self.composite_with_load(
                (&view, Some(region)),
                Filter {
                    opacity,
                    saturation: 1.,
                },
                None,
                None,
                (target, wgpu::LoadOp::Load),
                encoder,
            )
        });
        self.group_layers.push(layer);
        result
    }

    fn composite_with_load(
        &mut self,
        (layer, region): (&wgpu::TextureView, Option<super::ImageRegion>),
        filter: Filter,
        mask: Option<LinearAlphaMask>,
        coverage: Option<&super::PreparedPath>,
        (target, load): (ChromeTarget<'_>, wgpu::LoadOp<wgpu::Color>),
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        validate_layer(layer, region, filter, mask, &target)?;
        let coverage = if let Some(path) = coverage {
            let Some(part) = self.text.prepared_path(path)? else {
                return Ok(());
            };
            self.glyph_reset_pending |= self.text.finish(&[part.id])?;
            self.sync_glyph_atlases();
            let rect = self.text.rect(part.id)?;
            Some((
                part,
                rect,
                self.color_glyph_gpu
                    .as_ref()
                    .ok_or("missing clip atlas")?
                    .view
                    .clone(),
            ))
        } else {
            None
        };
        self.compositor.draw(
            &self.context,
            (layer, region),
            filter,
            mask,
            coverage,
            (target, load),
            encoder,
        )
    }
    /// Headless host bridge. Both buffers use the renderer's premultiplied RGBA
    /// convention; compositing still runs on the caller's existing device.
    pub fn composite_group_rgba(
        &mut self,
        layer: &[u8],
        backdrop: &[u8],
        size: [u32; 2],
        filter: Filter,
    ) -> Result<Vec<u8>> {
        self.composite_group_rgba_with_mask(layer, backdrop, size, filter, None)
    }
    pub fn composite_group_rgba_with_mask(
        &mut self,
        layer: &[u8],
        backdrop: &[u8],
        size: [u32; 2],
        filter: Filter,
        mask: Option<LinearAlphaMask>,
    ) -> Result<Vec<u8>> {
        filter.validate()?;
        if let Some(mask) = mask {
            mask.validate()?;
        }
        let [width, height] = size;
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("invalid group RGBA size".into());
        }
        let len = width as usize * height as usize * 4;
        if layer.len() != len || backdrop.len() != len {
            return Err("invalid group RGBA buffers".into());
        }
        let device = &self.context.device;
        let make = |label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            })
        };
        let source = make("headless group source");
        let destination = make("headless group destination");
        for (texture, bytes) in [(&source, layer), (&destination, backdrop)] {
            self.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.compositor.begin_frame();
        let mut encoder = device.create_command_encoder(&Default::default());
        self.composite_group_with_mask(
            &source.create_view(&Default::default()),
            filter,
            mask,
            ChromeTarget {
                width,
                height,
                scale: 1.,
                view: &destination.create_view(&Default::default()),
                format: wgpu::TextureFormat::Rgba8Unorm,
            },
            &mut encoder,
        )?;
        self.compositor.flush(&self.context);
        self.readback(&destination, encoder, width, height)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    // keeps: spec svg-renderer — a nonrectangular group clip clips all overlapping children once.
    #[test]
    fn svg_nonrectangular_clip_preserves_existing_group_opacity() {
        use bbt_ui_model::draw::{Color, DrawList, Paint, PathCommand, VectorPath};
        use bbt_ui_model::layout::Rect;
        let path = VectorPath {
            points: vec![
                PathCommand::Move([0., 0.]),
                PathCommand::Line([32., 0.]),
                PathCommand::Line([0., 32.]),
                PathCommand::Close,
            ]
            .into(),
            fill: Some(Paint::Solid(Color::WHITE)),
            transform: bbt_ui_model::draw::Affine::default(),
            fill_rule: bbt_ui_model::draw::FillRule::NonZero,
            fill_opacity: 1.,
            stroke: bbt_ui_model::draw::PathStroke::default(),
            stroke_paint: None,
            stroke_opacity: 1.,
            miter_limit: 4.,
            dash_offset: 0.,
        };
        let clip = super::super::prepare_vector_path(&path, [32, 32], [0., 0., 32., 32.]).unwrap();
        let mut r =
            super::super::ChromeRenderer::new(Vec::<super::super::icons::Image>::new()).unwrap();
        let pixels = r
            .render_to_rgba_with(32, 32, 1., |r, t, e| {
                let mut background = DrawList::default();
                background.rect(Rect::new(0., 0., 32., 32.), 0., Color::WHITE);
                r.draw(&background, t, e)?;
                r.paint_group_coverage(
                    super::Filter {
                        opacity: 0.5,
                        saturation: 1.,
                    },
                    Some(&clip),
                    t,
                    e,
                    |r, t, e| {
                        let mut child = DrawList::default();
                        child.rect(Rect::new(0., 0., 32., 32.), 0., Color([1., 0., 0., 1.]));
                        child.rect(Rect::new(0., 0., 32., 32.), 0., Color([0., 0., 1., 1.]));
                        r.draw(&child, t, e)
                    },
                )
            })
            .unwrap();
        assert_eq!(
            &pixels[(4 * 32 + 4) * 4..(4 * 32 + 4) * 4 + 4],
            &[128, 128, 255, 255]
        );
        assert_eq!(
            &pixels[(28 * 32 + 28) * 4..(28 * 32 + 28) * 4 + 4],
            &[255; 4]
        );
    }

    use super::*;
    use bbt_ui_model::{
        draw::{Color, DrawList, Prim},
        layout::Rect,
    };

    fn group(list: DrawList, opacity: f32, saturation: f32) -> Prim {
        Prim::Group {
            primitives: list.primitives,
            opacity,
            saturation,
            mask: None,
        }
    }
    fn sample(pixels: &[u8], x: usize, y: usize) -> &[u8] {
        &pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4]
    }
    fn near(actual: &[u8], expected: [u8; 4]) {
        assert!(
            actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
            "{actual:?} != {expected:?}"
        );
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn overlapping_children_dim_once_and_nested_groups_keep_painter_order() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut content = DrawList::default();
        content.rect(Rect::new(0., 0., 24., 24.), 0., Color::hex("ff0000"));
        content.rect(Rect::new(8., 0., 8., 24.), 0., Color::WHITE);
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 32., 32.), 0., Color::BLACK);
        list.push(group(content.clone(), 0.5, 1.));
        let once = renderer.render_to_rgba(&list, 32, 32, 1.).unwrap();
        near(sample(&once, 4, 8), [128, 0, 0, 255]);
        // Per-paint alpha would leave a pink overlap, [191,128,128].
        near(sample(&once, 12, 8), [128, 128, 128, 255]);
        let mut inner = DrawList::default();
        inner.push(group(content, 0.5, 1.));
        list.primitives.pop();
        list.push(group(inner, 0.5, 1.));
        list.rect(Rect::new(16., 0., 8., 24.), 0., Color::hex("00ff00"));
        let nested = renderer.render_to_rgba(&list, 32, 32, 1.).unwrap();
        near(sample(&nested, 12, 8), [64, 64, 64, 255]);
        near(sample(&nested, 20, 8), [0, 255, 0, 255]);
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn css_saturation_unpremultiplies_before_filter_and_preserves_destination_alpha() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        assert!(
            renderer
                .composite_group_rgba(&[], &[], [u32::MAX, u32::MAX], Filter::default())
                .is_err()
        );
        let pixels = renderer
            .composite_group_rgba(
                &[128, 0, 0, 128],
                &[0, 0, 64, 64],
                [1, 1],
                Filter {
                    opacity: 0.5,
                    saturation: 0.,
                },
            )
            .unwrap();
        near(&pixels, [14, 14, 62, 112]);
        let mut content = DrawList::default();
        content.rect(Rect::new(0., 0., 32., 32.), 0., Color::hex("ff0000"));
        let list = DrawList {
            primitives: vec![group(content, 0.5, 0.)],
            ..Default::default()
        };
        let pixels = renderer.render_to_rgba(&list, 32, 32, 1.).unwrap();
        near(sample(&pixels, 16, 16), [27, 27, 27, 128]);
        assert!(
            renderer
                .composite_group_rgba(
                    &pixels,
                    &pixels,
                    [32, 32],
                    Filter {
                        opacity: f32::NAN,
                        saturation: 1.
                    }
                )
                .is_err()
        );
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn outer_and_child_clips_survive_group_splits_and_do_not_leak_to_following_paint() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut content = DrawList::default();
        content.clip(Rect::new(12., 0., 8., 24.), [0.; 4]);
        content.rect(Rect::new(0., 0., 32., 32.), 0., Color::hex("ff0000"));
        content.unclip();
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 32., 32.), 0., Color::BLACK);
        list.clip(Rect::new(8., 0., 16., 24.), [4.; 4]);
        list.push(group(content, 0.5, 1.));
        list.rect(Rect::new(20., 8., 4., 8.), 0., Color::WHITE);
        list.unclip();
        list.rect(Rect::new(0., 24., 32., 8.), 0., Color::hex("00ff00"));
        let pixels = renderer.render_to_rgba(&list, 32, 32, 1.).unwrap();
        near(sample(&pixels, 10, 8), [0, 0, 0, 255]);
        near(sample(&pixels, 16, 8), [128, 0, 0, 255]);
        near(sample(&pixels, 22, 10), [255, 255, 255, 255]);
        near(sample(&pixels, 2, 28), [0, 255, 0, 255]);
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn empty_and_identity_groups_are_transparent_and_use_the_same_device_at_scale() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let empty = DrawList {
            primitives: vec![group(DrawList::default(), 0.4, 0.1)],
            ..Default::default()
        };
        assert!(
            renderer
                .render_to_rgba(&empty, 32, 32, 2.)
                .unwrap()
                .iter()
                .all(|v| *v == 0)
        );
        let mut content = DrawList::default();
        content.rect(
            Rect::new(1.25, 1.25, 10.5, 10.5),
            3.,
            Color::hex("3399cc80"),
        );
        let plain = renderer.render_to_rgba(&content, 32, 32, 2.).unwrap();
        let grouped = DrawList {
            primitives: vec![group(content, 1., 1.)],
            ..Default::default()
        };
        let isolated = renderer.render_to_rgba(&grouped, 32, 32, 2.).unwrap();
        assert_eq!(plain, isolated);
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn isolated_text_carries_a_premultiplied_mask_and_matches_opaque_compositing() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut text = DrawList::default();
        text.text(
            Rect::new(0., 0., 32., 30.),
            "Hg",
            TextStyle {
                scale: TypeScale::Custom(20.),
                ..Default::default()
            },
            Color::WHITE,
        );
        let isolated = DrawList {
            primitives: vec![group(text.clone(), 0.45, 0.1)],
            ..Default::default()
        };
        let pixels = renderer.render_to_rgba(&isolated, 32, 32, 1.).unwrap();
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 0 && p[3] < 114)
        );
        for p in pixels.as_chunks::<4>().0 {
            assert!(
                p[..3].iter().all(|c| c.abs_diff(p[3]) <= 1),
                "non-premultiplied text edge: {p:?}"
            );
        }
        let mut opaque = DrawList::default();
        opaque.rect(Rect::new(0., 0., 32., 32.), 0., Color::hex("333333"));
        opaque.primitives.extend(text.primitives);
        let plain = renderer.render_to_rgba(&opaque, 32, 32, 1.).unwrap();
        let grouped = DrawList {
            primitives: vec![group(opaque, 1., 1.)],
            ..Default::default()
        };
        let corrected = renderer.render_to_rgba(&grouped, 32, 32, 1.).unwrap();
        assert!(
            plain
                .iter()
                .zip(&corrected)
                .all(|(a, b)| a.abs_diff(*b) <= 1)
        );
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn linear_mask_filters_the_completed_layer_once_at_1x_and_2x() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mask = LinearAlphaMask {
            start: [8., 0.],
            end: [24., 0.],
            alpha0: 1.,
            alpha1: 0.,
        };
        for scale in [1., 2.] {
            let size = (32. * scale) as u32;
            let mut children = DrawList::default();
            children.rect(Rect::new(0., 0., 32., 32.), 0., Color::hex("ff000080"));
            children.rect(Rect::new(4., 0., 28., 32.), 0., Color::hex("0000ff80"));
            let unmasked = renderer
                .render_to_rgba(&children, size, size, scale)
                .unwrap();
            let list = DrawList {
                primitives: vec![Prim::Group {
                    primitives: children.primitives,
                    opacity: 0.6,
                    saturation: 1.,
                    mask: Some(mask),
                }],
                ..Default::default()
            };
            let pixels = renderer.render_to_rgba(&list, size, size, scale).unwrap();
            for x in [2, 10, 16, 22, 28] {
                let px = (x as f32 * scale) as usize;
                let at = (16 * scale as usize * size as usize + px) * 4;
                let t = (((px as f32 + 0.5) / scale - 8.) / 16.).clamp(0., 1.);
                for i in 0..4 {
                    let expected = (unmasked[at + i] as f32 * 0.6 * (1. - t)).round() as u8;
                    assert!(
                        pixels[at + i].abs_diff(expected) <= 2,
                        "scale {scale}, x {x}, channel {i}: {} != {expected}",
                        pixels[at + i]
                    );
                }
            }
        }
    }
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn mask_bridge_preserves_destination_alpha_and_the_old_api() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let layer = [128, 0, 0, 128].repeat(8);
        let behind = [0, 0, 64, 64].repeat(8);
        let filter = Filter {
            opacity: 0.5,
            saturation: 1.,
        };
        let old = renderer
            .composite_group_rgba(&layer, &behind, [8, 1], filter)
            .unwrap();
        assert_eq!(
            old,
            renderer
                .composite_group_rgba_with_mask(&layer, &behind, [8, 1], filter, None)
                .unwrap()
        );
        let mask = LinearAlphaMask {
            start: [2., 0.],
            end: [6., 0.],
            alpha0: 1.,
            alpha1: 0.,
        };
        let faded = renderer
            .composite_group_rgba_with_mask(&layer, &behind, [8, 1], filter, Some(mask))
            .unwrap();
        assert_eq!(&faded[..4], &old[..4]);
        assert_eq!(&faded[28..], &behind[28..]);
        assert!(faded[16 + 3] > 64 && faded[16 + 3] < old[16 + 3]);
        let invalid = LinearAlphaMask {
            end: mask.start,
            ..mask
        };
        assert!(
            renderer
                .composite_group_rgba_with_mask(&layer, &behind, [8, 1], filter, Some(invalid))
                .is_err()
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod clearing_tests {
    use super::*;
    // keeps: spec renderer §1a/§1b — ordered isolated layers, alpha, clips and scaled masks
    #[test]
    fn a_clearing_composite_equals_compositing_onto_transparent() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let [width, height] = [24u32, 12];
        let len = (width * height * 4) as usize;
        // Premultiplied: each channel at most its pixel's alpha.
        let layer: Vec<u8> = (0..len)
            .map(|i| {
                let a = ((i / 4) * 29 % 256) as u8;
                if i % 4 == 3 {
                    a
                } else {
                    ((i * 53) % 256) as u8 % a.max(1)
                }
            })
            .collect();
        let filter = Filter {
            opacity: 0.7,
            saturation: 0.5,
        };
        let expected = renderer
            .composite_group_rgba(&layer, &vec![0; len], [width, height], filter)
            .unwrap();
        let make = |bytes: &[u8]| {
            let texture = renderer
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
                });
            renderer.context.queue.write_texture(
                texture.as_image_copy(),
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                texture.size(),
            );
            texture
        };
        let source = make(&layer);
        // Garbage the clearing pass must replace, as a separate clear pass did.
        let destination = make(&vec![200; len]);
        renderer.begin_frame();
        let mut encoder = renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        renderer
            .composite_group_clearing(
                &source.create_view(&Default::default()),
                filter,
                ChromeTarget {
                    width,
                    height,
                    scale: 1.,
                    view: &destination.create_view(&Default::default()),
                    format: wgpu::TextureFormat::Rgba8Unorm,
                },
                &mut encoder,
            )
            .unwrap();
        renderer.flush();
        let pixels = renderer
            .readback(&destination, encoder, width, height)
            .unwrap();
        assert_eq!(pixels, expected);
    }
}
