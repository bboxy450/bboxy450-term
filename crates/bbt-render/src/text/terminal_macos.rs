// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{Format, Glyph};
use crate::{FontConfig, FontMetrics, Result};
use objc2_core_foundation::{
    CFArray, CFAttributedString, CFBoolean, CFDictionary, CFNumber, CFRange, CFRetained, CFString,
    CFType, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext, kCGColorSpaceSRGB};
use objc2_core_text::{
    CTFont, CTFontOrientation, CTFontSymbolicTraits, CTLine, CTRun, kCTFontAttributeName,
    kCTForegroundColorFromContextAttributeName, kCTLigatureAttributeName,
};
use std::ptr::{self, NonNull};

pub struct Fonts {
    faces: [CFRetained<CTFont>; 4],
    symbols: CFRetained<CTFont>,
    cjk: CFRetained<CTFont>,
    emoji: CFRetained<CTFont>,
    metrics: FontMetrics,
    scale: f64,
    thicken: bool,
}

// SAFETY: Fonts stores only immutable CTFont handles plus plain metrics/scalars.
// Apple permits font objects across operations, work queues and threads:
// https://developer.apple.com/documentation/coretext
// CTLine, CTRun and CGContext are call-local in rasterize and are never stored.
// The exhaustive guard below forces review when any stored field changes.
unsafe impl Send for Fonts {}

const _: fn(Fonts) = |fonts| {
    // No `..`: adding a field must fail here until its thread safety is audited.
    let Fonts {
        faces,
        symbols,
        cjk,
        emoji,
        metrics,
        scale,
        thicken,
    } = fonts;
    let _: [CFRetained<CTFont>; 4] = faces;
    let _: CFRetained<CTFont> = symbols;
    let _: CFRetained<CTFont> = cjk;
    let _: CFRetained<CTFont> = emoji;
    fn assert_send<T: Send>(_: T) {}
    assert_send((metrics, scale, thicken));
};

fn named(name: &str, size: f64) -> CFRetained<CTFont> {
    // SAFETY: a retained string, positive validated size and identity transform.
    unsafe { CTFont::with_name(&CFString::from_str(name), size, ptr::null()) }
}

fn covers(font: &CTFont, text: &str) -> bool {
    let chars: Vec<u16> = text.encode_utf16().collect();
    if chars.is_empty() {
        return true;
    }
    let mut glyphs = vec![0; chars.len()];
    // SAFETY: both buffers have count elements and outlive the synchronous call.
    unsafe {
        font.glyphs_for_characters(
            NonNull::new(chars.as_ptr().cast_mut()).unwrap(),
            NonNull::new(glyphs.as_mut_ptr()).unwrap(),
            chars.len() as isize,
        )
    }
}

