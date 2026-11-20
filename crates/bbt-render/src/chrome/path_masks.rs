// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Generic vector coverage for the shared R8 shadow atlas. No UI mark definitions
//! live here: the caller supplies the same path, fill and stroke as foreground.
use super::text_glyphs::Mask;
use crate::Result;
use bbt_ui_model::draw::{
    Color, FillRule, LineCap, LineJoin, Paint, PathCommand, PathStroke, VectorPath, path_geometry,
};
use resvg::tiny_skia as sk;
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Command {
    Move([u32; 2]),
    Line([u32; 2]),
    Cubic([[u32; 2]; 3]),
    Close,
}

/// Colour and whole-device-pixel translation cannot change coverage. Fractional
/// x/y placement, effective scale, geometry, stroke, fill and blur all can.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct Key {
    commands: Vec<Command>,
    scale: u32,
    device_scale: u32,
    phase: [u32; 2],
    width: u32,
    dash: Vec<u32>,
    cap: u8,
    join: u8,
    filled: bool,
    pub blur_sigma: u32,
    foreground: Option<Foreground>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Ink {
    Solid([u32; 4]),
    Linear {
        start: [u32; 2],
        end: [u32; 2],
        stops: Vec<(u32, [u32; 4])>,
    },
}
impl Ink {
    fn new(paint: &Paint, opacity: f32) -> Self {
        let color = |c: Color| c.alpha(opacity).0.map(f32::to_bits);
        match paint {
            Paint::Solid(c) => Self::Solid(color(*c)),
            Paint::LinearGradient { start, end, stops } => Self::Linear {
                start: start.map(f32::to_bits),
                end: end.map(f32::to_bits),
                stops: stops
                    .iter()
                    .map(|s| (s.offset.to_bits(), color(s.color)))
                    .collect(),
            },
        }
    }
    fn paint(&self, bbox: [f32; 4]) -> Result<sk::Paint<'static>> {
        let color = |c: [u32; 4]| -> Result<sk::Color> {
            let [r, g, b, a] = c.map(f32::from_bits);
            sk::Color::from_rgba(r, g, b, a).ok_or_else(|| "invalid coverage colour".into())
        };
        let mut paint = sk::Paint::default();
        match self {
            Self::Solid(c) => paint.set_color(color(*c)?),
            Self::Linear { start, end, stops } => {
                let point = |p: [u32; 2]| {
                    sk::Point::from_xy(
                        bbox[0] + f32::from_bits(p[0]) * bbox[2],
                        bbox[1] + f32::from_bits(p[1]) * bbox[3],
                    )
                };
                let stops = stops
                    .iter()
                    .map(|(offset, c)| {
                        Ok(sk::GradientStop::new(f32::from_bits(*offset), color(*c)?))
                    })
                    .collect::<Result<Vec<_>>>()?;
                paint.shader = sk::LinearGradient::new(
                    point(*start),
                    point(*end),
                    stops,
                    sk::SpreadMode::Pad,
                    sk::Transform::identity(),
                )
                .ok_or("invalid coverage gradient")?;
            }
        }
        Ok(paint)
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
struct Foreground {
    transform: [u32; 6],
    clip: [u32; 4],
    viewport: [u32; 2],
    fill: Option<Ink>,
    stroke: Option<Ink>,
    fill_rule: FillRule,
    miter_limit: u32,
    dash_offset: u32,
    bbox: [u32; 4],
}
/// Immutable worker-prepared coverage. Drawing it never invokes rasterization.
#[derive(Clone)]
pub struct PreparedPath {
    pub(super) key: Key,
    pub(super) mask: Arc<Mask>,
}
impl PreparedPath {
    pub fn bytes(&self) -> usize {
        self.mask.pixels.len()
    }
    pub(super) fn clip(&self) -> [f32; 4] {
        self.key
            .foreground
            .as_ref()
            .expect("prepared foreground")
            .clip
            .map(f32::from_bits)
    }
}
/// Call on the host's existing blocking preparation worker, before a UI frame.
/// The visible physical viewport and tighter host clip bound allocation.
pub fn prepare_vector_path(
    path: &VectorPath,
    viewport: [u32; 2],
    clip: [f32; 4],
) -> Result<PreparedPath> {
    path.validate()?;
    if viewport.iter().any(|n| *n == 0 || *n > 4096)
        || u64::from(viewport[0]) * u64::from(viewport[1]) > 16_777_216
        || clip.iter().any(|n| !n.is_finite())
        || clip[2] < 0.
        || clip[3] < 0.
    {
        return Err("invalid foreground coverage viewport".into());
    }
    let mut key = Key::new(
        &path.points,
        &path.stroke,
        path.fill.is_some(),
        1.,
        1.,
        [0.; 2],
        0.,
    )?;
    key.foreground = Some(Foreground {
        transform: path.transform.0.map(f32::to_bits),
        clip: clip.map(f32::to_bits),
        viewport,
        fill: path.fill.as_ref().map(|p| Ink::new(p, path.fill_opacity)),
        stroke: path
            .stroke_paint
            .as_ref()
            .map(|p| Ink::new(p, path.stroke_opacity)),
        fill_rule: path.fill_rule,
        miter_limit: path.miter_limit.to_bits(),
        dash_offset: path.dash_offset.to_bits(),
        bbox: path_geometry(&path.points).0.map(f32::to_bits),
    });
    let mask = Arc::new(rasterize(&key)?);
    Ok(PreparedPath { key, mask })
}

impl Key {
    pub fn new(
        points: &[PathCommand],
        stroke: &PathStroke,
        filled: bool,
        scale: f32,
        device_scale: f32,
        phase: [f32; 2],
        sigma: f32,
    ) -> Result<Self> {
        stroke.validate(points)?;
        if !scale.is_finite()
            || scale <= 0.
            || !device_scale.is_finite()
            || device_scale <= 0.
            || phase
                .iter()
                .any(|v| !v.is_finite() || !(0. ..1.).contains(v))
            || !sigma.is_finite()
            || sigma < 0.
        {
            return Err("invalid path shadow transform or blur".into());
        }
        let bits = |v: f32| if v == 0. { 0 } else { v.to_bits() };
        Ok(Self {
            commands: points
                .iter()
                .map(|p| match *p {
                    PathCommand::Move(p) => Command::Move(p.map(bits)),
                    PathCommand::Line(p) => Command::Line(p.map(bits)),
                    PathCommand::Cubic(p) => Command::Cubic(p.map(|p| p.map(bits))),
                    PathCommand::Close => Command::Close,
                })
                .collect(),
            scale: scale.to_bits(),
            device_scale: device_scale.to_bits(),
            phase: phase.map(bits),
            width: bits(stroke.width),
            dash: stroke.dash.iter().copied().map(bits).collect(),
            cap: stroke.cap as u8,
            join: stroke.join as u8,
            filled,
            blur_sigma: bits(sigma),
            foreground: None,
        })
    }
}

pub(super) fn rasterize(key: &Key) -> Result<Mask> {
    let empty = || Mask {
        format: crate::text::Format::Mask,
        bounds: [0; 4],
        merge_bounds: [0; 4],
        pixels: vec![],
    };
    let scale = if key.foreground.is_some() {
        1.
    } else {
        f32::from_bits(key.scale) * f32::from_bits(key.device_scale)
    };
    let phase = key.phase.map(f32::from_bits);
    let point =
        |p: [u32; 2]| std::array::from_fn::<_, 2, _>(|i| f32::from_bits(p[i]) * scale + phase[i]);
    let mut builder = sk::PathBuilder::new();
    for command in &key.commands {
        match *command {
            Command::Close => builder.close(),
            Command::Move(p) => {
                let [x, y] = point(p);
                builder.move_to(x, y);
            }
            Command::Line(p) => {
                let [x, y] = point(p);
                builder.line_to(x, y);
            }
            Command::Cubic(p) => {
                let [a, b, c] = p.map(point);
                builder.cubic_to(a[0], a[1], b[0], b[1], c[0], c[1]);
            }
        }
    }
    let Some(path) = builder.finish() else {
        return Ok(empty());
    };
    let width = f32::from_bits(key.width) * scale;
    if !key.filled && width == 0. {
        return Ok(empty());
    }
    // Shadow inputs are already in device coordinates. Foreground paths keep
    // their local stroke/paint space and apply the effective affine once.
    let affine = key
        .foreground
        .as_ref()
        .map_or(sk::Transform::identity(), |f| {
            let [a, b, c, d, e, g] = f.transform.map(f32::from_bits);
            sk::Transform::from_row(a, b, c, d, e, g)
        });
    let transformed = path
        .clone()
        .transform(affine)
        .ok_or("invalid coverage transform")?;
    let b = transformed.bounds();
    let miter = key
        .foreground
        .as_ref()
        .map_or(4., |f| f32::from_bits(f.miter_limit));
    let factor = if let Some(f) = &key.foreground {
        let [a, b, c, d, _, _] = f.transform.map(f32::from_bits);
        a.abs() + b.abs() + c.abs() + d.abs()
    } else {
        1.
    };
    let pad = width * 0.5 * miter.max(2.) * factor + 2.;
    let mut extent = [
        (b.left() - pad).floor(),
        (b.top() - pad).floor(),
        (b.right() + pad).ceil(),
        (b.bottom() + pad).ceil(),
    ];
    if let Some(f) = &key.foreground {
        let [x, y, w, h] = f.clip.map(f32::from_bits);
        extent = [
            extent[0].max(x.floor()).max(0.),
            extent[1].max(y.floor()).max(0.),
            extent[2].min((x + w).ceil()).min(f.viewport[0] as f32),
            extent[3].min((y + h).ceil()).min(f.viewport[1] as f32),
        ];
        if extent[2] <= extent[0] || extent[3] <= extent[1] {
            return Ok(empty());
        }
    }
    if extent
        .iter()
        .any(|v| !v.is_finite() || v.abs() > i32::MAX as f32 / 2.)
        || extent[2] - extent[0]
            > if key.foreground.is_some() {
                4096.
            } else {
                16_384.
            }
        || extent[3] - extent[1]
            > if key.foreground.is_some() {
                4096.
            } else {
                2_048.
            }
        || (key.foreground.is_some()
            && (extent[2] - extent[0]) * (extent[3] - extent[1]) > 16_777_216.)
    {
        return Err("path shadow mask exceeds coverage budget".into());
    }
    let [left, top, right, bottom] = extent.map(|v| v as i32);
    let mut pixmap = sk::Pixmap::new((right - left) as u32, (bottom - top) as u32)
        .ok_or("invalid path shadow coverage dimensions")?;
    let mut paint = sk::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    let transform = affine.post_translate(-left as f32, -top as f32);
    let bbox = key
        .foreground
        .as_ref()
        .map_or([0.; 4], |f| f.bbox.map(f32::from_bits));
    if key.filled {
        let fill = key
            .foreground
            .as_ref()
            .and_then(|f| f.fill.as_ref())
            .map(|p| p.paint(bbox))
            .transpose()?;
        let rule = if key
            .foreground
            .as_ref()
            .is_some_and(|f| f.fill_rule == FillRule::EvenOdd)
        {
            sk::FillRule::EvenOdd
        } else {
            sk::FillRule::Winding
        };
        pixmap.fill_path(
            &path,
            fill.as_ref().unwrap_or(&paint),
            rule,
            transform,
            None,
        );
    }
    if width > 0. && key.foreground.as_ref().is_none_or(|f| f.stroke.is_some()) {
        let dash = if key.dash.is_empty() {
            None
        } else {
            let mut intervals: Vec<_> = key
                .dash
                .iter()
                .map(|v| f32::from_bits(*v) * scale)
                .collect();
            // SVG repeats an odd dash list to make a complete on/off cycle.
            if intervals.len() % 2 != 0 {
                intervals.extend_from_within(..);
            }
            sk::StrokeDash::new(
                intervals,
                key.foreground
                    .as_ref()
                    .map_or(0., |f| f32::from_bits(f.dash_offset)),
            )
        };
        let stroke = sk::Stroke {
            width,
            miter_limit: miter,
            dash,
            line_cap: match key.cap {
                x if x == LineCap::Round as u8 => sk::LineCap::Round,
                x if x == LineCap::Square as u8 => sk::LineCap::Square,
                _ => sk::LineCap::Butt,
            },
            line_join: match key.join {
                x if x == LineJoin::Round as u8 => sk::LineJoin::Round,
                x if x == LineJoin::Bevel as u8 => sk::LineJoin::Bevel,
                _ => sk::LineJoin::Miter,
            },
        };
        let ink = key
            .foreground
            .as_ref()
            .and_then(|f| f.stroke.as_ref())
            .map(|p| p.paint(bbox))
            .transpose()?;
        pixmap.stroke_path(
            &path,
            ink.as_ref().unwrap_or(&paint),
            &stroke,
            transform,
            None,
        );
    }
    // The shared text atlas uses y-up bounds and top-down scanlines.
    let bounds = [left, -bottom, right, -top];
    let foreground = key.foreground.is_some();
    let pixels = if foreground {
        pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect()
    } else {
        pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3])
            .collect()
    };
    Ok(Mask {
        format: if foreground {
            crate::text::Format::Bgra
        } else {
            crate::text::Format::Mask
        },
        bounds,
        merge_bounds: bounds,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec svg-renderer — transformed coverage is cropped before allocation and keyed by actual zoom/clip.
    #[test]
    fn svg_foreground_crop_and_colour_are_bounded_and_identity_complete() {
        use bbt_ui_model::draw::{Affine, ColorStop};
        let mut path = VectorPath {
            points: vec![
                PathCommand::Move([-1_000_000., -1_000_000.]),
                PathCommand::Line([1_000_000., -1_000_000.]),
                PathCommand::Line([1_000_000., 1_000_000.]),
                PathCommand::Line([-1_000_000., 1_000_000.]),
                PathCommand::Close,
            ]
            .into(),
            fill: Some(Paint::Solid(Color::WHITE)),
            transform: bbt_ui_model::draw::Affine::default(),
            fill_rule: bbt_ui_model::draw::FillRule::NonZero,
            fill_opacity: 1.,
            stroke: bbt_ui_model::draw::PathStroke::default(),
            stroke_paint: None,
            stroke_opacity: 1.,
            miter_limit: 4.,
            dash_offset: 0.,
        };
        let a = prepare_vector_path(&path, [64, 32], [0., 0., 64., 32.]).unwrap();
        assert!(a.bytes() <= 64 * 32 * 4);
        assert_eq!(a.mask.bounds, [0, -32, 64, 0]);
        path.transform = Affine([8., 0., 0., 8., 0., 0.]);
        let b = prepare_vector_path(&path, [64, 32], [0., 0., 64., 32.]).unwrap();
        assert!(a.key != b.key);
        path.dash_offset = 2.;
        let c = prepare_vector_path(&path, [64, 32], [1., 0., 63., 32.]).unwrap();
        assert!(b.key != c.key);
        assert!(prepare_vector_path(&path, [4097, 1], [0., 0., 4097., 1.]).is_err());
        path.points = vec![
            PathCommand::Move([0., 0.]),
            PathCommand::Line([64., 0.]),
            PathCommand::Line([64., 32.]),
            PathCommand::Line([0., 32.]),
            PathCommand::Close,
        ]
        .into();
        path.transform = Affine::default();
        path.fill = Some(Paint::LinearGradient {
            start: [0., 0.],
            end: [1., 0.],
            stops: vec![
                ColorStop {
                    offset: 0.,
                    color: Color([1., 0., 0., 1.]),
                },
                ColorStop {
                    offset: 1.,
                    color: Color([0., 0., 1., 1.]),
                },
            ]
            .into(),
        });
        let gradient = prepare_vector_path(&path, [64, 32], [0., 0., 64., 32.]).unwrap();
        let row = &gradient.mask.pixels[16 * 64 * 4..17 * 64 * 4];
        assert!(row[2] > row[0]);
        assert!(row[63 * 4] > row[63 * 4 + 2]);
    }
    // keeps: spec svg-renderer — shadow coverage preserves the foreground close join.
    #[test]
    fn close_changes_shadow_join_and_cache_identity() {
        let open = [
            PathCommand::Move([2., 2.]),
            PathCommand::Line([12., 2.]),
            PathCommand::Line([12., 12.]),
            PathCommand::Line([2., 2.]),
        ];
        let mut closed = open.to_vec();
        *closed.last_mut().unwrap() = PathCommand::Close;
        let stroke = PathStroke {
            width: 4.,
            ..Default::default()
        };
        let a = Key::new(&open, &stroke, false, 1., 1., [0.; 2], 0.).unwrap();
        let b = Key::new(&closed, &stroke, false, 1., 1., [0.; 2], 0.).unwrap();
        assert!(a != b);
        assert_ne!(rasterize(&a).unwrap().pixels, rasterize(&b).unwrap().pixels);
    }
    // keeps: spec renderer §4.4a — exact vector geometry and coverage
    #[test]
    fn phase_and_stroke_change_coverage_without_colour_or_translation_inputs() {
        let points = [PathCommand::Move([-3., -3.]), PathCommand::Line([3., 3.])];
        let stroke = PathStroke {
            width: 1.5,
            cap: LineCap::Round,
            ..Default::default()
        };
        let key = Key::new(&points, &stroke, false, 2., 1., [0., 0.], 0.).unwrap();
        let mask = rasterize(&key).unwrap();
        assert!(mask.pixels.iter().any(|p| *p > 0));
        for phase in [[0.25, 0.], [0., 0.25]] {
            let next = Key::new(&points, &stroke, false, 2., 1., phase, 0.).unwrap();
            assert!(key != next);
            assert_ne!(mask.pixels, rasterize(&next).unwrap().pixels);
        }
        let filled = Key::new(&points, &stroke, true, 2., 1., [0., 0.], 0.).unwrap();
        assert!(key != filled);
        let big = Key::new(&points, &stroke, false, 100_000., 1., [0., 0.], 0.).unwrap();
        assert!(rasterize(&big).is_err());
    }
}
