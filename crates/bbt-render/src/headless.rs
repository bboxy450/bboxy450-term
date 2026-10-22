//! Offscreen sRGB rendering and padded GPU buffer readback.
use crate::{
    Color, RenderError, Result, Scene,
    atlas::{AtlasRect, GlyphAtlas},
    cells::InstanceData,
    device::DeviceContext,
    text::Font,
};
use bytemuck::{Pod, Zeroable};
use std::{
    collections::HashMap,
    path::Path,
    sync::{OnceLock, mpsc},
    time::Duration,
};
use wgpu::util::DeviceExt;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const MAX_INSTANCES: usize = 65_536;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    screen: [f32; 2],
    cell: [f32; 2],
    grid: [u32; 2],
    padding: [u32; 2],
}

#[derive(Clone, Copy)]
struct PlacedGlyph {
    rect: AtlasRect,
    left: i16,
    top: i16,
}

pub struct HeadlessRenderer {
    context: DeviceContext,
    backgrounds: wgpu::RenderPipeline,
    text: wgpu::RenderPipeline,
}

impl HeadlessRenderer {
    #[allow(clippy::disallowed_methods, reason = "headless/offline capture; the app UI thread uses with_context and bbt-rt capture")]
    pub fn new() -> Result<Self> {
        let context = pollster::block_on(DeviceContext::new())?;
        let backgrounds = pipeline(
            &context.device,
            "cells.bg",
            include_str!("cells/bg.wgsl"),
            &[],
            None,
        );
        let text = pipeline(
            &context.device,
            "cells.text",
            include_str!("cells/text.wgsl"),
            &[Some(InstanceData::layout())],
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        Ok(Self {
            context,
            backgrounds,
            text,
        })
    }

    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.context.adapter_info
    }

    #[allow(clippy::disallowed_methods, reason = "headless/offline capture; the app UI thread uses with_context and bbt-rt capture")]
    pub fn render_to_rgba(&self, scene: &Scene, width: u32, height: u32) -> Result<Vec<u8>> {
        let context = &self.context;
        let limit = context.device.limits().max_texture_dimension_2d;
        if width == 0
            || height == 0
            || width > limit
            || height > limit
            || scene.columns == 0
            || scene.rows == 0
            || scene.cell_size.contains(&0)
        {
            return Err("invalid viewport/grid dimensions".into());
        }
        if scene.pixel_size() != [width, height] {
            return Err("viewport must match the grid's physical cell dimensions".into());
        }
        let count = usize::from(scene.columns) * usize::from(scene.rows);
        if scene.backgrounds.len() != count
            || count * 4 > context.device.limits().max_storage_buffer_binding_size as usize
        {
            return Err("background count exceeds grid or GPU storage limits".into());
        }
        let uniform = ViewUniform {
            screen: [width as f32, height as f32],
            cell: scene.cell_size.map(|v| v as f32),
            grid: [scene.columns.into(), scene.rows.into()],
            padding: [0; 2],
        };
        let view_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("scene view"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let colors: Vec<u32> = scene
            .backgrounds
            .iter()
            .map(|v| u32::from_le_bytes(*v))
            .collect();
        let colors = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cell backgrounds"),
                contents: bytemuck::cast_slice(&colors),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let bg_bind = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cells.bg bindings"),
                layout: &self.backgrounds.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: colors.as_entire_binding(),
                    },
                ],
            });
        let (atlas, instances) = prepare_text(scene, limit.min(32_768))?;
        let atlas_texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("grayscale glyph atlas"),
            size: wgpu::Extent3d {
                width: atlas.size(),
                height: atlas.size(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.size()),
                rows_per_image: Some(atlas.size()),
            },
            wgpu::Extent3d {
                width: atlas.size(),
                height: atlas.size(),
                depth_or_array_layers: 1,
            },
        );
        let atlas_view = atlas_texture.create_view(&Default::default());
        let text_bind = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cells.text bindings"),
                layout: &self.text.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                ],
            });
        let instance_buffer = (!instances.is_empty()).then(|| {
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("32-byte glyph instances"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless sRGB target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = texture.create_view(&Default::default());
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cells.bg"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.backgrounds);
            pass.set_bind_group(0, &bg_bind, &[]);
            pass.draw(0..6, 0..1);
        }
        if let Some(buffer) = &instance_buffer {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cells.text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.text);
            pass.set_bind_group(0, &text_bind, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..instances.len() as u32);
        }
        let row_bytes = width * 4;
        let padded_row_bytes = row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("headless padded readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let submission = context.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(20)),
            })
            .map_err(|e| RenderError(format!("GPU readback poll: {e}")))?;
        rx.recv_timeout(Duration::from_secs(20))
            .map_err(|e| RenderError(format!("GPU map callback: {e}")))?
            .map_err(|e| RenderError(format!("GPU map: {e}")))?;
        let mapped = readback
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError(format!("GPU mapped range: {e}")))?;
        let mut rgba = Vec::with_capacity((u64::from(row_bytes) * u64::from(height)) as usize);
        for row in mapped.chunks_exact(padded_row_bytes as usize) {
            rgba.extend_from_slice(&row[..row_bytes as usize]);
        }
        drop(mapped);
        readback.unmap();
        Ok(rgba)
    }
}

