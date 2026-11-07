// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Surface ownership stays with winit; device setup runs on the shared IO runtime.
use crate::budget::GpuSample;
use crate::display_link::Drawable;
mod native_probe;
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
    /// Foreground painted over the cells, before the retained layer.
    pub overlay: Vec<Paint>,
    /// The conversation dock: retained in its own layer while unchanged.
    pub retained: Option<std::sync::Arc<Vec<Paint>>>,
    /// Foreground painted after the retained layer (rims, modals, tooltips).
    pub overlay_after: Vec<Paint>,
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
#[derive(Default)]
struct ChromeCache {
    entry: Option<CachedChrome>,
}
struct CachedChrome {
    size: wgpu::Extent3d,
    scale: u32,
    ops: Vec<Paint>,
    texture: wgpu::Texture,
}
impl ChromeCache {
    /// Retain only self-contained, time-independent chrome. Live cells and
    /// foreground overlays are encoded after the copy, so paging never forces
    /// hundreds of unchanged chrome passes back through Metal.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        renderer: &mut ChromeRenderer,
        materials: &m::Pass,
        destination: &wgpu::Texture,
        backdrop: &wgpu::Texture,
        ops: Vec<Paint>,
        timing: FrameTime,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), String> {
        if ops.iter().any(timed_paint) {
            self.entry = None;
            return encode_ops(
                renderer,
                materials,
                destination,
                backdrop,
                ops,
                timing,
                encoder,
            )
            .map_err(|e| e.to_string());
        }
        let size = destination.size();
        // Exact value equality, including clips, masks, colors, text and target
        // geometry. No hash collision can substitute different chrome pixels.
        let hit = self.entry.as_ref().is_some_and(|old| {
            old.size == size && old.scale == timing.scale.to_bits() && old.ops == ops
        });
        if !hit {
            let cached_ops = ops.clone();
            let layer = texture(&renderer.context.device, [size.width, size.height]);
            encode_ops(
                renderer,
                materials,
                &layer,
                backdrop,
                ops,
                FrameTime {
                    clear: true,
                    ..timing
                },
                encoder,
            )
            .map_err(|e| e.to_string())?;
            self.entry = Some(CachedChrome {
                size,
                scale: timing.scale.to_bits(),
                ops: cached_ops,
                texture: layer,
            });
        }
        let layer = &self.entry.as_ref().expect("prepared chrome layer").texture;
        bbt_render::passes::count(bbt_render::passes::Kind::Copy);
        encoder.copy_texture_to_texture(layer.as_image_copy(), destination.as_image_copy(), size);
        Ok(())
    }
}

