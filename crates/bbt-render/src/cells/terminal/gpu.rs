use super::{
    Grid, Options,
    sprites::{self, Sprite},
};
use crate::{
    FontMetrics, RenderError, Result,
    atlas::{AtlasRect, GlyphAtlas},
    cells::InstanceData,
    device::DeviceContext,
    text::terminal::{Fonts, Format},
};
use bytemuck::{Pod, Zeroable};
use std::{
    collections::{BTreeSet, HashMap},
    sync::mpsc,
    time::Duration,
};
use wgpu::util::DeviceExt;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct View {
    screen: [f32; 2],
    cell: [f32; 2],
    grid: [u32; 2],
    padding: [u32; 2],
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Text(String, bool, bool, u8),
    Sprite(Sprite),
}
#[derive(Clone, Copy)]
struct Placed {
    rect: AtlasRect,
    bearing: [i16; 2],
    color: bool,
}
struct Uploaded {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: u32,
    version: u64,
}

pub struct Image {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// An immutable frame prepared off the UI thread. Encoding only records GPU
/// commands: no fonts, parser access, readback, polling, locks or CPU wait.
/// The target view must use Rgba8UnormSrgb on the preparing device.
pub struct PreparedGrid {
    pub width: u32,
    pub height: u32,
    bg: wgpu::RenderPipeline,
    text: wgpu::RenderPipeline,
    bg_bind: wgpu::BindGroup,
    text_bind: wgpu::BindGroup,
    buffer: Option<wgpu::Buffer>,
    instances: u32,
}

impl PreparedGrid {
    /// Draw at a device-pixel origin, clipped to the visible pane body. Existing
    /// pixels outside the clip are preserved. One frame may be reused in panes.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        origin: [u32; 2],
        clip: [u32; 4],
    ) {
        let size = target.texture().size();
        let x = clip[0].max(origin[0]);
        let y = clip[1].max(origin[1]);
        let right = clip[0]
            .saturating_add(clip[2])
            .min(origin[0].saturating_add(self.width))
            .min(size.width);
        let bottom = clip[1]
            .saturating_add(clip[3])
            .min(origin[1].saturating_add(self.height))
            .min(size.height);
        if right <= x || bottom <= y {
            return;
        }
        crate::passes::count(crate::passes::Kind::Cells);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terminal live cells"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_viewport(
            origin[0] as f32,
            origin[1] as f32,
            self.width as f32,
            self.height as f32,
            0.,
            1.,
        );
        pass.set_scissor_rect(x, y, right - x, bottom - y);
        pass.set_pipeline(&self.bg);
        pass.set_bind_group(0, &self.bg_bind, &[]);
        pass.draw(0..6, 0..1);
        if let Some(buffer) = &self.buffer {
            pass.set_pipeline(&self.text);
            pass.set_bind_group(0, &self.text_bind, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..self.instances);
        }
    }
}

/// Retained font/glyph/sprite cache and two atlases. No parser or app dependency.
pub struct TerminalRenderer {
    options: Options,
    fonts: Fonts,
    bg: wgpu::RenderPipeline,
    text: wgpu::RenderPipeline,
    gray: GlyphAtlas,
    color: GlyphAtlas,
    cache: HashMap<Key, Option<Placed>>,
    gray_version: u64,
    color_version: u64,
    gray_gpu: Option<Uploaded>,
    color_gpu: Option<Uploaded>,
    faces: BTreeSet<String>,
}

impl TerminalRenderer {
    pub fn new(context: &DeviceContext, options: Options) -> Result<Self> {
        let fonts = Fonts::new(&options.font, options.line_height, options.thicken)?;
        let limit = context.device.limits().max_texture_dimension_2d.min(32768);
        Ok(Self {
            options,
            fonts,
            bg: pipeline(&context.device, "cells.bg", include_str!("bg.wgsl"), false),
            text: pipeline(
                &context.device,
                "cells.text+deco",
                include_str!("terminal.wgsl"),
                true,
            ),
            gray: GlyphAtlas::new(64, limit)?,
            color: GlyphAtlas::new_color(64, limit)?,
            cache: HashMap::new(),
            gray_version: 1,
            color_version: 1,
            gray_gpu: None,
            color_gpu: None,
            faces: BTreeSet::new(),
        })
    }
    pub fn metrics(&self) -> FontMetrics {
        self.fonts.metrics()
    }
    pub fn cached_glyphs(&self) -> usize {
        self.cache.len()
    }
    pub fn effective_faces(&self) -> impl Iterator<Item = &str> {
        self.faces.iter().map(String::as_str)
    }

