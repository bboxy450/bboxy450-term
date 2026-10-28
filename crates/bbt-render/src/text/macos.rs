// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{FontConfig, FontMetrics, RasterizedGlyph};
use crate::Result;
use objc2_core_foundation::{CFRetained, CFString, CFURL, CGPoint};
use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext};
use objc2_core_text::{
    CTFont, CTFontManagerRegisterFontsForURL, CTFontManagerScope, CTFontOrientation,
};
use std::ptr::{self, NonNull};

/// ASCII-only font for legacy HeadlessRenderer fixtures, separate from terminal::Fonts.
pub struct Font {
    native: CFRetained<CTFont>,
    metrics: FontMetrics,
}

/// Register an existing font only for the current process; no system/user installation.
pub fn register_font_file(path: &std::path::Path) -> Result<()> {
    let url = CFURL::from_file_path(path).ok_or("invalid font file URL")?;
    // SAFETY: a retained file URL and null optional error output; registration is process-local.
    if !unsafe {
        CTFontManagerRegisterFontsForURL(&url, CTFontManagerScope::Process, ptr::null_mut())
    } {
        return Err(format!("CoreText could not register {}", path.display()).into());
    }
    Ok(())
}

impl Font {
    pub fn new(config: &FontConfig) -> Result<Self> {
        if !config.size.is_finite()
            || !(1.0..=256.0).contains(&config.size)
            || !(1..=4).contains(&config.scale)
        {
            return Err("invalid font size/scale".into());
        }
        let size = f64::from(config.size) * f64::from(config.scale);
        // SAFETY: both CFStrings are retained across each call; a null matrix means identity.
        let query = if config.family == "SF Mono" {
            "SF Mono Terminal"
        } else {
            &config.family
        };
        let mut native =
            unsafe { CTFont::with_name(&CFString::from_str(query), size, ptr::null()) };
        let normalize = |s: &str| {
            s.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let actual = unsafe { native.family_name() }.to_string();
        if !normalize(&actual).contains(&normalize(&config.family)) {
            native = unsafe { CTFont::with_name(&CFString::from_str("Menlo"), size, ptr::null()) };
        }
        let mut code = u16::from(b'M');
        let mut glyph = 0;
        // SAFETY: each non-null pointer names one writable value for count=1.
        unsafe {
            native.glyphs_for_characters(NonNull::from(&mut code), NonNull::from(&mut glyph), 1);
        }
        let advance = unsafe {
            native.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut glyph),
                ptr::null_mut(),
                1,
            )
        };
        let ascent = unsafe { native.ascent() };
        let descent = unsafe { native.descent() };
        let leading = unsafe { native.leading() };
        let metrics = FontMetrics {
            cell_width: advance.ceil().max(1.0) as u32,
            cell_height: (ascent + descent + leading + f64::from(config.scale * 2)).ceil() as u32,
            baseline: (ascent.ceil() + f64::from(config.scale)) as i16,
        };
        Ok(Self { native, metrics })
    }
    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }
    pub fn effective_name(&self) -> String {
        unsafe { self.native.post_script_name() }.to_string()
    }

    pub fn rasterize(&self, ch: char) -> Result<Option<RasterizedGlyph>> {
        if !ch.is_ascii() || ch.is_ascii_control() {
            return Err("legacy HeadlessRenderer fixture font accepts printable ASCII only (current terminal path: cells::terminal)".into());
        }
        if ch == ' ' {
            return Ok(None);
        }
        let mut code = ch as u16;
        let mut glyph = 0;
        // SAFETY: count=1 and both pointers reference one live element.
        if !unsafe {
            self.native.glyphs_for_characters(
                NonNull::from(&mut code),
                NonNull::from(&mut glyph),
                1,
            )
        } || glyph == 0
        {
            return Err(format!("missing ASCII glyph {ch:?}").into());
        }
        let bounds = unsafe {
            self.native.bounding_rects_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut glyph),
                ptr::null_mut(),
                1,
            )
        };
        let left = bounds.origin.x.floor() as i16 - 1;
        let bottom = bounds.origin.y.floor() as i16 - 1;
        let top = (bounds.origin.y + bounds.size.height).ceil() as i16 + 1;
        let width = (bounds.origin.x + bounds.size.width).ceil() as i32 - i32::from(left) + 1;
        let height = i32::from(top) - i32::from(bottom) + 1;
        if width <= 0 || height <= 0 {
            return Ok(None);
        }
        let (width, height) = (width as u32, height as u32);
        let mut mask = vec![0; (width * height) as usize];
        let gray =
            CGColorSpace::new_device_gray().ok_or("CoreGraphics grayscale space unavailable")?;
        // SAFETY: the context uses the fixed mask allocation until explicitly dropped below;
        // width*height bytes, 8 bits/component, width bytes/row, grayscale with no alpha.
        let context = unsafe {
            CGBitmapContextCreate(
                mask.as_mut_ptr().cast(),
                width as usize,
                height as usize,
                8,
                width as usize,
                Some(&gray),
                0,
            )
        }
        .ok_or("CoreGraphics glyph context unavailable")?;
        CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
        CGContext::set_should_antialias(Some(&context), true);
        CGContext::set_should_smooth_fonts(Some(&context), false);
        CGContext::set_allows_font_smoothing(Some(&context), false);
        let mut position = CGPoint {
            x: -f64::from(left),
            y: -f64::from(bottom),
        };
        // SAFETY: one glyph and one point; context and font outlive this synchronous draw.
        unsafe {
            self.native.draw_glyphs(
                NonNull::from(&mut glyph),
                NonNull::from(&mut position),
                1,
                &context,
            );
        }
        drop(context);
        // CGBitmapContext's first memory row already contains the top of this image.
        // Glyph-space y points up, but flipping the stored rows would invert the text.
        Ok(Some(RasterizedGlyph {
            left,
            top,
            width,
            height,
            mask,
        }))
    }
}
