// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Swash text for targets without CoreText. Layout stays in logical pixels.
use super::{FontConfig, FontMetrics, Format, RasterizedGlyph, terminal::Glyph};
use crate::{
    Result,
    chrome::text::{LineMask, TextMetrics},
};
use bbt_ui_model::draw::TextStyle;
use std::{cell::RefCell, sync::LazyLock};
use swash::{
    FontRef, GlyphId,
    scale::{Render, ScaleContext, Source, StrikeWith, image::Content},
    shape::ShapeContext,
    text::{Codepoint, Script},
    zeno::{Angle, Transform, Vector},
};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) const FAMILY: &str = "JetBrains Mono";
const FACE_NAME: &str = "JetBrainsMono-Regular";
// Copied from JetBrains Mono: https://github.com/JetBrains/JetBrainsMono
// The unmodified font and its OFL-1.1 notice are checked together by the license gate.
static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        include_bytes!("../../tests/fonts/JetBrainsMono-Regular.ttf"),
        0,
    )
    .expect("bundled JetBrains Mono font")
});

thread_local! {
    static SHAPER: RefCell<ShapeContext> = RefCell::new(ShapeContext::new());
    static SCALER: RefCell<ScaleContext> = RefCell::new(ScaleContext::new());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Face {
    size_bits: u32,
    weight: u16,
    italic: bool,
}

impl Face {
    fn new(size: f32, weight: u16, italic: bool) -> Self {
        Self {
            size_bits: size.to_bits(),
            weight,
            italic,
        }
    }

    fn size(self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    pub(crate) fn bitmap(self, id: GlyphId, position: [f64; 2], scale: f32) -> Option<Bitmap> {
        let size = self.size() * scale;
        let origin = position.map(|v| v.floor() as i32);
        let [x, y] = std::array::from_fn(|i| (position[i] - f64::from(origin[i])) as f32);
        let transform = self
            .italic
            .then(|| Transform::skew(Angle::from_degrees(12.), Angle::ZERO));
        let strength = size * f32::from(self.weight.clamp(400, 900) - 400) / 10_000.;
        let image = SCALER.with_borrow_mut(|cx| {
            let mut scaler = cx.builder(*FONT).size(size).build();
            Render::new(&[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ])
            .offset(Vector::new(x, y))
            .embolden(strength)
            .transform(transform)
            .render(&mut scaler, id)
        })?;
        let p = image.placement;
        if p.width == 0 || p.height == 0 {
            return None;
        }
        let left = origin[0] + p.left;
        let top = origin[1] + p.top;
        let mut pixels = image.data;
        let format = if image.content == Content::Color {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
                // Swash composites colour outlines into premultiplied RGBA.
                // Its decoded colour PNG strikes carry straight RGBA.
                if matches!(image.source, Source::ColorBitmap(_)) {
                    let a = u16::from(pixel[3]);
                    for channel in &mut pixel[..3] {
                        *channel = ((u16::from(*channel) * a + 127) / 255) as u8;
                    }
                }
            }
            Format::Bgra
        } else {
            Format::Mask
        };
        Some(Bitmap {
            bounds: [left, top - p.height as i32, left + p.width as i32, top],
            format,
            pixels,
        })
    }
}

pub(crate) struct Bitmap {
    /// Left, bottom, right, top in device coordinates, with y pointing up.
    pub bounds: [i32; 4],
    pub format: Format,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    pub(crate) fn composite(bounds: [i32; 4], format: Format, sources: &[Self]) -> Self {
        let [left, bottom, right, top] = bounds;
        let width = (right - left) as usize;
        let channels = format.channels();
        let mut pixels = vec![0; width * (top - bottom) as usize * channels];
        for glyph in sources {
            let [x0, y0, x1, y1] = glyph.bounds;
            let stride = (x1 - x0) as usize * glyph.format.channels();
            for y in y0.max(bottom)..y1.min(top) {
                for x in x0.max(left)..x1.min(right) {
                    let src = (y1 - 1 - y) as usize * stride
                        + (x - x0) as usize * glyph.format.channels();
                    let dst = ((top - 1 - y) as usize * width + (x - left) as usize) * channels;
                    let color = match glyph.format {
                        Format::Bgra => glyph.pixels[src..src + 4].try_into().unwrap(),
                        Format::Mask => [glyph.pixels[src]; 4],
                    };
                    let inverse_alpha = 255 - u16::from(color[3]);
                    let source = if format == Format::Mask {
                        &color[3..]
                    } else {
                        &color[..]
                    };
                    for (out, &input) in pixels[dst..dst + channels].iter_mut().zip(source) {
                        *out = input + ((u16::from(*out) * inverse_alpha + 127) / 255) as u8;
                    }
                }
            }
        }
        Self {
            bounds,
            format,
            pixels,
        }
    }
}

struct Positioned {
    id: GlyphId,
    x: f32,
    y: f32,
}

fn shape(text: &str, size: f32, tracking: f32, terminal: bool) -> (f32, Vec<Positioned>) {
    let mut advance = 0.;
    let mut glyphs = Vec::new();
    // This face covers Latin, Greek and Cyrillic, all left-to-right. Common
    // and inherited characters stay with the surrounding script.
    let mut script = None;
    let runs = text.char_indices().filter_map(|(at, ch)| {
        let next = ch.script();
        if !next.is_real() || script == Some(next) {
            return None;
        }
        Some((at, script.replace(next)?))
    });
    let mut start = 0;
    let mut run = |end, script| {
        SHAPER.with_borrow_mut(|cx| {
            let mut builder = cx.builder(*FONT).script(script).size(size);
            if terminal {
                builder = builder.features([("liga", 0), ("clig", 0), ("calt", 0)]);
            }
            let mut shaper = builder.build();
            shaper.add_str(&text[start..end]);
            shaper.shape_with(|cluster| {
                for glyph in cluster.glyphs {
                    glyphs.push(Positioned {
                        id: glyph.id,
                        x: advance + glyph.x,
                        y: glyph.y,
                    });
                    advance += glyph.advance;
                }
                if !cluster.is_empty() {
                    advance += tracking;
                }
            });
        });
        start = end;
    };
    for (end, previous) in runs {
        run(end, previous);
    }
    run(text.len(), script.unwrap_or(Script::Latin));
    (advance.max(0.), glyphs)
}

fn chrome_size(style: TextStyle) -> Result<f32> {
    let size = style.pixels();
    if size <= 0. || size > 512. || !size.is_finite() || !(style.tracking_em * size).is_finite() {
        return Err("invalid chrome font size or tracking".into());
    }
    Ok(size)
}

fn chrome_line(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
) -> Result<(TextMetrics, Vec<Positioned>)> {
    let size = chrome_size(style)?;
    let tracking = style.tracking_em * size;
    let shape = |text: &str| shape(text, size, tracking, false);
    let uppercase;
    let text = if style.uppercase {
        uppercase = text.to_uppercase();
        &uppercase
    } else {
        text
    };
    let (mut advance, mut glyphs) = shape(text);
    if let Some(limit) = max_width.filter(|n| n.is_finite()).map(|n| n.max(0.))
        && advance > limit
    {
        let ellipsis = shape("…");
        (advance, glyphs) = if ellipsis.0 <= limit {
            let mut ends: Vec<_> = text.grapheme_indices(true).map(|(i, _)| i).collect();
            ends.push(text.len());
            let (mut lo, mut hi) = (0, ends.len() - 1);
            let mut best = ellipsis;
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                let trial = shape(&format!("{}…", &text[..ends[mid]]));
                if trial.0 <= limit {
                    lo = mid;
                    best = trial;
                } else {
                    hi = mid - 1;
                }
            }
            best
        } else {
            (0., Vec::new())
        };
    }
    let metrics = FONT.metrics(&[]).scale(size);
    Ok((
        TextMetrics {
            advance,
            ascent: metrics.ascent,
            descent: metrics.descent,
            leading: metrics.leading,
        },
        glyphs,
    ))
}