fn pipeline(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: FORMAT,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn prepare_text(scene: &Scene, limit: u32) -> Result<(GlyphAtlas, Vec<InstanceData>)> {
    let mut atlas = GlyphAtlas::new(64, limit)?;
    let mut instances = Vec::new();
    if scene.text.is_empty() {
        return Ok((atlas, instances));
    }
    let font = Font::new(&scene.font)?;
    let mut cache = HashMap::<char, Option<PlacedGlyph>>::new();
    for run in &scene.text {
        let end = usize::from(run.column) + run.text.chars().count();
        if run.row >= scene.rows || end > usize::from(scene.columns) {
            return Err("text run exceeds grid bounds".into());
        }
        for (offset, ch) in run.text.chars().enumerate() {
            if !cache.contains_key(&ch) {
                let glyph = match font.rasterize(ch)? {
                    Some(raster) => Some(PlacedGlyph {
                        rect: atlas.insert(raster.width, raster.height, &raster.mask)?,
                        left: raster.left,
                        top: raster.top,
                    }),
                    None => None,
                };
                cache.insert(ch, glyph);
            }
            if let Some(glyph) = cache[&ch] {
                if instances.len() == MAX_INSTANCES {
                    return Err("glyph batch exceeds 65536 instances".into());
                }
                let column = run.column + offset as u16;
                let rect = glyph.rect;
                let background: Color = scene.backgrounds
                    [usize::from(run.row) * usize::from(scene.columns) + usize::from(column)];
                instances.push(InstanceData {
                    cell: [column, run.row],
                    bearing: [glyph.left, font.metrics().baseline - glyph.top],
                    size: [rect.width as i16, rect.height as i16],
                    atlas: [rect.x, rect.y, rect.width, rect.height],
                    foreground: run.foreground,
                    background,
                    flags: 0,
                });
            }
        }
    }
    Ok((atlas, instances))
}

/// Convenience entry point sharing one headless device/pipeline set per process.
pub fn render_to_rgba(scene: &Scene, width: u32, height: u32) -> Result<Vec<u8>> {
    static RENDERER: OnceLock<Result<HeadlessRenderer>> = OnceLock::new();
    RENDERER
        .get_or_init(HeadlessRenderer::new)
        .as_ref()
        .map_err(Clone::clone)?
        .render_to_rgba(scene, width, height)
}

#[allow(clippy::disallowed_methods, reason = "headless/offline capture; the app UI thread uses with_context and bbt-rt capture")]
pub fn write_png(path: impl AsRef<Path>, rgba: &[u8], width: u32, height: u32) -> Result<()> {
    let expected = u64::from(width) * u64::from(height) * 4;
    if width == 0 || height == 0 || expected != rgba.len() as u64 {
        return Err("PNG dimensions do not match RGBA data".into());
    }
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RenderError(e.to_string()))?;
    }
    let file = std::fs::File::create(path).map_err(|e| RenderError(e.to_string()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder
        .write_header()
        .map_err(|e| RenderError(e.to_string()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| RenderError(e.to_string()))?;
    writer.finish().map_err(|e| RenderError(e.to_string()))
}
