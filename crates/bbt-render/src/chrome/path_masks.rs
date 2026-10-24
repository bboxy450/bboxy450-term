//! Generic vector coverage for the shared R8 shadow atlas. No UI mark definitions
//! live here: the caller supplies the same path, fill and stroke as foreground.
use super::text_glyphs::Mask;
use crate::Result;
use bbt_ui_model::draw::{LineCap, LineJoin, PathCommand, PathStroke};
use resvg::tiny_skia as sk;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Command {
    Move([u32; 2]),
    Line([u32; 2]),
    Cubic([[u32; 2]; 3]),
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
        })
    }
}

pub(super) fn rasterize(key: &Key) -> Result<Mask> {
    let empty = || Mask {
        bounds: [0; 4],
        merge_bounds: [0; 4],
        pixels: vec![],
    };
    let scale = f32::from_bits(key.scale) * f32::from_bits(key.device_scale);
    let phase = key.phase.map(f32::from_bits);
    let point =
        |p: [u32; 2]| std::array::from_fn::<_, 2, _>(|i| f32::from_bits(p[i]) * scale + phase[i]);
    let mut builder = sk::PathBuilder::new();
    for command in &key.commands {
        match *command {
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
    // The control hull is conservative; a four-half-width miter plus two guard
    // pixel bounds the same default miter limit used by the foreground path.
    let b = path.bounds();
    let pad = width * 2. + 2.;
    let extent = [
        (b.left() - pad).floor(),
        (b.top() - pad).floor(),
        (b.right() + pad).ceil(),
        (b.bottom() + pad).ceil(),
    ];
    if extent
        .iter()
        .any(|v| !v.is_finite() || v.abs() > i32::MAX as f32 / 2.)
        || extent[2] - extent[0] > 16_384.
        || extent[3] - extent[1] > 2_048.
    {
        return Err("path shadow mask exceeds coverage budget".into());
    }
    let [left, top, right, bottom] = extent.map(|v| v as i32);
    let mut pixmap = sk::Pixmap::new((right - left) as u32, (bottom - top) as u32)
        .ok_or("invalid path shadow coverage dimensions")?;
    let mut paint = sk::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    let transform = sk::Transform::from_translate(-left as f32, -top as f32);
    if key.filled {
        pixmap.fill_path(&path, &paint, sk::FillRule::Winding, transform, None);
    }
    if width > 0. {
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
            sk::StrokeDash::new(intervals, 0.)
        };
        let stroke = sk::Stroke {
            width,
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
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
    }
    // The shared text atlas uses y-up bounds and top-down scanlines.
    let bounds = [left, -bottom, right, -top];
    Ok(Mask {
        bounds,
        merge_bounds: bounds,
        pixels: pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3])
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
