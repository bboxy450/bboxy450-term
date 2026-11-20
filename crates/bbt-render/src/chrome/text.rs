// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! CoreText owns line shaping, fallback and composed-character boundaries.
//! The retained glyph renderer preserves native color; this reference API returns coverage.
use crate::Result;
use bbt_ui_model::draw::TextStyle;

/// A verified face painted from retained logical text at an affine baseline.
/// `transform` maps baseline-relative SVG coordinates into target logical pixels.
pub struct FaceText<'a> {
    pub value: &'a str,
    /// Optional UTF-16 subrange of a shared shaping run, preserving kerning across tspans.
    pub range: Option<[usize; 2]>,
    pub postscript: &'a str,
    pub style: TextStyle,
    pub transform: bbt_ui_model::draw::Affine,
    pub color: bbt_ui_model::draw::Color,
}

pub struct LineMask {
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    /// Logical font metrics for baseline layout; the bitmap/render path is unchanged.
    pub ascent: f32,
    pub descent: f32,
    pub mask: Vec<u8>,
}

/// Logical CoreText advances and typographic bounds, with no bitmap or draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub advance: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
}

#[cfg(target_os = "macos")]
struct NativeMetrics {
    advance: f64,
    ascent: f64,
    descent: f64,
    leading: f64,
}
#[cfg(target_os = "macos")]
impl NativeMetrics {
    fn logical(&self) -> TextMetrics {
        TextMetrics {
            advance: self.advance as f32,
            ascent: self.ascent as f32,
            descent: self.descent as f32,
            leading: self.leading as f32,
        }
    }
}
#[cfg(target_os = "macos")]
fn typographic_bounds(line: &objc2_core_text::CTLine) -> NativeMetrics {
    let (mut ascent, mut descent, mut leading) = (0., 0., 0.);
    let advance = unsafe { line.typographic_bounds(&mut ascent, &mut descent, &mut leading) };
    NativeMetrics {
        advance,
        ascent,
        descent,
        leading,
    }
}

/// Shape chrome text without allocating a bitmap, reading coverage or drawing
/// glyphs. The same line construction and scalar conversions serve painting.
#[cfg(target_os = "macos")]
pub fn measure(text: &str, style: TextStyle, max_width: Option<f32>) -> Result<TextMetrics> {
    let line = logical_line(text, style, max_width)?;
    Ok(typographic_bounds(&line).logical())
}

/// Baseline-relative SVG text cells, measured by the same kerned line builder.
pub struct FaceLayout {
    pub metrics: TextMetrics,
    pub bounds: [f32; 4],
    pub characters: Vec<[f32; 2]>,
}
#[cfg(target_os = "macos")]
pub fn layout_face(
    value: &str,
    style: TextStyle,
    postscript: &str,
    range: Option<[usize; 2]>,
) -> Result<FaceLayout> {
    use objc2_core_text::CTFontOrientation;
    let (runs, metrics) = face_runs(value, style, postscript, range)?;
    let mut left = 0_f32;
    let mut right = metrics.advance;
    for run in &runs {
        for (&id, p) in run.glyphs.iter().zip(&run.positions) {
            let mut glyph = id;
            let ink = unsafe {
                run.font.bounding_rects_for_glyphs(
                    CTFontOrientation::Horizontal,
                    std::ptr::NonNull::from(&mut glyph),
                    std::ptr::null_mut(),
                    1,
                )
            };
            left = left.min((p.x + ink.origin.x) as f32);
            right = right.max((p.x + ink.origin.x + ink.size.width) as f32);
        }
    }
    let ascent = metrics.ascent.round();
    let descent = metrics.descent.round();
    let positions: std::collections::BTreeMap<_, _> = runs
        .iter()
        .flat_map(|run| {
            run.indices
                .iter()
                .copied()
                .zip(run.positions.iter().map(|p| p.x as f32))
        })
        .collect();
    let [start, end] = range.unwrap_or([0, value.encode_utf16().count()]);
    let mut index = start;
    let mut characters = Vec::new();
    for ch in value.chars().skip_while({
        let mut count = 0;
        move |ch| {
            let skip = count < start;
            count += ch.len_utf16();
            skip
        }
    }) {
        if index >= end {
            break;
        }
        let begin = positions
            .get(&(index as isize))
            .copied()
            .ok_or("character has no independent glyph in bounded SVG text")?;
        index += ch.len_utf16();
        let finish = positions
            .get(&(index as isize))
            .copied()
            .unwrap_or(metrics.advance);
        characters.push([begin, finish]);
    }
    Ok(FaceLayout {
        metrics,
        bounds: [left, -ascent, right - left, ascent + descent],
        characters,
    })
}
/// Slice actual glyph runs, never independently reshape a tspan at its boundary.
#[cfg(target_os = "macos")]
pub(super) fn face_runs(
    value: &str,
    style: TextStyle,
    postscript: &str,
    range: Option<[usize; 2]>,
) -> Result<(Vec<GlyphRun>, TextMetrics)> {
    let line = logical_line_for(value, style, None, Some(postscript))?;
    let mut metrics = typographic_bounds(&line).logical();
    let mut runs = glyph_runs(&line)?;
    if let Some([start, end]) = range {
        if start >= end || end > value.encode_utf16().count() {
            return Err("invalid face glyph range".into());
        }
        let positions: std::collections::BTreeMap<_, _> = runs
            .iter()
            .flat_map(|run| {
                run.indices
                    .iter()
                    .copied()
                    .zip(run.positions.iter().map(|p| p.x))
            })
            .collect();
        let origin = *positions
            .get(&(start as isize))
            .ok_or("tspan begins inside a glyph cluster")?;
        let finish = if end == value.encode_utf16().count() {
            f64::from(metrics.advance)
        } else {
            *positions
                .get(&(end as isize))
                .ok_or("tspan ends inside a glyph cluster")?
        };
        metrics.advance = (finish - origin) as f32;
        for run in &mut runs {
            let selected: Vec<_> = (0..run.glyphs.len())
                .filter(|&i| (start..end).contains(&(run.indices[i] as usize)))
                .collect();
            run.glyphs = selected.iter().map(|&i| run.glyphs[i]).collect();
            run.positions = selected
                .iter()
                .map(|&i| {
                    let mut p = run.positions[i];
                    p.x -= origin;
                    p
                })
                .collect();
            run.indices = selected.iter().map(|&i| run.indices[i]).collect();
        }
    }
    Ok((runs, metrics))
}
#[cfg(not(target_os = "macos"))]
pub fn layout_face(_: &str, _: TextStyle, _: &str, _: Option<[usize; 2]>) -> Result<FaceLayout> {
    Err("SVG text geometry requires CoreText".into())
}

