// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native glyph coverage, with no line bitmap cache.
use crate::Result;
use bbt_ui_model::draw::TextStyle;

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Font(objc2_core_foundation::CFRetained<objc2_core_text::CTFont>);
// SAFETY: these fonts originate only in our immutable CoreText shaping runs.
// The private wrapper never bridges to NSFont or exposes Objective-C mutation.
// Apple permits concurrent use of CTFont objects; layout objects (CTLine/CTRun)
// remain local to shape_at and are never retained in the cache.
// https://developer.apple.com/documentation/coretext
// objc2 conservatively leaves its toll-free-bridged CTFont type !Send/!Sync.
#[cfg(target_os = "macos")]
unsafe impl Send for Font {}
#[cfg(target_os = "macos")]
unsafe impl Sync for Font {}
#[cfg(not(target_os = "macos"))]
type Font = u64;

/// CFEqual/CFHash bind the actual fallback face, variation, size and matrix.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Glyph {
    font: Font,
    id: u16,
    position: [u64; 2],
    run_start: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub scale: u32,
    pub glyphs: Vec<Glyph>,
    pub blur_sigma: u32,
}
#[derive(Clone)]
pub(super) struct Placed {
    pub key: Key,
    pub origin: [i32; 2],
    pub run: usize,
}
pub(super) struct Shape {
    pub width: u32,
    pub height: u32,
    pub baseline: f32,
    pub advance: f32,
    pub glyphs: Vec<Placed>,
}

