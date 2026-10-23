//! L2 materials, independent of components/Style. All passes use the caller's
//! existing device and a distinct sampled backdrop; no window/device ownership.
use super::{ChromeRenderer, ChromeTarget};
use crate::{Result, device::DeviceContext};
use bbt_ui_model::{
    Theme,
    draw::{Color, DrawList},
    layout::Rect,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flat,
    Gloss,
    Frosted,
    Glow,
    /// The confined border lane (G1-G3); not in `ALL`, which the parity galleries enumerate.
    Lane,
}
impl Kind {
    pub const ALL: [Self; 4] = [Self::Flat, Self::Gloss, Self::Frosted, Self::Glow];
    pub fn name(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Gloss => "gloss",
            Self::Frosted => "frosted",
            Self::Glow => "glow",
            Self::Lane => "lane",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gloss {
    pub color: Color,
    pub top_white: f32,
    pub bottom_black: f32,
    pub highlight: f32,
    pub bottom_shade: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frosted {
    pub tint: Color,
    pub blur_radius: f32,
    pub saturation: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Active,
    Waiting,
    Errored,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    pub color: Color,
    pub waiting: Color,
    pub errored: Color,
    pub activity: Activity,
    pub focus: f32,
    pub focused_boost: f32,
    pub radius: f32,
    /// Counter-hash stream position, integral and exactly representable in f32.
    pub seed: f32,
    /// Independent CSS outer-halo opacity; state/focus drive the emitter.
    pub halo_alpha: f32,
    pub idle_alpha: f32,
    pub active_alpha: [f32; 2],
    pub waiting_alpha: [f32; 2],
    pub errored_alpha: f32,
    pub active_period_ms: f32,
    pub waiting_period_ms: f32,
    pub particles: u32,
    pub speed: f32,
    pub particle_size: f32,
}
/// A border effect confined to a lane of `lane` logical px just outside the
/// box: zero inside the box and beyond the lane, so it never moves layout.
/// Port of Wren's lab/web/shaders.html (prelude, main and the `fx` bodies).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lane {
    /// Seat colour; waiting and errored use their state colours.
    pub color: Color,
    pub waiting: Color,
    pub errored: Color,
    pub activity: Activity,
    /// 1 for the focused pane (x1.3), 0 otherwise.
    pub focus: f32,
    pub lane: f32,
    pub effect: LaneFx,
    /// Added to the frame clock, so neighbouring panes do not move in step.
    pub phase_ms: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneFx {
    Silk,
    Swell,
    Tide,
    /// The control: today's uniform breathing, confined to the lane.
    Breathe,
}
impl LaneFx {
    pub const ALL: [Self; 4] = [Self::Silk, Self::Swell, Self::Tide, Self::Breathe];
    pub fn name(self) -> &'static str {
        match self {
            Self::Silk => "silk",
            Self::Swell => "swell",
            Self::Tide => "tide",
            Self::Breathe => "breathe",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Material {
    Flat(Color),
    Gloss(Gloss),
    Frosted(Frosted),
    Glow(Glow),
    Lane(Lane),
}
impl Material {
    pub fn kind(self) -> Kind {
        match self {
            Self::Flat(_) => Kind::Flat,
            Self::Gloss(_) => Kind::Gloss,
            Self::Frosted(_) => Kind::Frosted,
            Self::Glow(_) => Kind::Glow,
            Self::Lane(_) => Kind::Lane,
        }
    }
}
/// An ancestor's border box, in logical pixels; masks intersect in the shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clip {
    pub rect: Rect,
    pub radii: [f32; 4],
}
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct ClipInstance {
    rect: [f32; 4],
    radii: [f32; 4],
}
impl Clip {
    fn physical(self, scale: f32) -> Result<ClipInstance> {
        let data = ClipInstance {
            rect: [
                self.rect.x * scale,
                self.rect.y * scale,
                self.rect.w * scale,
                self.rect.h * scale,
            ],
            radii: self.radii.map(|r| r * scale),
        };
        if bytemuck::cast_slice::<ClipInstance, f32>(&[data])
            .iter()
            .any(|v| !v.is_finite())
            || self.rect.w < 0.
            || self.rect.h < 0.
            || self.radii.iter().any(|r| *r < 0.)
        {
            return Err("invalid material ancestor clip".into());
        }
        Ok(data)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub rect: Rect,
    pub radii: [f32; 4],
    pub clips: Vec<Clip>,
    pub material: Material,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct Instance {
    rect: [f32; 4],
    color: [f32; 4],
    radii: [f32; 4],
    p0: [f32; 4],
    p1: [f32; 4],
    p2: [f32; 4],
    p3: [f32; 4],
    p4: [f32; 4],
    p5: [f32; 4],
    clip: [f32; 4],
    clip_range: [u32; 4],
}
impl Surface {
    /// Physical backdrop texels sampled by this surface, clipped to the target.
    /// Frosted surfaces include the Gaussian's 3-sigma support and one texel for
    /// fractional edge coverage/filtering. Other materials do not sample it.
    pub fn backdrop_region(&self, scale: f32, width: u32, height: u32) -> Result<Option<[u32; 4]>> {
        if !matches!(self.material, Material::Frosted(_)) {
            return Ok(None);
        }
        if width == 0 || height == 0 || !scale.is_finite() || !(0.5..=4.).contains(&scale) {
            return Err("invalid material frame".into());
        }
        Ok(blur_region(
            &self.instance(scale, width, height)?,
            width,
            height,
        ))
    }
    fn instance(&self, scale: f32, width: u32, height: u32) -> Result<Instance> {
        let physical = [
            self.rect.x * scale,
            self.rect.y * scale,
            self.rect.w * scale,
            self.rect.h * scale,
        ];
        let x = physical[0].round();
        let y = physical[1].round();
        let mut i = Instance {
            rect: [
                x,
                y,
                (physical[0] + physical[2]).round() - x,
                (physical[1] + physical[3]).round() - y,
            ],
            radii: self.radii.map(|r| r * scale),
            clip: [0., 0., width as f32, height as f32],
            ..Default::default()
        };
        match self.material {
            Material::Flat(c) => i.color = c.0,
            Material::Gloss(g) => {
                i.color = g.color.0;
                i.p1 = [g.top_white, g.bottom_black, g.highlight, g.bottom_shade];
            }
            Material::Frosted(f) => {
                i.color = f.tint.0;
                i.p0 = [0., f.blur_radius * scale, f.saturation, 0.];
                i.p2 = physical;
                if !(0.0..=256.).contains(&f.blur_radius) || !(0.0..=4.).contains(&f.saturation) {
                    return Err("invalid frosted parameters".into());
                }
            }
            Material::Glow(g) => {
                i.color = match g.activity {
                    Activity::Waiting => g.waiting.0,
                    Activity::Errored => g.errored.0,
                    _ => g.color.0,
                };
                i.p0[3] = g.radius * scale * 4.;
                i.p0[2] = g.halo_alpha;
                i.p1 = physical;
                i.p2 = [
                    match g.activity {
                        Activity::Idle => 0.,
                        Activity::Active => 1.,
                        Activity::Waiting => 2.,
                        Activity::Errored => 3.,
                    },
                    g.focus,
                    g.focused_boost,
                    g.radius * scale,
                ];
                i.p3 = [
                    g.idle_alpha,
                    g.active_alpha[0],
                    g.active_alpha[1],
                    g.waiting_alpha[0],
                ];
                i.p4 = [
                    g.waiting_alpha[1],
                    g.errored_alpha,
                    g.particles.min(48) as f32,
                    g.speed * scale,
                ];
                i.p5 = [
                    g.particle_size * scale,
                    g.active_period_ms,
                    g.waiting_period_ms,
                    g.seed,
                ];
                if g.radius < 0.
                    || !(0.0..=1.).contains(&g.halo_alpha)
                    || !(0.0..=16777215.).contains(&g.seed)
                    || g.seed.fract() != 0.
                    || g.active_period_ms <= 0.
                    || g.waiting_period_ms <= 0.
                    || g.particle_size <= 0.
                    || g.focus < 0.
                    || g.focus > 1.
                {
                    return Err("invalid glow parameters".into());
                }
            }
            Material::Lane(l) => {
                i.color = match l.activity {
                    Activity::Waiting => l.waiting.0,
                    Activity::Errored => l.errored.0,
                    _ => l.color.0,
                };
                // The quad covers the lane (vs adds one more pixel).
                i.p0[3] = l.lane * scale;
                i.p1 = [
                    match l.effect {
                        LaneFx::Silk => 0.,
                        LaneFx::Swell => 1.,
                        LaneFx::Tide => 2.,
                        LaneFx::Breathe => 3.,
                    },
                    match l.activity {
                        Activity::Idle => 0.,
                        Activity::Active => 1.,
                        Activity::Waiting => 2.,
                        Activity::Errored => 3.,
                    },
                    l.focus,
                    l.lane * scale,
                ];
                i.p2[0] = l.phase_ms;
                if !(1.0..=32.).contains(&l.lane) || !(0.0..=1.).contains(&l.focus) {
                    return Err("invalid lane parameters".into());
                }
            }
        }
        if bytemuck::cast_slice::<Instance, f32>(&[i])
            .iter()
            .any(|n| !n.is_finite())
            || self.rect.w <= 0.
            || self.rect.h <= 0.
            || self.radii.iter().any(|r| *r < 0.)
            || i.color.iter().any(|c| !(0.0..=1.).contains(c))
        {
            return Err("invalid material surface".into());
        }
        Ok(i)
    }
}

fn blur_region(instance: &Instance, width: u32, height: u32) -> Option<[u32; 4]> {
    // Use the unsnapped border box used by mirror_box, not the rounded paint
    // rectangle. ceil(3*sigma) matches the shader's last Gaussian tap.
    let support = (instance.p0[1] * 3.).ceil() + 1.;
    let [x, y, w, h] = instance.p2;
    let left = (x - support).floor().clamp(0., width as f32) as u32;
    let top = (y - support).floor().clamp(0., height as f32) as u32;
    let right = (x + w + support).ceil().clamp(0., width as f32) as u32;
    let bottom = (y + h + support).ceil().clamp(0., height as f32) as u32;
    (right > left && bottom > top).then_some([left, top, right - left, bottom - top])
}
pub struct Frame<'a> {
    pub target: ChromeTarget<'a>,
    pub backdrop: &'a wgpu::TextureView,
    pub items: &'a [Surface],
    pub time_ms: f32,
    pub theme: Theme,
}
pub struct Pass {
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 8],
    blur: [wgpu::RenderPipeline; 2],
    sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
}
impl Pass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L2 material bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L2 materials"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("named flat/gloss/frosted/glow shaders"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("shapes.wgsl"),
                    include_str!("materials.wgsl")
                )
                .into(),
            ),
        });
        let preserve_alpha = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipelines = std::array::from_fn(|index| {
            let entry = [
                "flat", "gloss", "frosted", "glow", "glow", "glow", "lane", "lane",
            ][index];
            let blend = if index == 5 || index == 7 {
                // A transparent group must retain the effect's coverage for
                // its later source-over composite. Normal fx keep dest alpha.
                wgpu::BlendState::ALPHA_BLENDING
            } else if index >= 3 {
                wgpu::BlendState {
                    color: wgpu::BlendComponent::OVER,
                    alpha: preserve_alpha,
                }
            } else {
                wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
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
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material backdrop"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let blur = std::array::from_fn(|axis| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if axis == 0 {
                    "frosted Gaussian horizontal"
                } else {
                    "frosted Gaussian vertical"
                }),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_blur"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(if axis == 0 { "blur_x" } else { "blur_y" }),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
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
        });
        Self {
            layout,
            pipelines,
            blur,
            sampler,
            format,
        }
    }
    pub fn draw(
        &self,
        context: &DeviceContext,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) -> Result<()> {
        self.draw_into(context, encoder, frame, false)
    }
    fn draw_into(
        &self,
        context: &DeviceContext,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
        group: bool,
    ) -> Result<()> {
        let Frame {
            target,
            backdrop,
            items,
            time_ms,
            theme,
        } = frame;
        if target.width == 0
            || target.height == 0
            || target.format != self.format
            || !target.scale.is_finite()
            || !(0.5..=4.).contains(&target.scale)
            || !time_ms.is_finite()
        {
            return Err("invalid material frame".into());
        }
        if items.is_empty() {
            return Ok(());
        }
        let mut instances: Vec<_> = items
            .iter()
            .map(|s| s.instance(target.scale, target.width, target.height))
            .collect::<Result<_>>()?;
        let mut clip_instances = Vec::new();
        for (surface, instance) in items.iter().zip(&mut instances) {
            instance.clip_range = [
                clip_instances.len() as u32,
                surface.clips.len() as u32,
                0,
                0,
            ];
            for clip in &surface.clips {
                clip_instances.push(clip.physical(target.scale)?);
            }
        }
        if clip_instances.is_empty() {
            clip_instances.push(ClipInstance::default());
        }
        let clip_data = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material ancestor clip chain"),
                contents: bytemuck::cast_slice(&clip_instances),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let data = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material parameters"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let view = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material viewport and clock"),
                contents: bytemuck::cast_slice(&[
                    target.width as f32,
                    target.height as f32,
                    time_ms,
                    target.scale,
                ]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind_backdrop = |backdrop: &wgpu::TextureView| {
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("material frame"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: view.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: data.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(backdrop),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: clip_data.as_entire_binding(),
                        },
                    ],
                })
        };
        let bind = bind_backdrop(backdrop);
        let mut filtered = Vec::new();
        for (index, item) in items.iter().enumerate() {
            if !matches!(item.material,Material::Frosted(f) if f.blur_radius>0.) {
                filtered.push(None);
                continue;
            }
            let Some([left, top, width, height]) =
                blur_region(&instances[index], target.width, target.height)
            else {
                filtered.push(None);
                continue;
            };
            let texture = || {
                context.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("frosted sRGB-value intermediate"),
                    size: wgpu::Extent3d {
                        width: target.width,
                        height: target.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
            };
            let horizontal = texture();
            let vertical = texture();
            let x = horizontal.create_view(&Default::default());
            let y = vertical.create_view(&Default::default());
            let bind_x = bind_backdrop(&x);
            for (axis, output, source) in [(0, &x, &bind), (1, &y, &bind_x)] {
                let mut blur = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("frosted separable Gaussian"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                blur.set_pipeline(&self.blur[axis]);
                blur.set_bind_group(0, source, &[]);
                // Keep the original full-target coordinates and sampling math;
                // only omit fragments that the frosted element cannot use.
                blur.set_scissor_rect(left, top, width, height);
                blur.draw(0..3, index as u32..index as u32 + 1);
            }
            filtered.push(Some(bind_backdrop(&y)));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("L2 named materials"),
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
        for (index, item) in items.iter().enumerate() {
            pass.set_bind_group(0, filtered[index].as_ref().unwrap_or(&bind), &[]);
            let pipeline = match item.material.kind() {
                Kind::Flat => 0,
                Kind::Gloss => 1,
                Kind::Frosted => 2,
                Kind::Glow => {
                    if group {
                        5
                    } else if theme == Theme::Light {
                        4
                    } else {
                        3
                    }
                }
                Kind::Lane => {
                    if group {
                        7
                    } else {
                        6
                    }
                }
            };
            pass.set_pipeline(&self.pipelines[pipeline]);
            pass.draw(0..6, index as u32..index as u32 + 1);
        }
        Ok(())
    }
}
pub enum Backdrop<'a> {
    DrawList(&'a DrawList),
    Rgba(&'a [u8]),
}
pub struct Scene<'a> {
    pub backdrop: Backdrop<'a>,
    pub items: &'a [Surface],
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub time_ms: f32,
    pub theme: Theme,
}
pub fn render_to_rgba(renderer: &mut ChromeRenderer, scene: Scene<'_>) -> Result<Vec<u8>> {
    render_composed_to_rgba(renderer, scene, &DrawList::default())
}
/// Gallery composition helper: L2 is effect-only; Style-owned fill, border and
/// text are a caller-supplied foreground, painted on the same device/target.
pub fn render_composed_to_rgba(
    renderer: &mut ChromeRenderer,
    scene: Scene<'_>,
    foreground: &DrawList,
) -> Result<Vec<u8>> {
    render_layered_to_rgba(renderer, scene, &DrawList::default(), foreground)
}
/// The underlay is painted to the destination, never into the sampled backdrop.
/// A Style shadow therefore cannot feed back into its own backdrop blur.
pub fn render_layered_to_rgba(
    renderer: &mut ChromeRenderer,
    scene: Scene<'_>,
    underlay: &DrawList,
    foreground: &DrawList,
) -> Result<Vec<u8>> {
    let Scene {
        backdrop,
        items,
        width,
        height,
        scale,
        time_ms,
        theme,
    } = scene;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err("invalid materials target size".into());
    }
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let make = |label| {
        renderer
            .context
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[format.add_srgb_suffix()],
            })
    };
    let behind = make("L2 sampled backdrop");
    let texture = make("L2 material output");
    let behind_view = behind.create_view(&Default::default());
    let view = texture.create_view(&Default::default());
    let mut encoder = renderer
        .context
        .device
        .create_command_encoder(&Default::default());
    match backdrop {
        Backdrop::DrawList(list) => renderer.draw(
            list,
            ChromeTarget {
                width,
                height,
                scale,
                view: &behind_view,
                format,
            },
            &mut encoder,
        )?,
        Backdrop::Rgba(pixels) => {
            if pixels.len() != (width as usize * height as usize * 4) {
                return Err("invalid backdrop RGBA size".into());
            }
            renderer.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &behind,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
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
    }
    encoder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &behind,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    if !underlay.primitives.is_empty() {
        renderer.draw_overlay(
            underlay,
            ChromeTarget {
                width,
                height,
                scale,
                view: &view,
                format,
            },
            &mut encoder,
        )?;
    }
    let pass = Pass::new(&renderer.context.device, format);
    pass.draw_into(
        &renderer.context,
        &mut encoder,
        Frame {
            target: ChromeTarget {
                width,
                height,
                scale,
                view: &view,
                format,
            },
            backdrop: &behind_view,
            items,
            time_ms,
            theme,
        },
        renderer.group_depth > 0,
    )?;
    if !foreground.primitives.is_empty() {
        renderer.draw_overlay(
            foreground,
            ChromeTarget {
                width,
                height,
                scale,
                view: &view,
                format,
            },
            &mut encoder,
        )?;
    }
    renderer.readback(&texture, encoder, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backdrop_copy_covers_fractional_support_and_clips_offscreen_edges() {
        let surface = Surface {
            rect: Rect::new(8.25, -4.5, 30.2, 12.25),
            radii: [0.; 4],
            clips: vec![],
            material: Material::Frosted(Frosted {
                tint: Color::WHITE,
                blur_radius: 2.5,
                saturation: 1.,
            }),
        };
        assert_eq!(
            surface.backdrop_region(2., 200, 100).unwrap(),
            Some([0, 0, 93, 32])
        );
        let outside = Surface {
            rect: Rect::new(-100., -100., 1., 1.),
            ..surface.clone()
        };
        assert_eq!(outside.backdrop_region(1., 200, 100).unwrap(), None);
        let flat = Surface {
            material: Material::Flat(Color::WHITE),
            ..surface.clone()
        };
        assert_eq!(flat.backdrop_region(1., 200, 100).unwrap(), None);
        assert!(surface.backdrop_region(f32::NAN, 200, 100).is_err());
    }
    #[test]
    fn zero_blur_still_samples_the_backdrop_and_fractional_edge_texels() {
        let surface = Surface {
            rect: Rect::new(3.75, 4.25, 8.5, 7.25),
            radii: [0.; 4],
            clips: vec![],
            material: Material::Frosted(Frosted {
                tint: Color::WHITE,
                blur_radius: 0.,
                saturation: 1.,
            }),
        };
        assert_eq!(
            surface.backdrop_region(1., 64, 64).unwrap(),
            Some([2, 3, 12, 10])
        );
    }
    #[test]
    fn nonfinite_and_negative_material_parameters_are_rejected() {
        let s = Surface {
            rect: Rect::new(0., 0., 10., 10.),
            radii: [1.; 4],
            clips: vec![],
            material: Material::Frosted(Frosted {
                tint: Color::WHITE,
                blur_radius: f32::NAN,
                saturation: 1.,
            }),
        };
        assert!(s.instance(1., 20, 20).is_err());
        let s = Surface {
            material: Material::Flat(Color::WHITE),
            rect: Rect::new(0., 0., -1., 10.),
            ..s
        };
        assert!(s.instance(1., 20, 20).is_err());
    }
}
