//! One ordered, instanced chrome pass on the caller's device; no window ownership.
mod arena;
mod atlas_gpu;
pub mod diff;
pub mod group;
pub mod icons;
pub mod materials;
mod paths;
pub mod pipeline;
pub mod text;
mod text_cache;
mod text_glyphs;
use crate::{RenderError, Result, device::DeviceContext};
use bbt_ui_model::{
    draw::{Color, DrawList, Prim},
    layout::Rect,
};
use pipeline::Instance;
use std::{collections::HashMap, sync::mpsc, time::Duration};

pub struct ChromeTarget<'a> {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

pub use text_cache::Stats as GlyphCacheStats;
#[derive(Clone, Copy, Debug)]
pub struct TextCacheStats {
    pub mode: &'static str,
    pub whole_line_masks: usize,
    pub coverage: GlyphCacheStats,
}

pub struct ChromeRenderer {
    pub context: DeviceContext,
    text: text_cache::Cache,
    icons: icons::IconAtlas,
    icon_gpu: atlas_gpu::AtlasTexture,
    glyph_gpu: Option<atlas_gpu::AtlasTexture>,
    glyph_reset_pending: bool,
    image_sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    group_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    group_depth: usize,
    paths: paths::Renderer,
    /// Headless material renders reuse one Pass (bind layout and pipelines) per target
    /// format; see materials::render_layered_to_rgba.
    material_pass: Option<(wgpu::TextureFormat, materials::Pass)>,
    /// Per-draw uniform/instance slices, rewound by begin_frame at a submission boundary.
    arenas: arena::Arenas,
}
impl ChromeRenderer {
    /// A submission boundary: per-draw buffer slices may be reused from here on. Call it
    /// before encoding each submitted command buffer, never while one is being encoded.
    pub fn begin_frame(&mut self) {
        self.arenas.begin_frame();
    }
    /// Upload this frame's per-draw slices: call right before submitting the commands that use
    /// them (and before the next begin_frame, which debug-asserts that this happened).
    pub fn flush(&mut self) {
        self.arenas.flush(&self.context);
    }
    /// Read-only diagnostics for cache-capacity and long-lived-renderer gates.
    pub fn text_cache_stats(&self) -> TextCacheStats {
        TextCacheStats {
            mode: "glyph-components",
            whole_line_masks: 0,
            coverage: self.text.stats(),
        }
    }
    #[allow(
        clippy::disallowed_methods,
        reason = "headless/offline capture; the app UI thread uses with_context and bbt-rt capture"
    )]
    pub fn new(images: impl Into<icons::Assets>) -> Result<Self> {
        Self::with_context(pollster::block_on(DeviceContext::new())?, images)
    }
    pub fn with_context(context: DeviceContext, images: impl Into<icons::Assets>) -> Result<Self> {
        let mut icons = icons::IconAtlas::new(images)?;
        let icon_gpu = atlas_gpu::AtlasTexture::new(
            &context.device,
            &context.queue,
            icons.size,
            &icons.pixels,
            wgpu::TextureFormat::Rgba8Unorm,
            4,
            1,
        );
        let image_sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear chrome image atlas"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        icons.dirty.clear();
        Ok(Self {
            context,
            text: text_cache::Cache::new()?,
            icons,
            icon_gpu,
            glyph_gpu: None,
            glyph_reset_pending: false,
            image_sampler,
            pipelines: HashMap::new(),
            group_pipelines: HashMap::new(),
            group_depth: 0,
            paths: paths::Renderer::default(),
            material_pass: None,
            arenas: arena::Arenas::new("chrome draw"),
        })
    }
    pub fn draw(
        &mut self,
        list: &DrawList,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.draw_with_load(
            list,
            target,
            encoder,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        )
    }
    /// Paint an ordered foreground over a material pass on the same target/device.
    pub fn draw_overlay(
        &mut self,
        list: &DrawList,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.draw_with_load(list, target, encoder, wgpu::LoadOp::Load)
    }
    /// As draw_overlay onto an uncleared target: the first pass clears it to transparent
    /// (folded into that pass's load op when the list is flat), exactly what a separate clear
    /// pass followed by draw_overlay draws.
    pub fn draw_clearing(
        &mut self,
        list: &DrawList,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        self.draw_with_load(list, target, encoder, clear)
    }
    fn draw_with_load(
        &mut self,
        list: &DrawList,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        load: wgpu::LoadOp<wgpu::Color>,
    ) -> Result<()> {
        list.validate().map_err(RenderError::from)?;
        if !list
            .primitives
            .iter()
            .any(|p| matches!(p, Prim::Group { .. } | Prim::Path { .. }))
        {
            return self.draw_flat_with_load(list, target, encoder, load);
        }
        let ChromeTarget {
            width,
            height,
            scale,
            view,
            format,
        } = target;
        if width == 0 || height == 0 || !scale.is_finite() || !(0.5..=4.0).contains(&scale) {
            return Err("invalid chrome viewport".into());
        }
        if let wgpu::LoadOp::Clear(color) = load {
            clear(view, color, encoder);
        }
        let mut clips = Vec::new();
        let mut batch = DrawList::default();
        for primitive in &list.primitives {
            if let Prim::Group {
                primitives,
                opacity,
                saturation,
                mask,
            } = primitive
            {
                for _ in &clips {
                    batch.unclip();
                }
                self.draw_flat_with_load(
                    &batch,
                    ChromeTarget {
                        width,
                        height,
                        scale,
                        view,
                        format,
                    },
                    encoder,
                    wgpu::LoadOp::Load,
                )?;
                let mut children = DrawList {
                    primitives: clips.clone(),
                    ..Default::default()
                };
                children.primitives.extend(primitives.iter().cloned());
                for _ in &clips {
                    children.unclip();
                }
                let layer = group::texture(&self.context.device, width, height);
                let layer_view = layer.create_view(&Default::default());
                self.with_group_layer(|renderer| {
                    renderer.draw(
                        &children,
                        ChromeTarget {
                            width,
                            height,
                            scale,
                            view: &layer_view,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                        },
                        encoder,
                    )
                })?;
                self.composite_group_with_mask(
                    &layer_view,
                    group::Filter {
                        opacity: *opacity,
                        saturation: *saturation,
                    },
                    *mask,
                    ChromeTarget {
                        width,
                        height,
                        scale,
                        view,
                        format,
                    },
                    encoder,
                )?;
                batch = DrawList {
                    primitives: clips.clone(),
                    ..Default::default()
                };
            } else if matches!(primitive, Prim::Path { .. }) {
                for _ in &clips {
                    batch.unclip();
                }
                self.draw_flat_with_load(
                    &batch,
                    ChromeTarget {
                        width,
                        height,
                        scale,
                        view,
                        format,
                    },
                    encoder,
                    wgpu::LoadOp::Load,
                )?;
                self.paths.draw(
                    &self.context,
                    &mut self.arenas,
                    ChromeTarget {
                        width,
                        height,
                        scale,
                        view,
                        format,
                    },
                    encoder,
                    primitive,
                    &clips,
                )?;
                batch = DrawList {
                    primitives: clips.clone(),
                    ..Default::default()
                };
            } else {
                match primitive {
                    Prim::Clip { .. } => clips.push(primitive.clone()),
                    Prim::Unclip => {
                        clips.pop();
                    }
                    _ => {}
                }
                batch.push(primitive.clone());
            }
        }
        self.draw_flat_with_load(
            &batch,
            ChromeTarget {
                width,
                height,
                scale,
                view,
                format,
            },
            encoder,
            wgpu::LoadOp::Load,
        )
    }
    fn draw_flat_with_load(
        &mut self,
        list: &DrawList,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        load: wgpu::LoadOp<wgpu::Color>,
    ) -> Result<()> {
        let ChromeTarget {
            width,
            height,
            scale,
            view: target,
            format,
        } = target;
        list.validate().map_err(RenderError::from)?;
        if width == 0 || height == 0 || !scale.is_finite() || !(0.5..=4.0).contains(&scale) {
            return Err("invalid chrome viewport".into());
        }
        let mut instances = Vec::new();
        // UVs are resolved only after every glyph in this flat pass is pinned.
        // LRU repacking cannot invalidate an instance built earlier in the pass.
        let mut glyph_instances = Vec::new();
        let mut clips = vec![(
            Rect::new(0.0, 0.0, width as f32 / scale, height as f32 / scale),
            [0.0; 4],
        )];
        let physical_rect = |r: Rect| {
            let x = (r.x * scale).round();
            let y = (r.y * scale).round();
            [
                x,
                y,
                ((r.x + r.w) * scale).round() - x,
                ((r.y + r.h) * scale).round() - y,
            ]
        };
        let make = |r: Rect, c: Color, kind: f32, clip: (Rect, [f32; 4])| Instance {
            rect: if kind == 0. || kind == 1. || kind == 4. {
                physical_rect(r)
            } else {
                [r.x * scale, r.y * scale, r.w * scale, r.h * scale]
            },
            fill: c.0,
            params: [kind, 0.0, 0.0, 0.0],
            clip: [
                clip.0.x * scale,
                clip.0.y * scale,
                clip.0.w * scale,
                clip.0.h * scale,
            ],
            clip_radii: clip.1.map(|v| v * scale),
            ..Default::default()
        };
        for prim in &list.primitives {
            let clip = *clips.last().unwrap();
            match prim {
                Prim::Group { .. } | Prim::Path { .. } => {
                    unreachable!("groups and paths split before the instance pass")
                }
                Prim::Clip { rect, radii } => {
                    let x = rect.x.max(clip.0.x);
                    let y = rect.y.max(clip.0.y);
                    clips.push((
                        Rect::new(
                            x,
                            y,
                            (rect.right().min(clip.0.right()) - x).max(0.0),
                            (rect.bottom().min(clip.0.bottom()) - y).max(0.0),
                        ),
                        *radii,
                    ));
                }
                Prim::Unclip => {
                    clips.pop();
                }
                Prim::Rect {
                    rect,
                    radii,
                    fill,
                    stroke,
                    inner_highlight,
                    shadow,
                } => {
                    if let Some((offset, blur, color)) = shadow {
                        let mut item = make(
                            Rect::new(rect.x, rect.y + offset, rect.w, rect.h),
                            *color,
                            4.0,
                            clip,
                        );
                        item.radii = radii.map(|v| v * scale);
                        // Prim shadow carries the CSS blur radius; the kernel takes sigma.
                        item.params[2] = blur * scale * 0.5;
                        item.uv = physical_rect(*rect);
                        item.stroke = radii.map(|v| v * scale);
                        instances.push(item);
                    }
                    let mut item = make(*rect, *fill, 0.0, clip);
                    item.radii = radii.map(|v| v * scale);
                    if let Some((width, color)) = stroke {
                        item.params[1] = width * scale;
                        item.stroke = color.0;
                    }
                    if let Some(c) = inner_highlight {
                        item.params[3] = c.0[3];
                    }
                    instances.push(item);
                }
                Prim::Gloss { rect, radii, color } => {
                    let mut item = make(*rect, *color, 1.0, clip);
                    item.radii = radii.map(|v| v * scale);
                    item.params[3] = 0.18;
                    instances.push(item);
                }
                Prim::Hairline {
                    from,
                    to,
                    width,
                    color,
                } => {
                    let pad = width * 0.5;
                    let r = Rect::new(
                        from[0].min(to[0]) - pad,
                        from[1].min(to[1]) - pad,
                        (from[0] - to[0]).abs() + width,
                        (from[1] - to[1]).abs() + width,
                    );
                    let mut item = make(r, *color, 5.0, clip);
                    item.params[1] = width * scale;
                    item.uv = [
                        from[0] * scale,
                        from[1] * scale,
                        to[0] * scale,
                        to[1] * scale,
                    ];
                    instances.push(item);
                }
                Prim::Text {
                    origin,
                    text,
                    style,
                    max_width,
                    color,
                } => {
                    if text.is_empty() {
                        continue;
                    }
                    // Quantize each glyph once, after adding the line origin.
                    // Rounding the origin first loses eighth-pixel positions.
                    let physical_x = origin[0] * scale;
                    let phase = physical_x.rem_euclid(1.);
                    let line = self.text.line(text, *style, *max_width, scale, phase)?;
                    if line.advance <= 0.0 {
                        continue;
                    }
                    let left = physical_x.floor() - 2.;
                    let top = ((origin[1]
                        + (style.pixels() * 1.4 - line.height as f32 / scale) * 0.5)
                        * scale)
                        .round();
                    for part in line.parts {
                        // Preserve the old line canvas' clipping of overhangs.
                        let x0 = part.offset[0].max(0);
                        let y0 = part.offset[1].max(0);
                        let x1 = (part.offset[0] + part.size[0] as i32).min(line.width as i32);
                        let y1 = (part.offset[1] + part.size[1] as i32).min(line.height as i32);
                        let x = left + x0 as f32;
                        let y = top + y0 as f32;
                        if x0 >= x1
                            || y0 >= y1
                            || x >= clip.0.right() * scale
                            || y >= clip.0.bottom() * scale
                            || x + (x1 - x0) as f32 <= clip.0.x * scale
                            || y + (y1 - y0) as f32 <= clip.0.y * scale
                        {
                            continue;
                        }
                        let mut item = make(Rect::default(), *color, 2.0, clip);
                        item.rect = [x, y, (x1 - x0) as f32, (y1 - y0) as f32];
                        glyph_instances.push((
                            instances.len(),
                            part.id,
                            [(x0 - part.offset[0]) as f32, (y0 - part.offset[1]) as f32],
                        ));
                        instances.push(item);
                    }
                }
                Prim::Icon { rect, icon, color } => {
                    if rect.w == 0. || rect.h == 0. {
                        continue;
                    }
                    let mut item = make(*rect, *color, 3.0, clip);
                    item.uv = self.icons.raster(
                        icon.as_ref(),
                        [rect.w * scale, rect.h * scale],
                        scale,
                    )?;
                    if self.icons.is_svg(icon.as_ref()) {
                        // The Chrome 1x/2x device-origin probe snaps SVG origins
                        // in CSS pixels, before the device transform. Place the
                        // target-size texels once, without fractional filtering.
                        item.rect[0] = (rect.x + 0.5).floor() * scale;
                        item.rect[1] = (rect.y + 0.5).floor() * scale;
                        item.rect[2..].copy_from_slice(&item.uv[2..]);
                    }
                    instances.push(item);
                }
                Prim::Image {
                    rect,
                    image,
                    radius,
                } => {
                    let uv = self
                        .icons
                        .rects
                        .get(image.as_ref())
                        .ok_or_else(|| RenderError(format!("unbaked image: {image}")))?;
                    let mut item = make(*rect, Color::WHITE, 3.0, clip);
                    item.uv = *uv;
                    item.radii = [radius * scale; 4];
                    instances.push(item);
                }
            }
        }
        self.glyph_reset_pending |= self.text.finish(
            &glyph_instances
                .iter()
                .map(|(_, id, _)| *id)
                .collect::<Vec<_>>(),
        )?;
        for (index, id, offset) in glyph_instances {
            let rect = self.text.rect(id)?;
            let item = &mut instances[index];
            item.uv = [
                f32::from(rect.x) + offset[0],
                f32::from(rect.y) + offset[1],
                item.rect[2],
                item.rect[3],
            ];
        }
        if instances.is_empty() {
            if let wgpu::LoadOp::Clear(color) = load {
                clear(target, color, encoder);
            }
            return Ok(());
        }
        if instances.len() > 65_536 {
            return Err("chrome instance budget exceeded".into());
        }
        let (atlas, dirty) = self.text.upload();
        atlas_gpu::sync_glyphs(
            &mut self.glyph_gpu,
            &self.context.device,
            &self.context.queue,
            atlas,
            dirty,
            self.glyph_reset_pending,
        );
        self.glyph_reset_pending = false;
        for [x, y, width, height] in self.icons.dirty.drain(..) {
            let start = ((y * self.icons.size + x) * 4) as usize;
            let end = start + (((height - 1) * self.icons.size + width) * 4) as usize;
            self.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.icon_gpu.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &self.icons.pixels[start..end],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.icons.size * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            self.icon_gpu.writes += 1;
            self.icon_gpu.bytes += u64::from(width) * u64::from(height) * 4;
        }
        let device = &self.context.device;
        let gamma_format = format.remove_srgb_suffix();
        self.pipelines
            .entry(gamma_format)
            .or_insert_with(|| pipeline::create(device, gamma_format));
        let gamma_view = target.texture().create_view(&wgpu::TextureViewDescriptor {
            format: Some(gamma_format),
            ..Default::default()
        });
        let uniform = self.arenas.uniform(
            &self.context,
            bytemuck::cast_slice(&[width as f32, height as f32, scale, 0.0]),
        );
        let items = self
            .arenas
            .storage(&self.context, bytemuck::cast_slice(&instances));
        let mask = &self
            .glyph_gpu
            .as_ref()
            .expect("synchronized glyph atlas")
            .view;
        let icons = &self.icon_gpu.view;
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chrome bindings"),
            layout: &self.pipelines[&gamma_format].get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: items.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(mask),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(icons),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                },
            ],
        });
        // Chrome's grayscale CSS text, like its chrome fills, uses source-over
        // in sRGB. One UNORM pass also stores the right premultiplied values
        // inside an isolated group; no backdrop-dependent alpha remap is used.
        crate::passes::count(crate::passes::Kind::Flat);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("CSS gamma chrome and text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &gamma_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipelines[&gamma_format]);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..6, 0..instances.len() as u32);
        drop(pass);
        Ok(())
    }
    pub fn render_to_rgba(
        &mut self,
        list: &DrawList,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<Vec<u8>> {
        let format = wgpu::TextureFormat::Rgba8Unorm;
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("invalid chrome target size".into());
        }
        let texture = self
            .context
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("chrome headless"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[format.add_srgb_suffix()],
            });
        let view = texture.create_view(&Default::default());
        self.begin_frame();
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());
        self.draw(
            list,
            ChromeTarget {
                width,
                height,
                scale,
                view: &view,
                format,
            },
            &mut encoder,
        )?;
        self.flush();
        self.readback(&texture, encoder, width, height)
    }
    #[allow(
        clippy::disallowed_methods,
        reason = "headless/offline capture; the app UI thread uses with_context and bbt-rt capture"
    )]
    fn readback(
        &self,
        texture: &wgpu::Texture,
        mut encoder: wgpu::CommandEncoder,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chrome readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let index = self.context.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::sync_channel(1);
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(Duration::from_secs(20)),
            })
            .map_err(|e| RenderError(e.to_string()))?;
        rx.recv_timeout(Duration::from_secs(20))
            .map_err(|e| RenderError(e.to_string()))?
            .map_err(|e| RenderError(e.to_string()))?;
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError(e.to_string()))?;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks(stride as usize).take(height as usize) {
            rgba.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        buffer.unmap();
        Ok(rgba)
    }
}

