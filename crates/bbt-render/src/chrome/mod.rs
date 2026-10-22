//! One ordered, instanced chrome pass on the caller's device; no window ownership.
pub mod diff;
pub mod group;
pub mod icons;
pub mod materials;
pub mod pipeline;
pub mod text;
use crate::{
    RenderError, Result,
    atlas::{AtlasRect, GlyphAtlas},
    device::DeviceContext,
};
use bbt_ui_model::{
    draw::{Color, DrawList, Prim},
    layout::Rect,
};
use pipeline::Instance;
use std::{collections::HashMap, sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

pub struct ChromeTarget<'a> {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

pub struct ChromeRenderer {
    pub context: DeviceContext,
    atlas: GlyphAtlas,
    text: HashMap<String, (AtlasRect, f32)>,
    icons: icons::IconAtlas,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    group_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    group_depth: usize,
}
impl ChromeRenderer {
    pub fn new(images: Vec<icons::Image>) -> Result<Self> {
        Self::with_context(pollster::block_on(DeviceContext::new())?, images)
    }
    pub fn with_context(context: DeviceContext, images: Vec<icons::Image>) -> Result<Self> {
        Ok(Self {
            context,
            atlas: GlyphAtlas::new(512, 8192)?,
            text: HashMap::new(),
            icons: icons::IconAtlas::new(images)?,
            pipelines: HashMap::new(),
            group_pipelines: HashMap::new(),
            group_depth: 0,
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
            .any(|p| matches!(p, Prim::Group { .. }))
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
                self.composite_group(
                    &layer_view,
                    group::Filter {
                        opacity: *opacity,
                        saturation: *saturation,
                    },
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
                Prim::Group { .. } => {
                    unreachable!("isolated groups split before the instance pass")
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
                    let key = format!("{text}\0{style:?}\0{max_width:?}\0{scale}\0{phase}");
                    let (atlas, advance) = if let Some(cached) = self.text.get(&key) {
                        *cached
                    } else {
                        let line = text::rasterize_at(text, *style, *max_width, scale, phase)?;
                        let rect = self.atlas.insert(line.width, line.height, &line.mask)?;
                        let value = (rect, line.advance);
                        self.text.insert(key, value);
                        value
                    };
                    if advance <= 0.0 {
                        continue;
                    }
                    let r = Rect::new(
                        (physical_x.floor() - 2.) / scale,
                        ((origin[1] + (style.pixels() * 1.4 - atlas.height as f32 / scale) * 0.5)
                            * scale)
                            .round()
                            / scale,
                        atlas.width as f32 / scale,
                        atlas.height as f32 / scale,
                    );
                    let mut item = make(r, *color, 2.0, clip);
                    item.uv = [
                        atlas.x as f32,
                        atlas.y as f32,
                        atlas.width as f32,
                        atlas.height as f32,
                    ];
                    instances.push(item);
                }
                Prim::Icon { rect, icon, color } => {
                    let uv = self
                        .icons
                        .rects
                        .get(icon.as_ref())
                        .ok_or_else(|| RenderError(format!("unbaked icon: {icon}")))?;
                    let mut item = make(*rect, *color, 3.0, clip);
                    item.uv = *uv;
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
        if instances.is_empty() {
            if let wgpu::LoadOp::Clear(color) = load {
                clear(target, color, encoder);
            }
            return Ok(());
        }
        if instances.len() > 65_536 {
            return Err("chrome instance budget exceeded".into());
        }
        let device = &self.context.device;
        let queue = &self.context.queue;
        let gamma_format = format.remove_srgb_suffix();
        self.pipelines
            .entry(gamma_format)
            .or_insert_with(|| pipeline::create(device, gamma_format));
        let gamma_view = target.texture().create_view(&wgpu::TextureViewDescriptor {
            format: Some(gamma_format),
            ..Default::default()
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chrome view"),
            contents: bytemuck::cast_slice(&[width as f32, height as f32, scale, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let items = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("retained chrome list"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let mask = upload(
            device,
            queue,
            self.atlas.size(),
            self.atlas.pixels(),
            wgpu::TextureFormat::R8Unorm,
            1,
        );
        let icons = upload(
            device,
            queue,
            self.icons.size,
            &self.icons.pixels,
            wgpu::TextureFormat::Rgba8Unorm,
            4,
        );
        let mask = mask.create_view(&Default::default());
        let icons = icons.create_view(&Default::default());
        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear chrome image atlas"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chrome bindings"),
            layout: &self.pipelines[&gamma_format].get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: items.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&mask),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&icons),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&image_sampler),
                },
            ],
        });
        // Chrome's grayscale CSS text, like its chrome fills, uses source-over
        // in sRGB. One UNORM pass also stores the right premultiplied values
        // inside an isolated group; no backdrop-dependent alpha remap is used.
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
        self.readback(&texture, encoder, width, height)
    }
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