/// Measure an explicitly verified face through the same CoreText line builder.
#[cfg(target_os = "macos")]
pub fn measure_face(text: &str, style: TextStyle, postscript: &str) -> Result<TextMetrics> {
    let line = logical_line_for(text, style, None, Some(postscript))?;
    Ok(typographic_bounds(&line).logical())
}
#[cfg(not(target_os = "macos"))]
pub fn measure_face(_: &str, _: TextStyle, _: &str) -> Result<TextMetrics> {
    Err("chrome text metrics require CoreText".into())
}
#[cfg(not(target_os = "macos"))]
pub fn measure(_: &str, _: TextStyle, _: Option<f32>) -> Result<TextMetrics> {
    Err("chrome text metrics require CoreText".into())
}

// The UI font has a CSS wght axis. Menlo uses CoreText's discrete faces.
#[cfg(target_os = "macos")]
fn upright_font(
    style: TextStyle,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CFType};
    use objc2_core_text::{
        CTFont, CTFontDescriptor, CTFontUIFontType, kCTFontFamilyNameAttribute,
        kCTFontTraitsAttribute, kCTFontWeightTrait,
    };
    // Optical size is in CSS points, independently of the backing scale.
    let size = f64::from(style.pixels());
    // CSS's discrete-face fallback chooses regular for Menlo 400..500.
    let base = unsafe {
        if style.mono {
            CTFont::with_name(&CFString::from_str("Menlo"), size, std::ptr::null())
        } else {
            CTFont::new_ui_font_for_language(CTFontUIFontType::System, size, None)
                .ok_or("system UI font unavailable")?
        }
    };
    if style.weight == 400 || (style.mono && (400..=500).contains(&style.weight)) {
        return Ok(base);
    }
    if !style.mono {
        // Named SF Medium/Semibold are wght=510/590. CSS requests the exact
        // variable weight (500/600); retaining the logical font also keeps opsz.
        let axis = CFNumber::new_i32(i32::from_be_bytes(*b"wght"));
        let descriptor = unsafe {
            base.font_descriptor()
                .copy_with_variation(&axis, f64::from(style.weight.clamp(100, 900)))
        };
        return Ok(unsafe { CTFont::with_font_descriptor(&descriptor, size, std::ptr::null()) });
    }
    let anchors = [-0.8, -0.6, -0.4, 0., 0.23, 0.3, 0.4, 0.56, 0.62];
    let w = style.weight.clamp(100, 900);
    let lower = usize::from((w / 100).saturating_sub(1));
    let upper = (lower + 1).min(8);
    let fraction = f64::from(w % 100) / 100.;
    let weight = CFNumber::new_f64(anchors[lower] + (anchors[upper] - anchors[lower]) * fraction);
    let weight_key = unsafe { kCTFontWeightTrait };
    let traits = CFDictionary::<CFString, CFNumber>::from_slices(&[weight_key], &[weight.as_ref()]);
    let family = CFString::from_str("Menlo");
    let values: [&CFType; 2] = [family.as_ref(), traits.as_ref()];
    let keys = unsafe { [kCTFontFamilyNameAttribute, kCTFontTraitsAttribute] };
    let attrs = CFDictionary::from_slices(&keys, &values);
    let descriptor = unsafe { CTFontDescriptor::with_attributes(attrs.as_opaque()) };
    Ok(unsafe { CTFont::with_font_descriptor(&descriptor, size, std::ptr::null()) })
}