/// Bounds are integer device coordinates, y upwards, relative to the key origin.
/// Pixels retain the bitmap's top-to-bottom row order.
pub(super) struct Mask {
    pub bounds: [i32; 4],
    pub merge_bounds: [i32; 4],
    pub pixels: Vec<u8>,
}
impl Mask {
    pub fn width(&self) -> u32 {
        (self.bounds[2] - self.bounds[0]) as u32
    }
    pub fn height(&self) -> u32 {
        (self.bounds[3] - self.bounds[1]) as u32
    }
    pub fn empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

#[cfg(target_os = "macos")]
pub(super) fn shape(
    value: &str,
    style: TextStyle,
    max_width: Option<f32>,
    scale: f32,
    phase: f32,
) -> Result<Shape> {
    let line = super::text::shape_at(value, style, max_width, scale, phase)?;
    let mut glyphs = Vec::new();
    let s = f64::from(scale);
    for (run_index, run) in line.runs.iter().enumerate() {
        for (&id, point) in run.glyphs.iter().zip(&run.positions) {
            let physical = [point.x * s, point.y * s];
            let origin = physical.map(|v| v.floor() as i32);
            let phase = [
                physical[0] - f64::from(origin[0]),
                physical[1] - f64::from(origin[1]),
            ];
            glyphs.push(Placed {
                key: Key {
                    scale: scale.to_bits(),
                    blur_sigma: 0,
                    glyphs: vec![Glyph {
                        font: Font(run.font.clone()),
                        id,
                        position: phase.map(|v| {
                            if v == 0. {
                                0_f64.to_bits()
                            } else {
                                v.to_bits()
                            }
                        }),
                        run_start: true,
                    }],
                },
                origin,
                run: run_index,
            });
        }
    }
    Ok(Shape {
        width: line.width,
        height: line.height,
        baseline: line.baseline,
        advance: line.advance,
        glyphs,
    })
}
#[cfg(not(target_os = "macos"))]
pub(super) fn shape(_: &str, _: TextStyle, _: Option<f32>, _: f32, _: f32) -> Result<Shape> {
    Err("chrome glyph shaping requires macOS".into())
}

/// Minimal connected overlaps retain native run boundaries, never a text key.
pub(super) fn component(shape: &Shape, indices: &[usize]) -> (Key, [i32; 2]) {
    let first = &shape.glyphs[indices[0]];
    let origin = first.origin;
    let mut last_run = None;
    let mut glyphs = Vec::with_capacity(indices.len());
    for &index in indices {
        let placed = &shape.glyphs[index];
        let mut glyph = placed.key.glyphs[0].clone();
        for (axis, base) in origin.iter().enumerate() {
            let p = f64::from_bits(glyph.position[axis]) + f64::from(placed.origin[axis] - base);
            glyph.position[axis] = if p == 0. {
                0_f64.to_bits()
            } else {
                p.to_bits()
            };
        }
        glyph.run_start = last_run != Some(placed.run);
        last_run = Some(placed.run);
        glyphs.push(glyph);
    }
    (
        Key {
            scale: first.key.scale,
            glyphs,
            blur_sigma: 0,
        },
        origin,
    )
}

/// Components are minimal under intersecting device-pixel coverage bounds.
pub(super) fn connected(bounds: &[[i32; 4]]) -> Vec<Vec<usize>> {
    fn root(parents: &[usize], mut i: usize) -> usize {
        while parents[i] != i {
            i = parents[i];
        }
        i
    }
    let mut parents: Vec<_> = (0..bounds.len()).collect();
    for (i, a) in bounds.iter().enumerate() {
        if a[0] >= a[2] || a[1] >= a[3] {
            continue;
        }
        for (j, b) in bounds[..i].iter().enumerate() {
            if b[0] < b[2]
                && b[1] < b[3]
                && a[0] < b[2]
                && b[0] < a[2]
                && a[1] < b[3]
                && b[1] < a[3]
            {
                let x = root(&parents, i);
                let y = root(&parents, j);
                parents[x] = y;
            }
        }
    }
    let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    for (i, b) in bounds.iter().enumerate() {
        if b[0] < b[2] && b[1] < b[3] {
            groups.entry(root(&parents, i)).or_default().push(i);
        }
    }
    let mut out: Vec<_> = groups.into_values().collect();
    out.sort_by_key(|g| g[0]);
    out
}

/// Blur actual coverage, retaining zero padding for the atlas quad's AA edge.
/// The source is a cached glyph or minimal ink-overlap component, never a line bitmap.
pub(super) fn blur(mask: &Mask, sigma: f32) -> Result<Mask> {
    let pad = (3. * sigma).ceil();
    if !sigma.is_finite() || sigma <= 0. || pad > 16_384. {
        return Err("invalid chrome text shadow blur".into());
    }
    if mask.empty() {
        return Ok(Mask {
            bounds: mask.bounds,
            merge_bounds: mask.merge_bounds,
            pixels: vec![],
        });
    }
    let pad = pad as usize;
    let source_width = mask.width() as usize;
    let source_height = mask.height() as usize;
    let width = source_width + 2 * pad;
    let height = source_height + 2 * pad;
    if width > 16_384 || height > 2_048 {
        return Err("chrome text shadow component too large".into());
    }
    let mut weights: Vec<_> = (0..=2 * pad)
        .map(|i| {
            let x = i as f32 - pad as f32;
            let distance = x / sigma;
            (-0.5 * distance * distance).exp()
        })
        .collect();
    let total: f32 = weights.iter().sum();
    for weight in &mut weights {
        *weight /= total;
    }
    let mut horizontal = vec![0.; width * source_height];
    for y in 0..source_height {
        for x in 0..width {
            let mut value = 0.;
            for (tap, weight) in weights.iter().enumerate() {
                let source_x = x as isize + tap as isize - 2 * pad as isize;
                if (0..source_width as isize).contains(&source_x) {
                    value += f32::from(mask.pixels[y * source_width + source_x as usize]) * weight;
                }
            }
            horizontal[y * width + x] = value;
        }
    }
    let mut pixels = vec![0; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut value = 0.;
            for (tap, weight) in weights.iter().enumerate() {
                let source_y = y as isize + tap as isize - 2 * pad as isize;
                if (0..source_height as isize).contains(&source_y) {
                    value += horizontal[source_y as usize * width + x] * weight;
                }
            }
            pixels[y * width + x] = value.round().clamp(0., 255.) as u8;
        }
    }
    let pad = pad as i32;
    let expand = |b: [i32; 4]| [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad];
    Ok(Mask {
        bounds: expand(mask.bounds),
        merge_bounds: expand(mask.merge_bounds),
        pixels,
    })
}

