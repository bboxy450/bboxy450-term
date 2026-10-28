// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Surface ownership stays with winit; device setup runs on the shared IO runtime.
use crate::budget::GpuSample;
use crate::display_link::Drawable;
use bboxy450_term::model::{self, DrawCtx, Model};
use bbt_proto::{Screen, SeatId};
use bbt_render::cells::terminal::PreparedGrid;
use bbt_render::{
    chrome::{ChromeRenderer, ChromeTarget, group, materials as m},
    device::DeviceContext,
};
use bbt_ui::{self as ui, components::Paint};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::{Color, DrawList},
    layout::Rect,
    tokens::Theme,
};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice,
    MTLResourceOptions, MTLTexture, MTLTextureType,
};
use objc2_quartz_core::CAMetalDrawable;
use std::time::{Duration, Instant};
use std::{
    collections::BTreeMap,
    ptr::NonNull,
    sync::Arc,
    sync::mpsc::{self, Receiver, SyncSender},
};

enum Acquired {
    Surface(wgpu::SurfaceTexture),
    Metal {
        texture: wgpu::Texture,
        drawable: Drawable,
    },
}
/// An acquired target is owned by exactly one following frame submission.
pub struct AcquiredFrame {
    frame: Acquired,
    wait: Duration,
}
impl AcquiredFrame {
    pub fn wait(&self) -> Duration {
        self.wait
    }
}
impl Acquired {
    fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(frame) => &frame.texture,
            Self::Metal { texture, .. } => texture,
        }
    }
    fn present(self, context: &DeviceContext) -> Result<(), String> {
        match self {
            Self::Surface(frame) => context.queue.present(frame),
            Self::Metal { drawable, .. } => {
                // SAFETY: this exact Metal queue/device rendered the imported
                // drawable. A later command buffer on the same serial queue
                // presents only after wgpu's submitted work; it retains drawable.
                let queue = unsafe { context.queue.as_hal::<wgpu::hal::api::Metal>() }
                    .ok_or("Metal queue unavailable")?;
                let command = queue
                    .as_raw()
                    .commandBuffer()
                    .ok_or("Metal present command unavailable")?;
                command.presentDrawable(
                    AsRef::<ProtocolObject<dyn objc2_metal::MTLDrawable>>::as_ref(&*drawable),
                );
                command.commit();
            }
        }
        Ok(())
    }
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
fn timing_marker(
    queue: &wgpu::Queue,
) -> Result<Retained<ProtocolObject<dyn MTLCommandBuffer>>, String> {
    // SAFETY: use this device's serial Metal queue without changing wgpu state.
    // The command buffer retains its four-byte scratch buffer until completion.
    let queue =
        unsafe { queue.as_hal::<wgpu::hal::api::Metal>() }.ok_or("Metal queue unavailable")?;
    let queue = queue.as_raw();
    let scratch = queue
        .device()
        .newBufferWithLength_options(4, MTLResourceOptions::StorageModeShared)
        .ok_or("GPU timing scratch unavailable")?;
    let command = queue
        .commandBuffer()
        .ok_or("GPU timing command unavailable")?;
    let blit = command
        .blitCommandEncoder()
        .ok_or("GPU timing blit unavailable")?;
    blit.fillBuffer_range_value(&scratch, NSRange::new(0, 4), 0);
    blit.endEncoding();
    Ok(command)
}
#[derive(Clone, Copy)]
struct FrameTime {
    scale: f32,
    time_ms: f32,
    /// The host drew into the target by other means before these ops (an
    /// external texture, a draw hook): no frosted blur is reused (backdrop_keys).
    external_before: bool,
    /// The target is uncleared: its first pass clears it (to transparent) instead of a separate
    /// clear pass. Identical pixels, one empty pass fewer.
    clear: bool,
}
pub struct Panes<'a> {
    pub model: &'a Model,
    pub bounds: &'a [(SeatId, Rect)],
    /// Window-local focus, projected in the same frame as the pane chrome.
    pub focused: Option<SeatId>,
    pub overlay: Vec<Paint>,
}

/// Match the terminal group's default background across its padding and the
/// unused fraction of the last cell. Same sRGB math as chrome/group.wgsl;
/// the RGBA8 wash is quantized before source-over, just like the GPU target.
pub fn terminal_background(background: [u8; 4], theme: Theme, focused: bool) -> Color {
    let source = Color(background.map(|v| f32::from(v) / 255.));
    if focused {
        return source;
    }
    let wash = Color::hex(theme.palette().bg)
        .mix(Color::hex(theme.palette().fg_dim), TOKENS.dim.wash as f32);
    let [r, g, b, _] = source.0;
    let luma = r * 0.213 + g * 0.715 + b * 0.072;
    Color(std::array::from_fn(|i| {
        if i == 3 {
            1.
        } else {
            let filtered = luma + (source.0[i] - luma) * TOKENS.dim.saturate as f32;
            filtered * TOKENS.dim.opacity as f32
                + (wash.0[i] * 255.).round() / 255. * (1. - TOKENS.dim.opacity as f32)
        }
    }))
}
struct CellTexture {
    screen: Arc<Screen>,
    frame: Arc<PreparedGrid>,
    texture: wgpu::Texture,
    dimmed: Option<DimmedCellTexture>,
}
struct DimmedCellTexture {
    texture: wgpu::Texture,
    theme: Option<Theme>,
}
#[derive(Default)]
struct CellTextures(BTreeMap<SeatId, CellTexture>);
pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    renderer: ChromeRenderer,
    materials: m::Pass,
    /// Render passes and copies encoded by the last draw (profiling; see bbt_render::passes).
    pub passes: bbt_render::passes::Counts,
    target: wgpu::Texture,
    backdrop: wgpu::Texture,
    cells: CellTextures,
    gpu_sender: SyncSender<GpuSample>,
    gpu_samples: Receiver<GpuSample>,
}

