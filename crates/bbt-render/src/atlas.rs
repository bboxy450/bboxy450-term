// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Mask or premultiplied BGRA shelf atlas. Growth preserves pixel coordinates.
use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct GlyphAtlas {
    size: u32,
    limit: u32,
    pixels: Vec<u8>,
    next_x: u32,
    next_y: u32,
    row_height: u32,
    channels: u32,
}

impl GlyphAtlas {
    pub fn new(size: u32, limit: u32) -> Result<Self> {
        if size == 0 || !size.is_power_of_two() || size > limit || limit > u16::MAX as u32 {
            return Err("invalid atlas dimensions".into());
        }
        Ok(Self {
            size,
            limit,
            pixels: vec![0; (size * size) as usize],
            next_x: 0,
            next_y: 0,
            row_height: 0,
            channels: 1,
        })
    }
    /// Premultiplied BGRA shelf with the same grow-in-place coordinate contract.
    pub fn new_color(size: u32, limit: u32) -> Result<Self> {
        let mut atlas = Self::new(size, limit)?;
        atlas.channels = 4;
        atlas.pixels.resize((size * size * 4) as usize, 0);
        Ok(atlas)
    }
    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn channels(&self) -> u32 {
        self.channels
    }

    fn grow(&mut self) -> Result<()> {
        let next = self.size * 2;
        if next > self.limit {
            return Err("glyph atlas capacity exhausted".into());
        }
        let mut pixels = vec![0; (next * next * self.channels) as usize];
        let old_stride = (self.size * self.channels) as usize;
        let next_stride = (next * self.channels) as usize;
        for y in 0..self.size as usize {
            pixels[y * next_stride..y * next_stride + old_stride]
                .copy_from_slice(&self.pixels[y * old_stride..(y + 1) * old_stride]);
        }
        self.size = next;
        self.pixels = pixels;
        Ok(())
    }

    pub fn insert(&mut self, width: u32, height: u32, mask: &[u8]) -> Result<AtlasRect> {
        if width == 0 || height == 0 || mask.len() != (width * height * self.channels) as usize {
            return Err("invalid glyph mask dimensions".into());
        }
        // A transparent guard texel between glyphs prevents sampling neighbouring ink.
        while width + 1 > self.size || height + 1 > self.size {
            self.grow()?;
        }
        if self.next_x + width + 1 > self.size {
            self.next_x = 0;
            self.next_y += self.row_height;
            self.row_height = 0;
        }
        while self.next_y + height + 1 > self.size {
            self.grow()?;
        }
        let rect = AtlasRect {
            x: self.next_x as u16,
            y: self.next_y as u16,
            width: width as u16,
            height: height as u16,
        };
        for y in 0..height as usize {
            let stride = (width * self.channels) as usize;
            let dst = (self.next_y as usize + y) * (self.size * self.channels) as usize
                + (self.next_x * self.channels) as usize;
            self.pixels[dst..dst + stride].copy_from_slice(&mask[y * stride..(y + 1) * stride]);
        }
        self.next_x += width + 1;
        self.row_height = self.row_height.max(height + 1);
        Ok(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec renderer §1b — grayscale and premultiplied-color atlas growth preserves prior glyphs
    #[test]
    fn growth_keeps_uploaded_glyph_coordinates_and_masks() {
        let mut atlas = GlyphAtlas::new(8, 64).unwrap();
        let a = atlas.insert(5, 5, &[93; 25]).unwrap();
        let b = atlas.insert(15, 15, &[201; 225]).unwrap();
        assert_eq!(atlas.size(), 32);
        for (r, value) in [(a, 93), (b, 201)] {
            for y in r.y..r.y + r.height {
                for x in r.x..r.x + r.width {
                    assert_eq!(
                        atlas.pixels[(u32::from(y) * atlas.size + u32::from(x)) as usize],
                        value
                    );
                }
            }
        }
        assert_eq!(
            atlas.pixels[(u32::from(a.y) * atlas.size + u32::from(a.x + a.width)) as usize],
            0
        );
        assert!(atlas.insert(64, 1, &[0; 64]).is_err());
    }

    // keeps: spec renderer §1b — grayscale and premultiplied-color atlas growth preserves prior glyphs
    #[test]
    fn color_atlas_growth_preserves_all_four_premultiplied_channels() {
        let mut atlas = GlyphAtlas::new_color(8, 64).unwrap();
        let a = atlas.insert(5, 5, &[10, 20, 30, 40].repeat(25)).unwrap();
        atlas.insert(15, 15, &[60, 70, 80, 90].repeat(225)).unwrap();
        for y in a.y..a.y + a.height {
            for x in a.x..a.x + a.width {
                let i = ((u32::from(y) * atlas.size() + u32::from(x)) * 4) as usize;
                assert_eq!(&atlas.pixels()[i..i + 4], &[10, 20, 30, 40]);
            }
        }
        assert!(atlas.insert(1, 1, &[0]).is_err());
    }
}
