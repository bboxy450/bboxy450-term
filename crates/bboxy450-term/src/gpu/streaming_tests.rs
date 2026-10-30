// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Synthetic Codex-style streaming through the real parser, wire replicas,
//! pane preparer, retained cell textures and a privately presented Metal drawable.
//! The layer never attaches to a window; physical scanout is not observed.
//!
//! State starts with an 80x24 empty Term, sequence zero, one fixed View and empty
//! stream/replica/preparer/texture caches. Atomic test actions are resize, parse
//! one bounded chunk, offer/apply/ACK a snapshot, prepare, encode/submit/present.
//! Six finite sizes include A -> B -> A. All chunks eventually arrive; the Metal
//! queue is FIFO and completes. Safety checks exact replica cells/dimensions,
//! prepared-source identity and source -> copy -> final right-edge pixels. GPU
//! completions are delayed until the end; no map/poll/readback wait can heal an
//! intermediate frame. Blank source bands during a resize are not called ink
//! loss: completed batches must contain the deliberately filled right edge.
//! This is a real API/GPU witness, not a TLA refinement proof. Native display-link
//! delivery, physical scanout, process recovery and actual Codex timing remain
//! outside this deterministic protocol fixture.
#![allow(
    clippy::disallowed_methods,
    reason = "offline GPU readback after the entire submitted stream, never an application UI turn"
)]

use super::*;
use bboxy450_term::grids::Grids;
use bbt_mux::{Ulid, streams::ViewStream};
use bbt_proto::{Snapshot, View, registry::SeatRecord};
use objc2_metal::MTLPixelFormat;
use objc2_quartz_core::CAMetalLayer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    None,
    Grid,
    Glyph,
    Copy,
    Final,
}

struct Bands {
    frame: usize,
    dimensions: [usize; 2],
    width: u32,
    height: u32,
    stride: u32,
    buffer: wgpu::Buffer,
    cell_pixels: [u32; 2],
    expected_ink: Vec<bool>,
}

// Adapted from bboxy450-term src/gpu.rs::Gpu::acquire (texture import).
// Inspired by the private-layer diagnostic in a4c0d8d1 (unattached layer).
// Test-only COPY_SRC/DST; no prior observation/lifecycle machinery is reused.
// Origin: https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn drawable_target(
    context: &DeviceContext,
    layer: &CAMetalLayer,
    size: [u32; 2],
) -> (wgpu::Texture, Drawable) {
    let mut extent = layer.drawableSize();
    extent.width = f64::from(size[0]);
    extent.height = f64::from(size[1]);
    layer.setDrawableSize(extent);
    let drawable = layer
        .nextDrawable()
        .expect("private Metal layer must supply a drawable unattended");
    let raw = drawable.texture();
    assert_eq!(
        [raw.width(), raw.height()],
        [size[0] as usize, size[1] as usize]
    );
    let size = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };
    // SAFETY: the layer uses this exact device; its checked extent, compatible
    // BGRA format and framebufferOnly=false match the imported descriptor.
    let texture = unsafe {
        let hal = wgpu::hal::metal::Device::texture_from_raw(
            raw,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            MTLTextureType::Type2D,
            1,
            1,
            size.into(),
            None,
        );
        context
            .device
            .create_texture_from_hal::<wgpu::hal::api::Metal>(
                hal,
                &wgpu::TextureDescriptor {
                    label: Some("private streaming final drawable"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[wgpu::TextureFormat::Bgra8Unorm],
                },
                wgpu::TextureUses::UNINITIALIZED,
            )
    };
    (texture, drawable)
}

// Independently generated protocol fixture, not a captured/private Codex session.
// A row erases itself, rewrites styled text and always ends in eight ink cells.
fn batch(cols: usize, rows: usize, tick: usize) -> Vec<u8> {
    let mut bytes = b"\x1b[?2026h\x1b[?25l\x1b[?7l".to_vec();
    for row in 0..rows {
        let prefix = format!("stream {tick:03} row {row:02} ");
        bytes.extend_from_slice(
            format!("\x1b[{};1H\x1b[2K\x1b[{}m", row + 1, 31 + tick % 7).as_bytes(),
        );
        bytes.extend_from_slice(prefix.as_bytes());
        bytes.extend(std::iter::repeat_n(b'.', cols - prefix.len() - 8));
        bytes.extend_from_slice(b"\x1b[0mWWWWWWWW");
    }
    bytes.extend_from_slice(b"\x1b[?2026l");
    bytes
}