fn texture(device: &wgpu::Device, size: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("app composition"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[FORMAT.add_srgb_suffix()],
    })
}
fn clear(view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
    clear_color(view, wgpu::Color::TRANSPARENT, encoder);
}
fn clear_color(view: &wgpu::TextureView, color: wgpu::Color, encoder: &mut wgpu::CommandEncoder) {
    bbt_render::passes::count(bbt_render::passes::Kind::Clear);
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("app clear"),
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
fn target(view: &wgpu::TextureView, scale: f32) -> ChromeTarget<'_> {
    let size = view.texture().size();
    ChromeTarget {
        width: size.width,
        height: size.height,
        scale,
        view,
        format: FORMAT,
    }
}

impl Gpu {
    /// Clone handles to the same device while bootstrap still owns Gpu on IO.
    pub fn shared_context(&self) -> std::sync::Arc<DeviceContext> {
        std::sync::Arc::new(DeviceContext {
            device: self.renderer.context.device.clone(),
            queue: self.renderer.context.queue.clone(),
            adapter_info: self.renderer.context.adapter_info.clone(),
        })
    }
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: [u32; 2],
    ) -> Result<Self, String> {
        bbt_rt::assert_not_ui_thread();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .ok_or("no sRGB surface")?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .map_err(|e| e.to_string())?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size[0].max(1),
            height: size[1].max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![format.remove_srgb_suffix()],
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        let target = texture(&device, [config.width, config.height]);
        let backdrop = texture(&device, [config.width, config.height]);
        let materials = m::Pass::new(&device, FORMAT);
        let renderer = ChromeRenderer::with_context(
            DeviceContext {
                device,
                queue,
                adapter_info,
            },
            crate::assets::images()?,
        )
        .map_err(|e| e.to_string())?;
        let (gpu_sender, gpu_samples) = mpsc::sync_channel(crate::budget::SAMPLES);
        Ok(Self {
            surface,
            config,
            renderer,
            materials,
            passes: Default::default(),
            target,
            backdrop,
            cells: CellTextures::default(),
            gpu_sender,
            gpu_samples,
        })
    }
    pub fn configure(&self) {
        self.surface
            .configure(&self.renderer.context.device, &self.config);
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.configure();
        self.target = texture(&self.renderer.context.device, [width, height]);
        self.backdrop = texture(&self.renderer.context.device, [width, height]);
    }
    pub fn adapter_name(&self) -> &str {
        &self.renderer.context.adapter_info.name
    }
    pub fn maximum_drawables(&self) -> Option<usize> {
        // SAFETY: this is the configured Metal surface; the guard owns its
        // lifetime and we only read the layer while a nonblocking lock is held.
        let surface = unsafe { self.surface.as_hal::<wgpu::hal::api::Metal>() }?;
        let layer = surface.render_layer().try_lock()?;
        Some(layer.maximumDrawableCount())
    }
    pub fn capture(
        &self,
        path: std::path::PathBuf,
        crop: Option<crate::capture::Crop>,
        logger: crate::log::Logger,
        done: impl FnOnce(Result<(), String>) + Send + 'static,
    ) {
        crate::capture::save(
            &self.renderer.context.device,
            &self.renderer.context.queue,
            &self.target,
            path,
            crop,
            logger,
            done,
        );
    }
    pub fn collect_gpu(
        &self,
        budget: &mut crate::budget::Budget,
        mut complete: impl FnMut(&GpuSample),
    ) {
        for sample in self.gpu_samples.try_iter() {
            complete(&sample);
            budget.gpu(sample);
        }
    }
    /// Prepare first-use text and pipelines on IO, before handing state to UI.
    pub fn warm(&mut self, ops: Vec<Paint>, scale: f32) -> Result<(), String> {
        bbt_rt::assert_not_ui_thread();
        // A submission boundary: per-draw buffer slices may be reused from here.
        self.renderer.begin_frame();
        self.materials.begin_frame();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        encode_ops(
            &mut self.renderer,
            &self.materials,
            &self.target,
            &self.backdrop,
            ops,
            FrameTime {
                scale,
                time_ms: 450.,
                external_before: false,
                clear: true,
            },
            &mut encoder,
        )
        .map_err(|e| e.to_string())?;
        self.materials.flush(&self.renderer.context);
        self.renderer.flush();
        self.renderer.context.queue.submit([encoder.finish()]);
        Ok(())
    }
    pub fn acquire(&mut self, drawable: Option<Drawable>) -> Result<Option<AcquiredFrame>, String> {
        let acquire = Instant::now();
        let frame = if let Some(drawable) = drawable {
            let raw = drawable.texture();
            if raw.width() != self.config.width as usize
                || raw.height() != self.config.height as usize
            {
                return Ok(None);
            }
            let size = wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            };
            // SAFETY: this drawable comes from this configured surface's layer
            // and exact Metal device. Format, dimensions and usage match; we
            // discard prior contents and clear before drawing. wgpu retains the
            // imported texture and the native present command retains drawable.
            let texture = unsafe {
                let hal = wgpu::hal::metal::Device::texture_from_raw(
                    raw,
                    self.config.format,
                    MTLTextureType::Type2D,
                    1,
                    1,
                    size.into(),
                    None,
                );
                self.renderer
                    .context
                    .device
                    .create_texture_from_hal::<wgpu::hal::api::Metal>(
                        hal,
                        &wgpu::TextureDescriptor {
                            label: Some("CAMetalDisplayLink drawable"),
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: self.config.format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &self.config.view_formats,
                        },
                        wgpu::TextureUses::UNINITIALIZED,
                    )
            };
            Acquired::Metal { texture, drawable }
        } else {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Acquired::Surface(frame),
                wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                    self.configure();
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err("surface validation error".into());
                }
            }
        };
        let wait = acquire.elapsed();
        Ok(Some(AcquiredFrame { frame, wait }))
    }
    pub fn draw(
        &mut self,
        ops: Vec<Paint>,
        scale: f32,
        time_ms: f32,
        acquired: AcquiredFrame,
        frame_id: u64,
        panes: Option<Panes<'_>>,
    ) -> Result<Option<(Duration, Duration)>, String> {
        let AcquiredFrame { frame, wait } = acquired;
        let encode = Instant::now();
        self.renderer.begin_frame();
        self.materials.begin_frame();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        let view = self.target.create_view(&Default::default());
        encode_ops(
            &mut self.renderer,
            &self.materials,
            &self.target,
            &self.backdrop,
            ops,
            FrameTime {
                scale,
                time_ms,
                // Nothing has touched the target yet this frame; its first pass clears it.
                external_before: false,
                clear: true,
            },
            &mut encoder,
        )
        .map_err(|e| e.to_string())?;
        if let Some(panes) = panes {
            self.cells.draw(
                &mut self.renderer,
                &self.target,
                &panes,
                scale,
                &mut encoder,
            )?;
            if !panes.overlay.is_empty() {
                encode_ops(
                    &mut self.renderer,
                    &self.materials,
                    &self.target,
                    &self.backdrop,
                    panes.overlay,
                    FrameTime {
                        scale,
                        time_ms,
                        external_before: true,
                        clear: false,
                    },
                    &mut encoder,
                )
                .map_err(|e| e.to_string())?;
            }
        }
        let surface_view = frame.texture().create_view(&Default::default());
        self.renderer
            .composite_group_clearing(
                &view,
                group::Filter {
                    opacity: 1.,
                    saturation: 1.,
                },
                ChromeTarget {
                    width: self.config.width,
                    height: self.config.height,
                    scale,
                    view: &surface_view,
                    format: self.config.format,
                },
                &mut encoder,
            )
            .map_err(|e| e.to_string())?;
        // wgpu 30 prohibits mixing its encoding API with as_hal_mut. Native
        // queue markers bracket the submission (including staging uploads),
        // yielding a conservative GPU span without inspecting private state.
        self.materials.flush(&self.renderer.context);
        self.renderer.flush();
        let gpu_start = timing_marker(&self.renderer.context.queue)?;
        gpu_start.commit();
        self.renderer.context.queue.submit([encoder.finish()]);
        self.passes = bbt_render::passes::take();
        let gpu_end = timing_marker(&self.renderer.context.queue)?;
        let sender = self.gpu_sender.clone();
        let completed = RcBlock::new(
            move |command: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                // SAFETY: Metal keeps this command buffer alive during its callback.
                // The handler captures only a bounded nonblocking sender, no UI state.
                let command = unsafe { command.as_ref() };
                let start = gpu_start.GPUStartTime();
                let end = command.GPUEndTime();
                if start > 0. && end >= start && start.is_finite() && end.is_finite() {
                    let _ = sender.try_send(GpuSample {
                        frame: frame_id,
                        time: Duration::from_secs_f64(end - start),
                        completed_at: Instant::now(),
                    });
                }
            },
        );
        // SAFETY: Metal copies the handler, which retains the start command and
        // captures only a thread-safe bounded sender; no UI state crosses over.
        unsafe {
            gpu_end.addCompletedHandler(RcBlock::as_ptr(&completed));
        }
        gpu_end.commit();
        let encoded = encode.elapsed();
        let present = Instant::now();
        frame.present(&self.renderer.context)?;
        Ok(Some((encoded, wait + present.elapsed())))
    }
}