pub fn measure(text: &str, style: TextStyle, max_width: Option<f32>) -> Result<TextMetrics> {
    Ok(chrome_line(text, style, max_width)?.0)
}

pub fn x_height(style: TextStyle) -> Result<f32> {
    Ok(FONT.metrics(&[]).scale(chrome_size(style)?).x_height)
}

pub(crate) struct Line {
    pub width: u32,
    pub height: u32,
    pub baseline: f32,
    pub metrics: TextMetrics,
    pub face: Face,
    pub glyphs: Vec<(GlyphId, [f64; 2])>,
}

pub(crate) fn shape_at(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
    phase: f32,
) -> Result<Line> {
    if !(0.5..=4.).contains(&scale) || !(0. ..1.).contains(&phase) {
        return Err("invalid chrome raster scale or phase".into());
    }
    let (metrics, glyphs) = chrome_line(text, style, max_width)?;
    let width = (metrics.advance * scale).ceil();
    if !width.is_finite() || width > (i32::MAX / 2) as f32 {
        return Err("chrome text layout dimensions overflow".into());
    }
    let height = ((metrics.ascent + metrics.descent) * scale).ceil() as u32 + 4;
    let baseline = (metrics.descent * scale).ceil() + 2.;
    let mut placed = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        let x = ((glyph.x * scale + 2. + phase) * 4.).round() / 4.;
        if x.abs() > (i32::MAX / 2) as f32 || !x.is_finite() {
            return Err("chrome text layout dimensions overflow".into());
        }
        placed.push((
            glyph.id,
            [f64::from(x), f64::from(glyph.y * scale + baseline)],
        ));
    }
    Ok(Line {
        width: width as u32 + 5,
        height,
        baseline: height as f32 - baseline,
        metrics,
        face: Face::new(style.pixels(), style.weight, style.italic),
        glyphs: placed,
    })
}

