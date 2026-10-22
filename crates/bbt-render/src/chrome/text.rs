//! CoreText lines use the same grayscale shelf atlas as the terminal pass.
//! Line shaping/fallback is CoreText-owned; colour emoji is intentionally grayscale here.
use crate::Result;
use bbt_ui_model::draw::TextStyle;

pub struct LineMask {
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    /// Logical font metrics for baseline layout; the bitmap/render path is unchanged.
    pub ascent: f32,
    pub descent: f32,
    pub mask: Vec<u8>,
}

// AppKit's verified system-font weight anchors; interpolate numeric CSS weights.
// CoreText descriptor matching selects the closest real face/variation available.
#[cfg(target_os = "macos")]
fn native_font(
    style: TextStyle,
    scale: f32,
) -> Result<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CFType};
    use objc2_core_text::{
        CTFont, CTFontDescriptor, CTFontUIFontType, kCTFontFamilyNameAttribute,
        kCTFontTraitsAttribute, kCTFontWeightTrait,
    };
    let size = f64::from(style.pixels() * scale);
    // No regular/bold threshold: weight 400 is the family base, other weights
    // are requested through the weight trait, including 500 medium/600 semibold.
    let base = unsafe {
        if style.mono {
            CTFont::with_name(&CFString::from_str("Menlo"), size, std::ptr::null())
        } else {
            CTFont::new_ui_font_for_language(CTFontUIFontType::System, size, None)
                .ok_or("system UI font unavailable")?
        }
    };
    if style.weight == 400 {
        return Ok(base);
    }
    let anchors = [-0.8, -0.6, -0.4, 0., 0.23, 0.3, 0.4, 0.56, 0.62];
    let w = style.weight.clamp(100, 900);
    let lower = usize::from((w / 100).saturating_sub(1));
    let upper = (lower + 1).min(8);
    let fraction = f64::from(w % 100) / 100.;
    let weight = CFNumber::new_f64(anchors[lower] + (anchors[upper] - anchors[lower]) * fraction);
    let weight_key = unsafe { kCTFontWeightTrait };
    let traits = CFDictionary::<CFString, CFNumber>::from_slices(&[weight_key], &[weight.as_ref()]);
    let values: [&CFType; 1] = [traits.as_ref()];
    let traits_key = unsafe { kCTFontTraitsAttribute };
    let attrs = CFDictionary::from_slices(&[traits_key], &values);
    let descriptor = unsafe {
        if style.mono {
            let family = CFString::from_str("Menlo");
            let values: [&CFType; 2] = [family.as_ref(), traits.as_ref()];
            let attrs = CFDictionary::from_slices(
                &[kCTFontFamilyNameAttribute, kCTFontTraitsAttribute],
                &values,
            );
            CTFontDescriptor::with_attributes(attrs.as_opaque())
        } else {
            base.font_descriptor()
                .copy_with_attributes(attrs.as_opaque())
        }
    };
    Ok(unsafe { CTFont::with_font_descriptor(&descriptor, size, std::ptr::null()) })
}

#[cfg(target_os = "macos")]
pub fn rasterize(
    text: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
) -> Result<LineMask> {
    use objc2_core_foundation::{
        CFAttributedString, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType,
    };
    use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext};
    use objc2_core_text::{
        CTLine, kCTFontAttributeName, kCTForegroundColorFromContextAttributeName,
        kCTKernAttributeName,
    };
    use std::ptr;
    let font = native_font(style, scale)?;
    let kern = CFNumber::new_f64(f64::from(style.tracking_em * style.pixels() * scale));
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
        let limit = limit.max(0.0) * scale;
        if measure(&line) > limit {
            let chars: Vec<char> = value.chars().collect();
            let mut lo = 0;
            let mut hi = chars.len();
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                let trial: String = chars[..mid].iter().chain(std::iter::once(&'…')).collect();
                let trial_line = make(&trial)?;
                if measure(&trial_line) <= limit {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            value = chars[..lo].iter().chain(std::iter::once(&'…')).collect();
            line = make(&value)?;
            if measure(&line) > limit {
                line = make("")?;
            }
        }
    }
    let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
    let advance =
        unsafe { line.typographic_bounds(&mut ascent, &mut descent, &mut leading) } as f32;
    let width = (advance.ceil() as u32 + 4).max(1);
    let height = ((ascent + descent).ceil() as u32 + 4).max(1);
    if width > 16_384 || height > 2_048 {
        return Err("chrome text line too large".into());
    }
    let mut mask = vec![0; width as usize * height as usize];
    let gray = CGColorSpace::new_device_gray().ok_or("grayscale colour space")?;
    // SAFETY: fixed byte allocation, correct 8-bit grayscale stride; context is dropped before mask moves.
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
    .ok_or("chrome text bitmap")?;
    CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
    CGContext::set_should_antialias(Some(&context), true);
    CGContext::set_should_smooth_fonts(Some(&context), false);
    CGContext::set_allows_font_smoothing(Some(&context), false);
    CGContext::set_text_position(Some(&context), 2.0, descent.ceil() + 2.0);
    unsafe { line.draw(&context) };
    drop(context);
    Ok(LineMask {
        width,
        height,
        advance: advance / scale,
        ascent: ascent as f32 / scale,
        descent: descent as f32 / scale,
        mask,
    })
}
#[cfg(not(target_os = "macos"))]
pub fn rasterize(_: &str, _: TextStyle, _: Option<f32>, _: f32) -> Result<LineMask> {
    Err("chrome text currently requires CoreText".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
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