impl CellTextures {
    /// Keep the terminal raster and its inactive CSS group separately. Focused
    /// output remains a byte-exact copy; only changed source/theme recomposites
    /// the dim texture. Both native and windowless drivers use this cache.
    // Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    fn draw(
        &mut self,
        renderer: &mut ChromeRenderer,
        destination: &wgpu::Texture,
        panes: &Panes<'_>,
        scale: f32,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), String> {
        let Panes {
            model,
            bounds,
            focused,
            ..
        } = panes;
        self.0
            .retain(|id, _| bounds.iter().any(|(visible, _)| visible == id));
        let theme = model.ui.window.theme;
        for (id, rect) in *bounds {
            let (Some(screen), Some(frame)) = (model.screens.get(id), model.frames.get(id)) else {
                continue;
            };
            let changed = self.0.get(id).is_none_or(|old| {
                !Arc::ptr_eq(&old.screen, screen) || !Arc::ptr_eq(&old.frame, frame)
            });
            if changed {
                let old = self.0.remove(id).filter(|old| {
                    old.texture.width() == frame.width && old.texture.height() == frame.height
                });
                let (texture, mut dimmed) =
                    old.map(|old| (old.texture, old.dimmed)).unwrap_or_else(|| {
                        (
                            texture(&renderer.context.device, [frame.width, frame.height]),
                            None,
                        )
                    });
                if let Some(dimmed) = &mut dimmed {
                    dimmed.theme = None;
                }
                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(FORMAT.add_srgb_suffix()),
                    ..Default::default()
                });
                clear(&view, encoder);
                model::draw_pane(
                    screen,
                    Rect::new(
                        0.,
                        0.,
                        frame.width as f32 / scale,
                        frame.height as f32 / scale,
                    ),
                    &mut DrawCtx {
                        frame,
                        encoder,
                        target: &view,
                        scale,
                    },
                );
                self.0.insert(
                    *id,
                    CellTexture {
                        screen: screen.clone(),
                        frame: frame.clone(),
                        texture,
                        dimmed,
                    },
                );
            }
            let cached = self.0.get_mut(id).expect("visible terminal cache");
            let focused = *focused == Some(*id);
            if !focused
                && cached
                    .dimmed
                    .as_ref()
                    .is_none_or(|old| old.theme != Some(theme))
            {
                let dimmed = cached.dimmed.get_or_insert_with(|| DimmedCellTexture {
                    texture: texture(&renderer.context.device, [frame.width, frame.height]),
                    theme: None,
                });
                let background = Color::hex(theme.palette().bg)
                    .mix(Color::hex(theme.palette().fg_dim), TOKENS.dim.wash as f32)
                    .0;
                let view = dimmed.texture.create_view(&Default::default());
                clear_color(
                    &view,
                    wgpu::Color {
                        r: f64::from(background[0]),
                        g: f64::from(background[1]),
                        b: f64::from(background[2]),
                        a: 1.,
                    },
                    encoder,
                );
                renderer
                    .composite_group(
                        &cached.texture.create_view(&Default::default()),
                        group::Filter {
                            opacity: TOKENS.dim.opacity as f32,
                            saturation: TOKENS.dim.saturate as f32,
                        },
                        target(&view, scale),
                        encoder,
                    )
                    .map_err(|error| error.to_string())?;
                dimmed.theme = Some(theme);
            }
            let source = if focused {
                &cached.texture
            } else {
                &cached
                    .dimmed
                    .as_ref()
                    .expect("inactive terminal cache")
                    .texture
            };
            let x = (rect.x * scale).round().max(0.) as u32;
            let y = (rect.y * scale).round().max(0.) as u32;
            let width = ((rect.w * scale).floor().max(0.) as u32)
                .min(frame.width)
                .min(destination.width().saturating_sub(x));
            let height = ((rect.h * scale).floor().max(0.) as u32)
                .min(frame.height)
                .min(destination.height().saturating_sub(y));
            if width > 0 && height > 0 {
                bbt_render::passes::count(bbt_render::passes::Kind::Copy);
                encoder.copy_texture_to_texture(
                    source.as_image_copy(),
                    wgpu::TexelCopyTextureInfo {
                        origin: wgpu::Origin3d { x, y, z: 0 },
                        ..destination.as_image_copy()
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        Ok(())
    }
}

/// For each frosted material op, a digest of every op painted before it (its
/// backdrop), the target and, if any of those ops is time-driven, the frame
/// time. Equal digests mean an identical backdrop, so the blur is reused
/// (materials::Frame::backdrop_key). Other ops get None, and so does every op
/// once the prefix is too large to digest cheaply.
///
/// The digest sees only these ops. When the host has already drawn into the
/// target by other means (an external texture, a draw hook, live cell
/// textures), `timing.external_before` is true and no frosted op gets a key: it always
/// recomputes, since content the digest cannot see may have changed.
fn backdrop_keys(ops: &[Paint], timing: FrameTime, size: wgpu::Extent3d) -> Vec<Option<u64>> {
    use std::hash::{Hash, Hasher};
    const LIMIT: usize = 256 * 1024;
    // Exhaustive on purpose: a new Paint variant that samples a texture must
    // decide here whether its pixels are fully described by its Debug form.
    fn timed(op: &Paint) -> bool {
        match op {
            Paint::Material { kind, .. } => {
                matches!(kind, ui::Material::Glow | ui::Material::Rim)
            }
            Paint::Group { ops, .. } => ops.iter().any(timed),
            Paint::Draw(_) | Paint::Underlay(_) => false,
        }
    }
    let frosted = |op: &Paint| {
        matches!(
            op,
            Paint::Material {
                kind: ui::Material::Frosted,
                ..
            }
        )
    };
    let mut keys = vec![None; ops.len()];
    if timing.external_before {
        return keys;
    }
    let Some(last) = ops.iter().rposition(frosted) else {
        return keys;
    };
    let mut prefix = std::collections::hash_map::DefaultHasher::new();
    (size.width, size.height, timing.scale.to_bits()).hash(&mut prefix);
    let (mut bytes, mut dynamic) = (0, false);
    for (index, op) in ops[..=last].iter().enumerate() {
        if frosted(op) {
            let mut key = prefix.clone();
            if dynamic {
                timing.time_ms.to_bits().hash(&mut key);
            }
            keys[index] = Some(key.finish());
        }
        let text = format!("{op:?}");
        bytes += text.len();
        if bytes > LIMIT {
            break;
        }
        text.hash(&mut prefix);
        dynamic |= timed(op);
    }
    keys
}
/// R5's windowless consumer exercises the same chrome ordering and retained
/// terminal grids. It owns a submission boundary, just like Gpu::draw.
pub(crate) struct VerificationFrame {
    renderer: ChromeRenderer,
    materials: m::Pass,
    target: wgpu::Texture,
    backdrop: wgpu::Texture,
    cells: CellTextures,
}
impl VerificationFrame {
    pub async fn new() -> Result<Self, String> {
        bbt_rt::assert_not_ui_thread();
        let renderer = ChromeRenderer::new(crate::assets::images()?).map_err(|e| e.to_string())?;
        let materials = m::Pass::new(&renderer.context.device, FORMAT);
        let target = texture(&renderer.context.device, [1600, 900]);
        let backdrop = texture(&renderer.context.device, [1600, 900]);
        Ok(Self {
            renderer,
            materials,
            target,
            backdrop,
            cells: CellTextures::default(),
        })
    }
    pub fn context(&self) -> std::sync::Arc<DeviceContext> {
        std::sync::Arc::new(DeviceContext {
            device: self.renderer.context.device.clone(),
            queue: self.renderer.context.queue.clone(),
            adapter_info: self.renderer.context.adapter_info.clone(),
        })
    }
    /// The offscreen target at a new size (`ctl app resize`); the next draw fills it.
    pub fn resize(&mut self, size: [u32; 2]) {
        self.target = texture(&self.renderer.context.device, size);
        self.backdrop = texture(&self.renderer.context.device, size);
    }
    pub fn draw(
        &mut self,
        ops: Vec<Paint>,
        model: &Model,
        bounds: &[(SeatId, Rect)],
        frame: u64,
        completed: SyncSender<(u64, Instant)>,
    ) -> Result<Duration, String> {
        self.draw_notifying(ops, model, bounds, frame, completed, || {})
    }

    /// A windowless submission whose completion wakes its sleeping GUI owner.
    pub fn draw_notifying(
        &mut self,
        ops: Vec<Paint>,
        model: &Model,
        bounds: &[(SeatId, Rect)],
        frame: u64,
        completed: SyncSender<(u64, Instant)>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Duration, String> {
        self.draw_layers_notifying(
            ops,
            Panes {
                model,
                bounds,
                focused: model.ui.window.stage.focused,
                overlay: vec![],
            },
            frame,
            450.,
            completed,
            wake,
        )
    }

    /// The windowless driver uses the same foreground-after-pane order as winit.
    #[allow(
        clippy::too_many_arguments,
        reason = "same render layers as native; completion belongs to the driver"
    )]
    pub fn draw_layers_notifying(
        &mut self,
        ops: Vec<Paint>,
        panes: Panes<'_>,
        frame: u64,
        time_ms: f32,
        completed: SyncSender<(u64, Instant)>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Duration, String> {
        let start = Instant::now();
        self.renderer.begin_frame();
        self.materials.begin_frame();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        clear(&self.target.create_view(&Default::default()), &mut encoder);
        encode_ops(
            &mut self.renderer,
            &self.materials,
            &self.target,
            &self.backdrop,
            ops,
            FrameTime {
                scale: 1.,
                time_ms,
                external_before: false,
                // R5 keeps its explicit clear above.
                clear: false,
            },
            &mut encoder,
        )
        .map_err(|e| e.to_string())?;
        self.cells
            .draw(&mut self.renderer, &self.target, &panes, 1., &mut encoder)?;
        if !panes.overlay.is_empty() {
            encode_ops(
                &mut self.renderer,
                &self.materials,
                &self.target,
                &self.backdrop,
                panes.overlay,
                FrameTime {
                    scale: 1.,
                    time_ms,
                    external_before: true,
                    clear: false,
                },
                &mut encoder,
            )
            .map_err(|e| e.to_string())?;
        }
        self.materials.flush(&self.renderer.context);
        self.renderer.flush();
        self.renderer.context.queue.submit([encoder.finish()]);
        let marker = timing_marker(&self.renderer.context.queue)?;
        let handler = RcBlock::new(move |_: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
            let _ = completed.try_send((frame, Instant::now()));
            wake();
        });
        // SAFETY: Metal retains the handler, whose only capture is a bounded
        // thread-safe sender. The marker follows this fixture's submission.
        unsafe {
            marker.addCompletedHandler(RcBlock::as_ptr(&handler));
        }
        marker.commit();
        Ok(start.elapsed())
    }

    /// Read back the actual offscreen composition using the window's PNG path.
    pub fn capture(
        &self,
        path: std::path::PathBuf,
        crop: Option<crate::capture::Crop>,
        logger: crate::log::Logger,
        written: impl FnOnce(Result<(), String>) + Send + 'static,
    ) {
        crate::capture::save(
            &self.renderer.context.device,
            &self.renderer.context.queue,
            &self.target,
            path,
            crop,
            logger,
            written,
        );
    }
}