fn copy_band(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    origin: [u32; 2],
    buffer: &wgpu::Buffer,
    offset: u64,
    size: [u32; 2],
    stride: u32,
) {
    let mut source = texture.as_image_copy();
    source.origin = wgpu::Origin3d {
        x: origin[0],
        y: origin[1],
        z: 0,
    };
    encoder.copy_texture_to_buffer(
        source,
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(stride),
                rows_per_image: Some(size[1]),
            },
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
}

fn check_bands(bands: &Bands, bytes: &[u8]) -> Result<usize, String> {
    let section = (bands.stride * bands.height) as usize;
    let mut ink = 0;
    // An independent glyph oracle is necessary: comparing two images encoded
    // from the same broken PreparedGrid would otherwise accept missing glyphs.
    for (cell, expected) in bands.expected_ink.iter().enumerate() {
        if !expected {
            continue;
        }
        let [cw, ch] = bands.cell_pixels.map(|v| v as usize);
        let [left, top] = [(cell % 8) * cw, (cell / 8) * ch];
        let visible = (top..top + ch).any(|y| {
            (left..left + cw).any(|x| {
                let offset = y * bands.stride as usize + x * 4;
                bytes[offset] > 160 && bytes[offset + 1] > 160 && bytes[offset + 2] > 160
            })
        });
        if !visible {
            return Err(format!(
                "glyph strip: frame={} grid={:?} cell={},{}",
                bands.frame,
                bands.dimensions,
                cell % 8,
                cell / 8
            ));
        }
    }
    for y in 0..bands.height as usize {
        for x in 0..bands.width as usize {
            let offset = y * bands.stride as usize + x * 4;
            let source = &bytes[offset..offset + 4];
            let composition = &bytes[section + offset..section + offset + 4];
            let bgra = &bytes[2 * section + offset..2 * section + offset + 4];
            let final_pixel = &[bgra[2], bgra[1], bgra[0], bgra[3]];
            if source != composition {
                return Err(format!(
                    "copy strip: frame={} grid={:?} pixel={x},{y} source={source:?} composition={composition:?}",
                    bands.frame, bands.dimensions
                ));
            }
            if composition != final_pixel {
                return Err(format!(
                    "final strip: frame={} grid={:?} pixel={x},{y} composition={composition:?} final={final_pixel:?}",
                    bands.frame, bands.dimensions
                ));
            }
            // The fixture uses an opaque dark background and light W glyphs.
            ink += usize::from(source[0] > 160 && source[1] > 160 && source[2] > 160);
        }
    }
    Ok(ink)
}