impl Fonts {
    pub fn new(config: &FontConfig, line_height: Option<f32>, thicken: bool) -> Result<Self> {
        if !config.size.is_finite()
            || !(1.0..=256.0).contains(&config.size)
            || !(1..=4).contains(&config.scale)
            || line_height.is_some_and(|v| !v.is_finite() || !(0.5..=4.0).contains(&v))
        {
            return Err("invalid terminal font size, scale or line height".into());
        }
        let size = f64::from(config.size);
        let mut regular = named(&config.family, size);
        let actual = unsafe { regular.family_name() }.to_string().to_lowercase();
        if !actual
            .replace(' ', "")
            .contains(&config.family.to_lowercase().replace(' ', ""))
        {
            regular = named("Menlo", size);
        }
        let styled = |bold: bool, italic: bool| {
            let mut traits = CTFontSymbolicTraits::empty();
            if bold {
                traits |= CTFontSymbolicTraits::TraitBold;
            }
            if italic {
                traits |= CTFontSymbolicTraits::TraitItalic;
            }
            // CoreText selects same-family faces; synthetic fallback is applied at draw time.
            unsafe { regular.copy_with_symbolic_traits(0., ptr::null(), traits, traits) }
                .unwrap_or_else(|| regular.clone())
        };
        let faces = [
            regular.clone(),
            styled(true, false),
            styled(false, true),
            styled(true, true),
        ];
        let mut code = b'M' as u16;
        let mut glyph = 0;
        unsafe {
            regular.glyphs_for_characters(NonNull::from(&mut code), NonNull::from(&mut glyph), 1);
        }
        let advance = unsafe {
            regular.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut glyph),
                ptr::null_mut(),
                1,
            )
        };
        let ascent = unsafe { regular.ascent() };
        let descent = unsafe { regular.descent() };
        let leading = unsafe { regular.leading() };
        let scale = f64::from(config.scale);
        let logical_height = (ascent + descent + leading) * f64::from(line_height.unwrap_or(1.));
        let cell_height = (logical_height * scale).round().max(1.) as u32;
        let baseline = (ascent * scale
            + (f64::from(cell_height) - (ascent + descent) * scale) * 0.5)
            .round() as i16;
        Ok(Self {
            faces,
            symbols: named("Apple Symbols", size),
            cjk: named("PingFang SC", size),
            emoji: named("Apple Color Emoji", size),
            metrics: FontMetrics {
                cell_width: (advance * scale).round().max(1.) as u32,
                cell_height,
                baseline,
            },
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
        if text.is_empty() || text.chars().all(char::is_whitespace) {
            return Ok(None);
        }
        if span == 0 || span > 2 {
            return Err("glyph span must be one or two cells".into());
        }
        let primary = &self.faces[usize::from(bold) + 2 * usize::from(italic)];
        let emoji = text
            .chars()
            .any(|c| matches!(c as u32, 0x1f000..=0x1faff | 0xfe0f));
        let font = if emoji {
            &self.emoji
        } else if covers(primary, text) {
            primary
        } else if span == 2 && covers(&self.cjk, text) {
            // Fullwidth forms exist in Apple Symbols, but its small symbol
            // designs are unsuitable for a two-cell CJK fallback.
            &self.cjk
        } else if covers(&self.symbols, text) {
            &self.symbols
        } else if covers(&self.cjk, text) {
            &self.cjk
        } else {
            primary
        }; // CTLine retains CoreText's locale-aware final fallback.
        let ligatures = CFNumber::new_i32(0);
        let keys = unsafe {
            [
                kCTFontAttributeName,
                kCTForegroundColorFromContextAttributeName,
                kCTLigatureAttributeName,
            ]
        };
        let values: [&CFType; 3] = [
            font.as_ref(),
            CFBoolean::new(true).as_ref(),
            ligatures.as_ref(),
        ];
        let attrs = CFDictionary::from_slices(&keys, &values);
        let string = CFString::from_str(text);
        let attributed =
            unsafe { CFAttributedString::new(None, Some(&string), Some(attrs.as_opaque())) }
                .ok_or("terminal attributed string")?;
        let line = unsafe { CTLine::with_attributed_string(&attributed) };
        let runs = unsafe { CFRetained::cast_unchecked::<CFArray<CTRun>>(line.glyph_runs()) };
        let mut color = false;
        let mut faces = Vec::new();
        for run in runs.iter() {
            let attrs = unsafe {
                CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(run.attributes())
            };
            let face = attrs
                .get(unsafe { kCTFontAttributeName })
                .and_then(|v| v.downcast::<CTFont>().ok())
                .ok_or("terminal run font")?;
            let name = unsafe { face.post_script_name() }.to_string();
            if name == "LastResort" {
                return Err(format!("no terminal fallback for {text:?}").into());
            }
            color |=
                unsafe { face.symbolic_traits() }.contains(CTFontSymbolicTraits::TraitColorGlyphs);
            if !faces.contains(&name) {
                faces.push(name);
            }
        }
        let pad = (2. * self.scale) as u32;
        let width = self.metrics.cell_width * u32::from(span) + 2 * pad;
        let height = self.metrics.cell_height + 2 * pad;
        let channels = if color { 4 } else { 1 };
        let mut pixels = vec![0; (width * height * channels) as usize];
        let space = if color {
            Some(
                CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }))
                    .ok_or("terminal sRGB space")?,
            )
        } else {
            None
        };
        // BGRA: little-endian premultiplied-first. Alpha-only masks preserve
        // coverage directly, without a device-gray transfer curve or LCD AA.
        let bitmap_info = if color { 0x2002 } else { 7 };
        let context = unsafe {
            CGBitmapContextCreate(
                pixels.as_mut_ptr().cast(),
                width as usize,
                height as usize,
                8,
                (width * channels) as usize,
                space.as_deref(),
                bitmap_info,
            )
        }
        .ok_or("terminal glyph bitmap")?;
        CGContext::set_should_antialias(Some(&context), true);
        CGContext::set_allows_font_smoothing(Some(&context), self.thicken);
        CGContext::set_should_smooth_fonts(Some(&context), self.thicken);
        CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
        CGContext::set_should_subpixel_position_fonts(Some(&context), true);
        CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);
        if color {
            CGContext::set_rgb_fill_color(Some(&context), 1., 1., 1., 1.);
        } else {
            CGContext::set_gray_fill_color(Some(&context), 1., 1.);
        }
        CGContext::scale_ctm(Some(&context), self.scale, self.scale);
        let advance =
            unsafe { line.typographic_bounds(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
        let cell_width = f64::from(self.metrics.cell_width * u32::from(span)) / self.scale;
        let cell_height = f64::from(self.metrics.cell_height) / self.scale;
        let padding = f64::from(pad) / self.scale;
        let (x, y) = if color {
            // Color-font ink can exceed its point size and the primary font's
            // line height. Fit the complete shaped cluster, preserving aspect.
            // SAFETY: line retains its glyph runs; a null context returns local bounds.
            let bounds = unsafe { line.image_bounds(None) };
            if bounds.size.width.is_finite()
                && bounds.size.width > 0.
                && bounds.size.height.is_finite()
                && bounds.size.height > 0.
            {
                let fit = (cell_width / bounds.size.width)
                    .min(cell_height / bounds.size.height)
                    .min(1.);
                // Keep rasterization fringes inside this cell, never in an
                // adjacent row. Bounds fitting happens before this final clip.
                CGContext::clip_to_rect(
                    Some(&context),
                    CGRect::new(
                        CGPoint::new(padding, padding),
                        CGSize::new(cell_width, cell_height),
                    ),
                );
                CGContext::translate_ctm(
                    Some(&context),
                    padding + (cell_width - bounds.size.width * fit) * 0.5 - bounds.origin.x * fit,
                    padding + (cell_height - bounds.size.height * fit) * 0.5
                        - bounds.origin.y * fit,
                );
                CGContext::scale_ctm(Some(&context), fit, fit);
                (0., 0.)
            } else {
                (
                    (cell_width - advance) * 0.5 + padding,
                    f64::from(height - pad) / self.scale
                        - f64::from(self.metrics.baseline) / self.scale,
                )
            }
        } else {
            (
                (cell_width - advance) * 0.5 + padding,
                f64::from(height - pad) / self.scale
                    - f64::from(self.metrics.baseline) / self.scale,
            )
        };
        let actual_traits = unsafe { font.symbolic_traits() };
        if bold && !color && !actual_traits.contains(CTFontSymbolicTraits::TraitBold) {
            CGContext::set_text_drawing_mode(
                Some(&context),
                objc2_core_graphics::CGTextDrawingMode::FillStroke,
            );
            CGContext::set_line_width(Some(&context), (advance / 14.).max(0.5));
        }
        if italic && !color && !actual_traits.contains(CTFontSymbolicTraits::TraitItalic) {
            CGContext::concat_ctm(
                Some(&context),
                objc2_core_foundation::CGAffineTransform {
                    a: 1.,
                    b: 0.,
                    c: 0.21256,
                    d: 1.,
                    tx: -y * 0.21256,
                    ty: 0.,
                },
            );
        }
        for run in runs.iter() {
            let count = unsafe { run.glyph_count() } as usize;
            if count == 0 {
                continue;
            }
            let mut glyphs = vec![0; count];
            let mut positions = vec![CGPoint::ZERO; count];
            let range = CFRange::new(0, count as isize);
            unsafe {
                run.glyphs(range, NonNull::new(glyphs.as_mut_ptr()).unwrap());
                run.positions(range, NonNull::new(positions.as_mut_ptr()).unwrap());
            }
            let attrs = unsafe {
                CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(run.attributes())
            };
            let face = attrs
                .get(unsafe { kCTFontAttributeName })
                .and_then(|v| v.downcast::<CTFont>().ok())
                .ok_or("terminal run face")?;
            for p in &mut positions {
                p.x += x;
                p.y += y;
            }
            unsafe {
                face.draw_glyphs(
                    NonNull::new(glyphs.as_mut_ptr()).unwrap(),
                    NonNull::new(positions.as_mut_ptr()).unwrap(),
                    count,
                    &context,
                );
            }
        }
        drop(context);
        Ok(Some(Glyph {
            width,
            height,
            left: -(pad as i16),
            top: -(pad as i16),
            format: if color { Format::Bgra } else { Format::Mask },
            pixels,
            faces,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec renderer §1b — native fallback/emoji ink remains inside terminal cells
    #[test]
    fn color_emoji_ink_fits_its_cell_box_at_each_density() {
        for scale in [1, 2] {
            let fonts = Fonts::new(
                &FontConfig {
                    scale,
                    ..Default::default()
                },
                None,
                false,
            )
            .unwrap();
            let metrics = fonts.metrics();
            for text in [
                "🦊",
                "👩\u{200d}💻",
                "👨\u{200d}👩\u{200d}👧\u{200d}👦",
                "🏳️\u{200d}🌈",
                "👍🏽",
            ] {
                let glyph = fonts.rasterize(text, false, false, 2).unwrap().unwrap();
                assert_eq!(glyph.format, Format::Bgra);
                let mut ink = 0;
                for y in 0..glyph.height {
                    for x in 0..glyph.width {
                        if glyph.pixels[((y * glyph.width + x) * 4 + 3) as usize] == 0 {
                            continue;
                        }
                        ink += 1;
                        let px = i32::from(glyph.left) + x as i32;
                        let py = i32::from(glyph.top) + y as i32;
                        assert!(
                            px >= 0
                                && px < (2 * metrics.cell_width) as i32
                                && py >= 0
                                && py < metrics.cell_height as i32,
                            "{text} {scale}x: ink at ({px}, {py}) outside {}x{}",
                            2 * metrics.cell_width,
                            metrics.cell_height,
                        );
                    }
                }
                assert!(ink > 0, "{text} must not become blank");
            }
        }
    }

    // keeps: spec renderer §1b — native fallback/emoji ink remains inside terminal cells
    #[test]
    fn fullwidth_forms_use_cjk_fallback_and_ascii_keeps_primary_face() {
        for scale in [1, 2] {
            let fonts = Fonts::new(
                &FontConfig {
                    scale,
                    ..Default::default()
                },
                None,
                false,
            )
            .unwrap();
            for text in ["１", "２", "３", "："] {
                let glyph = fonts.rasterize(text, false, false, 2).unwrap().unwrap();
                assert!(
                    glyph.faces.iter().all(|f| f.starts_with("PingFang")),
                    "{text}: {:?}",
                    glyph.faces
                );
            }
            for text in ["1", "2", "3", ":"] {
                let glyph = fonts.rasterize(text, false, false, 1).unwrap().unwrap();
                assert!(
                    glyph.faces.iter().all(|f| f.starts_with("Menlo")),
                    "{text}: {:?}",
                    glyph.faces
                );
            }
        }
    }
}