fn encode_ops(
    renderer: &mut ChromeRenderer,
    pass: &m::Pass,
    destination: &wgpu::Texture,
    backdrop: &wgpu::Texture,
    ops: Vec<Paint>,
    timing: FrameTime,
    encoder: &mut wgpu::CommandEncoder,
) -> bbt_render::Result<()> {
    let FrameTime { scale, time_ms, .. } = timing;
    let view = destination.create_view(&Default::default());
    // An uncleared target is cleared by the first flat flush, in that pass; a material or group
    // that comes first reads or composites the target, so it gets an explicit clear instead.
    let uncleared = std::cell::Cell::new(timing.clear);
    let flush = |renderer: &mut ChromeRenderer,
                 list: &DrawList,
                 encoder: &mut wgpu::CommandEncoder|
     -> bbt_render::Result<()> {
        if uncleared.take() {
            renderer.draw_clearing(list, target(&view, scale), encoder)
        } else {
            renderer.draw_overlay(list, target(&view, scale), encoder)
        }
    };
    let mut underlay = DrawList::default();
    let mut pending = DrawList::default();
    let keys = backdrop_keys(&ops, timing, destination.size());
    for (index, op) in ops.into_iter().enumerate() {
        match op {
            Paint::Underlay(list) => underlay.primitives.extend(list.primitives),
            Paint::Draw(mut list) => {
                pending.primitives.append(&mut underlay.primitives);
                pending.primitives.append(&mut list.primitives);
            }
            Paint::Material {
                rect,
                radii,
                kind,
                color,
                blur,
                rim: glow,
                clips,
            } => {
                if !pending.primitives.is_empty() {
                    flush(renderer, &std::mem::take(&mut pending), encoder)?;
                }
                if uncleared.take() {
                    clear(&view, encoder);
                }
                let material = match kind {
                    ui::Material::Flat => m::Material::Flat(color),
                    ui::Material::Gloss => m::Material::Gloss(m::Gloss {
                        color,
                        top_white: 0.12,
                        bottom_black: 0.18,
                        highlight: 0.18,
                        bottom_shade: 0.25,
                    }),
                    ui::Material::Frosted => m::Material::Frosted(m::Frosted {
                        tint: Color::TRANSPARENT,
                        blur_radius: blur.map_or(STYLE.material.panel_blur_px as f32, |b| b.radius),
                        saturation: blur
                            .map_or(STYLE.material.panel_saturate as f32, |b| b.saturation),
                    }),
                    ui::Material::Rim => {
                        // The pane's alive rim (rim-glow), in the seat colour, to the depth.
                        let s = glow.unwrap_or_default();
                        m::Material::Rim(m::Rim {
                            color: s
                                .tint
                                .map(Color)
                                .unwrap_or(color)
                                .alpha(s.inferred_multiplier),
                            waiting: Color::hex(TOKENS.color.state.waiting),
                            errored: Color::hex(TOKENS.color.state.errored),
                            activity: match s.activity {
                                ui::GlowActivity::Idle => m::Activity::Idle,
                                ui::GlowActivity::Active => m::Activity::Active,
                                ui::GlowActivity::Waiting => m::Activity::Waiting,
                                ui::GlowActivity::Errored => m::Activity::Errored,
                            },
                            focus: if s.focused { 1. } else { 0. },
                            depth: s.depth.unwrap_or(TOKENS.glow.depth as f32),
                            size: s.size.unwrap_or(TOKENS.glow.rim as f32),
                            phase_ms: (s.seed % 10_000) as f32,
                            outer: s
                                .outer
                                .unwrap_or(f32::MAX)
                                .clamp(0., s.size.unwrap_or(TOKENS.glow.rim as f32) / 2.),
                            alpha: s.rim_alpha.unwrap_or(1.),
                            swell: s.swell.clamp(0., 1.),
                            attention: s.attention,
                        })
                    }
                    ui::Material::Glow => {
                        let g = TOKENS.glow;
                        let state = glow.map_or(m::Activity::Idle, |s| match s.activity {
                            ui::GlowActivity::Idle => m::Activity::Idle,
                            ui::GlowActivity::Active => m::Activity::Active,
                            ui::GlowActivity::Waiting => m::Activity::Waiting,
                            ui::GlowActivity::Errored => m::Activity::Errored,
                        });
                        m::Material::Glow(m::Glow {
                            color,
                            waiting: Color::hex(TOKENS.color.state.waiting),
                            errored: Color::hex(TOKENS.color.state.errored),
                            activity: state,
                            focus: 1.,
                            focused_boost: g.focused_boost as f32,
                            radius: g.pane_radius as f32,
                            seed: glow.map_or(0., |s| s.seed as f32),
                            halo_alpha: glow.and_then(|s| s.halo_alpha).unwrap_or(0.45),
                            idle_alpha: g.idle_alpha as f32,
                            active_alpha: [g.active_alpha_min as f32, g.active_alpha_max as f32],
                            waiting_alpha: [g.waiting_alpha_min as f32, g.waiting_alpha_max as f32],
                            errored_alpha: g.errored_alpha as f32,
                            active_period_ms: g.active_pulse_ms as f32,
                            waiting_period_ms: TOKENS.motion.pulse_ms as f32,
                            particles: g.particles_idle as u32,
                            speed: g.particle_speed as f32,
                            particle_size: g.particle_size as f32,
                        })
                    }
                };
                let items = [m::Surface {
                    rect,
                    radii,
                    material,
                    clips: clips
                        .into_iter()
                        .map(|c| m::Clip {
                            rect: c.rect,
                            radii: c.radii,
                        })
                        .collect(),
                }];
                let size = destination.size();
                if let Some([x, y, width, height]) =
                    items[0].backdrop_region(scale, size.width, size.height)?
                {
                    let origin = wgpu::Origin3d { x, y, z: 0 };
                    let mut source = destination.as_image_copy();
                    source.origin = origin;
                    let mut sampled = backdrop.as_image_copy();
                    sampled.origin = origin;
                    bbt_render::passes::count(bbt_render::passes::Kind::Copy);
                    encoder.copy_texture_to_texture(
                        source,
                        sampled,
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                if !underlay.primitives.is_empty() {
                    flush(renderer, &std::mem::take(&mut underlay), encoder)?;
                }
                pass.draw(
                    &renderer.context,
                    encoder,
                    m::Frame {
                        target: target(&view, scale),
                        backdrop: &backdrop.create_view(&Default::default()),
                        items: &items,
                        time_ms,
                        theme: ui::Theme::Dark,
                        backdrop_key: keys[index],
                    },
                )?;
            }
            Paint::Group {
                ops,
                opacity,
                saturation,
                mask,
            } => {
                if !pending.primitives.is_empty() {
                    flush(renderer, &std::mem::take(&mut pending), encoder)?;
                }
                if uncleared.take() {
                    clear(&view, encoder);
                }
                if !underlay.primitives.is_empty() {
                    flush(renderer, &std::mem::take(&mut underlay), encoder)?;
                }
                let size = destination.size();
                let layer = texture(&renderer.context.device, [size.width, size.height]);
                let behind = texture(&renderer.context.device, [size.width, size.height]);
                let layer_view = layer.create_view(&Default::default());
                renderer.with_group_layer(|renderer| {
                    // A group layer starts cleared (by its first pass): only these ops reach it.
                    let timing = FrameTime {
                        external_before: false,
                        clear: true,
                        ..timing
                    };
                    encode_ops(renderer, pass, &layer, &behind, ops, timing, encoder)
                })?;
                renderer.composite_group_with_mask(
                    &layer_view,
                    group::Filter {
                        opacity,
                        saturation,
                    },
                    mask,
                    target(&view, scale),
                    encoder,
                )?;
            }
        }
    }
    pending.primitives.append(&mut underlay.primitives);
    if !pending.primitives.is_empty() {
        flush(renderer, &pending, encoder)?;
    }
    if uncleared.take() {
        clear(&view, encoder);
    }
    Ok(())
}

#[cfg(test)]
mod backdrop_key_tests {
    use super::*;
    use bbt_ui_model::layout::Rect;

    fn material(kind: ui::Material) -> Paint {
        Paint::Material {
            rect: Rect::new(0., 0., 400., 38.),
            radii: [0.; 4],
            kind,
            color: Color::TRANSPARENT,
            blur: None,
            rim: None,
            clips: vec![],
        }
    }
    fn draw(hex: &str) -> Paint {
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 400., 300.), 0., Color::hex(hex));
        Paint::Draw(list)
    }
    fn keys(ops: &[Paint], time_ms: f32, external: bool) -> Vec<Option<u64>> {
        let size = wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        };
        backdrop_keys(
            ops,
            FrameTime {
                scale: 2.,
                time_ms,
                external_before: external,
                clear: false,
            },
            size,
        )
    }

    // keeps: spec runtime §4
    #[test]
    fn a_frosted_op_after_host_drawing_is_never_keyed() {
        let ops = [draw("#0B0B10"), material(ui::Material::Frosted)];
        assert!(keys(&ops, 0., false)[1].is_some());
        assert_eq!(keys(&ops, 0., true), vec![None, None]);
    }

    // keeps: spec runtime §4
    #[test]
    fn keys_follow_prior_content_and_only_timed_content_follows_time() {
        let fixed = [draw("#0B0B10"), material(ui::Material::Frosted)];
        let k = |ops: &[Paint], t| keys(ops, t, false)[1];
        // Deterministic, and a static backdrop ignores time (the title bar's steady state).
        assert_eq!(k(&fixed, 0.), k(&fixed, 0.));
        assert_eq!(k(&fixed, 0.), k(&fixed, 900.));
        // Different content under it: a different key.
        let other = [draw("#2E6BFF"), material(ui::Material::Frosted)];
        assert_ne!(k(&fixed, 0.), k(&other, 0.));
        // A time-driven rim before it: the key changes every frame.
        let timed = [
            draw("#0B0B10"),
            material(ui::Material::Rim),
            material(ui::Material::Frosted),
        ];
        assert_ne!(keys(&timed, 0., false)[2], keys(&timed, 900., false)[2]);
        // Ops that are not frosted get no key.
        assert_eq!(keys(&timed, 0., false)[..2], [None, None]);
    }
}