fn clear(view: &wgpu::TextureView, color: wgpu::Color, encoder: &mut wgpu::CommandEncoder) {
    crate::passes::count(crate::passes::Kind::Clear);
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("chrome group clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}
fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
    pixels: &[u8],
    format: wgpu::TextureFormat,
    bpp: u32,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chrome atlas"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size * bpp),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
    );
    texture
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    #[should_panic(expected = "never flushed")]
    fn an_unflushed_frame_fails_at_the_next_begin_frame() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        renderer.begin_frame();
        let _slice = renderer.arenas.uniform(&renderer.context, &[0; 16]);
        renderer.begin_frame();
    }
    #[test]
    #[should_panic(expected = "never flushed")]
    fn an_unflushed_frame_fails_when_its_owner_drops() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        renderer.begin_frame();
        let _slice = renderer.arenas.storage(&renderer.context, &[0; 16]);
        drop(renderer);
    }
    #[test]
    fn a_flushed_frame_rewinds_cleanly() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        for _ in 0..3 {
            renderer.begin_frame();
            let _slice = renderer.arenas.uniform(&renderer.context, &[0; 16]);
            renderer.flush();
        }
    }
    #[test]
    fn arena_slices_match_per_draw_buffers_across_many_draws_and_mid_frame_growth() {
        use bbt_ui_model::draw::{PathCommand, PathStroke};
        let mut list = DrawList::default();
        for i in 0..400 {
            let (x, y) = ((i % 40) as f32 * 6., (i / 40) as f32 * 6.);
            let c = Color([
                (i % 7) as f32 / 7.,
                (i % 11) as f32 / 11.,
                (i % 13) as f32 / 13.,
                1.,
            ]);
            list.rect(Rect::new(x, y, 4., 4.), 1., c);
            // A path ends the flat batch, so every rect is its own draw: 800 draws, each
            // with its own uniform and instance (or clip) slice, in one submission.
            list.push(Prim::Path {
                origin: [x + 1., y + 1.],
                scale: 1.,
                points: std::sync::Arc::from([
                    PathCommand::Move([0., 0.]),
                    PathCommand::Line([3., 3.]),
                ]),
                stroke: PathStroke::default(),
                color: Color([c.0[2], c.0[0], c.0[1], 1.]),
                fill: None,
            });
        }
        let mut fallback = ChromeRenderer::new(vec![]).unwrap();
        fallback.arenas.fallback_only();
        let expected = fallback.render_to_rgba(&list, 240, 64, 1.).unwrap();
        assert_eq!(fallback.arenas.capacity(), [0, 0], "per-draw buffers only");
        let mut arenas = ChromeRenderer::new(vec![]).unwrap();
        assert_eq!(arenas.render_to_rgba(&list, 240, 64, 1.).unwrap(), expected);
        let grown = arenas.arenas.capacity();
        assert!(
            grown.iter().all(|&c| c > 64 * 1024),
            "the frame outgrew both first arenas mid-frame: {grown:?}"
        );
        // The next frame rewinds into the grown arenas and still matches.
        assert_eq!(arenas.render_to_rgba(&list, 240, 64, 1.).unwrap(), expected);
        assert_eq!(arenas.arenas.capacity(), grown);
    }
    #[test]
    fn svg_origins_follow_measured_css_half_up_ties_at_both_device_scales() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8.5" fill="white"/></svg>"#;
        let mut renderer =
            ChromeRenderer::new(icons::Assets::with_svgs(vec![], &[("circle", svg)])).unwrap();
        for scale in [1., 2.] {
            for (origin, snapped) in [
                (-1.5, -1.),
                (-0.5, 0.),
                (-0.25, 0.),
                (0., 0.),
                (0.25, 0.),
                (0.5, 1.),
                (0.75, 1.),
            ] {
                let draw = |x| {
                    let mut list = DrawList::default();
                    list.push(Prim::Icon {
                        rect: Rect::new(x, 2., 16., 16.),
                        icon: "circle".into(),
                        color: Color::WHITE,
                    });
                    list
                };
                let extent = (24. * scale) as u32;
                let actual = renderer
                    .render_to_rgba(&draw(origin), extent, extent, scale)
                    .unwrap();
                let expected = renderer
                    .render_to_rgba(&draw(snapped), extent, extent, scale)
                    .unwrap();
                assert_eq!(
                    actual, expected,
                    "CSS origin {origin}, device scale {scale}"
                );
            }
        }
    }
    #[test]
    fn svg_device_rasters_upload_once_and_survive_isolated_passes_and_scale_changes() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8.5" fill="white"/></svg>"#;
        let mut renderer =
            ChromeRenderer::new(icons::Assets::with_svgs(vec![], &[("circle", svg)])).unwrap();
        let mut list = DrawList::default();
        list.push(Prim::Group {
            primitives: vec![Prim::Icon {
                rect: Rect::new(2., 2., 14., 14.),
                icon: "circle".into(),
                color: Color::WHITE,
            }],
            opacity: 0.7,
            saturation: 0.4,
            mask: None,
        });
        let first = renderer.render_to_rgba(&list, 20, 20, 1.).unwrap();
        let writes = renderer.icon_gpu.writes;
        let bytes = renderer.icon_gpu.bytes;
        assert_eq!(writes, 2, "one initial texture and one 14px raster upload");
        assert_eq!(bytes, 2048 * 2048 * 4 + 14 * 14 * 4);
        assert_eq!(first, renderer.render_to_rgba(&list, 20, 20, 1.).unwrap());
        assert_eq!(renderer.icon_gpu.writes, writes);
        assert_eq!(renderer.icon_gpu.bytes, bytes);
        renderer.render_to_rgba(&list, 40, 40, 2.).unwrap();
        assert_eq!(renderer.icon_gpu.writes, writes + 1);
        assert_eq!(renderer.icon_gpu.bytes, bytes + 28 * 28 * 4);
        assert_eq!(first, renderer.render_to_rgba(&list, 20, 20, 1.).unwrap());
        assert_eq!(renderer.icon_gpu.writes, writes + 1);
    }
    #[test]
    fn immutable_icons_and_warm_glyphs_are_shared_across_isolated_passes() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let image = || icons::Image {
            name: "test".into(),
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 0, 255,
            ],
        };
        let mut cached = ChromeRenderer::new(vec![image()]).unwrap();
        let mut label = DrawList::default();
        label.text(
            Rect::new(4., 4., 140., 24.),
            "persistent atlas",
            TextStyle {
                scale: TypeScale::Custom(13.),
                ..Default::default()
            },
            Color::WHITE,
        );
        label.push(Prim::Image {
            rect: Rect::new(160., 4., 24., 24.),
            image: "test".into(),
            radius: 3.,
        });
        let mut list = DrawList::default();
        list.rect(
            Rect::new(0., 0., 220., 60.),
            0.,
            Color([0.1, 0.05, 0.2, 1.]),
        );
        list.push(Prim::Group {
            primitives: label.primitives,
            opacity: 0.7,
            saturation: 0.4,
            mask: None,
        });
        let first = cached.render_to_rgba(&list, 220, 60, 1.).unwrap();
        let writes = cached.glyph_gpu.as_ref().unwrap().writes;
        let bytes = cached.glyph_gpu.as_ref().unwrap().bytes;
        let second = cached.render_to_rgba(&list, 220, 60, 1.).unwrap();
        assert_eq!(first, second);
        assert_eq!(cached.icon_gpu.writes, 1);
        assert_eq!(cached.icon_gpu.bytes, 2048 * 2048 * 4);
        assert_eq!(cached.glyph_gpu.as_ref().unwrap().writes, writes);
        assert_eq!(cached.glyph_gpu.as_ref().unwrap().bytes, bytes);
        let mut fresh = ChromeRenderer::new(vec![image()]).unwrap();
        assert_eq!(second, fresh.render_to_rgba(&list, 220, 60, 1.).unwrap());
    }
    #[test]
    fn glyph_dirty_writes_and_growth_keep_earlier_texture_coordinates_correct() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut cached = ChromeRenderer::new(vec![]).unwrap();
        cached.text = text_cache::Cache::with_limits(32, 1024, 32 * 1024 * 1024, 32_768).unwrap();
        let style = TextStyle {
            scale: TypeScale::Custom(10.),
            ..Default::default()
        };
        let mut scene = DrawList::default();
        scene.rect(Rect::new(0., 0., 256., 80.), 0., Color::BLACK);
        scene.text(Rect::new(2., 2., 20., 20.), "A", style, Color::WHITE);
        let first = cached.render_to_rgba(&scene, 256, 80, 1.).unwrap();
        let generation = cached.glyph_gpu.as_ref().unwrap().generation;
        let bytes = cached.glyph_gpu.as_ref().unwrap().bytes;
        scene.text(Rect::new(26., 2., 20., 20.), "B", style, Color::WHITE);
        cached.render_to_rgba(&scene, 256, 80, 1.).unwrap();
        assert_eq!(cached.glyph_gpu.as_ref().unwrap().generation, generation);
        assert_eq!(cached.glyph_gpu.as_ref().unwrap().writes, 2);
        assert!(cached.glyph_gpu.as_ref().unwrap().bytes - bytes < 32 * 32);
        scene.text(
            Rect::new(2., 30., 250., 20.),
            "glyph coordinates survive a larger atlas",
            style,
            Color::WHITE,
        );
        let grown = cached.render_to_rgba(&scene, 256, 80, 1.).unwrap();
        assert_eq!(
            cached.glyph_gpu.as_ref().unwrap().generation,
            generation + 1
        );
        assert!(cached.text.upload().1.is_empty());
        for y in 0..22 {
            let at = y * 256 * 4;
            assert_eq!(&first[at..at + 22 * 4], &grown[at..at + 22 * 4]);
        }
        let mut fresh = ChromeRenderer::new(vec![]).unwrap();
        assert_eq!(grown, fresh.render_to_rgba(&scene, 256, 80, 1.).unwrap());
        let writes = cached.glyph_gpu.as_ref().unwrap().writes;
        assert_eq!(grown, cached.render_to_rgba(&scene, 256, 80, 1.).unwrap());
        assert_eq!(writes, cached.glyph_gpu.as_ref().unwrap().writes);
    }
    #[test]
    fn growth_in_a_later_group_preserves_the_earlier_encoded_texture() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut cached = ChromeRenderer::new(vec![]).unwrap();
        cached.text = text_cache::Cache::with_limits(32, 1024, 32 * 1024 * 1024, 32_768).unwrap();
        let style = TextStyle {
            scale: TypeScale::Custom(10.),
            ..Default::default()
        };
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 256., 80.), 0., Color::BLACK);
        list.text(Rect::new(2., 2., 20., 20.), "A", style, Color::WHITE);
        let mut later = DrawList::default();
        later.text(
            Rect::new(2., 30., 250., 20.),
            "later group grows the texture",
            style,
            Color::WHITE,
        );
        list.push(Prim::Group {
            primitives: later.primitives,
            opacity: 0.6,
            saturation: 1.,
            mask: None,
        });
        let pixels = cached.render_to_rgba(&list, 256, 80, 1.).unwrap();
        assert_eq!(cached.glyph_gpu.as_ref().unwrap().generation, 2);
        let mut fresh = ChromeRenderer::new(vec![]).unwrap();
        assert_eq!(pixels, fresh.render_to_rgba(&list, 256, 80, 1.).unwrap());
    }
    #[test]
    fn lru_repack_in_later_groups_preserves_prior_gpu_generations() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut cached = ChromeRenderer::new(vec![]).unwrap();
        cached.text = text_cache::Cache::with_limits(64, 64, 4096, 6).unwrap();
        let style = TextStyle {
            scale: TypeScale::Custom(10.),
            ..Default::default()
        };
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 128., 160.), 0., Color::BLACK);
        for (i, word) in ["ABCD", "EFGH", "IJKL", "MNOP", "QRST", "UVWX", "YZ01"]
            .iter()
            .enumerate()
        {
            let mut group = DrawList::default();
            group.text(
                Rect::new(2., 2. + i as f32 * 20., 120., 18.),
                *word,
                style,
                Color::WHITE,
            );
            list.push(Prim::Group {
                primitives: group.primitives,
                opacity: 0.6,
                saturation: 0.8,
                mask: None,
            });
        }
        let actual = cached.render_to_rgba(&list, 128, 160, 1.).unwrap();
        assert!(cached.text.stats().evictions > 0);
        assert!(cached.text.stats().rebuilds > 0);
        assert!(cached.glyph_gpu.as_ref().unwrap().generation > 1);
        let mut fresh = ChromeRenderer::new(vec![]).unwrap();
        assert_eq!(actual, fresh.render_to_rgba(&list, 128, 160, 1.).unwrap());
        assert_eq!(actual, cached.render_to_rgba(&list, 128, 160, 1.).unwrap());
    }
    #[test]
    fn empty_clipped_pass_retains_a_pending_gpu_reset_after_eviction() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        renderer.text = text_cache::Cache::with_limits(64, 64, 4096, 6).unwrap();
        let style = TextStyle {
            scale: TypeScale::Custom(10.),
            ..Default::default()
        };
        let mut visible = DrawList::default();
        visible.text(Rect::new(2., 2., 60., 18.), "ABCD", style, Color::WHITE);
        let expected = renderer.render_to_rgba(&visible, 64, 24, 1.).unwrap();
        let generation = renderer.glyph_gpu.as_ref().unwrap().generation;
        let mut hidden = DrawList::default();
        hidden.text(
            Rect::new(2., 40., 250., 18.),
            "EFGHIJKLMNOPQRSTUVWXYZ",
            style,
            Color::WHITE,
        );
        assert!(
            renderer
                .render_to_rgba(&hidden, 64, 24, 1.)
                .unwrap()
                .iter()
                .all(|v| *v == 0)
        );
        assert!(renderer.glyph_reset_pending);
        assert_eq!(generation, renderer.glyph_gpu.as_ref().unwrap().generation);
        assert_eq!(
            expected,
            renderer.render_to_rgba(&visible, 64, 24, 1.).unwrap()
        );
        assert_eq!(
            generation + 1,
            renderer.glyph_gpu.as_ref().unwrap().generation
        );
        assert!(!renderer.glyph_reset_pending);
    }
    #[test]
    fn colored_text_uses_css_source_over_with_opaque_and_transparent_backdrops() {
        use bbt_ui_model::draw::{TextStyle, TypeScale};
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let style = TextStyle {
            scale: TypeScale::Custom(13.),
            ..Default::default()
        };
        for origin_x in [2.125f32, 2.25] {
            let mask =
                text::rasterize_at("Hamburgefonts AV", style, None, 1., origin_x.fract()).unwrap();
            let foreground = Color([0.18, 0.42, 0.93, 0.8]);
            for background in [
                Color::WHITE,
                Color([0.1, 0.06, 0.2, 0.4]),
                Color::TRANSPARENT,
            ] {
                for opacity in [1., 0.6] {
                    let mut glyphs = DrawList::default();
                    glyphs.text(
                        Rect::new(
                            origin_x,
                            2. - (style.pixels() * 1.4 - mask.height as f32) * 0.5,
                            200.,
                            30.,
                        ),
                        "Hamburgefonts AV",
                        style,
                        foreground,
                    );
                    let mut list = DrawList::default();
                    list.rect(Rect::new(0., 0., 256., 48.), 0., background);
                    if opacity == 1. {
                        // Exercise the ordinary opaque-backdrop path as well as
                        // the isolated group below. Empty-layer text alone cannot
                        // expose a backdrop-dependent luminance correction.
                        list.primitives.extend(glyphs.primitives);
                    } else {
                        list.push(Prim::Group {
                            primitives: glyphs.primitives,
                            opacity,
                            saturation: 1.,
                            mask: None,
                        });
                    }
                    let pixels = renderer.render_to_rgba(&list, 256, 48, 1.).unwrap();
                    for (i, coverage) in mask.mask.iter().enumerate() {
                        let x = i % mask.width as usize;
                        let y = i / mask.width as usize + 2;
                        let a = f32::from(*coverage) / 255. * foreground.0[3] * opacity;
                        let at = (y * 256 + x) * 4;
                        for channel in 0..4 {
                            let expected = if channel == 3 {
                                a + background.0[3] * (1. - a)
                            } else {
                                foreground.0[channel] * a
                                    + background.0[channel] * background.0[3] * (1. - a)
                            };
                            assert!(
                                (f32::from(pixels[at + channel]) - expected * 255.).abs() <= 2.,
                                "CSS source-over at {x},{y}, channel {channel}: {} vs {}",
                                pixels[at + channel],
                                expected * 255.
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn sdf_rounding_stroke_and_clip_survive_gpu_readback() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut list = DrawList::default();
        list.rect(Rect::new(0.0, 0.0, 64.0, 64.0), 0.0, Color::BLACK);
        list.push(Prim::Rect {
            rect: Rect::new(8.0, 8.0, 40.0, 40.0),
            radii: [10.0; 4],
            fill: Color::hex("#FF0000"),
            stroke: Some((2.0, Color::hex("#0000FF"))),
            inner_highlight: None,
            shadow: None,
        });
        list.clip(Rect::new(52.0, 0.0, 4.0, 64.0), [0.0; 4]);
        list.rect(Rect::new(48.0, 0.0, 16.0, 64.0), 0.0, Color::WHITE);
        list.unclip();
        let image = renderer.render_to_rgba(&list, 64, 64, 1.0).unwrap();
        let pixel = |x: usize, y: usize| &image[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
        assert_eq!(pixel(8, 8), [0, 0, 0, 255]);
        assert_eq!(pixel(25, 25), [255, 0, 0, 255]);
        assert_eq!(pixel(8, 26), [0, 0, 255, 255]);
        assert_eq!(pixel(53, 26), [255, 255, 255, 255]);
        assert_eq!(pixel(59, 26), [0, 0, 0, 255]);
    }
}
