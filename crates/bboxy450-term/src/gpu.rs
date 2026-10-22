//! Surface ownership stays with winit; device setup runs on the shared IO runtime.
use bbt_render::{
    chrome::{ChromeRenderer, ChromeTarget, group, materials as m},
    device::DeviceContext,
};
use bbt_ui::{self as ui, components::Paint};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::{Color, DrawList},
};
use std::time::{Duration, Instant};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    renderer: ChromeRenderer,
    materials: m::Pass,
    target: wgpu::Texture,
    backdrop: wgpu::Texture,
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
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("app clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
        Ok(Self {
            surface,
            config,
            renderer,
            materials,
            target,
            backdrop,
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
    pub fn capture(&self, path: std::path::PathBuf, logger: crate::log::Logger) {
        crate::capture::save(
            &self.renderer.context.device,
            &self.renderer.context.queue,
            &self.target,
            path,
            logger,
        );
    }
    /// Prepare first-use text and pipelines on IO, before handing state to UI.
    pub fn warm(&mut self, ops: Vec<Paint>, scale: f32) -> Result<(), String> {
        bbt_rt::assert_not_ui_thread();
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
            scale,
            &mut encoder,
        )
        .map_err(|e| e.to_string())?;
        self.renderer.context.queue.submit([encoder.finish()]);
        Ok(())
    }
    pub fn draw(
        &mut self,
        ops: Vec<Paint>,
        scale: f32,
    ) -> Result<Option<(Duration, Duration)>, String> {
        let acquire = Instant::now();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
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
        };
        let wait = acquire.elapsed();
        let encode = Instant::now();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        let view = self.target.create_view(&Default::default());
        clear(&view, &mut encoder);
        encode_ops(
            &mut self.renderer,
            &self.materials,
            &self.target,
            &self.backdrop,
            ops,
            scale,
            &mut encoder,
        )
        .map_err(|e| e.to_string())?;
        let surface_view = frame.texture.create_view(&Default::default());
        clear(&surface_view, &mut encoder);
        self.renderer
            .composite_group(
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
        self.renderer.context.queue.submit([encoder.finish()]);
        let encoded = encode.elapsed();
        let present = Instant::now();
        self.renderer.context.queue.present(frame);
        Ok(Some((encoded, wait + present.elapsed())))
    }
}

fn encode_ops(
    renderer: &mut ChromeRenderer,
    pass: &m::Pass,
    destination: &wgpu::Texture,
    backdrop: &wgpu::Texture,
    ops: Vec<Paint>,
    scale: f32,
    encoder: &mut wgpu::CommandEncoder,
) -> bbt_render::Result<()> {
    let view = destination.create_view(&Default::default());
    let mut underlay = DrawList::default();
    let mut pending = DrawList::default();
    for op in ops {
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
                glow,
                clips,
            } => {
                if !pending.primitives.is_empty() {
                    renderer.draw_overlay(
                        &std::mem::take(&mut pending),
                        target(&view, scale),
                        encoder,
                    )?;
                }
                encoder.copy_texture_to_texture(
                    destination.as_image_copy(),
                    backdrop.as_image_copy(),
                    destination.size(),
                );
                if !underlay.primitives.is_empty() {
                    renderer.draw_overlay(
                        &std::mem::take(&mut underlay),
                        target(&view, scale),
                        encoder,
                    )?;
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
                pass.draw(
                    &renderer.context,
                    encoder,
                    m::Frame {
                        target: target(&view, scale),
                        backdrop: &backdrop.create_view(&Default::default()),
                        items: &items,
                        time_ms: 450.,
                        theme: ui::Theme::Dark,
                    },
                )?;
            }
            Paint::Group {
                ops,
                opacity,
                saturation,
            } => {
                if !pending.primitives.is_empty() {
                    renderer.draw_overlay(
                        &std::mem::take(&mut pending),
                        target(&view, scale),
                        encoder,
                    )?;
                }
                if !underlay.primitives.is_empty() {
                    renderer.draw_overlay(
                        &std::mem::take(&mut underlay),
                        target(&view, scale),
                        encoder,
                    )?;
                }
                let size = destination.size();
                let layer = texture(&renderer.context.device, [size.width, size.height]);
                let behind = texture(&renderer.context.device, [size.width, size.height]);
                let layer_view = layer.create_view(&Default::default());
                clear(&layer_view, encoder);
                renderer.with_group_layer(|renderer| {
                    encode_ops(renderer, pass, &layer, &behind, ops, scale, encoder)
                })?;
                renderer.composite_group(
                    &layer_view,
                    group::Filter {
                        opacity,
                        saturation,
                    },
                    target(&view, scale),
                    encoder,
                )?;
            }
        }
    }
    pending.primitives.append(&mut underlay.primitives);
    if !pending.primitives.is_empty() {
        renderer.draw_overlay(&pending, target(&view, scale), encoder)?;
    }
    Ok(())
}