/// Fonts are rebuilt only when (mono, italic, weight, size) changes. The cache is
/// per thread, so CTFont objects never cross threads through it (no Send wrapper);
/// it is bounded and simply cleared when full.
#[cfg(target_os = "macos")]
fn native_font(
    style: TextStyle,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    native_font_for(style, None)
}

#[cfg(target_os = "macos")]
fn native_font_for(
    style: TextStyle,
    postscript: Option<&str>,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    use objc2_core_foundation::CFRetained;
    use objc2_core_text::CTFont;
    use std::{cell::RefCell, collections::HashMap};
    const FONT_BUDGET: usize = 64;
    /// (mono, italic, weight, pixel size bits): every input build_font reads.
    type Fonts = HashMap<(Option<String>, bool, bool, u16, u32), CFRetained<CTFont>>;
    thread_local! {
        static FONTS: RefCell<Fonts> = RefCell::new(HashMap::new());
    }
    let key = (
        postscript.map(str::to_owned),
        style.mono,
        style.italic,
        style.weight,
        style.pixels().to_bits(),
    );
    if let Some(font) = FONTS.with_borrow(|fonts| fonts.get(&key).cloned()) {
        return Ok(font);
    }
    let font = if let Some(name) = postscript {
        if style.italic || style.mono || !style.pixels().is_finite() || style.pixels() <= 0. {
            return Err("invalid explicit-face text style".into());
        }
        let font = unsafe {
            CTFont::with_name(
                &objc2_core_foundation::CFString::from_str(name),
                f64::from(style.pixels()),
                std::ptr::null(),
            )
        };
        if unsafe { font.post_script_name() }.to_string() != name {
            return Err("explicit font face resolved through fallback".into());
        }
        font
    } else {
        build_font(style)?
    };
    FONTS.with_borrow_mut(|fonts| {
        if fonts.len() >= FONT_BUDGET {
            fonts.clear();
        }
        fonts.insert(key, font.clone());
    });
    Ok(font)
}

#[cfg(target_os = "macos")]
fn build_font(
    style: TextStyle,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    use objc2_core_foundation::{CFDictionary, CFNumber, CFType, CGAffineTransform};
    use objc2_core_text::{
        CTFont, CTFontSymbolicTraits, kCTFontSlantTrait, kCTFontSymbolicTrait,
        kCTFontTraitsAttribute,
    };
    let base = upright_font(style)?;
    if !style.italic {
        return Ok(base);
    }
    // Request a real slanted face numerically. Discrete families such as Menlo
    // may ignore that value; the same-family symbolic lookup is the next step.
    let slant = CFNumber::new_f64(0.2);
    let italic = CFNumber::new_i32(1);
    let keys = unsafe { [kCTFontSlantTrait, kCTFontSymbolicTrait] };
    let traits =
        CFDictionary::<_, CFNumber>::from_slices(&keys, &[slant.as_ref(), italic.as_ref()]);
    let values: [&CFType; 1] = [traits.as_ref()];
    let attrs = CFDictionary::from_slices(&[unsafe { kCTFontTraitsAttribute }], &values);
    let descriptor = unsafe {
        base.font_descriptor()
            .copy_with_attributes(attrs.as_opaque())
    };
    let numeric = unsafe {
        CTFont::with_font_descriptor(&descriptor, f64::from(style.pixels()), std::ptr::null())
    };
    let mut face =
        if unsafe { numeric.symbolic_traits() }.contains(CTFontSymbolicTraits::TraitItalic) {
            Some(numeric)
        } else {
            unsafe {
                base.copy_with_symbolic_traits(
                    0.,
                    std::ptr::null(),
                    CTFontSymbolicTraits::TraitItalic,
                    CTFontSymbolicTraits::TraitItalic,
                )
            }
        };
    face = face.filter(|font| {
        unsafe { font.symbolic_traits() }.contains(CTFontSymbolicTraits::TraitItalic)
    });
    if !style.mono
        && let Some(font) = face.as_ref()
    {
        // Selecting the real italic family must retain the exact CSS wght axis.
        let axis = CFNumber::new_i32(i32::from_be_bytes(*b"wght"));
        let descriptor = unsafe {
            font.font_descriptor()
                .copy_with_variation(&axis, f64::from(style.weight.clamp(100, 900)))
        };
        face = Some(unsafe {
            CTFont::with_font_descriptor(&descriptor, f64::from(style.pixels()), std::ptr::null())
        });
    }
    if let Some(font) = face {
        return Ok(font);
    }
    // No real family face exists. Synthetic oblique is confined to this fallback.
    let matrix = CGAffineTransform {
        a: 1.,
        b: 0.,
        c: 12f64.to_radians().tan(),
        d: 1.,
        tx: 0.,
        ty: 0.,
    };
    Ok(unsafe { base.copy_with_attributes(0., &matrix, None) })
}