/// The conversation dock rendered once into its own layer, then composited
/// source-over each frame: same as an isolated group of opacity 1. Re-encoding
/// its ~20k primitives was the whole overlay cost of an idle frame.
#[derive(Default)]
struct OverlayLayer {
    entry: Option<CachedOverlay>,
}
struct CachedOverlay {
    list: std::sync::Arc<Vec<Paint>>,
    size: wgpu::Extent3d,
    scale: u32,
    texture: wgpu::Texture,
}
/// How the last frame drew its retained overlay, for the hitch log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayDraw {
    #[default]
    None,
    Hit,
    Rebuilt,
    /// Not cacheable this frame: the first frosted, rim or glow material
    /// inside the dock.
    Live(&'static str),
}
/// Content whose pixels depend only on itself: draw lists, groups, and the
/// flat and gloss fills. Frosted samples what lies beneath; rims and glows
/// animate.
fn plain_paint(op: &Paint) -> bool {
    match op {
        Paint::Draw(_) | Paint::Underlay(_) => true,
        Paint::Group { ops, .. } => ops.iter().all(plain_paint),
        Paint::Material { kind, .. } => {
            matches!(kind, ui::Material::Flat | ui::Material::Gloss)
        }
    }
}
/// The first material that keeps a dock list live, for the hitch log.
fn first_material(ops: &[Paint]) -> Option<&'static str> {
    ops.iter().find_map(|op| match op {
        Paint::Material { kind, .. } => match kind {
            ui::Material::Flat | ui::Material::Gloss => None,
            ui::Material::Frosted => Some("dock:Frosted"),
            ui::Material::Rim => Some("dock:Rim"),
            ui::Material::Glow => Some("dock:Glow"),
        },
        Paint::Group { ops, .. } => first_material(ops),
        Paint::Draw(_) | Paint::Underlay(_) => None,
    })
}
/// Before, the retained layer, then after, in painter's order on `destination`.
/// Overlay calls are external (`backdrop_keys` keys nothing), so a frosted op
/// after the layer always blurs the live target, which already holds the
/// composited layer: the same pixels one combined call would sample.
#[allow(
    clippy::too_many_arguments,
    reason = "the overlay's three ordered parts"
)]
fn draw_overlay(
    renderer: &mut ChromeRenderer,
    materials: &m::Pass,
    layer: &mut OverlayLayer,
    destination: &wgpu::Texture,
    backdrop: &wgpu::Texture,
    before: Vec<Paint>,
    list: std::sync::Arc<Vec<Paint>>,
    after: Vec<Paint>,
    timing: FrameTime,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<OverlayDraw, String> {
    let live = FrameTime {
        external_before: true,
        clear: false,
        ..timing
    };
    if !before.is_empty() {
        encode_ops(
            renderer,
            materials,
            destination,
            backdrop,
            before,
            live,
            encoder,
        )
        .map_err(|e| e.to_string())?;
    }
    let size = destination.size();
    let scale = timing.scale.to_bits();
    let hit = layer.entry.as_ref().is_some_and(|old| {
        old.size == size
            && old.scale == scale
            && (std::sync::Arc::ptr_eq(&old.list, &list) || *old.list == *list)
    });
    let state = if hit {
        // Keep the newest identity so the next frame hits by pointer.
        layer.entry.as_mut().expect("retained overlay").list = list;
        OverlayDraw::Hit
    } else {
        let texture = layer
            .entry
            .take()
            .map(|old| old.texture)
            .filter(|t| t.size() == size)
            .unwrap_or_else(|| texture(&renderer.context.device, [size.width, size.height]));
        encode_ops(
            renderer,
            materials,
            &texture,
            backdrop,
            list.as_ref().clone(),
            FrameTime {
                external_before: false,
                clear: true,
                ..timing
            },
            encoder,
        )
        .map_err(|e| e.to_string())?;
        layer.entry = Some(CachedOverlay {
            list,
            size,
            scale,
            texture,
        });
        OverlayDraw::Rebuilt
    };
    let source = layer
        .entry
        .as_ref()
        .expect("retained overlay")
        .texture
        .create_view(&Default::default());
    renderer
        .composite_group(
            &source,
            group::Filter {
                opacity: 1.,
                saturation: 1.,
            },
            target(&destination.create_view(&Default::default()), timing.scale),
            encoder,
        )
        .map_err(|e| e.to_string())?;
    if !after.is_empty() {
        encode_ops(
            renderer,
            materials,
            destination,
            backdrop,
            after,
            live,
            encoder,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(state)
}
/// One live call, today's path, when the overlay cannot use its layer.
#[allow(
    clippy::too_many_arguments,
    reason = "the overlay's three ordered parts"
)]
fn draw_overlay_or_live(
    renderer: &mut ChromeRenderer,
    materials: &m::Pass,
    layer: &mut OverlayLayer,
    destination: &wgpu::Texture,
    backdrop: &wgpu::Texture,
    before: Vec<Paint>,
    retained: Option<std::sync::Arc<Vec<Paint>>>,
    after: Vec<Paint>,
    timing: FrameTime,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<OverlayDraw, String> {
    if let Some(list) = retained
        .as_ref()
        .filter(|list| list.iter().all(plain_paint))
    {
        return draw_overlay(
            renderer,
            materials,
            layer,
            destination,
            backdrop,
            before,
            list.clone(),
            after,
            timing,
            encoder,
        );
    }
    layer.entry = None;
    let reason = retained
        .as_ref()
        .map(|list| first_material(list).unwrap_or("dock"));
    let mut ops = before;
    if let Some(list) = retained {
        ops.extend(list.iter().cloned());
    }
    ops.extend(after);
    if !ops.is_empty() {
        encode_ops(
            renderer,
            materials,
            destination,
            backdrop,
            ops,
            FrameTime {
                external_before: true,
                clear: false,
                ..timing
            },
            encoder,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(reason.map_or(OverlayDraw::None, OverlayDraw::Live))
}

fn timed_paint(op: &Paint) -> bool {
    match op {
        Paint::Material { kind, .. } => matches!(kind, ui::Material::Glow | ui::Material::Rim),
        Paint::Group { ops, .. } => ops.iter().any(timed_paint),
        Paint::Draw(_) | Paint::Underlay(_) => false,
    }
}

/// A stable cockpit below the alive rims. Pulse frames restore only the
/// maximum rim bands, then repaint their intersecting rims in original order.
/// Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct AttentionCache {
    below: wgpu::Texture,
    suffix: Vec<Paint>,
    scale: u32,
    static_time_ms: f32,
    damage: Vec<[u32; 4]>,
}

fn region_intersection(a: [u32; 4], b: [u32; 4]) -> Option<[u32; 4]> {
    let left = a[0].max(b[0]);
    let top = a[1].max(b[1]);
    let right = (a[0] + a[2]).min(b[0] + b[2]);
    let bottom = (a[1] + a[3]).min(b[1] + b[3]);
    (right > left && bottom > top).then_some([
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    ])
}

/// Add a rectangle to a disjoint pixel union: no pulse pixel is blended twice.
fn add_damage(regions: &mut Vec<[u32; 4]>, next: [u32; 4]) {
    let mut pieces = vec![next];
    for &old in regions.iter() {
        pieces = pieces
            .into_iter()
            .flat_map(|piece| {
                let Some(overlap) = region_intersection(piece, old) else {
                    return vec![piece];
                };
                let [x, y, w, h] = piece;
                let [left, top, width, height] = overlap;
                [
                    [x, y, w, top - y],
                    [x, top + height, w, y + h - top - height],
                    [x, top, left - x, height],
                    [left + width, top, x + w - left - width, height],
                ]
                .into_iter()
                .filter(|r| r[2] > 0 && r[3] > 0)
                .collect()
            })
            .collect();
    }
    regions.extend(pieces);
}

fn pixel_region(rect: Rect, scale: f32, size: wgpu::Extent3d) -> Option<[u32; 4]> {
    if rect.w <= 0. || rect.h <= 0. {
        return None;
    }
    let left = (rect.x * scale).floor().clamp(0., size.width as f32) as u32;
    let top = (rect.y * scale).floor().clamp(0., size.height as f32) as u32;
    let right = ((rect.x + rect.w) * scale)
        .ceil()
        .clamp(0., size.width as f32) as u32;
    let bottom = ((rect.y + rect.h) * scale)
        .ceil()
        .clamp(0., size.height as f32) as u32;
    (right > left && bottom > top).then_some([
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    ])
}

fn rim_extent(rect: Rect, rim: ui::Rim, scale: f32) -> Rect {
    let size = rim.size.unwrap_or(TOKENS.glow.rim as f32);
    let outer = rim.outer.unwrap_or(size * 0.5).clamp(0., size * 0.5);
    // Material vertex expansion and fragment antialiasing get two extra pixels.
    let reach = outer + TOKENS.glow.swell as f32 + 2. / scale;
    Rect::new(
        rect.x - reach,
        rect.y - reach,
        rect.w + 2. * reach,
        rect.h + 2. * reach,
    )
}

fn attention_damage(suffix: &[Paint], scale: f32, size: wgpu::Extent3d) -> Option<Vec<[u32; 4]>> {
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let mut damage = vec![];
    for op in suffix {
        match op {
            Paint::Material {
                rect,
                radii,
                kind: ui::Material::Rim,
                rim: Some(rim),
                ..
            } => {
                if !rim.attention {
                    continue;
                }
                let Some(outer) = pixel_region(rim_extent(*rect, *rim, scale), scale, size) else {
                    continue;
                };
                // Rounded corners, the band and inward bloom stay outside this
                // conservative inner rectangle. Terminal interiors are untouched.
                let inset = radii
                    .iter()
                    .copied()
                    .fold(rim.depth.unwrap_or(TOKENS.glow.depth as f32), f32::max)
                    .max(rim.size.unwrap_or(TOKENS.glow.rim as f32) * 0.5)
                    + 2. / scale;
                let inner = Rect::new(
                    rect.x + inset,
                    rect.y + inset,
                    (rect.w - 2. * inset).max(0.),
                    (rect.h - 2. * inset).max(0.),
                );
                let Some(inner) = pixel_region(inner, scale, size)
                    .and_then(|inner| region_intersection(outer, inner))
                else {
                    add_damage(&mut damage, outer);
                    continue;
                };
                let [x, y, w, h] = outer;
                let [ix, iy, iw, ih] = inner;
                for band in [
                    [x, y, w, iy - y],
                    [x, iy + ih, w, y + h - iy - ih],
                    [x, iy, ix - x, ih],
                    [ix + iw, iy, x + w - ix - iw, ih],
                ] {
                    if band[2] > 0 && band[3] > 0 {
                        add_damage(&mut damage, band);
                    }
                }
            }
            Paint::Draw(_) | Paint::Underlay(_) | Paint::Material { .. } => {}
            Paint::Group { ops, .. } => {
                // The host updates top-level attention rims only. Static groups
                // retain their painter order, mask and group opacity below.
                if ops.iter().any(nested_attention) {
                    return None;
                }
            }
        }
    }
    // A frosted overlay samples earlier pixels beyond its visible rectangle.
    // If that dependency changes, restore and replay its entire sampling area
    // before painting it once. Merely clipping a blur separately into strips
    // would let later strips sample that same overlay's earlier output.
    let mut dependencies = vec![];
    for op in suffix {
        frost_dependencies(op, scale, size, &mut dependencies)?;
    }
    let mut included = vec![false; dependencies.len()];
    loop {
        let mut changed = false;
        for (index, &(sampled, painted)) in dependencies.iter().enumerate() {
            if !included[index]
                && damage
                    .iter()
                    .any(|region| region_intersection(*region, sampled).is_some())
            {
                included[index] = true;
                add_damage(&mut damage, sampled);
                add_damage(&mut damage, painted);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (!damage.is_empty()).then_some(damage)
}

fn nested_attention(op: &Paint) -> bool {
    match op {
        Paint::Material { rim: Some(rim), .. } => rim.attention,
        Paint::Group { ops, .. } => ops.iter().any(nested_attention),
        _ => false,
    }
}

fn frost_region(op: &Paint, scale: f32, size: wgpu::Extent3d) -> Option<Option<[u32; 4]>> {
    let Paint::Material {
        rect,
        radii,
        kind: ui::Material::Frosted,
        blur,
        ..
    } = op
    else {
        return Some(None);
    };
    m::Surface {
        rect: *rect,
        radii: *radii,
        material: m::Material::Frosted(m::Frosted {
            tint: Color::TRANSPARENT,
            blur_radius: blur.map_or(STYLE.material.panel_blur_px as f32, |b| b.radius),
            saturation: blur.map_or(STYLE.material.panel_saturate as f32, |b| b.saturation),
        }),
        clips: vec![],
    }
    .backdrop_region(scale, size.width, size.height)
    .ok()
}

fn frost_dependencies(
    op: &Paint,
    scale: f32,
    size: wgpu::Extent3d,
    out: &mut Vec<([u32; 4], [u32; 4])>,
) -> Option<()> {
    if let Paint::Group { ops, .. } = op {
        for op in ops {
            frost_dependencies(op, scale, size, out)?;
        }
    }
    if let Some(sampled) = frost_region(op, scale, size)? {
        let Paint::Material { rect, .. } = op else {
            unreachable!()
        };
        let margin = 2. / scale;
        let painted = pixel_region(
            Rect::new(
                rect.x - margin,
                rect.y - margin,
                rect.w + 2. * margin,
                rect.h + 2. * margin,
            ),
            scale,
            size,
        )?;
        out.push((sampled, painted));
    }
    Some(())
}

fn clip_attention_suffix(
    op: &Paint,
    damage: &[[u32; 4]],
    scale: f32,
    size: wgpu::Extent3d,
) -> Vec<Paint> {
    if let Paint::Group {
        ops,
        opacity,
        saturation,
        mask,
    } = op
    {
        let ops: Vec<_> = ops
            .iter()
            .flat_map(|op| clip_attention_suffix(op, damage, scale, size))
            .collect();
        return if ops.is_empty() {
            vec![]
        } else {
            vec![Paint::Group {
                ops,
                opacity: *opacity,
                saturation: *saturation,
                mask: *mask,
            }]
        };
    }
    if let Paint::Material {
        kind: ui::Material::Frosted,
        ..
    } = op
    {
        return frost_region(op, scale, size)
            .flatten()
            .filter(|sampled| {
                damage
                    .iter()
                    .any(|region| region_intersection(*sampled, *region).is_some())
            })
            .map_or_else(Vec::new, |_| vec![op.clone()]);
    }
    damage
        .iter()
        .filter_map(|&[x, y, w, h]| {
            let clip = Rect::new(
                x as f32 / scale,
                y as f32 / scale,
                w as f32 / scale,
                h as f32 / scale,
            );
            match op {
                Paint::Material {
                    rect,
                    rim: Some(rim),
                    kind: ui::Material::Rim,
                    ..
                } if pixel_region(rim_extent(*rect, *rim, scale), scale, size)
                    .and_then(|extent| region_intersection(extent, [x, y, w, h]))
                    .is_none() =>
                {
                    None
                }
                Paint::Material { .. } => {
                    let mut op = op.clone();
                    if let Paint::Material { clips, .. } = &mut op {
                        clips.push(ui::components::Clip {
                            rect: clip,
                            radii: [0.; 4],
                        });
                    }
                    Some(op)
                }
                Paint::Draw(list) | Paint::Underlay(list) => {
                    if list.primitives.is_empty() {
                        return None;
                    }
                    let mut clipped = DrawList::default();
                    clipped.clip(clip, [0.; 4]);
                    clipped.primitives.extend(list.primitives.iter().cloned());
                    clipped.unclip();
                    Some(if matches!(op, Paint::Underlay(_)) {
                        Paint::Underlay(clipped)
                    } else {
                        Paint::Draw(clipped)
                    })
                }
                Paint::Group { .. } => unreachable!(),
            }
        })
        .collect()
}

impl AttentionCache {
    fn capture(
        device: &wgpu::Device,
        target: &wgpu::Texture,
        suffix: Vec<Paint>,
        scale: f32,
        time_ms: f32,
        encoder: &mut wgpu::CommandEncoder,
        previous: Option<Self>,
    ) -> Option<Self> {
        let damage = attention_damage(&suffix, scale, target.size())?;
        let below = previous
            .map(|old| old.below)
            .filter(|texture| texture.size() == target.size())
            .unwrap_or_else(|| texture(device, [target.width(), target.height()]));
        bbt_render::passes::count(bbt_render::passes::Kind::Copy);
        encoder.copy_texture_to_texture(
            target.as_image_copy(),
            below.as_image_copy(),
            target.size(),
        );
        Some(Self {
            below,
            suffix,
            scale: scale.to_bits(),
            static_time_ms: time_ms,
            damage,
        })
    }

    fn updated_suffix(&self, mut update: impl FnMut(&mut Vec<Paint>)) -> Option<Vec<Paint>> {
        let mut next = self.suffix.clone();
        update(&mut next);
        let mut allowed = self.suffix.clone();
        for (old, new) in allowed.iter_mut().zip(&next) {
            if let (Paint::Material { rim: Some(old), .. }, Paint::Material { rim: Some(new), .. }) =
                (old, new)
                && old.attention
            {
                if !new.swell.is_finite() || !(0. ..=1.).contains(&new.swell) {
                    return None;
                }
                old.swell = new.swell;
            }
        }
        (allowed == next).then_some(next)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "retained target and ordered rim composition"
    )]
    fn repaint(
        &mut self,
        renderer: &mut ChromeRenderer,
        materials: &m::Pass,
        target: &wgpu::Texture,
        backdrop: &wgpu::Texture,
        suffix: Vec<Paint>,
        time_ms: f32,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), String> {
        let scale = f32::from_bits(self.scale);
        for &[x, y, width, height] in &self.damage {
            let origin = wgpu::Origin3d { x, y, z: 0 };
            let mut source = self.below.as_image_copy();
            source.origin = origin;
            let mut destination = target.as_image_copy();
            destination.origin = origin;
            bbt_render::passes::count(bbt_render::passes::Kind::Copy);
            encoder.copy_texture_to_texture(
                source,
                destination,
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let mut groups: Vec<(f32, Vec<Paint>)> = vec![];
        let mut pending = vec![];
        for op in &suffix {
            let mut clipped = clip_attention_suffix(op, &self.damage, scale, target.size());
            if matches!(op, Paint::Draw(_) | Paint::Underlay(_)) {
                // Keep an underlay in the same encoder call as its following
                // material, whose backdrop must be sampled before the underlay.
                pending.append(&mut clipped);
                continue;
            }
            // Unchanged neighbours and overlays retain their original shader
            // phase. Only the host's top-level attention rims advance.
            let phase = if matches!(op, Paint::Material { rim: Some(rim), .. } if rim.attention) {
                time_ms
            } else {
                self.static_time_ms
            };
            if groups
                .last()
                .is_none_or(|(old, _)| old.to_bits() != phase.to_bits())
            {
                groups.push((phase, vec![]));
            }
            let group = &mut groups.last_mut().expect("suffix phase group").1;
            group.append(&mut pending);
            group.append(&mut clipped);
        }
        if !pending.is_empty() {
            if groups.is_empty() {
                groups.push((self.static_time_ms, vec![]));
            }
            groups
                .last_mut()
                .expect("suffix phase group")
                .1
                .append(&mut pending);
        }
        for (time_ms, clipped) in groups {
            encode_ops(
                renderer,
                materials,
                target,
                backdrop,
                clipped,
                FrameTime {
                    scale,
                    time_ms,
                    external_before: true,
                    clear: false,
                },
                encoder,
            )
            .map_err(|error| error.to_string())?;
        }
        self.suffix = suffix;
        Ok(())
    }
}

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    renderer: ChromeRenderer,
    materials: m::Pass,
    /// Render passes and copies encoded by the last draw (profiling; see bbt_render::passes).
    pub passes: bbt_render::passes::Counts,
    /// The last frame's encode breakdown, read with `passes` after a hitch.
    pub last_split: EncodeSplit,
    target: wgpu::Texture,
    backdrop: wgpu::Texture,
    cells: CellTextures,
    chrome: ChromeCache,
    overlay_layer: OverlayLayer,
    attention_enabled: bool,
    attention_cache: Option<AttentionCache>,
    /// Last presented frame reused the static cockpit and repainted only rim damage.
    pub pulse_frame: bool,
    gpu_sender: SyncSender<GpuSample>,
    gpu_samples: Receiver<GpuSample>,
    native_probe: Option<native_probe::NativeProbe>,
    configured: bool,
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

/// Expensive asset rasterization and pipeline construction must not occupy an
/// async IO worker. Only the completed value crosses back to its existing owner.
/// Inspired by Zed GPUI's foreground/background context separation; implemented
/// independently: https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md
/// Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
async fn prepare_cpu<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(move || {
        bbt_rt::assert_not_ui_thread();
        job()
    })
    .await
    .map_err(|error| format!("renderer preparation worker stopped: {error}"))?
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
    pub fn install_document_images(
        &mut self,
        images: &[std::sync::Arc<bbt_render::chrome::icons::Image>],
    ) -> Result<(), String> {
        for image in images {
            self.renderer.add_image(image).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Initial surface configuration precedes publication to submitting workers.
    pub fn shared_context(&self) -> std::sync::Arc<DeviceContext> {
        debug_assert!(self.configured, "configure before sharing the GPU queue");
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
        let mut native_probe = native_probe::NativeProbe::from_args()?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | if native_probe.is_some() {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            format,
            width: size[0].max(1),
            height: size[1].max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![format.remove_srgb_suffix()],
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        prepare_cpu(move || {
            if let Some(probe) = &mut native_probe {
                probe.prepare_controls(&device, config.format);
            }
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
                last_split: EncodeSplit::default(),
                target,
                backdrop,
                cells: CellTextures::default(),
                chrome: ChromeCache::default(),
                overlay_layer: OverlayLayer::default(),
                attention_enabled: false,
                attention_cache: None,
                pulse_frame: false,
                gpu_sender,
                gpu_samples,
                native_probe,
                configured: false,
            })
        })
        .await
    }
    pub fn configure(&mut self) {
        self.surface
            .configure(&self.renderer.context.device, &self.config);
        self.configured = true;
        if let Some(probe) = &mut self.native_probe {
            probe.surface_configured();
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.attention_cache = None;
        self.configure();
        self.target = texture(&self.renderer.context.device, [width, height]);
        self.backdrop = texture(&self.renderer.context.device, [width, height]);
    }
    /// Display-link drawables are imported directly, so their layer owns the
    /// current extent. Reconfiguring wgpu would demand global queue idleness,
    /// which worker preparation cannot promise after the context is shared.
    // Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn resize_native(&mut self, width: u32, height: u32) -> Result<(), String> {
        debug_assert!(self.configured);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let layer = {
            // SAFETY: this UI owner configures the same layer bound by the
            // display link. No drawable is imported until its new extent matches.
            let surface = unsafe { self.surface.as_hal::<wgpu::hal::api::Metal>() }
                .ok_or("native resize requires Metal")?;
            let layer = surface
                .render_layer()
                .try_lock()
                .ok_or("Metal layer busy")?;
            layer.clone()
        };
        let mut extent = layer.drawableSize();
        extent.width = f64::from(width);
        extent.height = f64::from(height);
        layer.setDrawableSize(extent);
        self.config.width = width;
        self.config.height = height;
        self.attention_cache = None;
        self.target = texture(&self.renderer.context.device, [width, height]);
        self.backdrop = texture(&self.renderer.context.device, [width, height]);
        if let Some(probe) = &mut self.native_probe {
            probe.layer_resized();
        }
        Ok(())
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
        self.chrome
            .draw(
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
                if let Some(probe) = &mut self.native_probe {
                    probe.rejected_drawable();
                }
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
                            usage: self.config.usage,
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
        self.pulse_frame = false;
        let previous_attention = self.attention_cache.take();
        let AcquiredFrame { frame, wait } = acquired;
        let encode = Instant::now();
        let mut split = EncodeSplit::default();
        let (misses0, shadows0) = self.renderer.mask_stats();
        if std::env::var_os("BBOXY_OVERLAY_DUMP").is_some()
            && let Some(panes) = &panes
        {
            let mut counts = std::collections::BTreeMap::new();
            for op in &panes.overlay {
                dump_overlay_op(op, &mut counts);
            }
            eprintln!("overlay prims {counts:?}");
        }
        self.renderer.begin_frame();
        self.materials.begin_frame();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        let view = self.target.create_view(&Default::default());
        self.chrome
            .draw(
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
        split.chrome = encode.elapsed();
        let mut probe_sample = self.native_probe.as_ref().and_then(|probe| {
            panes.as_ref().and_then(|panes| {
                probe.begin(
                    &self.renderer.context.device,
                    panes,
                    &self.target,
                    scale,
                    frame_id,
                )
            })
        });
        if let Some(sample) = &mut probe_sample {
            sample.surface(&self.surface, matches!(frame, Acquired::Metal { .. }));
        }
        if let (Some(probe), Some(sample)) = (&self.native_probe, &probe_sample) {
            probe.inject(sample, "source", &self.target, &mut encoder);
        }
        if let Some(panes) = panes {
            let at = Instant::now();
            self.cells.draw(
                &mut self.renderer,
                &self.target,
                &panes,
                scale,
                &mut encoder,
            )?;
            split.cells = at.elapsed();
            let at = Instant::now();
            let mut after = panes.overlay_after;
            let attention_start = (self.attention_enabled && self.native_probe.is_none())
                .then(|| {
                    after.iter().position(|op| {
                        matches!(
                            op,
                            Paint::Material {
                                kind: ui::Material::Rim,
                                ..
                            }
                        )
                    })
                })
                .flatten()
                .filter(|at| attention_damage(&after[*at..], scale, self.target.size()).is_some());
            let suffix = attention_start.map(|at| after.split_off(at));
            split.retained = draw_overlay_or_live(
                &mut self.renderer,
                &self.materials,
                &mut self.overlay_layer,
                &self.target,
                &self.backdrop,
                panes.overlay,
                panes.retained,
                after,
                FrameTime {
                    scale,
                    time_ms,
                    external_before: true,
                    clear: false,
                },
                &mut encoder,
            )?;
            if let Some(suffix) = suffix {
                self.attention_cache = AttentionCache::capture(
                    &self.renderer.context.device,
                    &self.target,
                    suffix.clone(),
                    scale,
                    time_ms,
                    &mut encoder,
                    previous_attention,
                );
                encode_ops(
                    &mut self.renderer,
                    &self.materials,
                    &self.target,
                    &self.backdrop,
                    suffix,
                    FrameTime {
                        scale,
                        time_ms,
                        external_before: true,
                        clear: false,
                    },
                    &mut encoder,
                )
                .map_err(|error| error.to_string())?;
            }
            split.overlay = at.elapsed();
        }
        if let (Some(probe), Some(sample)) = (&self.native_probe, &probe_sample) {
            probe.inject(sample, "copy", &self.target, &mut encoder);
        }
        let at = Instant::now();
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
        split.composite = at.elapsed();
        if let (Some(probe), Some(sample)) = (&mut self.native_probe, probe_sample) {
            probe.inject(&sample, "final", frame.texture(), &mut encoder);
            probe.record(sample, &self.target, frame.texture(), &mut encoder);
        }
        self.submit_frame(
            frame,
            wait,
            frame_id,
            encoder,
            encode,
            split,
            (misses0, shadows0),
        )
    }

    /// Only an ordinary frame may replace the static scene under attention rims.
    pub fn set_attention_cache(&mut self, enabled: bool) {
        self.attention_enabled = enabled;
        if !enabled {
            self.attention_cache = None;
        }
    }

    pub fn can_draw_attention(&self) -> bool {
        self.attention_enabled
            && self.native_probe.is_none()
            && self
                .attention_cache
                .as_ref()
                .is_some_and(|cache| cache.below.size() == self.target.size())
    }

    pub fn draw_attention(
        &mut self,
        acquired: AcquiredFrame,
        frame_id: u64,
        scale: f32,
        time_ms: f32,
        update: impl FnMut(&mut Vec<Paint>),
    ) -> Result<Option<(Duration, Duration)>, String> {
        self.pulse_frame = false;
        if !self.can_draw_attention() {
            return Ok(None);
        }
        let mut cache = self
            .attention_cache
            .take()
            .expect("attention cache checked");
        if cache.scale != scale.to_bits() {
            return Ok(None);
        }
        let Some(suffix) = cache.updated_suffix(update) else {
            return Ok(None);
        };
        let AcquiredFrame { frame, wait } = acquired;
        let encode = Instant::now();
        let masks = self.renderer.mask_stats();
        self.renderer.begin_frame();
        self.materials.begin_frame();
        let mut encoder = self
            .renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        cache.repaint(
            &mut self.renderer,
            &self.materials,
            &self.target,
            &self.backdrop,
            suffix,
            time_ms,
            &mut encoder,
        )?;
        let mut split = EncodeSplit {
            overlay: encode.elapsed(),
            ..Default::default()
        };
        let at = Instant::now();
        let source = self.target.create_view(&Default::default());
        let destination = frame.texture().create_view(&Default::default());
        self.renderer
            .composite_group_clearing(
                &source,
                group::Filter {
                    opacity: 1.,
                    saturation: 1.,
                },
                ChromeTarget {
                    width: self.config.width,
                    height: self.config.height,
                    scale,
                    view: &destination,
                    format: self.config.format,
                },
                &mut encoder,
            )
            .map_err(|error| error.to_string())?;
        split.composite = at.elapsed();
        self.attention_cache = Some(cache);
        self.pulse_frame = true;
        self.submit_frame(frame, wait, frame_id, encoder, encode, split, masks)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "shared measured native submission and present"
    )]
    fn submit_frame(
        &mut self,
        frame: Acquired,
        wait: Duration,
        frame_id: u64,
        encoder: wgpu::CommandEncoder,
        encode: Instant,
        mut split: EncodeSplit,
        (misses0, shadows0): (u64, u64),
    ) -> Result<Option<(Duration, Duration)>, String> {
        let at = Instant::now();
        // wgpu 30 prohibits mixing its encoding API with as_hal_mut. Native
        // queue markers bracket the submission (including staging uploads),
        // yielding a conservative GPU span without inspecting private state.
        self.materials.flush(&self.renderer.context);
        self.renderer.flush();
        let gpu_start = timing_marker(&self.renderer.context.queue)?;
        gpu_start.commit();
        self.renderer.context.queue.submit([encoder.finish()]);
        if let Some(probe) = &mut self.native_probe {
            probe.submitted(&self.renderer.context.device);
        }
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
        split.submit = at.elapsed();
        let (misses1, shadows1) = self.renderer.mask_stats();
        split.mask_misses = misses1.saturating_sub(misses0);
        split.shadow_requests = shadows1.saturating_sub(shadows0);
        self.last_split = split;
        let encoded = encode.elapsed();
        let present = Instant::now();
        frame.present(&self.renderer.context)?;
        Ok(Some((encoded, wait + present.elapsed())))
    }
}

/// Where one frame's encode time went, for the hitch log (CPU-side, before present).
#[derive(Clone, Copy, Debug, Default)]
pub struct EncodeSplit {
    pub chrome: Duration,
    pub cells: Duration,
    pub overlay: Duration,
    pub composite: Duration,
    pub submit: Duration,
    /// Mask atlas misses and shadow (atlas path) requests during this frame.
    pub mask_misses: u64,
    pub shadow_requests: u64,
    /// How the conversation layer was drawn.
    pub retained: OverlayDraw,
}

/// BBOXY_OVERLAY_DUMP: count overlay primitives by kind on stderr, once per
/// frame (profiling only). Per-primitive printing costs more than a frame.
fn dump_overlay_op(op: &Paint, counts: &mut std::collections::BTreeMap<&'static str, usize>) {
    use bbt_ui_model::draw::Prim;
    match op {
        Paint::Draw(list) | Paint::Underlay(list) => {
            for p in &list.primitives {
                let kind = match p {
                    Prim::Path { .. } => "Path",
                    Prim::PathShadow { .. } => "PathShadow",
                    Prim::Text { .. } => "Text",
                    Prim::TextShadow { .. } => "TextShadow",
                    Prim::Rect { .. } => "Rect",
                    Prim::Icon { .. } => "Icon",
                    Prim::Image { .. } => "Image",
                    Prim::Group { .. } => "Group",
                    Prim::Clip { .. } | Prim::Unclip => "Clip",
                    _ => "Other",
                };
                *counts.entry(kind).or_default() += 1;
            }
        }
        Paint::Group { ops, .. } => {
            *counts.entry("PaintGroup").or_default() += 1;
            for op in ops {
                dump_overlay_op(op, counts);
            }
        }
        _ => *counts.entry("PaintOther").or_default() += 1,
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
            if let Some(raster) = model
                .prepared_screens
                .get(id)
                .and_then(|source| source.raster.as_ref())
                && raster.theme == theme
            {
                let source = if *focused == Some(*id) {
                    &raster.normal
                } else {
                    &raster.dimmed
                };
                copy_cells(source, destination, *rect, frame, scale, encoder);
                continue;
            }
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
            copy_cells(source, destination, *rect, frame, scale, encoder);
        }
        Ok(())
    }
}

fn copy_cells(
    source: &wgpu::Texture,
    destination: &wgpu::Texture,
    rect: Rect,
    frame: &PreparedGrid,
    scale: f32,
    encoder: &mut wgpu::CommandEncoder,
) {
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
    chrome: ChromeCache,
    overlay_layer: OverlayLayer,
}
impl VerificationFrame {
    pub fn install_document_images(
        &mut self,
        images: &[std::sync::Arc<bbt_render::chrome::icons::Image>],
    ) -> Result<(), String> {
        for image in images {
            self.renderer.add_image(image).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn new() -> Result<Self, String> {
        bbt_rt::assert_not_ui_thread();
        let context = DeviceContext::new().await.map_err(|e| e.to_string())?;
        prepare_cpu(move || {
            let renderer = ChromeRenderer::with_context(context, crate::assets::images()?)
                .map_err(|e| e.to_string())?;
            let materials = m::Pass::new(&renderer.context.device, FORMAT);
            let target = texture(&renderer.context.device, [1600, 900]);
            let backdrop = texture(&renderer.context.device, [1600, 900]);
            Ok(Self {
                renderer,
                materials,
                target,
                backdrop,
                cells: CellTextures::default(),
                chrome: ChromeCache::default(),
                overlay_layer: OverlayLayer::default(),
            })
        })
        .await
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
                retained: None,
                overlay_after: vec![],
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
        self.chrome
            .draw(
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
        draw_overlay_or_live(
            &mut self.renderer,
            &self.materials,
            &mut self.overlay_layer,
            &self.target,
            &self.backdrop,
            panes.overlay,
            panes.retained,
            panes.overlay_after,
            FrameTime {
                scale: 1.,
                time_ms,
                external_before: true,
                clear: false,
            },
            &mut encoder,
        )?;
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

fn rim_material(color: Color, rim: Option<ui::Rim>) -> m::Material {
    let s = rim.unwrap_or_default();
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
    let mut ops = ops.into_iter().enumerate().peekable();
    while let Some((index, op)) = ops.next() {
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
                    ui::Material::Rim => rim_material(color, glow),
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
                let mut items = vec![m::Surface {
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
                // The material renderer already preserves surface order inside
                // one pass. Consecutive rims never sample their backdrop, so
                // all visible pane rims can share one encoder/pass.
                if kind == ui::Material::Rim {
                    while matches!(
                        ops.peek(),
                        Some((
                            _,
                            Paint::Material {
                                kind: ui::Material::Rim,
                                ..
                            }
                        ))
                    ) {
                        let Some((
                            _,
                            Paint::Material {
                                rect,
                                radii,
                                color,
                                rim,
                                clips,
                                ..
                            },
                        )) = ops.next()
                        else {
                            unreachable!()
                        };
                        items.push(m::Surface {
                            rect,
                            radii,
                            material: rim_material(color, rim),
                            clips: clips
                                .into_iter()
                                .map(|c| m::Clip {
                                    rect: c.rect,
                                    radii: c.radii,
                                })
                                .collect(),
                        });
                    }
                }
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
#[allow(
    clippy::disallowed_methods,
    reason = "private current-thread runtime and bounded worker handshakes; never a UI callback"
)]
mod preparation_tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn gpu_cpu_preparation_leaves_the_io_executor_available() {
        let owner = std::thread::current().id();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::sync_channel(1);
        let task = tokio::spawn(prepare_cpu(move || {
            entered.send(std::thread::current().id()).unwrap();
            wait.recv_timeout(Duration::from_secs(3))
                .map_err(|e| e.to_string())?;
            Ok(17)
        }));
        let worker = tokio::time::timeout(Duration::from_secs(5), started)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(
            worker, owner,
            "CPU preparation must not execute on the async IO thread"
        );
        release.send(()).unwrap();
        assert_eq!(task.await.unwrap().unwrap(), 17);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn abandoned_gpu_cpu_preparation_retires_its_late_result() {
        #[derive(Debug)]
        struct Prepared(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for Prepared {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::sync_channel(1);
        let (retired, dropped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(prepare_cpu(move || {
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(3))
                .map_err(|e| e.to_string())?;
            Ok(Prepared(Some(retired)))
        }));
        tokio::time::timeout(Duration::from_secs(5), started)
            .await
            .unwrap()
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), dropped)
            .await
            .unwrap()
            .unwrap();
    }
}

#[cfg(test)]
mod backdrop_key_tests {
    use super::*;
    use bbt_ui_model::layout::Rect;

    pub(super) fn material(kind: ui::Material) -> Paint {
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
    pub(super) fn draw(hex: &str) -> Paint {
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

    fn attention_rim(rect: Rect, attention: bool, swell: f32) -> Paint {
        Paint::Material {
            rect,
            radii: [12.; 4],
            kind: ui::Material::Rim,
            color: Color::hex("#ffa030"),
            blur: None,
            rim: Some(ui::Rim {
                attention,
                swell,
                activity: ui::GlowActivity::Waiting,
                depth: Some(10.),
                size: Some(2.),
                outer: Some(1.),
                ..Default::default()
            }),
            clips: vec![],
        }
    }

    #[test]
    fn attention_frame_damage_is_disjoint_and_excludes_unchanged_interiors() {
        let mut union = vec![];
        let inputs = [[1, 1, 12, 9], [6, 5, 13, 10], [0, 8, 15, 11]];
        for region in inputs {
            add_damage(&mut union, region);
        }
        for x in 0..20 {
            for y in 0..20 {
                let count = union
                    .iter()
                    .filter(|region| region_intersection(**region, [x, y, 1, 1]).is_some())
                    .count();
                let wanted = inputs
                    .iter()
                    .any(|region| region_intersection(*region, [x, y, 1, 1]).is_some());
                assert_eq!(count, usize::from(wanted), "pixel ({x},{y})");
            }
        }
        let suffix = vec![attention_rim(Rect::new(100., 100., 600., 400.), true, 0.5)];
        let damage = attention_damage(
            &suffix,
            2.,
            wgpu::Extent3d {
                width: 1600,
                height: 1200,
                depth_or_array_layers: 1,
            },
        )
        .unwrap();
        assert!(
            damage
                .iter()
                .all(|region| region_intersection(*region, [400, 400, 800, 400]).is_none())
        );
        assert!(
            attention_damage(
                &[backdrop_key_tests::draw("#ffffff")],
                1.,
                wgpu::Extent3d {
                    width: 10,
                    height: 10,
                    depth_or_array_layers: 1
                }
            )
            .is_none()
        );
    }

    #[test]
    fn attention_frame_damage_matches_fresh_rims_without_pulse_trails() {
        use super::backdrop_key_tests::draw;
        let paint_rect = |rect, rgba| {
            let mut list = DrawList::default();
            list.rect(rect, 3., Color(rgba));
            Paint::Draw(list)
        };
        let frost = |rect| Paint::Material {
            rect,
            radii: [8.; 4],
            kind: ui::Material::Frosted,
            color: Color([0.1, 0.2, 0.3, 0.6]),
            blur: Some(ui::ResolvedBlur {
                radius: 8.,
                saturation: 0.8,
            }),
            rim: None,
            clips: vec![],
        };
        let suffix = vec![
            attention_rim(Rect::new(90.25, 80.5, 300., 240.), true, 1.),
            attention_rim(Rect::new(380.25, 130.5, 240., 180.), false, 0.),
            paint_rect(Rect::new(110., 70., 200., 40.), [0.7, 0.1, 0.3, 0.5]),
            Paint::Material {
                rect: Rect::new(360., 110., 100., 90.),
                radii: [6.; 4],
                kind: ui::Material::Flat,
                color: Color([0.1, 0.6, 0.3, 0.7]),
                blur: None,
                rim: None,
                clips: vec![],
            },
            {
                let Paint::Draw(list) =
                    paint_rect(Rect::new(190., 55., 170., 65.), [0., 0., 0., 0.3])
                else {
                    unreachable!()
                };
                Paint::Underlay(list)
            },
            frost(Rect::new(200., 65., 150., 40.)),
            paint_rect(Rect::new(210., 80., 120., 8.), [1., 1., 1., 0.8]),
            Paint::Group {
                ops: vec![
                    paint_rect(Rect::new(70., 260., 180., 85.), [0.2, 0.1, 0.8, 0.7]),
                    frost(Rect::new(100., 285., 160., 50.)),
                    paint_rect(Rect::new(110., 300., 120., 8.), [1., 1., 1., 0.8]),
                ],
                opacity: 0.75,
                saturation: 0.7,
                mask: None,
            },
        ];
        let mut final_suffix = suffix.clone();
        if let Paint::Material { rim: Some(rim), .. } = &mut final_suffix[0] {
            rim.swell = 0.25;
        }
        let expected = render_with(vec![draw("#172b42")], true, |renderer, target, encoder| {
            let pass = m::Pass::new(&renderer.context.device, FORMAT);
            let backdrop = texture(&renderer.context.device, [target.width(), target.height()]);
            let attention = final_suffix.remove(0);
            for (ops, time_ms) in [(vec![attention], 900.), (final_suffix, 450.)] {
                encode_ops(
                    renderer,
                    &pass,
                    target,
                    &backdrop,
                    ops,
                    FrameTime {
                        scale: 1.,
                        time_ms,
                        external_before: true,
                        clear: false,
                    },
                    encoder,
                )
                .unwrap();
            }
            pass.flush(&renderer.context);
        })
        .0;
        let (actual, _) = render_with(vec![draw("#172b42")], true, |renderer, target, encoder| {
            let pass = m::Pass::new(&renderer.context.device, FORMAT);
            let backdrop = texture(&renderer.context.device, [target.width(), target.height()]);
            let mut cache = AttentionCache::capture(
                &renderer.context.device,
                target,
                suffix.clone(),
                1.,
                450.,
                encoder,
                None,
            )
            .unwrap();
            encode_ops(
                renderer,
                &pass,
                target,
                &backdrop,
                suffix.clone(),
                FrameTime {
                    scale: 1.,
                    time_ms: 450.,
                    external_before: true,
                    clear: false,
                },
                encoder,
            )
            .unwrap();
            // A full peak, trough and new rising edge must exactly match a
            // fresh full composition, including a neighbouring static rim.
            for (swell, time_ms) in [(0., 700.), (0.25, 900.)] {
                let next = cache
                    .updated_suffix(|ops| {
                        if let Paint::Material { rim: Some(rim), .. } = &mut ops[0] {
                            rim.swell = swell;
                        }
                    })
                    .unwrap();
                cache
                    .repaint(renderer, &pass, target, &backdrop, next, time_ms, encoder)
                    .unwrap();
            }
            assert!(
                cache
                    .updated_suffix(|ops| {
                        if let Paint::Material { rect, .. } = &mut ops[0] {
                            rect.x += 1.;
                        }
                    })
                    .is_none(),
                "geometry must be replaced by an ordinary frame"
            );
            pass.flush(&renderer.context);
        });
        let mismatches = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
        let first = actual.iter().zip(&expected).position(|(a, b)| a != b);
        assert!(
            actual.len() == expected.len() && mismatches == 0,
            "rim damage changed stable pixels or retained a previous pulse: bytes={mismatches} first={first:?}"
        );
    }

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
    fn batched_rims_match_separate_passes_byte_for_byte() {
        use super::backdrop_key_tests::material;
        let rims: Vec<_> = (0..50)
            .map(|i| {
                let mut op = material(ui::Material::Rim);
                if let Paint::Material { rim, .. } = &mut op {
                    *rim = Some(ui::Rim {
                        seed: i * 13,
                        tint: Some([i as f32 / 50., 0.2, 0.8, 0.7]),
                        ..Default::default()
                    });
                }
                op
            })
            .collect();
        let separate = rims
            .iter()
            .cloned()
            .flat_map(|rim| [rim, Paint::Draw(DrawList::default())])
            .collect();
        let (expected, before) = render(separate, true);
        let (actual, after) = render(rims, true);
        assert_eq!(actual, expected);
        assert_eq!(before.material, 50);
        assert_eq!(after.material, 1);
    }

    #[test]
    fn retained_chrome_matches_fresh_pixels_and_invalidates_changed_paint() {
        use super::backdrop_key_tests::{draw, material};
        for ops in [
            vec![draw("#123456")],
            vec![draw("#123456"), material(ui::Material::Frosted)],
            vec![Paint::Group {
                ops: vec![draw("#abcdef")],
                opacity: 0.7,
                saturation: 0.3,
                mask: None,
            }],
        ] {
            let expected = render(ops.clone(), true).0;
            let (actual, _) = render_with(vec![], true, |renderer, target, encoder| {
                let pass = m::Pass::new(&renderer.context.device, FORMAT);
                let backdrop = texture(&renderer.context.device, [target.width(), target.height()]);
                let mut cache = ChromeCache::default();
                let timing = FrameTime {
                    scale: 1.,
                    time_ms: 450.,
                    external_before: false,
                    clear: true,
                };
                // An unrelated prior layer must not survive a paint replacement.
                cache
                    .draw(
                        renderer,
                        &pass,
                        target,
                        &backdrop,
                        vec![draw("#ff0000")],
                        timing,
                        encoder,
                    )
                    .unwrap();
                cache
                    .draw(
                        renderer,
                        &pass,
                        target,
                        &backdrop,
                        ops.clone(),
                        timing,
                        encoder,
                    )
                    .unwrap();
                let retained = cache.entry.as_ref().unwrap().texture.clone();
                bbt_render::passes::take();
                cache
                    .draw(
                        renderer,
                        &pass,
                        target,
                        &backdrop,
                        ops.clone(),
                        timing,
                        encoder,
                    )
                    .unwrap();
                assert_eq!(retained, cache.entry.as_ref().unwrap().texture);
                let counts = bbt_render::passes::take();
                assert_eq!(counts.passes(), 0, "a hit only copies pixels");
                assert_eq!(counts.copy, 1);
                pass.flush(&renderer.context);
            });
            assert_eq!(actual, expected);
        }
        assert!(timed_paint(&material(ui::Material::Rim)));
        assert!(timed_paint(&Paint::Group {
            ops: vec![material(ui::Material::Glow)],
            opacity: 1.,
            saturation: 1.,
            mask: None,
        }));
    }

    // keeps: the retained conversation layer composites to the live pixels
    #[test]
    fn retained_overlay_layer_matches_live_pixels_hits_and_falls_back() {
        use super::backdrop_key_tests::{draw, material};
        use bbt_ui_model::layout::Rect;
        let rect = |x: f32, rgba: [f32; 4]| {
            let mut list = DrawList::default();
            list.rect(Rect::new(x, 10., 120., 80.), 6., Color(rgba));
            Paint::Draw(list)
        };
        let before = vec![rect(0., [0., 0., 1., 0.5])];
        let retained = std::sync::Arc::new(vec![
            rect(40., [1., 0., 0., 0.5]),
            material(ui::Material::Flat),
            Paint::Group {
                ops: vec![rect(80., [0., 1., 0., 1.])],
                opacity: 0.5,
                saturation: 1.,
                mask: None,
            },
        ]);
        let after = vec![
            rect(120., [1., 1., 0., 0.4]),
            material(ui::Material::Frosted),
            material(ui::Material::Rim),
        ];
        let live = FrameTime {
            scale: 1.,
            time_ms: 450.,
            external_before: true,
            clear: false,
        };
        let mut all = before.clone();
        all.extend(retained.iter().cloned());
        all.extend(after.clone());
        let (expected, _) =
            render_with(vec![draw("#204060")], true, |renderer, target, encoder| {
                let pass = m::Pass::new(&renderer.context.device, FORMAT);
                let backdrop = texture(&renderer.context.device, [target.width(), target.height()]);
                encode_ops(
                    renderer,
                    &pass,
                    target,
                    &backdrop,
                    all.clone(),
                    live,
                    encoder,
                )
                .unwrap();
                pass.flush(&renderer.context);
            });
        let (actual, _) = render_with(vec![draw("#204060")], true, |renderer, target, encoder| {
            let pass = m::Pass::new(&renderer.context.device, FORMAT);
            let backdrop = texture(&renderer.context.device, [target.width(), target.height()]);
            let scratch = texture(&renderer.context.device, [target.width(), target.height()]);
            let mut layer = OverlayLayer::default();
            let first = draw_overlay_or_live(
                renderer,
                &pass,
                &mut layer,
                &scratch,
                &backdrop,
                before.clone(),
                Some(retained.clone()),
                after.clone(),
                live,
                encoder,
            )
            .unwrap();
            assert_eq!(first, OverlayDraw::Rebuilt);
            bbt_render::passes::take();
            // A rebuilt, equal list hits by value and keeps the layer.
            let equal = std::sync::Arc::new(retained.as_ref().clone());
            let second = draw_overlay_or_live(
                renderer,
                &pass,
                &mut layer,
                target,
                &backdrop,
                before.clone(),
                Some(equal),
                after.clone(),
                live,
                encoder,
            )
            .unwrap();
            assert_eq!(second, OverlayDraw::Hit);
            let counts = bbt_render::passes::take();
            assert_eq!(counts.group, 1, "a hit composites the layer once");
            assert_eq!(counts.flat, 2, "only before and after encode: {counts:?}");
            assert_eq!(
                counts.material, 2,
                "the frosted and rim after the layer: {counts:?}"
            );
            // A frosted op inside the dock samples what lies beneath: live.
            let with_material = std::sync::Arc::new(vec![material(ui::Material::Frosted)]);
            assert_eq!(
                draw_overlay_or_live(
                    renderer,
                    &pass,
                    &mut layer,
                    &scratch,
                    &backdrop,
                    vec![],
                    Some(with_material),
                    vec![],
                    live,
                    encoder,
                )
                .unwrap(),
                OverlayDraw::Live("dock:Frosted")
            );
            pass.flush(&renderer.context);
        });
        let worst = expected
            .iter()
            .zip(&actual)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= 1, "layer composite differs by {worst} from live");
    }

    #[test]
    fn worker_rasters_keep_old_publications_immutable_across_a_b_a() {
        let (pixels, _) = render_with(vec![], true, |renderer, target, encoder| {
            let context = Arc::new(DeviceContext {
                device: renderer.context.device.clone(),
                queue: renderer.context.queue.clone(),
                adapter_info: renderer.context.adapter_info.clone(),
            });
            let mut preparer = model::PanePreparer::new(context).unwrap();
            let mut model = Model::default();
            let seat = SeatId::from_u128(1);
            let mut rasters = vec![];
            for color in ["255;0;0", "0;0;255", "255;0;0"] {
                let mut term = bbt_core::Term::new(8, 2, 0);
                term.advance(format!("\x1b[48;2;{color}m\x1b[2J\x1b[?25l").as_bytes());
                model.screens.insert(
                    seat,
                    Arc::new(Screen::from_term(&term, String::new(), None, None)),
                );
                preparer.prepare(&mut model).unwrap();
                rasters.push(
                    model.prepared_screens[&seat]
                        .raster
                        .as_ref()
                        .unwrap()
                        .clone(),
                );
            }
            assert_ne!(
                rasters[0].normal, rasters[2].normal,
                "ABA needs new immutable storage"
            );
            for (i, raster) in rasters.iter().enumerate() {
                copy_cells(
                    &raster.normal,
                    target,
                    Rect::new(i as f32 * 128., 0., 32., 24.),
                    &model.frames[&seat],
                    1.,
                    encoder,
                );
            }
        });
        let pixel = |x: usize| &pixels[x * 4..x * 4 + 3];
        assert_eq!(pixel(0), &[255, 0, 0]);
        assert_eq!(pixel(128), &[0, 0, 255]);
        assert_eq!(pixel(256), &[255, 0, 0]);
    }

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
                            retained: None,
                            overlay_after: vec![],
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

#[cfg(test)]
#[path = "gpu/streaming_tests.rs"]
mod streaming_tests;
