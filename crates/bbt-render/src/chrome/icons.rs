//! Bitmap images and SVGs share a premultiplied atlas. SVG rasters are retained
//! at their requested physical size; a warm draw does no rasterisation/upload.
use crate::Result;
use std::collections::HashMap;
pub struct Image {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M3 3L21 21M3 21L21 3" fill="none" stroke="white" stroke-width="2.4" stroke-linecap="round"/></svg>"#;

    #[test]
    fn bitmap_minification_integrates_footprints_and_reuses_the_atlas_entry() {
        let image = Image {
            name: "bars".into(),
            width: 64,
            height: 64,
            rgba: (0..64 * 64)
                .flat_map(|i| {
                    let v = if i % 64 % 3 == 0 { 255 } else { 0 };
                    [v, v, v, 255]
                })
                .collect(),
        };
        let mut atlas = IconAtlas::new(vec![image]).unwrap();
        let rect = atlas.image("bars", [8., 8.], 1.).unwrap();
        assert_eq!(rect[2..], [8., 8.]);
        for y in 0..8 {
            for x in 0..8 {
                let at = (((rect[1] as u32 + y) * atlas.size + rect[0] as u32 + x) * 4) as usize;
                let v = if x % 3 == 2 { 64 } else { 96 };
                assert_eq!(&atlas.pixels[at..at + 4], &[v, v, v, 255]);
            }
        }
        let shelf = atlas.shelf;
        let dirty = atlas.dirty.len();
        assert_eq!(atlas.image("bars", [8., 8.], 1.).unwrap(), rect);
        assert_eq!(atlas.shelf, shelf);
        assert_eq!(atlas.dirty.len(), dirty);
        assert_eq!(
            atlas.image("bars", [128., 128.], 2.).unwrap(),
            atlas.rects["bars"]
        );
        assert!(atlas.image("bars", [f32::NAN, 8.], 1.).is_err());
        assert!(atlas.image("bars", [8., 8.], 0.).is_err());
    }

