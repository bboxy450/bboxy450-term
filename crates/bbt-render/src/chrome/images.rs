// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pictures over terminal cells (Kitty graphics): one texture per image, kept
//! while something shows it, drawn as clipped textured quads. Separate from the
//! icon atlas, whose shelf never frees; a picture may be a whole screenshot.
use super::ChromeTarget;
use crate::Result;
use std::collections::HashMap;

/// Where one picture goes, in physical pixels of the target.
#[derive(Debug, Clone, PartialEq)]
pub struct Quad {
    pub key: String,
    pub rect: [f32; 4],
    pub clip: [f32; 4],
    /// Source rectangle in texture UV: u0, v0, u1, v1.
    pub uv: [f32; 4],
}

struct Picture {
    /// The view keeps its texture alive.
    view: wgpu::TextureView,
    frame: u64,
}

#[derive(Default)]
pub struct Layer {
    pictures: HashMap<String, Picture>,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    frame: u64,
}
impl Layer {
    pub fn has(&self, key: &str) -> bool {
        self.pictures.contains_key(key)
    }
    /// Straight-alpha RGBA, uploaded once per key.
    pub fn upload(
        &mut self,
        context: &crate::device::DeviceContext,
        key: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        if self.pictures.contains_key(key) {
            return Ok(());
        }
        let limit = context.device.limits().max_texture_dimension_2d;
        if width == 0
            || height == 0
            || width > limit
            || height > limit
            || rgba.len() != (width as usize) * (height as usize) * 4
        {
            return Err("invalid picture".into());
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terminal picture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        context.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        self.pictures.insert(
            key.to_owned(),
            Picture {
                view,
                frame: self.frame,
            },
        );
        Ok(())
    }
    /// Pictures not drawn for `keep` frames are freed.
    pub fn sweep(&mut self, keep: u64) {
        let frame = self.frame;
        self.pictures
            .retain(|_, p| frame.saturating_sub(p.frame) <= keep);
    }
    pub fn len(&self) -> usize {
        self.pictures.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pictures.is_empty()
    }
}

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("terminal picture"),
        source: wgpu::ShaderSource::Wgsl(include_str!("images.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terminal picture source-over"),
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

impl super::ChromeRenderer {
    /// The picture layer: upload before a frame, draw inside it.
    pub fn pictures(&mut self) -> &mut Layer {
        &mut self.images
    }
    /// Straight-alpha RGBA under `key`, uploaded once; later calls with the
    /// same key are free.
    pub fn upload_picture(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.images.upload(&self.context, key, width, height, rgba)
    }
    /// Draws the quads in order onto the target; a quad whose picture was
    /// never uploaded is skipped, not an error.
    pub fn draw_pictures(
        &mut self,
        quads: &[Quad],
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.images.frame = self.images.frame.wrapping_add(1);
        if quads.is_empty() {
            return Ok(());
        }
        let device = &self.context.device;
        let format = target.format;
        let pipeline = self
            .images
            .pipelines
            .entry(format)
            .or_insert_with(|| pipeline(device, format));
        let mut bindings = Vec::with_capacity(quads.len());
        for quad in quads {
            let Some(picture) = self.images.pictures.get_mut(&quad.key) else {
                continue;
            };
            picture.frame = self.images.frame;
            let mut values = [0_f32; 16];
            values[..4].copy_from_slice(&quad.rect);
            values[4..8].copy_from_slice(&quad.clip);
            values[8..12].copy_from_slice(&quad.uv);
            values[12] = target.width as f32;
            values[13] = target.height as f32;
            let params = self
                .arenas
                .uniform(&self.context, bytemuck::cast_slice(&values));
            bindings.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terminal picture bindings"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&picture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params.binding(),
                    },
                ],
            }));
        }
        if bindings.is_empty() {
            return Ok(());
        }
        crate::passes::count(crate::passes::Kind::Group);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terminal pictures"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        for binding in &bindings {
            pass.set_bind_group(0, binding, &[]);
            pass.draw(0..6, 0..1);
        }
        drop(pass);
        Ok(())
    }
}