    fn glyph(&mut self, key: Key) -> Result<Option<Placed>> {
        if let Some(placed) = self.cache.get(&key) {
            return Ok(*placed);
        }
        let metrics = self.metrics();
        let placed = match &key {
            Key::Text(text, bold, italic, span) => {
                match self.fonts.rasterize(text, *bold, *italic, *span)? {
                    Some(glyph) => {
                        self.faces.extend(glyph.faces);
                        let color = glyph.format == Format::Bgra;
                        let rect = if color {
                            self.color_version += 1;
                            self.color
                                .insert(glyph.width, glyph.height, &glyph.pixels)?
                        } else {
                            self.gray_version += 1;
                            self.gray.insert(glyph.width, glyph.height, &glyph.pixels)?
                        };
                        Some(Placed {
                            rect,
                            bearing: [glyph.left, glyph.top],
                            color,
                        })
                    }
                    None => None,
                }
            }
            Key::Sprite(sprite) => {
                let mask = sprites::raster(
                    *sprite,
                    metrics.cell_width,
                    metrics.cell_height,
                    self.options.font.scale,
                );
                let rect = self
                    .gray
                    .insert(metrics.cell_width, metrics.cell_height, &mask)?;
                self.gray_version += 1;
                Some(Placed {
                    rect,
                    bearing: [0, 0],
                    color: false,
                })
            }
        };
        self.cache.insert(key, placed);
        Ok(placed)
    }

    fn instances(&mut self, grid: &Grid) -> Result<Vec<InstanceData>> {
        let mut instances = Vec::new();
        let mut append = |renderer: &mut Self,
                          key: Key,
                          col: u16,
                          row: u16,
                          fg: [u8; 4],
                          bg: [u8; 4],
                          sprite: bool|
         -> Result<()> {
            if let Some(placed) = renderer.glyph(key)? {
                let r = placed.rect;
                instances.push(InstanceData {
                    cell: [col, row],
                    bearing: placed.bearing,
                    size: [r.width as i16, r.height as i16],
                    atlas: [r.x, r.y, r.width, r.height],
                    foreground: fg,
                    background: bg,
                    flags: u32::from(placed.color) | if sprite { 2 } else { 0 },
                });
            }
            Ok(())
        };
        for row in 0..grid.rows {
            let mut joined_until = 0;
            let mut underline_start = 0;
            let mut previous_underline = None;
            for col in 0..grid.columns {
                let cell =
                    &grid.cells[usize::from(row) * usize::from(grid.columns) + usize::from(col)];
                let underline = (cell.underline, cell.underline_color);
                if previous_underline != Some(underline) {
                    underline_start = col;
                    previous_underline = Some(underline);
                }
                let mut fg = cell.foreground;
                let mut bg = cell.background;
                if let Some(cursor) = grid.cursor
                    && cursor.focused
                    && cursor.shape == super::CursorShape::Block
                    && cursor.row == row
                    && cursor.column == col
                {
                    fg = cell.background;
                    bg = cursor.color;
                }
                if cell.dim {
                    fg[3] /= 2;
                }
                if cell.width > 0 && !cell.hidden && col >= joined_until {
                    let (text, consumed) = emoji_cluster(grid, row, col);
                    joined_until = col + consumed;
                    let mut chars = text.chars();
                    let first = chars.next();
                    let sprite = first.is_some_and(sprites::character) && chars.next().is_none();
                    let key = if sprite {
                        Key::Sprite(Sprite::Character(first.unwrap()))
                    } else {
                        // Unicode 11 assigns the white-flag base one cell, but
                        // the joined rainbow-flag glyph occupies two cells.
                        let span = if consumed > u16::from(cell.width) {
                            2
                        } else {
                            cell.width
                        };
                        Key::Text(text, cell.bold, cell.italic, span)
                    };
                    append(self, key, col, row, fg, bg, sprite)?;
                }
                if cell.underline != super::Underline::None {
                    let width = self.metrics().cell_width;
                    append(
                        self,
                        Key::Sprite(Sprite::underline(
                            cell.underline,
                            u32::from(col - underline_start) * width,
                            width,
                            self.options.font.scale,
                        )),
                        col,
                        row,
                        cell.underline_color,
                        bg,
                        true,
                    )?;
                }
                if cell.strike {
                    append(self, Key::Sprite(Sprite::Strike), col, row, fg, bg, true)?;
                }
            }
        }
        if let Some(cursor) = grid.cursor
            && !(cursor.focused && cursor.shape == super::CursorShape::Block)
        {
            let bg = grid.cells
                [usize::from(cursor.row) * usize::from(grid.columns) + usize::from(cursor.column)]
            .background;
            append(
                self,
                Key::Sprite(Sprite::Cursor(cursor.shape, cursor.focused)),
                cursor.column,
                cursor.row,
                cursor.color,
                bg,
                true,
            )?;
        }
        Ok(instances)
    }