    #[test]
    fn transparent_bitmap_texels_contribute_no_hidden_rgb_to_minification() {
        let mut atlas = IconAtlas::new(vec![Image {
            name: "alpha".into(),
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 0, 0, 0, 255, 255],
        }])
        .unwrap();
        let rect = atlas.image("alpha", [1., 1.], 1.).unwrap();
        let at = ((rect[1] as u32 * atlas.size + rect[0] as u32) * 4) as usize;
        assert_eq!(&atlas.pixels[at..at + 4], &[0, 0, 128, 128]);
    }

    #[test]
    fn size_and_scale_cache_preserves_prior_rasters_and_rejects_unbounded_inputs() {
        let mut atlas = IconAtlas::new(Assets::with_svgs(vec![], &[("cross", SVG)])).unwrap();
        let small = atlas.raster("cross", [14., 14.], 1.).unwrap();
        let original = atlas.pixels.clone();
        let shelf = atlas.shelf;
        assert_eq!(atlas.raster("cross", [14., 14.], 1.).unwrap(), small);
        assert_eq!(atlas.shelf, shelf);
        assert_eq!(
            atlas.dirty.len(),
            1,
            "warm rasters do not need another upload"
        );
        assert_eq!(atlas.pixels, original);
        let large = atlas.raster("cross", [28., 28.], 2.).unwrap();
        assert_eq!(&large[2..], &[28., 28.]);
        assert_ne!(small[..2], large[..2]);
        let other_scale = atlas.raster("cross", [14., 14.], 2.).unwrap();
        assert_ne!(small[..2], other_scale[..2]);
        assert_eq!(atlas.rasters.len(), 3);
        assert!(atlas.raster("missing", [14., 14.], 1.).is_err());
        for size in [0., -1., f32::NAN, f32::INFINITY, 2048.] {
            assert!(atlas.raster("cross", [size, 14.], 1.).is_err());
        }
        assert!(atlas.raster("cross", [14., 14.], f32::NAN).is_err());
    }
}
#[derive(Default)]
pub struct Assets {
    pub bitmaps: Vec<Image>,
    pub svgs: Vec<(String, String)>,
}
impl Assets {
    pub fn with_svgs(bitmaps: Vec<Image>, svgs: &[(&str, &str)]) -> Self {
        Self {
            bitmaps,
            svgs: svgs
                .iter()
                .map(|(name, source)| ((*name).into(), (*source).into()))
                .collect(),
        }
    }
}
impl From<Vec<Image>> for Assets {
    fn from(bitmaps: Vec<Image>) -> Self {
        Self {
            bitmaps,
            ..Self::default()
        }
    }
}
#[derive(Hash, PartialEq, Eq)]
struct RasterKey {
    name: String,
    physical_size: [u32; 2],
    scale: u32,
}
pub struct IconAtlas {
    pub size: u32,
    pub pixels: Vec<u8>,
    pub rects: HashMap<String, [f32; 4]>,
    bitmaps: HashMap<String, Image>,
    trees: HashMap<String, resvg::usvg::Tree>,
    rasters: HashMap<RasterKey, [f32; 4]>,
    shelf: [u32; 3],
    pub(super) dirty: Vec<[u32; 4]>,
}
impl IconAtlas {
    pub fn new(assets: impl Into<Assets>) -> Result<Self> {
        let assets = assets.into();
        let size = 2048u32;
        let mut atlas = Self {
            size,
            pixels: vec![0; (size * size * 4) as usize],
            rects: HashMap::new(),
            bitmaps: HashMap::new(),
            trees: HashMap::new(),
            rasters: HashMap::new(),
            shelf: [0; 3],
            dirty: Vec::new(),
        };
        for mut image in assets.bitmaps {
            atlas.validate_size(image.width, image.height, image.rgba.len())?;
            for pixel in image.rgba.as_chunks_mut::<4>().0 {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
            let rect = atlas.insert(image.width, image.height, &image.rgba)?;
            atlas.rects.insert(image.name.clone(), rect);
            atlas.bitmaps.insert(image.name.clone(), image);
        }
        for (name, source) in assets.svgs {
            if atlas.rects.contains_key(&name) || atlas.trees.contains_key(&name) {
                return Err(format!("duplicate icon asset: {name}").into());
            }
            let tree = resvg::usvg::Tree::from_str(&source, &Default::default())
                .map_err(|error| format!("invalid SVG {name}: {error}"))?;
            atlas.trees.insert(name, tree);
        }
        Ok(atlas)
    }
    fn validate_size(&self, width: u32, height: u32, bytes: usize) -> Result<()> {
        if width == 0
            || height == 0
            || width >= self.size
            || height >= self.size
            || bytes != (width * height * 4) as usize
        {
            return Err("invalid icon image".into());
        }
        Ok(())
    }
    fn insert(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<[f32; 4]> {
        self.validate_size(width, height, rgba.len())?;
        let [mut x, mut y, mut row] = self.shelf;
        if x + width + 1 > self.size {
            x = 0;
            y += row;
            row = 0;
        }
        if y + height + 1 > self.size {
            return Err("icon atlas capacity exhausted".into());
        }
        for dy in 0..height {
            let offset = (((y + dy) * self.size + x) * 4) as usize;
            let src = (dy * width * 4) as usize;
            self.pixels[offset..offset + (width * 4) as usize]
                .copy_from_slice(&rgba[src..src + (width * 4) as usize]);
        }
        self.shelf = [x + width + 1, y, row.max(height + 1)];
        self.dirty.push([x, y, width, height]);
        Ok([x as f32, y as f32, width as f32, height as f32])
    }
    pub(super) fn is_svg(&self, name: &str) -> bool {
        self.trees.contains_key(name)
    }
    /// Integrate source texels before minification; one level-zero bilinear
    /// sample cannot represent a footprint spanning many source pixels.
    pub(super) fn image(
        &mut self,
        name: &str,
        physical_size: [f32; 2],
        scale: f32,
    ) -> Result<[f32; 4]> {
        if !scale.is_finite()
            || scale <= 0.
            || physical_size.iter().any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid bitmap size or scale".into());
        }
        let image = self.bitmaps.get(name).ok_or("unregistered bitmap")?;
        let width = (physical_size[0].ceil().min(image.width as f32)) as u32;
        let height = (physical_size[1].ceil().min(image.height as f32)) as u32;
        if width == image.width && height == image.height {
            return Ok(self.rects[name]);
        }
        let key = RasterKey {
            name: name.into(),
            physical_size: physical_size.map(f32::to_bits),
            scale: scale.to_bits(),
        };
        if let Some(rect) = self.rasters.get(&key) {
            return Ok(*rect);
        }
        // Source RGB is already premultiplied, so transparent texels cannot
        // contribute hidden colour. Cache the integrated pixels in the same atlas.
        let rgba = downsample(image, width, height);
        let rect = self.insert(width, height, &rgba)?;
        self.rasters.insert(key, rect);
        Ok(rect)
    }
    pub fn raster(&mut self, name: &str, physical_size: [f32; 2], scale: f32) -> Result<[f32; 4]> {
        if let Some(rect) = self.rects.get(name) {
            return Ok(*rect);
        }
        if !scale.is_finite()
            || scale <= 0.
            || physical_size
                .iter()
                .any(|v| !v.is_finite() || *v <= 0. || v.ceil() >= self.size as f32)
        {
            return Err("invalid SVG raster size or scale".into());
        }
        let key = RasterKey {
            name: name.into(),
            physical_size: physical_size.map(f32::to_bits),
            scale: scale.to_bits(),
        };
        if let Some(rect) = self.rasters.get(&key) {
            return Ok(*rect);
        }
        let tree = self
            .trees
            .get(name)
            .ok_or_else(|| format!("unregistered icon: {name}"))?;
        let [width, height] = physical_size.map(|v| v.ceil() as u32);
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("SVG raster allocation failed")?;
        resvg::render(
            tree,
            resvg::tiny_skia::Transform::from_scale(
                physical_size[0] / tree.size().width(),
                physical_size[1] / tree.size().height(),
            ),
            &mut pixmap.as_mut(),
        );
        // tiny-skia already returns premultiplied RGBA. Preserve its coverage
        // without an unpremultiply/re-premultiply round trip.
        let rect = self.insert(width, height, pixmap.data())?;
        self.rasters.insert(key, rect);
        Ok(rect)
    }
}

fn downsample(image: &Image, width: u32, height: u32) -> Vec<u8> {
    let sx = f64::from(image.width) / f64::from(width);
    let sy = f64::from(image.height) / f64::from(height);
    let mut rgba = vec![0; (width * height * 4) as usize];
    for y in 0..height {
        let (top, bottom) = (f64::from(y) * sy, f64::from(y + 1) * sy);
        for x in 0..width {
            let (left, right) = (f64::from(x) * sx, f64::from(x + 1) * sx);
            let mut sum = [0.; 4];
            for iy in top.floor() as u32..(bottom.ceil() as u32).min(image.height) {
                let wy = (bottom.min(f64::from(iy + 1)) - top.max(f64::from(iy))).max(0.);
                for ix in left.floor() as u32..(right.ceil() as u32).min(image.width) {
                    let wx = (right.min(f64::from(ix + 1)) - left.max(f64::from(ix))).max(0.);
                    let at = ((iy * image.width + ix) * 4) as usize;
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += f64::from(image.rgba[at + c]) * wx * wy;
                    }
                }
            }
            let at = ((y * width + x) * 4) as usize;
            for (c, total) in sum.into_iter().enumerate() {
                rgba[at + c] = (total / (sx * sy)).round().clamp(0., 255.) as u8;
            }
        }
    }
    rgba
}