pub fn rasterize(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
) -> Result<LineMask> {
    rasterize_at(text, style, max_width, scale, 0.)
}

pub fn rasterize_at(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
    phase: f32,
) -> Result<LineMask> {
    let line = shape_at(text, style, max_width, scale, phase)?;
    if line.width > 16_384 || line.height > 2_048 {
        return Err("chrome text line too large".into());
    }
    let glyphs: Vec<_> = line
        .glyphs
        .iter()
        .filter_map(|&(id, p)| line.face.bitmap(id, p, scale))
        .collect();
    let mask = Bitmap::composite(
        [0, 0, line.width as i32, line.height as i32],
        Format::Mask,
        &glyphs,
    );
    Ok(LineMask {
        width: line.width,
        height: line.height,
        advance: line.metrics.advance,
        ascent: line.metrics.ascent,
        descent: line.metrics.descent,
        mask: mask.pixels,
    })
}

pub struct Font {
    face: Face,
    metrics: FontMetrics,
}

fn valid_config(config: &FontConfig) -> bool {
    (1. ..=256.).contains(&config.size) && (1..=4).contains(&config.scale)
}

fn cell_width(size: f32) -> f32 {
    FONT.glyph_metrics(&[])
        .scale(size)
        .advance_width(FONT.charmap().map('M'))
}

impl Font {
    pub fn new(config: &FontConfig) -> Result<Self> {
        if !valid_config(config) {
            return Err("invalid font size/scale".into());
        }
        let scale = config.scale as f32;
        let size = config.size * scale;
        let m = FONT.metrics(&[]).scale(size);
        Ok(Self {
            face: Face::new(size, 400, false),
            metrics: FontMetrics {
                cell_width: cell_width(size).ceil().max(1.) as u32,
                cell_height: (m.ascent + m.descent + m.leading + 2. * scale).ceil() as u32,
                baseline: (m.ascent.ceil() + scale) as i16,
            },
        })
    }

    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    pub fn effective_name(&self) -> String {
        FACE_NAME.into()
    }

    pub fn rasterize(&self, ch: char) -> Result<Option<RasterizedGlyph>> {
        if !ch.is_ascii() || ch.is_ascii_control() {
            return Err("legacy HeadlessRenderer fixture font accepts printable ASCII only (current terminal path: cells::terminal)".into());
        }
        Ok(self
            .face
            .bitmap(FONT.charmap().map(ch), [0.; 2], 1.)
            .map(|glyph| {
                let [left, bottom, right, top] = glyph.bounds;
                RasterizedGlyph {
                    left: left as i16,
                    top: top as i16,
                    width: (right - left) as u32,
                    height: (top - bottom) as u32,
                    mask: match glyph.format {
                        Format::Mask => glyph.pixels,
                        Format::Bgra => glyph.pixels.into_iter().skip(3).step_by(4).collect(),
                    },
                }
            }))
    }
}