    /// Rasterize and upload changed glyphs on the preparing worker. The returned
    /// immutable frame retains its atlas views across later growth/re-preparation.
    pub fn prepare(&mut self, context: &DeviceContext, grid: &Grid) -> Result<PreparedGrid> {
        grid.validate()?;
        let m = self.metrics();
        let width = u32::from(grid.columns) * m.cell_width;
        let height = u32::from(grid.rows) * m.cell_height;
        let limit = context.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err("terminal viewport exceeds GPU texture limit".into());
        }
        let instances = self.instances(grid)?;
        upload(
            context,
            &self.gray,
            self.gray_version,
            &mut self.gray_gpu,
            false,
        );
        upload(
            context,
            &self.color,
            self.color_version,
            &mut self.color_gpu,
            true,
        );
        let view = View {
            screen: [width as f32, height as f32],
            cell: [m.cell_width as f32, m.cell_height as f32],
            grid: [grid.columns.into(), grid.rows.into()],
            padding: [0; 2],
        };
        let uniform = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terminal view"),
                contents: bytemuck::bytes_of(&view),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let colors: Vec<u32> = grid
            .cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let cursor = grid.cursor.filter(|c| {
                    c.focused
                        && c.shape == super::CursorShape::Block
                        && usize::from(c.row) * usize::from(grid.columns) + usize::from(c.column)
                            == i
                });
                u32::from_le_bytes(cursor.map_or(cell.background, |c| c.color))
            })
            .collect();
        let backgrounds = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terminal backgrounds"),
                contents: bytemuck::cast_slice(&colors),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let bg_bind = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terminal bg bind"),
                layout: &self.bg.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: backgrounds.as_entire_binding(),
                    },
                ],
            });
        let text_bind = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terminal text bind"),
                layout: &self.text.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &self.gray_gpu.as_ref().unwrap().view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &self.color_gpu.as_ref().unwrap().view,
                        ),
                    },
                ],
            });
        let buffer = (!instances.is_empty()).then(|| {
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terminal instances"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        Ok(PreparedGrid {
            width,
            height,
            bg: self.bg.clone(),
            text: self.text.clone(),
            bg_bind,
            text_bind,
            buffer,
            instances: instances.len() as u32,
        })
    }

    /// Offline readback wrapper around the same prepare/encode path as the app.
    #[allow(
        clippy::disallowed_methods,
        reason = "headless terminal capture explicitly waits for GPU readback; not a UI API"
    )]
    pub fn render_to_rgba(&mut self, context: &DeviceContext, grid: &Grid) -> Result<Image> {
        let prepared = self.prepare(context, grid)?;
        let (width, height) = (prepared.width, prepared.height);
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terminal headless target"),
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
                label: Some("terminal frame"),
            });
        prepared.encode(&mut encoder, &target, [0, 0], [0, 0, width, height]);
        let stride = (width * 4).div_ceil(256) * 256;
        let readback = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal readback"),
            size: u64::from(stride) * u64::from(height),
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
        let submission = context.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::sync_channel(1);
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(20)),
            })
            .map_err(|e| RenderError(e.to_string()))?;
        rx.recv_timeout(Duration::from_secs(20))
            .map_err(|e| RenderError(e.to_string()))?
            .map_err(|e| RenderError(e.to_string()))?;
        let mapped = readback
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError(e.to_string()))?;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks_exact(stride as usize) {
            rgba.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        readback.unmap();
        Ok(Image {
            rgba,
            width,
            height,
        })
    }
}