/// Logical chrome font x-height; the terminal raster path never calls this.
#[cfg(target_os = "macos")]
pub fn x_height(style: TextStyle) -> Result<f32> {
    if !style.pixels().is_finite() || style.pixels() <= 0. {
        return Err("invalid chrome font size".into());
    }
    Ok(unsafe { native_font(style)?.x_height() } as f32)
}
#[cfg(not(target_os = "macos"))]
pub fn x_height(_: TextStyle) -> Result<f32> {
    Err("chrome font metrics require macOS".into())
}

#[cfg(target_os = "macos")]
pub fn rasterize(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
) -> Result<LineMask> {
    rasterize_at(text, style, max_width, scale, 0.)
}

/// CoreText owns logical shaping, fallback fonts and exact quarter-pixel origins.
#[cfg(target_os = "macos")]
pub(super) struct GlyphRun {
    pub font: objc2_core_foundation::CFRetained<objc2_core_text::CTFont>,
    pub glyphs: Vec<u16>,
    pub positions: Vec<objc2_core_foundation::CGPoint>,
    pub indices: Vec<isize>,
}
#[cfg(target_os = "macos")]
pub(super) struct ShapedLine {
    pub width: u32,
    pub height: u32,
    /// Device-pixel distance from the bitmap top to its CoreText baseline.
    pub baseline: f32,
    pub advance: f32,
    pub ascent: f32,
    pub descent: f32,
    pub scale: f32,
    pub runs: Vec<GlyphRun>,
}
#[cfg(target_os = "macos")]
fn logical_line(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTLine>> {
    logical_line_for(text, style, max_width, None)
}

#[cfg(target_os = "macos")]
pub(super) fn logical_line_for(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    postscript: Option<&str>,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTLine>> {
    use objc2_core_foundation::{
        CFAttributedString, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType,
    };
    use objc2_core_text::{
        CTLine, kCTFontAttributeName, kCTForegroundColorFromContextAttributeName,
        kCTKernAttributeName,
    };
    use std::ptr;
    let font = native_font_for(style, postscript)?;
    let kern = CFNumber::new_f64(f64::from(style.tracking_em * style.pixels()));
    let mut keys = unsafe {
        vec![
            kCTFontAttributeName,
            kCTForegroundColorFromContextAttributeName,
        ]
    };
    let mut values: Vec<&CFType> = vec![font.as_ref(), CFBoolean::new(true).as_ref()];
    // Explicit zero disables font kerning. Leave CoreText's default shaping in
    // charge unless a chrome tracking token deliberately supplies spacing.
    if style.tracking_em != 0. {
        keys.push(unsafe { kCTKernAttributeName });
        values.push(kern.as_ref());
    }
    let attrs = CFDictionary::from_slices(&keys, &values);
    let make = |s: &str| -> Result<CFRetained<CTLine>> {
        let string = CFString::from_str(s);
        let attributed =
            unsafe { CFAttributedString::new(None, Some(&string), Some(attrs.as_opaque())) }
                .ok_or("CoreText attributed string")?;
        Ok(unsafe { CTLine::with_attributed_string(&attributed) })
    };
    let measure = |line: &CTLine| unsafe {
        line.typographic_bounds(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) as f32
    };
    let mut value = if style.uppercase {
        text.to_uppercase()
    } else {
        text.to_owned()
    };
    let mut line = make(&value)?;
    if let Some(limit) = max_width.filter(|n| n.is_finite()) {
        let limit = limit.max(0.0);
        if measure(&line) > limit {
            // CoreFoundation supplies the same composed-sequence boundaries as
            // native shaping: never leave a half flag, modifier or ZWJ family.
            let native = CFString::from_str(&value);
            let units: Vec<_> = value.encode_utf16().collect();
            let mut chars = Vec::new();
            let mut index = 0;
            while index < units.len() {
                // SAFETY: index is inside this retained string's UTF-16 range.
                let range = unsafe { native.range_of_composed_characters_at_index(index as isize) };
                let end = (range.location + range.length) as usize;
                chars.push(String::from_utf16(&units[index..end]).map_err(|e| e.to_string())?);
                index = end;
            }
            let mut lo = 0;
            let mut hi = chars.len();
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                let trial = chars[..mid].concat() + "…";
                let trial_line = make(&trial)?;
                if measure(&trial_line) <= limit {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            value = chars[..lo].concat() + "…";
            line = make(&value)?;
            if measure(&line) > limit {
                line = make("")?;
            }
        }
    }
    Ok(line)
}

pub(super) fn quarter_pixel(value: f64) -> f64 {
    (value * 4.).round() / 4.
}

#[cfg(target_os = "macos")]
pub(super) fn shape_at(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
    phase_x: f32,
) -> Result<ShapedLine> {
    if !scale.is_finite()
        || !(0.5..=4.).contains(&scale)
        || !phase_x.is_finite()
        || !(0. ..1.).contains(&phase_x)
    {
        return Err("invalid chrome raster scale or phase".into());
    }
    let line = logical_line(text, style, max_width)?;
    // Keep the raster canvas arithmetic in its original f64 precision. Layout
    // consumes f32 fields, but rounding them before ceil could change coverage.
    let native = typographic_bounds(&line);
    let (ascent, descent) = (native.ascent, native.descent);
    let advance = native.advance as f32;
    // A shaped line is geometry, not a bitmap. The retained renderer rasterizes
    // bounded glyph/components and clips their quads to the viewport; its full
    // logical advance must survive horizontal scrolling of arbitrarily wide
    // tool output. Keep canvas limits at actual bitmap allocation sites.
    // Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let dimension = |pixels: f64, guard: u32| -> Result<u32> {
        if !pixels.is_finite() || pixels < 0. || pixels.ceil() > f64::from(i32::MAX as u32 - guard)
        {
            return Err("chrome text layout dimensions overflow".into());
        }
        Ok((pixels.ceil() as u32 + guard).max(1))
    };
    let width = dimension(f64::from(advance * scale), 5)?;
    let height = dimension((ascent + descent) * f64::from(scale), 4)?;
    let device_scale = f64::from(scale);
    let baseline = ((descent * device_scale).ceil() + 2.) / device_scale;
    let mut output = glyph_runs(&line)?;
    for run in &mut output {
        for position in &mut run.positions {
            position.x =
                quarter_pixel(position.x * device_scale + 2. + f64::from(phase_x)) / device_scale;
            position.y += baseline;
        }
    }
    Ok(ShapedLine {
        width,
        height,
        baseline: height as f32 - (baseline * device_scale) as f32,
        advance,
        ascent: ascent as f32,
        descent: descent as f32,
        scale,
        runs: output,
    })
}
#[cfg(target_os = "macos")]
pub(super) fn glyph_runs(line: &objc2_core_text::CTLine) -> Result<Vec<GlyphRun>> {
    use objc2_core_foundation::{
        CFArray, CFDictionary, CFRange, CFRetained, CFString, CFType, CGPoint,
    };
    use objc2_core_text::{CTFont, CTRun, kCTFontAttributeName};
    use std::ptr::NonNull;
    let mut output = Vec::new();
    // SAFETY: immutable CoreText runs, checked buffer lengths and retained fonts.
    unsafe {
        let runs = CFRetained::cast_unchecked::<CFArray<CTRun>>(line.glyph_runs());
        for run in runs.iter() {
            let count = run.glyph_count() as usize;
            if count == 0 {
                continue;
            }
            let mut glyphs = vec![0; count];
            let mut positions = vec![CGPoint::ZERO; count];
            let mut indices = vec![0; count];
            let range = CFRange::new(0, count as isize);
            run.glyphs(range, NonNull::new(glyphs.as_mut_ptr()).unwrap());
            run.positions(range, NonNull::new(positions.as_mut_ptr()).unwrap());
            run.string_indices(range, NonNull::new(indices.as_mut_ptr()).unwrap());
            let attrs =
                CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(run.attributes());
            let font = attrs
                .get(kCTFontAttributeName)
                .and_then(|v| v.downcast::<CTFont>().ok())
                .ok_or("CoreText run font missing")?;
            output.push(GlyphRun {
                font,
                glyphs,
                positions,
                indices,
            });
        }
    }
    Ok(output)
}
/// Grayscale chrome mask at a fractional device-pixel line origin in [0, 1).
/// The reference raster path and glyph-cache path share shaping and fallback.
#[cfg(target_os = "macos")]
pub fn rasterize_at(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
    phase_x: f32,
) -> Result<LineMask> {
    use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext};
    use std::ptr::NonNull;
    let line = shape_at(text, style, max_width, scale, phase_x)?;
    // This reference API does allocate one whole-line bitmap. Production text
    // uses text_glyphs, whose component allocation keeps the same native bounds.
    if line.width > 16_384 || line.height > 2_048 {
        return Err("chrome text line too large".into());
    }
    let color = line.runs.iter().any(|run| {
        unsafe { run.font.symbolic_traits() }
            .contains(objc2_core_text::CTFontSymbolicTraits::TraitColorGlyphs)
    });
    let channels = if color { 4 } else { 1 };
    let mut mask = vec![0; line.width as usize * line.height as usize * channels];
    let space = if color {
        CGColorSpace::with_name(Some(unsafe { objc2_core_graphics::kCGColorSpaceSRGB }))
    } else {
        CGColorSpace::new_device_gray()
    }
    .ok_or("chrome line colour space")?;
    // SAFETY: fixed channel-sized allocation; all native objects outlive the draw.
    let context = unsafe {
        CGBitmapContextCreate(
            mask.as_mut_ptr().cast(),
            line.width as usize,
            line.height as usize,
            8,
            line.width as usize * channels,
            Some(&space),
            if color { 0x2002 } else { 0 },
        )
    }
    .ok_or("chrome text bitmap")?;
    if color {
        CGContext::set_rgb_fill_color(Some(&context), 1., 1., 1., 1.);
    } else {
        CGContext::set_gray_fill_color(Some(&context), 1., 1.);
    }
    CGContext::set_should_antialias(Some(&context), true);
    CGContext::set_should_smooth_fonts(Some(&context), false);
    CGContext::set_allows_font_smoothing(Some(&context), false);
    CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
    CGContext::set_should_subpixel_position_fonts(Some(&context), true);
    CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
    CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);
    CGContext::scale_ctm(Some(&context), f64::from(line.scale), f64::from(line.scale));
    for run in &line.runs {
        // CTFontDrawGlyphs reads these immutable glyph and position arrays.
        unsafe {
            run.font.draw_glyphs(
                NonNull::new(run.glyphs.as_ptr().cast_mut()).unwrap(),
                NonNull::new(run.positions.as_ptr().cast_mut()).unwrap(),
                run.glyphs.len(),
                &context,
            );
        }
    }
    drop(context);
    if color {
        mask = mask.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
    }
    Ok(LineMask {
        width: line.width,
        height: line.height,
        advance: line.advance,
        ascent: line.ascent,
        descent: line.descent,
        mask,
    })
}
#[cfg(not(target_os = "macos"))]
pub fn rasterize(_: &str, _: TextStyle, _: Option<f32>, _: f32) -> Result<LineMask> {
    Err("chrome text currently requires CoreText".into())
}
#[cfg(not(target_os = "macos"))]
pub fn rasterize_at(_: &str, _: TextStyle, _: Option<f32>, _: f32, _: f32) -> Result<LineMask> {
    Err("chrome text currently requires CoreText".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    // keeps: spec svg-renderer — affine text uses the existing glyph cache, with logical kerning/metrics unchanged.
    #[cfg(target_os = "macos")]
    #[test]
    fn svg_affine_text_preserves_baseline_geometry_and_fresh_coverage() {
        use super::super::{ChromeRenderer, icons};
        use bbt_ui_model::draw::{Affine, Color, TypeScale};
        let style = TextStyle {
            scale: TypeScale::Custom(12.),
            ..Default::default()
        };
        let postscript = unsafe {
            native_font(TextStyle {
                mono: true,
                ..style
            })
            .unwrap()
            .post_script_name()
        }
        .to_string();
        let before = measure_face("AV", style, &postscript).unwrap();
        let mut r = ChromeRenderer::new(Vec::<icons::Image>::new()).unwrap();
        let paint = |r: &mut ChromeRenderer, transform| {
            r.render_to_rgba_with(64, 64, 1., |r, t, e| {
                r.draw_face_text(
                    &FaceText {
                        value: "AV",
                        range: None,
                        postscript: &postscript,
                        style,
                        transform,
                        color: Color::BLACK,
                    },
                    t,
                    e,
                )
            })
            .unwrap()
        };
        let a = paint(&mut r, Affine([1., 0., 0., 1., 8., 24.]));
        let b = paint(&mut r, Affine([0., 1., -1., 0., 32., 8.]));
        let bounds = |data: &[u8]| {
            let mut b = [64_i32, 64, 0, 0];
            for (i, p) in data.as_chunks::<4>().0.iter().enumerate() {
                if p[3] != 0 {
                    let x = (i % 64) as i32;
                    let y = (i / 64) as i32;
                    b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
                }
            }
            b
        };
        let a = bounds(&a);
        let b = bounds(&b);
        assert!(
            a[2] > a[0] && b[2] > b[0],
            "both transforms paint actual glyph coverage"
        );
        let expected = [56 - a[3], a[0], 56 - a[1], a[2]];
        assert!(
            b.into_iter().zip(expected).all(|(a, b)| (a - b).abs() <= 1),
            "rotated baseline/ink bounds: {b:?}, expected {expected:?}"
        );
        assert_eq!(before, measure_face("AV", style, &postscript).unwrap());
    }

    use super::*;
    // keeps: Unicode chrome ellipsis — native clusters are never split for available width.
    #[test]
    fn emoji_sequences_shape_as_one_native_glyph_and_truncate_only_between_clusters() {
        let style = TextStyle::default();
        for cluster in ["🦊", "👩🏽‍💻", "👨‍👩‍👧‍👦", "🇺🇦", "❤️", "1️⃣", "🏳️‍🌈"]
        {
            let shaped = shape_at(cluster, style, None, 2., 0.).unwrap();
            assert_eq!(
                shaped.runs.iter().map(|r| r.glyphs.len()).sum::<usize>(),
                1,
                "{cluster}"
            );
            assert!(
                shaped
                    .runs
                    .iter()
                    .all(|r| unsafe { r.font.symbolic_traits() }
                        .contains(objc2_core_text::CTFontSymbolicTraits::TraitColorGlyphs)),
                "{cluster}"
            );
            let limit = measure(&format!("{cluster}…"), style, None)
                .unwrap()
                .advance
                - 0.25;
            let actual = shape_at(&format!("{cluster}swarm"), style, Some(limit), 2., 0.).unwrap();
            let expected = shape_at("…", style, None, 2., 0.).unwrap();
            assert_eq!(
                actual
                    .runs
                    .iter()
                    .flat_map(|r| &r.glyphs)
                    .collect::<Vec<_>>(),
                expected
                    .runs
                    .iter()
                    .flat_map(|r| &r.glyphs)
                    .collect::<Vec<_>>(),
                "split cluster {cluster}"
            );
        }
    }

    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn native_fonts_are_reused_per_thread_and_keyed_by_every_input() {
        let style = TextStyle::default();
        let a = native_font(style).unwrap();
        let b = native_font(style).unwrap();
        assert!(std::ptr::eq(&*a, &*b), "same inputs reuse the font object");
        for other in [
            TextStyle {
                weight: 600,
                ..style
            },
            TextStyle {
                mono: true,
                ..style
            },
            TextStyle {
                italic: true,
                ..style
            },
            TextStyle {
                scale: bbt_ui_model::draw::TypeScale::Custom(21.),
                ..style
            },
        ] {
            assert_ne!(native_font(other).unwrap(), a);
        }
        assert_eq!(
            native_font(style).unwrap(),
            build_font(style).unwrap(),
            "cached equals fresh"
        );
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn measurement_matches_painted_metrics_bit_for_bit() {
        use bbt_ui_model::draw::TypeScale;
        let mut cases = 0;
        for size in [9., 12.5, 13.5, 19.] {
            for mono in [false, true] {
                for italic in [false, true] {
                    for weight in [400, 500, 600, 700] {
                        for tracking_em in [0., 0.08] {
                            for uppercase in [false, true] {
                                let style = TextStyle {
                                    scale: TypeScale::Custom(size),
                                    mono,
                                    italic,
                                    weight,
                                    tracking_em,
                                    uppercase,
                                };
                                for text in [
                                    "",
                                    "AV fi",
                                    "leading trailing ",
                                    "e\u{301} a\u{308}",
                                    "مرحبا",
                                    "বাংলা",
                                    "許可 漢字",
                                    "♔ ⌘1 🚀",
                                ] {
                                    for max in [None, Some(0.), Some(45.), Some(240.)] {
                                        let metrics = measure(text, style, max).unwrap();
                                        for scale in [1., 2.] {
                                            let painted =
                                                rasterize(text, style, max, scale).unwrap();
                                            assert_eq!(
                                                [metrics.advance, metrics.ascent, metrics.descent]
                                                    .map(f32::to_bits),
                                                [painted.advance, painted.ascent, painted.descent]
                                                    .map(f32::to_bits),
                                                "geometry changed: {text:?}, {style:?}, max={max:?}, scale={scale}"
                                            );
                                            cases += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 16_384);
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn measurement_does_not_require_a_raster_canvas() {
        let text = "W".repeat(10_000);
        let metrics = measure(&text, TextStyle::default(), None).unwrap();
        assert!(metrics.advance > 16_384.);
        assert!(metrics.ascent > 0. && metrics.descent >= 0.);
        assert!(rasterize(&text, TextStyle::default(), None, 1.).is_err());
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn italic_uses_real_family_faces_and_keeps_exact_ui_weight() {
        use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained};
        use objc2_core_text::CTFontSymbolicTraits;
        for mono in [false, true] {
            for weight in [400, 500, 600, 700] {
                let style = TextStyle {
                    mono,
                    weight,
                    italic: true,
                    ..Default::default()
                };
                let font = native_font(style).unwrap();
                let matrix = unsafe { font.matrix() };
                eprintln!(
                    "italic mono={mono} weight={weight} name={} angle={} matrix={matrix:?}",
                    unsafe { font.post_script_name() },
                    unsafe { font.slant_angle() }
                );
                assert!(
                    unsafe { font.symbolic_traits() }.contains(CTFontSymbolicTraits::TraitItalic)
                );
                assert_ne!(unsafe { font.slant_angle() }, 0.);
                assert_eq!(
                    [matrix.a, matrix.b, matrix.c, matrix.d],
                    [1., 0., 0., 1.],
                    "true face, not synthetic oblique"
                );
                if !mono {
                    let variation = unsafe {
                        CFRetained::cast_unchecked::<CFDictionary<CFNumber, CFNumber>>(
                            font.variation().unwrap(),
                        )
                    };
                    assert_eq!(
                        variation
                            .get(&CFNumber::new_i32(i32::from_be_bytes(*b"wght")))
                            .and_then(|v| v.as_f64())
                            .or((weight == 400).then_some(400.)),
                        Some(f64::from(weight))
                    );
                }
                let upright = rasterize(
                    "Hamburgefonts AV fi",
                    TextStyle {
                        italic: false,
                        ..style
                    },
                    None,
                    1.,
                )
                .unwrap();
                let italic = rasterize("Hamburgefonts AV fi", style, None, 1.).unwrap();
                assert_ne!(upright.mask, italic.mask);
                let retina = rasterize("Hamburgefonts AV fi", style, None, 2.).unwrap();
                assert_eq!(
                    [italic.advance, italic.ascent, italic.descent],
                    [retina.advance, retina.ascent, retina.descent]
                );
            }
        }
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn optical_size_and_logical_shaping_do_not_change_with_backing_scale() {
        use bbt_ui_model::draw::TypeScale;
        for mono in [false, true] {
            for weight in [400, 500, 600, 700] {
                let style = TextStyle {
                    mono,
                    weight,
                    scale: TypeScale::Custom(16.),
                    ..Default::default()
                };
                let a = rasterize("Hamburgefonts AV fi", style, None, 1.).unwrap();
                let b = rasterize("Hamburgefonts AV fi", style, None, 2.).unwrap();
                assert_eq!(
                    [a.advance, a.ascent, a.descent],
                    [b.advance, b.ascent, b.descent]
                );
                assert!(b.width >= a.width * 2 - 6 && b.height >= a.height * 2 - 4);
            }
        }
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn ui_weights_use_css_axis_values_and_menlo_medium_uses_regular_face() {
        use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained};
        for weight in [500, 600] {
            let font = native_font(TextStyle {
                weight,
                ..Default::default()
            })
            .unwrap();
            let variation = unsafe {
                CFRetained::cast_unchecked::<CFDictionary<CFNumber, CFNumber>>(
                    font.variation().unwrap(),
                )
            };
            let w = variation
                .get(&CFNumber::new_i32(i32::from_be_bytes(*b"wght")))
                .unwrap();
            assert_eq!(w.as_f64(), Some(f64::from(weight)));
        }
        let mono = |weight| {
            native_font(TextStyle {
                mono: true,
                weight,
                ..Default::default()
            })
            .unwrap()
        };
        assert_eq!(unsafe { mono(400).post_script_name() }, unsafe {
            mono(500).post_script_name()
        });
        assert_ne!(unsafe { mono(400).post_script_name() }, unsafe {
            mono(600).post_script_name()
        });
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn quarter_pixel_phases_change_coverage_without_changing_logical_metrics() {
        let a = rasterize_at("AV fi", TextStyle::default(), None, 1., 0.).unwrap();
        for phase in [0.25, 0.5, 0.75] {
            let b = rasterize_at("AV fi", TextStyle::default(), None, 1., phase).unwrap();
            assert_eq!(
                [a.advance, a.ascent, a.descent],
                [b.advance, b.ascent, b.descent]
            );
            assert_ne!(a.mask, b.mask);
        }
        assert!(rasterize_at("a", TextStyle::default(), None, 1., f32::NAN).is_err());
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn numeric_ui_weights_select_medium_and_semibold_between_regular_and_bold() {
        use bbt_ui_model::draw::TypeScale;
        let widths = [400, 500, 600, 700].map(|weight| {
            rasterize(
                "disabled",
                TextStyle {
                    scale: TypeScale::Custom(12.5),
                    weight,
                    ..Default::default()
                },
                None,
                1.,
            )
            .unwrap()
            .advance
        });
        for pair in widths.windows(2) {
            assert!(
                pair[1] > pair[0] + 0.5,
                "distinct UI weight advances: {widths:?}"
            );
        }
        assert!(widths[1] < widths[3] - 0.5 && widths[2] > widths[0] + 0.5);
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn zero_tracking_keeps_font_kerning_and_nonzero_tracking_is_applied() {
        use bbt_ui_model::draw::TypeScale;
        let style = TextStyle {
            scale: TypeScale::Custom(16.),
            ..Default::default()
        };
        let pair = rasterize("AV", style, None, 1.).unwrap().advance;
        let separate = ["A", "V"]
            .into_iter()
            .map(|letter| rasterize(letter, style, None, 1.).unwrap().advance)
            .sum::<f32>();
        assert!(
            pair < separate - 0.1,
            "font kerning must survive default tracking"
        );
        let tracked = rasterize(
            "AV",
            TextStyle {
                tracking_em: 0.2,
                ..style
            },
            None,
            1.,
        )
        .unwrap()
        .advance;
        assert!(
            tracked > separate + 3.,
            "nonzero token must add letter spacing"
        );
    }
    // keeps: spec renderer §1a and text-rendering Recommendation — native font metrics, shaping and coverage
    #[test]
    fn coretext_shapes_unicode_and_ellipsizes_at_the_given_width() {
        let style = TextStyle::default();
        let wide = rasterize("WWW", style, None, 1.0).unwrap();
        let narrow = rasterize("iii", style, None, 1.0).unwrap();
        assert!(wide.advance > narrow.advance * 2.0);
        let unicode = rasterize("♔ Claude ⌘1 · 許可 ⚿", style, None, 1.0).unwrap();
        assert!(unicode.mask.iter().any(|v| *v > 0));
        let clipped = rasterize("Mireille Bellweather-Lockridge", style, Some(45.0), 1.0).unwrap();
        assert!(clipped.advance <= 45.01);
    }
}