#[cfg(target_os = "macos")]
pub(super) fn rasterize(key: &Key) -> Result<Mask> {
    use objc2_core_foundation::CGPoint;
    use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext};
    use objc2_core_text::CTFontOrientation;
    use std::ptr::{self, NonNull};
    let scale = f64::from(f32::from_bits(key.scale));
    let mut geometry: Option<[i32; 4]> = None;
    for glyph in &key.glyphs {
        let mut id = glyph.id;
        let bounds = unsafe {
            glyph.font.0.bounding_rects_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut id),
                ptr::null_mut(),
                1,
            )
        };
        if bounds.size.width <= 0. || bounds.size.height <= 0. {
            continue;
        }
        let p = glyph.position.map(f64::from_bits);
        let b = [
            (bounds.origin.x * scale + p[0]).floor() as i32,
            (bounds.origin.y * scale + p[1]).floor() as i32,
            ((bounds.origin.x + bounds.size.width) * scale + p[0]).ceil() as i32,
            ((bounds.origin.y + bounds.size.height) * scale + p[1]).ceil() as i32,
        ];
        geometry = Some(match geometry {
            None => b,
            Some(a) => [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3]),
            ],
        });
    }
    let Some(geometry) = geometry else {
        return Ok(Mask {
            bounds: [0; 4],
            merge_bounds: [0; 4],
            pixels: vec![],
        });
    };
    let canvas = [
        geometry[0] - 2,
        geometry[1] - 2,
        geometry[2] + 2,
        geometry[3] + 2,
    ];
    let width = (canvas[2] - canvas[0]) as usize;
    let height = (canvas[3] - canvas[1]) as usize;
    if width > 16_384 || height > 2_048 {
        return Err("chrome glyph component too large".into());
    }
    let mut pixels = vec![0; width * height];
    let gray = CGColorSpace::new_device_gray().ok_or("grayscale colour space")?;
    // The allocation stays fixed and inaccessible until the synchronous context is dropped.
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            width,
            height,
            8,
            width,
            Some(&gray),
            0,
        )
    }
    .ok_or("chrome glyph bitmap")?;
    // These settings are identical to the line reference in text.rs.
    CGContext::set_gray_fill_color(Some(&context), 1., 1.);
    CGContext::set_should_antialias(Some(&context), true);
    CGContext::set_should_smooth_fonts(Some(&context), false);
    CGContext::set_allows_font_smoothing(Some(&context), false);
    CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
    CGContext::set_should_subpixel_position_fonts(Some(&context), true);
    CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
    CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);
    CGContext::scale_ctm(Some(&context), scale, scale);
    let mut start = 0;
    while start < key.glyphs.len() {
        let mut end = start + 1;
        while end < key.glyphs.len() && !key.glyphs[end].run_start {
            end += 1;
        }
        let mut ids = Vec::with_capacity(end - start);
        let mut points = Vec::with_capacity(end - start);
        for glyph in &key.glyphs[start..end] {
            ids.push(glyph.id);
            points.push(CGPoint {
                x: (f64::from_bits(glyph.position[0]) - f64::from(canvas[0])) / scale,
                y: (f64::from_bits(glyph.position[1]) - f64::from(canvas[1])) / scale,
            });
        }
        unsafe {
            key.glyphs[start].font.0.draw_glyphs(
                NonNull::new(ids.as_mut_ptr()).unwrap(),
                NonNull::new(points.as_mut_ptr()).unwrap(),
                ids.len(),
                &context,
            );
        }
        start = end;
    }
    drop(context);
    let mut crop = [width, height, 0, 0];
    for (i, &value) in pixels.iter().enumerate() {
        if value == 0 {
            continue;
        }
        let x = i % width;
        let y = i / width;
        crop[0] = crop[0].min(x);
        crop[1] = crop[1].min(y);
        crop[2] = crop[2].max(x + 1);
        crop[3] = crop[3].max(y + 1);
    }
    if crop[2] == 0 {
        return Ok(Mask {
            bounds: [0; 4],
            merge_bounds: [0; 4],
            pixels: vec![],
        });
    }
    let ink = [
        canvas[0] + crop[0] as i32,
        canvas[3] - crop[3] as i32,
        canvas[0] + crop[2] as i32,
        canvas[3] - crop[1] as i32,
    ];
    // Keep ink away from the quad's antialiased rectangle edge, just as the
    // original padded line bitmap did. Otherwise interpolation error at an
    // exactly half-pixel edge can reduce coverage by one output level.
    let stride = crop[2] - crop[0] + 2;
    let mut trimmed = Vec::with_capacity(stride * (crop[3] - crop[1] + 2));
    trimmed.resize(stride, 0);
    for y in crop[1]..crop[3] {
        trimmed.push(0);
        trimmed.extend_from_slice(&pixels[y * width + crop[0]..y * width + crop[2]]);
        trimmed.push(0);
    }
    trimmed.resize(trimmed.len() + stride, 0);
    let merge_bounds = [
        ink[0].min(geometry[0]),
        ink[1].min(geometry[1]),
        ink[2].max(geometry[2]),
        ink[3].max(geometry[3]),
    ];
    Ok(Mask {
        bounds: [ink[0] - 1, ink[1] - 1, ink[2] + 1, ink[3] + 1],
        merge_bounds,
        pixels: trimmed,
    })
}
#[cfg(not(target_os = "macos"))]
pub(super) fn rasterize(_: &Key) -> Result<Mask> {
    Err("chrome glyph rasterization requires macOS".into())
}