pub struct Fonts {
    metrics: FontMetrics,
    size: f32,
    scale: f32,
    thicken: bool,
}

impl Fonts {
    pub fn new(config: &FontConfig, line_height: Option<f32>, thicken: bool) -> Result<Self> {
        let line_height = line_height.unwrap_or(1.);
        if !valid_config(config) || !(0.5..=4.).contains(&line_height) {
            return Err("invalid terminal font size, scale or line height".into());
        }
        let scale = config.scale as f32;
        let size = config.size * scale;
        let m = FONT.metrics(&[]).scale(size);
        let cell_height = ((m.ascent + m.descent + m.leading) * line_height)
            .round()
            .max(1.) as u32;
        Ok(Self {
            metrics: FontMetrics {
                cell_width: cell_width(size).round().max(1.) as u32,
                cell_height,
                baseline: ((m.ascent - m.descent + cell_height as f32) * 0.5).round() as i16,
            },
            size,
            scale,
            thicken,
        })
    }

    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    pub fn rasterize(
        &self,
        text: &str,
        bold: bool,
        italic: bool,
        span: u8,
    ) -> Result<Option<Glyph>> {
        if text.chars().all(char::is_whitespace) {
            return Ok(None);
        }
        if !(1..=2).contains(&span) {
            return Err("glyph span must be one or two cells".into());
        }
        let (advance, glyphs) = shape(text, self.size, 0., true);
        let width = (self.metrics.cell_width * u32::from(span)) as i32;
        let height = self.metrics.cell_height as i32;
        let x = (width as f32 - advance) * 0.5;
        let y = height - i32::from(self.metrics.baseline);
        let weight = if bold {
            700
        } else if self.thicken {
            500
        } else {
            400
        };
        let face = Face::new(self.size, weight, italic);
        let glyphs: Vec<_> = glyphs
            .into_iter()
            .filter_map(|glyph| {
                face.bitmap(
                    glyph.id,
                    [f64::from(glyph.x + x), f64::from(glyph.y) + f64::from(y)],
                    1.,
                )
            })
            .collect();
        let format = if glyphs.iter().any(|g| g.format == Format::Bgra) {
            Format::Bgra
        } else {
            Format::Mask
        };
        let pad = if format == Format::Mask {
            (2. * self.scale) as i32
        } else {
            0
        };
        let bitmap = Bitmap::composite([-pad, -pad, width + pad, height + pad], format, &glyphs);
        Ok(Some(Glyph {
            width: (width + 2 * pad) as u32,
            height: (height + 2 * pad) as u32,
            left: -pad as i16,
            top: -pad as i16,
            format,
            pixels: bitmap.pixels,
            faces: vec![FACE_NAME.into()],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_swash_line_and_terminal_glyphs_have_ink() {
        let style = TextStyle::default();
        let line = rasterize("Hello, swarm", style, None, 1.).unwrap();
        assert_eq!(line.mask.len(), (line.width * line.height) as usize);
        assert!(line.mask.iter().any(|p| *p > 0));
        assert!(line.advance > 5. * style.pixels() && line.advance < 12. * style.pixels());
        assert!(line.ascent > line.descent && line.descent > 0.);

        let config = FontConfig::default();
        let legacy = Font::new(&config).unwrap();
        assert!(
            legacy
                .rasterize('H')
                .unwrap()
                .unwrap()
                .mask
                .iter()
                .any(|p| *p > 0)
        );
        let terminal = Fonts::new(&config, None, false).unwrap();
        let cell = terminal
            .rasterize("e\u{301}", true, true, 1)
            .unwrap()
            .unwrap();
        assert_eq!(cell.width, terminal.metrics().cell_width + 4);
        assert_eq!(cell.format, Format::Mask);
        assert!(cell.pixels.iter().any(|p| *p > 0));
    }
}
