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
            atlas.rects.insert(image.name, rect);
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