#[cfg(test)]
mod shadow_tests {
    use super::*;

    // keeps: spec renderer §1a — glyph coverage, fallback faces, phases and shadow masks
    #[test]
    fn gaussian_shadow_keeps_a_zero_atlas_border_and_rejects_unbounded_inputs() {
        let point = Mask {
            bounds: [0, 0, 3, 3],
            merge_bounds: [1, 1, 2, 2],
            pixels: vec![0, 0, 0, 0, 255, 0, 0, 0, 0],
        };
        let blurred = blur(&point, 1.).unwrap();
        assert_eq!(blurred.bounds, [-3, -3, 6, 6]);
        assert_eq!((blurred.width(), blurred.height()), (9, 9));
        assert_eq!(blurred.pixels[4 * 9 + 4], 41);
        assert_eq!(blurred.pixels[4 * 9 + 5], 25);
        assert_eq!(blurred.pixels[5 * 9 + 5], 15);
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(
                    blurred.pixels[y * 9 + x],
                    blurred.pixels[(8 - y) * 9 + 8 - x]
                );
                if x == 0 || y == 0 || x == 8 || y == 8 {
                    assert_eq!(blurred.pixels[y * 9 + x], 0);
                }
            }
        }
        let tiny = blur(&point, 1e-30).unwrap();
        assert_eq!(tiny.pixels[2 * 5 + 2], 255);
        for sigma in [f32::NAN, f32::INFINITY, -1., 10_000.] {
            assert!(blur(&point, sigma).is_err());
        }
        let empty = Mask {
            bounds: [0; 4],
            merge_bounds: [0; 4],
            pixels: vec![],
        };
        assert!(blur(&empty, 1.).unwrap().empty());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use bbt_ui_model::draw::TypeScale;
    use std::{collections::HashMap, sync::Arc};

    fn composite(shape: &Shape, cache: &mut HashMap<Key, Arc<Mask>>) -> Vec<u8> {
        let mut bounds = Vec::new();
        for placed in &shape.glyphs {
            let mask = cache
                .entry(placed.key.clone())
                .or_insert_with(|| Arc::new(rasterize(&placed.key).unwrap()));
            let b = mask.merge_bounds;
            bounds.push(if mask.empty() {
                [0; 4]
            } else {
                [
                    b[0] + placed.origin[0],
                    b[1] + placed.origin[1],
                    b[2] + placed.origin[0],
                    b[3] + placed.origin[1],
                ]
            });
        }
        let mut pixels = vec![0; (shape.width * shape.height) as usize];
        for group in connected(&bounds) {
            let (key, origin) = component(shape, &group);
            let mask = cache
                .entry(key.clone())
                .or_insert_with(|| Arc::new(rasterize(&key).unwrap()));
            let x = origin[0] + mask.bounds[0];
            let y = shape.height as i32 - origin[1] - mask.bounds[3];
            for row in 0..mask.height() as i32 {
                for column in 0..mask.width() as i32 {
                    let (dx, dy) = (x + column, y + row);
                    if dx < 0 || dy < 0 || dx >= shape.width as i32 || dy >= shape.height as i32 {
                        continue;
                    }
                    let source = mask.pixels[(row as u32 * mask.width() + column as u32) as usize];
                    let at = (dy as u32 * shape.width + dx as u32) as usize;
                    assert!(
                        pixels[at] == 0 || source == 0,
                        "coverage components overlap"
                    );
                    pixels[at] = pixels[at].max(source);
                }
            }
        }
        pixels
    }

    // keeps: spec renderer §1a — glyph coverage, fallback faces, phases and shadow masks
    #[test]
    fn cropped_glyph_components_reproduce_native_line_masks() {
        let mut cache = HashMap::new();
        let values = [
            "Hamburgefonts AV fi",
            "AVAV ffi fl fifi",
            "A\u{0301} e\u{0308} ẍ",
            "سلام بالعالم",
            "नमस्ते दुनिया",
            "♔ Claude ⌘1 · 許可 ⚿",
            "👩‍💻 👨‍👩‍👧‍👦",
            "Mireille Bellweather-Lockridge",
            "",
            " ",
        ];
        let mut cases = 0;
        for mono in [false, true] {
            for italic in [false, true] {
                let style = TextStyle {
                    mono,
                    italic,
                    weight: 600,
                    scale: TypeScale::Custom(13.),
                    ..Default::default()
                };
                for scale in [1., 1.5, 2.] {
                    for phase in [0., 0.125, 0.25, 0.5, 0.75] {
                        for value in values {
                            let line = shape(value, style, None, scale, phase).unwrap();
                            let expected =
                                super::super::text::rasterize_at(value, style, None, scale, phase)
                                    .unwrap();
                            assert_eq!(
                                [line.width, line.height],
                                [expected.width, expected.height]
                            );
                            assert_eq!(line.advance, expected.advance);
                            let actual = composite(&line, &mut cache);
                            let different = actual
                                .iter()
                                .zip(&expected.mask)
                                .filter(|(a, b)| a != b)
                                .count();
                            assert_eq!(
                                different, 0,
                                "mono={mono} italic={italic} scale={scale} phase={phase} value={value:?}"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
        eprintln!(
            "glyph component masks: {cases} exact cases, {} reusable glyph/component entries",
            cache.len()
        );
    }

    // keeps: spec renderer §1a — glyph coverage, fallback faces, phases and shadow masks
    #[test]
    fn cached_fallback_font_variations_and_phases_preserve_native_coverage() {
        let mut cache = HashMap::new();
        for (style, phase) in [
            (TextStyle::default(), 0.),
            (TextStyle::default(), 0.),
            (
                TextStyle {
                    weight: 600,
                    ..Default::default()
                },
                0.,
            ),
            (TextStyle::default(), 0.25),
            (TextStyle::default(), 0.),
        ] {
            let line = shape("Aa許", style, None, 1., phase).unwrap();
            let expected =
                super::super::text::rasterize_at("Aa許", style, None, 1., phase).unwrap();
            assert_eq!([line.width, line.height], [expected.width, expected.height]);
            assert_eq!(line.advance, expected.advance);
            assert_eq!(composite(&line, &mut cache), expected.mask);
        }
    }
    // keeps: spec renderer §1a — glyph coverage, fallback faces, phases and shadow masks
    #[test]
    fn compact_state_symbols_keep_exact_coverage_at_each_origin() {
        let mut cache = HashMap::new();
        for pixels in [7., 9., 10., 12., 13., 16.] {
            for mono in [true, false] {
                let style = TextStyle {
                    mono,
                    scale: TypeScale::Custom(pixels),
                    ..Default::default()
                };
                for phase in [0., 0.125, 0.25, 0.5, 0.75, 255.9_f32.fract()] {
                    for value in ["●", "○", "◐", "✕", "⚿", "⚷"] {
                        let line = shape(value, style, None, 1., phase).unwrap();
                        let expected =
                            super::super::text::rasterize_at(value, style, None, 1., phase)
                                .unwrap();
                        assert_eq!(
                            composite(&line, &mut cache),
                            expected.mask,
                            "pixels={pixels} mono={mono} phase={phase} value={value:?}"
                        );
                    }
                }
            }
        }
    }
}