#[cfg(test)]
mod clear_fold_tests {
    use super::*;

    /// Paint ops into a fresh 1600x960 target: `folded` folds the clear into the first pass,
    /// else an explicit clear pass runs first (the old path). Pixels and pass counts.
    fn render(ops: Vec<Paint>, folded: bool) -> (Vec<u8>, bbt_render::passes::Counts) {
        render_with(ops, folded, |_, _, _| {})
    }
    fn render_with(
        ops: Vec<Paint>,
        folded: bool,
        after: impl FnOnce(&mut ChromeRenderer, &wgpu::Texture, &mut wgpu::CommandEncoder),
    ) -> (Vec<u8>, bbt_render::passes::Counts) {
        let [w, h] = [1600, 960];
        let mut renderer = ChromeRenderer::new(crate::assets::images().unwrap()).unwrap();
        let pass = m::Pass::new(&renderer.context.device, FORMAT);
        let target = texture(&renderer.context.device, [w, h]);
        let backdrop = texture(&renderer.context.device, [w, h]);
        bbt_render::passes::take();
        renderer.begin_frame();
        pass.begin_frame();
        let mut encoder = renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        if !folded {
            clear(&target.create_view(&Default::default()), &mut encoder);
        }
        let timing = FrameTime {
            scale: 1.,
            time_ms: 450.,
            external_before: false,
            clear: folded,
        };
        encode_ops(
            &mut renderer,
            &pass,
            &target,
            &backdrop,
            ops,
            timing,
            &mut encoder,
        )
        .unwrap();
        after(&mut renderer, &target, &mut encoder);
        let counts = bbt_render::passes::take();
        let buffer = renderer
            .context
            .device
            .create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(w * 4 * h),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
            },
            target.size(),
        );
        pass.flush(&renderer.context);
        renderer.flush();
        renderer.context.queue.submit([encoder.finish()]);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.unwrap());
        renderer
            .context
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let pixels = buffer.slice(..).get_mapped_range().unwrap().to_vec();
        (pixels, counts)
    }

    // keeps: spec ux-prototype §3-4, local focus controls actual PTY pixels
    #[test]
    fn cached_terminal_ungrays_with_window_focus_before_backend_focus_catches_up() {
        let (pixels, _) = render_with(vec![], true, |renderer, target, encoder| {
            let mut term = bbt_core::Term::new(8, 2, 0);
            term.advance(b"\x1b[48;2;255;0;0m\x1b[2J\x1b[?25l");
            let id = SeatId::from_u128(1);
            let mut model = Model::default();
            model.screens.insert(
                id,
                Arc::new(Screen::from_term(&term, String::new(), None, None)),
            );
            let context = Arc::new(DeviceContext {
                device: renderer.context.device.clone(),
                queue: renderer.context.queue.clone(),
                adapter_info: renderer.context.adapter_info.clone(),
            });
            model::PanePreparer::new(context)
                .unwrap()
                .prepare(&mut model)
                .unwrap();
            assert!(model.ui.window.stage.focused.is_none());
            let mut cache = CellTextures::default();
            for (x, focused) in [(0., None), (128., Some(id)), (256., None)] {
                cache
                    .draw(
                        renderer,
                        target,
                        &Panes {
                            model: &model,
                            bounds: &[(id, Rect::new(x, 0., 32., 24.))],
                            focused,
                            overlay: vec![],
                        },
                        1.,
                        encoder,
                    )
                    .unwrap();
            }
        });
        let pixel = |x: usize| &pixels[(4 * 1600 + x) * 4..(4 * 1600 + x) * 4 + 4];
        assert_eq!(
            pixel(132),
            &[255, 0, 0, 255],
            "selected cached cells stayed dimmed"
        );
        assert_eq!(
            pixel(4),
            pixel(260),
            "refocusing changed the cached inactive pixels"
        );
        assert_ne!(pixel(4), pixel(132));
    }

    // keeps: spec runtime §4
    #[test]
    fn a_folded_clear_draws_the_scene_byte_identically_in_one_pass_fewer() {
        let scene = crate::scene::Scene::new();
        let size = [1600., 960.];
        let layouts = scene.layout(size, false).unwrap();
        let ops = || scene.paint(&layouts, size, None, None);
        let (reference, before) = render(ops(), false);
        let (pixels, after) = render(ops(), true);
        assert!(
            reference.iter().any(|&v| v != 0),
            "the scene drew something"
        );
        assert_eq!(pixels, reference);
        assert_eq!(after.clear + 1, before.clear, "{before:?} -> {after:?}");
        assert_eq!(after.passes() + 1, before.passes());
    }

    // keeps: spec runtime §4
    #[test]
    fn a_target_whose_first_op_is_a_material_or_nothing_is_still_cleared() {
        let frosted = || Paint::Material {
            rect: Rect::new(10., 10., 300., 40.),
            radii: [8.; 4],
            kind: ui::Material::Frosted,
            color: Color::hex("#2E6BFF"),
            blur: None,
            rim: None,
            clips: vec![],
        };
        let mut after = DrawList::default();
        after.rect(Rect::new(20., 20., 50., 20.), 4., Color::WHITE);
        let ops = || vec![frosted(), Paint::Draw(after.clone())];
        assert_eq!(render(ops(), true).0, render(ops(), false).0);
        let (empty, counts) = render(vec![], true);
        assert!(
            empty.iter().all(|&v| v == 0) && counts.clear == 1,
            "{counts:?}"
        );
    }
}