fn replay(control: Control, scale: u32) -> Result<(usize, usize), String> {
    let mut renderer = ChromeRenderer::new(crate::assets::images().unwrap()).unwrap();
    let context = Arc::new(DeviceContext {
        device: renderer.context.device.clone(),
        queue: renderer.context.queue.clone(),
        adapter_info: renderer.context.adapter_info.clone(),
    });
    let layer = CAMetalLayer::new();
    {
        // SAFETY: Metal remains the backend/device owning this private layer.
        let metal = unsafe { context.device.as_hal::<wgpu::hal::api::Metal>() }.unwrap();
        layer.setDevice(Some(metal.raw_device()));
    }
    layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm_sRGB);
    layer.setFramebufferOnly(false);
    layer.setAllowsNextDrawableTimeout(true);
    let mut preparer = model::PanePreparer::with_options(
        context.clone(),
        bbt_render::cells::terminal::Options {
            font: bbt_render::FontConfig {
                scale,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let mut cache = CellTextures::default();
    let mut model = Model::default();
    model.ui.window.theme = Theme::Dark;
    let seat = SeatId::from_u128(72);
    let record = SeatRecord::new(Ulid(72), Ulid(1), "stream", "codex");
    let view = View {
        seat: Ulid(72),
        generation: record.generation,
        view_generation: Ulid::new(),
    };
    let mut grids = Grids::new();
    grids.set_watched(&[(seat, view)]);
    let mut stream = ViewStream::default();
    let mut term = bbt_core::Term::new_with_spare_rows(80, 24, 0, 0);
    let mut sequence = 0;
    let mut captures = Vec::new();
    let mut ink_frames = 0;

    // Includes odd column counts and A -> B -> A, with repeated unchanged
    // frames exercising the retained texture cache after every completed batch.
    for (epoch, (cols, rows)) in [(80, 24), (121, 27), (69, 17), (80, 24), (167, 31), (95, 23)]
        .into_iter()
        .enumerate()
    {
        term.resize(cols, rows);
        for tick in 0..3 {
            let bytes = batch(cols, rows, epoch * 3 + tick);
            let mut offset = 0;
            let mut chunk = 0;
            while offset < bytes.len() {
                let end = bytes.len().min(offset + [1, 7, 31, 103, 4096][chunk % 5]);
                term.advance(&bytes[offset..end]);
                offset = end;
                chunk += 1;
                // Mirror Logic::snapshot: read the actual parser grid after
                // every chunk, including unfinished sync/escape sequences.
                // Do not add a publication fence that the product lacks.
                if epoch == 2 && tick == 1 && chunk == 3 {
                    assert!(term.sync_pending());
                    term.resize(cols + 7, rows);
                    term.resize(cols, rows);
                }
                assert!(captures.len() < 256, "finite fixture frame bound");
                sequence += 1;
                let expected = Screen::from_term(&term, String::new(), None, None);
                let mut offered = expected.clone();
                if control == Control::Grid && captures.len() == 10 {
                    for row in &mut offered.cells {
                        for cell in &mut row[cols - 8..] {
                            cell.c = ' ';
                        }
                    }
                }
                stream
                    .offer(Snapshot {
                        view,
                        sequence,
                        screen: offered,
                    })
                    .unwrap();
                grids.apply(seat, &stream.next_update().unwrap()).unwrap();
                if grids.screen(seat).unwrap().as_ref() != &expected {
                    return Err(format!(
                        "grid strip: frame={} grid={cols}x{rows}",
                        captures.len()
                    ));
                }
                stream.applied(view, sequence).unwrap();
                model = model::build(std::slice::from_ref(&record), &grids, &model);
                preparer.prepare(&mut model).unwrap();
                let frame = &model.frames[&seat];
                let metrics = model.terminal_cell_pixels.unwrap();
                assert_eq!(
                    [frame.width, frame.height],
                    [cols as u32 * metrics[0], rows as u32 * metrics[1]]
                );
                assert!(Arc::ptr_eq(
                    &model.prepared_screens[&seat].screen,
                    &model.screens[&seat]
                ));
                let [w, h] = [frame.width, frame.height];
                let [x, y] = [18, 10];
                let source = texture(&context.device, [w, h]);
                let target = texture(&context.device, [w + 48, h + 24]);
                let (final_target, drawable) = drawable_target(&context, &layer, [w + 48, h + 24]);
                let mut encoder = context.device.create_command_encoder(&Default::default());
                renderer.begin_frame();
                let source_view = source.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(FORMAT.add_srgb_suffix()),
                    ..Default::default()
                });
                clear(&source_view, &mut encoder);
                let reference_clip = if control == Control::Glyph && captures.len() == 10 {
                    w - 8 * metrics[0]
                } else {
                    w
                };
                frame.encode(
                    &mut encoder,
                    &source_view,
                    [0, 0],
                    [0, 0, reference_clip, h],
                );
                let bg = model::terminal_background(&expected, model.ui.window.theme)
                    .map(|v| f64::from(v) / 255.);
                let background = wgpu::Color {
                    r: bg[0],
                    g: bg[1],
                    b: bg[2],
                    a: bg[3],
                };
                // The real pane has an opaque background before copying cells.
                // Controls must erase ink to that color, not to transparency.
                clear_color(
                    &target.create_view(&Default::default()),
                    background,
                    &mut encoder,
                );
                let clip = if control == Control::Copy && captures.len() == 10 {
                    w - 8 * metrics[0]
                } else {
                    w
                };
                cache
                    .draw(
                        &mut renderer,
                        &target,
                        &Panes {
                            model: &model,
                            bounds: &[(
                                seat,
                                Rect::new(
                                    17.5 / scale as f32,
                                    9.5 / scale as f32,
                                    (clip as f32 + 0.75) / scale as f32,
                                    (h as f32 + 0.75) / scale as f32,
                                ),
                            )],
                            focused: Some(seat),
                            overlay: vec![],
                        },
                        scale as f32,
                        &mut encoder,
                    )
                    .unwrap();
                renderer
                    .composite_group_clearing(
                        &target.create_view(&Default::default()),
                        group::Filter {
                            opacity: 1.,
                            saturation: 1.,
                        },
                        ChromeTarget {
                            width: w + 48,
                            height: h + 24,
                            scale: 1.,
                            view: &final_target.create_view(&Default::default()),
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        },
                        &mut encoder,
                    )
                    .unwrap();
                if control == Control::Final && captures.len() == 10 {
                    let blank = context.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("injected final strip"),
                        size: wgpu::Extent3d {
                            width: 8 * metrics[0],
                            height: h,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[wgpu::TextureFormat::Bgra8Unorm],
                    });
                    clear_color(
                        &blank.create_view(&wgpu::TextureViewDescriptor {
                            format: Some(wgpu::TextureFormat::Bgra8Unorm),
                            ..Default::default()
                        }),
                        background,
                        &mut encoder,
                    );
                    let mut destination = final_target.as_image_copy();
                    destination.origin = wgpu::Origin3d {
                        x: x + w - 8 * metrics[0],
                        y,
                        z: 0,
                    };
                    encoder.copy_texture_to_texture(
                        blank.as_image_copy(),
                        destination,
                        blank.size(),
                    );
                }
                let width = 8 * metrics[0];
                let stride = (width * 4).div_ceil(256) * 256;
                let section = u64::from(stride) * u64::from(h);
                let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("streaming right-edge source/composition/final bands"),
                    size: 3 * section,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                copy_band(
                    &mut encoder,
                    &source,
                    [w - width, 0],
                    &buffer,
                    0,
                    [width, h],
                    stride,
                );
                copy_band(
                    &mut encoder,
                    &target,
                    [x + w - width, y],
                    &buffer,
                    section,
                    [width, h],
                    stride,
                );
                copy_band(
                    &mut encoder,
                    &final_target,
                    [x + w - width, y],
                    &buffer,
                    2 * section,
                    [width, h],
                    stride,
                );
                renderer.flush();
                context.queue.submit([encoder.finish()]);
                // Same native present implementation as Gpu::draw, once per
                // produced frame, after its final composite/readback submission.
                Acquired::Metal {
                    texture: final_target,
                    drawable,
                }
                .present(&context)
                .unwrap();
                captures.push(Bands {
                    frame: captures.len(),
                    dimensions: [cols, rows],
                    width,
                    height: h,
                    stride,
                    buffer,
                    cell_pixels: metrics,
                    expected_ink: expected
                        .cells
                        .iter()
                        .flat_map(|row| row[cols - 8..].iter().map(|cell| cell.c == 'W'))
                        .collect(),
                });
            }
            // After a full batch every source row has deliberate edge ink.
            assert!(
                Screen::from_term(&term, String::new(), None, None)
                    .cells
                    .iter()
                    .all(|row| row[cols - 8..].iter().all(|cell| cell.c == 'W'))
            );
        }
    }
    // No map/poll/readback wait, capture-triggered draw or frame repair occurs
    // during the stream. All submitted frames are observed, then read once at the end.
    for capture in &captures {
        capture
            .buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    }
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(45)),
        })
        .unwrap();
    // Drain the actual native queue only after every frame was produced.
    let completion = timing_marker(&context.queue).unwrap();
    let (sent, finished) = mpsc::sync_channel(1);
    let callback = RcBlock::new(
        move |command: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
            // SAFETY: Metal retains this command throughout its completion callback.
            let done = unsafe { command.as_ref() }.status()
                == objc2_metal::MTLCommandBufferStatus::Completed;
            let _ = sent.try_send(done);
        },
    );
    // SAFETY: Metal retains a callback owning only its bounded thread-safe sender.
    unsafe {
        completion.addCompletedHandler(RcBlock::as_ptr(&callback));
    }
    completion.commit();
    assert!(
        finished
            .recv_timeout(Duration::from_secs(45))
            .expect("native present queue completion timed out")
    );
    for capture in &captures {
        let bytes = capture.buffer.slice(..).get_mapped_range().unwrap();
        ink_frames += usize::from(check_bands(capture, &bytes)? > 0);
    }
    assert!(captures.len() >= 18);
    assert!(
        ink_frames >= 18,
        "source-ink coverage is required, not empty bands"
    );
    Ok((captures.len(), ink_frames))
}

#[test]
fn codex_style_streaming_resize_checks_every_produced_grid_and_gpu_band() {
    for scale in [1, 2] {
        let (frames, ink) = replay(Control::None, scale).unwrap();
        eprintln!(
            "streaming: scale={scale} frames={frames} ink_frames={ink}; no in-stream readback waits; private synthetic fixture and native Metal present calls; no physical scanout claim"
        );
    }
}

#[test]
fn streaming_oracle_rejects_injected_grid_glyph_copy_and_final_strips() {
    for (control, prefix) in [
        (Control::Grid, "grid strip:"),
        (Control::Glyph, "glyph strip:"),
        (Control::Copy, "copy strip:"),
        (Control::Final, "final strip:"),
    ] {
        let error = replay(control, 1).unwrap_err();
        assert!(
            error.starts_with(prefix),
            "wrong failure for {control:?}: {error}"
        );
        assert!(error.contains("frame=10"), "{error}");
        eprintln!("broken {control:?} control rejected: {error}");
    }
}