/// Join only explicit ZWJ/skin-tone sequences, preserving parser cell positions.
/// Ordinary programming characters remain separate, with ligatures off.
fn emoji_cluster(grid: &Grid, row: u16, col: u16) -> (String, u16) {
    let start = usize::from(row) * usize::from(grid.columns);
    let cell = &grid.cells[start + usize::from(col)];
    let mut text = cell.text.clone();
    let mut consumed = u16::from(cell.width.max(1));
    while col + consumed < grid.columns {
        let next = &grid.cells[start + usize::from(col + consumed)];
        let modifier = next
            .text
            .chars()
            .next()
            .is_some_and(|c| (0x1f3fb..=0x1f3ff).contains(&(c as u32)));
        if !(text.ends_with('\u{200d}') || modifier)
            || next.width == 0
            || next.bold != cell.bold
            || next.italic != cell.italic
            || next.foreground != cell.foreground
            || next.background != cell.background
            || next.dim != cell.dim
            || next.hidden
        {
            break;
        }
        text.push_str(&next.text);
        consumed += u16::from(next.width);
    }
    (text, consumed)
}

fn upload(
    context: &DeviceContext,
    atlas: &GlyphAtlas,
    version: u64,
    uploaded: &mut Option<Uploaded>,
    color: bool,
) {
    if uploaded.as_ref().is_none_or(|u| u.size != atlas.size()) {
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(if color {
                "terminal BGRA atlas"
            } else {
                "terminal mask atlas"
            }),
            size: wgpu::Extent3d {
                width: atlas.size(),
                height: atlas.size(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if color {
                wgpu::TextureFormat::Bgra8Unorm
            } else {
                wgpu::TextureFormat::R8Unorm
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        *uploaded = Some(Uploaded {
            texture,
            view,
            size: atlas.size(),
            version: 0,
        });
    }
    let u = uploaded.as_mut().unwrap();
    if u.version == version {
        return;
    }
    context.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &u.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        atlas.pixels(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas.size() * if color { 4 } else { 1 }),
            rows_per_image: Some(atlas.size()),
        },
        wgpu::Extent3d {
            width: atlas.size(),
            height: atlas.size(),
            depth_or_array_layers: 1,
        },
    );
    u.version = version;
}
fn pipeline(device: &wgpu::Device, label: &str, source: &str, text: bool) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let instance_layout = [Some(InstanceData::layout())];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: if text { &instance_layout } else { &[] },
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: FORMAT,
                blend: if text {
                    Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                } else {
                    None
                },
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::terminal::Cell;

    // keeps: spec renderer §1/§1b/§4.2 — terminal placement, atlas generations and immutable native fonts
    #[test]
    #[cfg(target_os = "macos")]
    #[allow(
        clippy::disallowed_methods,
        reason = "offline GPU test readback, not the UI thread"
    )]
    fn prepared_frame_encodes_pane_offsets_clips_and_survives_atlas_growth() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<PreparedGrid>();
        fn send<T: Send>() {}
        send::<TerminalRenderer>();
        let context = pollster::block_on(DeviceContext::new()).unwrap();
        let mut renderer = TerminalRenderer::new(&context, Options::default()).unwrap();
        let grid = Grid {
            columns: 2,
            rows: 1,
            cells: vec![
                Cell {
                    text: "F".into(),
                    background: [90, 20, 30, 255],
                    ..Default::default()
                },
                Cell {
                    text: "g".into(),
                    background: [20, 90, 30, 255],
                    ..Default::default()
                },
            ],
            cursor: None,
        };
        let reference = renderer.render_to_rgba(&context, &grid).unwrap();
        let frame = renderer.prepare(&context, &grid).unwrap();
        let old_size = renderer.gray.size();
        let growth = Grid {
            columns: 95,
            rows: 1,
            cells: (' '..='~')
                .map(|c| Cell {
                    text: c.to_string(),
                    ..Default::default()
                })
                .collect(),
            cursor: None,
        };
        renderer.prepare(&context, &growth).unwrap();
        assert!(renderer.gray.size() > old_size);
        let (width, height) = (80, 48);
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("live pane clip test"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let blue: Vec<u8> = [0, 0, 255, 255].repeat((width * height) as usize);
        context.queue.write_texture(
            texture.as_image_copy(),
            &blue,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            texture.size(),
        );
        let mut encoder = context.device.create_command_encoder(&Default::default());
        let target = texture.create_view(&Default::default());
        frame.encode(&mut encoder, &target, [8, 4], [9, 5, 10, 12]);
        // The same immutable frame can draw a second pane without a uniform write.
        frame.encode(
            &mut encoder,
            &target,
            [40, 24],
            [40, 24, frame.width, frame.height],
        );
        // A resize can leave the previous grid larger than the target for one
        // frame. Clip to the attachment as well as the pane, without waiting.
        frame.encode(&mut encoder, &target, [75, 40], [0, 0, width, height]);
        let stride = (width * 4).div_ceil(256) * 256;
        let readback = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(stride * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            texture.size(),
        );
        let submission = context.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::sync_channel(1);
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(120)),
            })
            .unwrap();
        rx.recv_timeout(Duration::from_secs(120)).unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        for y in 0..height {
            for x in 0..width {
                let local = if (9..19).contains(&x) && (5..17).contains(&y) {
                    Some((x - 8, y - 4))
                } else if (40..40 + frame.width).contains(&x)
                    && (24..24 + frame.height).contains(&y)
                {
                    Some((x - 40, y - 24))
                } else if x >= 75 && y >= 40 {
                    Some((x - 75, y - 40))
                } else {
                    None
                };
                let expected = local.map_or(&blue[..4], |(x, y)| {
                    let i = ((y * frame.width + x) * 4) as usize;
                    &reference.rgba[i..i + 4]
                });
                let i = (y * stride + x * 4) as usize;
                assert_eq!(&mapped[i..i + 4], expected, "pane pixel {x},{y}");
            }
        }
    }

    // keeps: spec renderer §1/§1b/§4.2 — terminal placement, atlas generations and immutable native fonts
    #[test]
    #[cfg(target_os = "macos")]
    fn immutable_fonts_move_to_worker_with_identical_rasterization() {
        let cases = [("F", 1), ("e\u{301}", 1), ("中", 2), ("👩\u{200d}💻", 2)];
        for scale in [1, 2] {
            let fonts = Fonts::new(
                &crate::FontConfig {
                    scale,
                    ..Default::default()
                },
                None,
                false,
            )
            .unwrap();
            let capture = move |fonts: &Fonts| {
                let mut images = Vec::new();
                for (text, span) in cases {
                    for (bold, italic) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        let glyph = fonts.rasterize(text, bold, italic, span).unwrap().unwrap();
                        images.push((
                            glyph.width,
                            glyph.height,
                            glyph.left,
                            glyph.top,
                            glyph.format,
                            glyph.pixels,
                            glyph.faces,
                        ));
                    }
                }
                images
            };
            let caller = capture(&fonts);
            let worker = std::thread::spawn(move || capture(&fonts)).join().unwrap();
            assert_eq!(
                caller, worker,
                "font rasterization changed after moving threads at {scale}x"
            );
        }
    }

    // keeps: spec renderer §1/§1b/§4.2 — terminal placement, atlas generations and immutable native fonts
    #[test]
    fn emoji_joining_preserves_parser_positions_and_style_boundaries() {
        let mut grid = Grid {
            columns: 4,
            rows: 1,
            cursor: None,
            cells: vec![
                Cell {
                    text: "🏳️\u{200d}".into(),
                    ..Default::default()
                },
                Cell {
                    text: "🌈".into(),
                    width: 2,
                    ..Default::default()
                },
                Cell {
                    text: String::new(),
                    width: 0,
                    ..Default::default()
                },
                Cell {
                    text: "x".into(),
                    ..Default::default()
                },
            ],
        };
        assert_eq!(emoji_cluster(&grid, 0, 0), ("🏳️\u{200d}🌈".into(), 3));
        assert_eq!(emoji_cluster(&grid, 0, 3), ("x".into(), 1));
        grid.cells[1].bold = true;
        assert_eq!(emoji_cluster(&grid, 0, 0), ("🏳️\u{200d}".into(), 1));
        grid.cells[0].text = "f".into();
        grid.cells[1].text = "i".into();
        grid.cells[1].bold = false;
        assert_eq!(emoji_cluster(&grid, 0, 0), ("f".into(), 1));
    }
}
