// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded PNG preparation shared by document pictures and retained SVG images.
use crate::Result;

pub const MAX_PNG_BYTES: usize = 16_777_216;
pub const MAX_PNG_SIDE: u32 = 2047;

pub struct Image {
    pub width: u32,
    pub height: u32,
    /// Straight-alpha sRGB, as consumed by the existing color atlas.
    pub rgba: Vec<u8>,
}

/// Inspect IHDR before allocating a decompression buffer. The decoder checks
/// the complete file, including chunk checksums, when pixels are requested.
pub fn png_dimensions(bytes: &[u8]) -> Result<[u32; 2]> {
    if bytes.len() > MAX_PNG_BYTES
        || bytes.len() < 33
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || bytes[8..12] != 13_u32.to_be_bytes()
        || &bytes[12..16] != b"IHDR"
    {
        return Err("invalid or oversized PNG".into());
    }
    let dimensions = [
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    ];
    if dimensions.iter().any(|n| *n == 0 || *n > MAX_PNG_SIDE) {
        return Err("image dimensions exceed the color atlas".into());
    }
    Ok(dimensions)
}

// Adapted from bboxy450-term src/documents/images.rs and src/assets.rs:
// https://github.com/bboxy450/bboxy450-term/tree/main/crates/bboxy450-term/src
pub fn decode_png(bytes: &[u8], remaining_pixels: u64) -> Result<Image> {
    let [width, height] = png_dimensions(bytes)?;
    let pixels = u64::from(width) * u64::from(height);
    if pixels > remaining_pixels {
        return Err("decoded PNG pixels exceed the image budget".into());
    }
    let mut decoder = png::Decoder::new_with_limits(
        std::io::Cursor::new(bytes),
        png::Limits {
            bytes: 32 * 1024 * 1024,
        },
    );
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    if reader.info().animation_control.is_some() {
        return Err("animated PNG is unsupported".into());
    }
    let size = reader
        .output_buffer_size()
        .ok_or("PNG output unavailable")?;
    if size > pixels as usize * 4 {
        return Err("PNG output exceeds its admitted dimensions".into());
    }
    let mut data = vec![0; size];
    let info = reader.next_frame(&mut data).map_err(|e| e.to_string())?;
    let data = &data[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|v| [*v, *v, *v, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return Err("unsupported PNG color format".into()),
    };
    Ok(Image {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec svg-renderer — dimensions and aggregate pixels precede decode.
    #[test]
    fn bounded_png_preserves_alpha_and_refuses_before_allocation() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[255, 0, 0, 128, 0, 255, 0, 0])
                .unwrap();
        }
        assert_eq!(
            decode_png(&bytes, 2).unwrap().rgba,
            [255, 0, 0, 128, 0, 255, 0, 0]
        );
        assert!(decode_png(&bytes, 1).is_err());
        bytes[16..20].copy_from_slice(&2048_u32.to_be_bytes());
        assert!(png_dimensions(&bytes).is_err());
        assert!(png_dimensions(&[0; 33]).is_err());
    }
}
