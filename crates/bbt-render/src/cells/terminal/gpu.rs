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
            bg: pipeline(
                &context.device,
                "cells.bg",
                include_str!("../bg.wgsl"),
                false,
            ),
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
                    metrics.baseline,
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
            for col in 0..grid.columns {
                let cell =
                    &grid.cells[usize::from(row) * usize::from(grid.columns) + usize::from(col)];
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
                    append(
                        self,
                        Key::Sprite(Sprite::Underline(cell.underline)),
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

    /// Offline GPU readback using the caller's shared device and a parser-free snapshot.
    #[allow(
        clippy::disallowed_methods,
        reason = "headless terminal capture explicitly waits for GPU readback; not a UI API"
    )]
    pub fn render_to_rgba(&mut self, context: &DeviceContext, grid: &Grid) -> Result<Image> {
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
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terminal cells.bg"),
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
            pass.set_pipeline(&self.bg);
            pass.set_bind_group(0, &bg_bind, &[]);
            pass.draw(0..6, 0..1);
        }
        if let Some(buffer) = &buffer {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terminal cells.text+deco"),
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
