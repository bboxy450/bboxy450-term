// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Shared preparation for document, transcript, and canvas images.
use crate::{
    Ulid,
    store::{Result, invalid},
};
#[cfg(feature = "canvas-image")]
use bbt_render::chrome::icons::Image;
use bbt_supervisor::store::{Blobs, blobs::LocalBlobs};
#[cfg(feature = "canvas-image")]
use bbt_ui_model::draw::{ImageAsset, RetainedImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(feature = "canvas-image")]
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

#[cfg(feature = "canvas-image")]
const MAX_PIXELS: u64 = 4_000_000;
#[cfg(feature = "canvas-image")]
const MAX_ATTEMPTS: usize = 16;

pub(crate) fn object_path(runtime: &Path, root: &str) -> PathBuf {
    runtime.join("artifact-objects").join(root)
}

pub(crate) fn payload(objects: &Path, digest: &str, bytes: usize) -> Result<Vec<u8>> {
    bbt_rt::assert_not_ui_thread();
    let value = LocalBlobs::open(objects)?
        .get(digest)
        .map_err(|e| invalid(format!("artifact_unavailable: {e}")))?;
    if value.len() != bytes || bytes > crate::provenance::stored::MAX_FILE {
        return Err(invalid(
            "artifact_unavailable: stored byte length disagrees",
        ));
    }
    Ok(value)
}

/// A small answer from the native artifact reader, scoped by the read owner.
/// The actual bytes remain in Blobs and are verified before image preparation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolved {
    pub root: String,
    pub id: Ulid,
    pub revision: u64,
    pub digest: String,
    pub media_type: String,
    pub bytes: usize,
}
impl Resolved {
    pub fn reference(&self) -> String {
        format!("artifact:{}:{}", self.id, self.revision)
    }
    pub fn bytes(&self, runtime: &Path) -> Result<Vec<u8>> {
        if self.root.len() != 64
            || !self
                .root
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(invalid("artifact has no canonical native root"));
        }
        payload(&object_path(runtime, &self.root), &self.digest, self.bytes)
    }
}
#[cfg(feature = "canvas-image")]
#[derive(Clone)]
struct Geometry {
    picture: RetainedImage,
    png_pixels: u64,
}
#[cfg(feature = "canvas-image")]
#[derive(Default)]
pub struct GeometryCache(Mutex<VecDeque<(String, Geometry)>>);

#[cfg(feature = "canvas-image")]
impl GeometryCache {
    fn scene(
        &self,
        digest: &str,
        remaining: u64,
        source: impl FnOnce() -> Result<Vec<u8>>,
    ) -> Result<Geometry> {
        let fonts = bboxy450_svg::FontSet::bundled()?;
        let key = format!("{digest}/{}", fonts.identity());
        if let Ok(mut cache) = self.0.try_lock()
            && let Some(i) = cache.iter().position(|(cached, _)| *cached == key)
        {
            let (_, geometry) = cache.remove(i).expect("found retained scene");
            if geometry.png_pixels > remaining {
                return Err(invalid("image exceeds the host decoded PNG budget"));
            }
            cache.push_back((key, geometry.clone()));
            return Ok(geometry);
        }
        let scene = bboxy450_svg::prepare_image(&source()?, remaining)?;
        let geometry = Geometry {
            png_pixels: scene.decoded_pixels(),
            picture: scene.retained_image(),
        };
        if let Ok(mut cache) = self.0.try_lock() {
            while cache.len() >= 8
                || cache.iter().map(|(_, value)| value.png_pixels).sum::<u64>()
                    + geometry.png_pixels
                    > MAX_PIXELS
            {
                cache.pop_front();
            }
            cache.push_back((key, geometry.clone()));
        }
        Ok(geometry)
    }
}

#[cfg(feature = "canvas-image")]
pub struct Prepared {
    asset: ImageAsset,
    raster: Option<Arc<Image>>,
    png_pixels: u64,
}

#[cfg(feature = "canvas-image")]
impl Prepared {
    pub fn raster(raster: Image) -> Self {
        let asset = ImageAsset {
            key: raster.name.clone(),
            width: raster.width,
            height: raster.height,
            retained: None,
            revision: None,
        };
        let png_pixels = u64::from(raster.width) * u64::from(raster.height);
        Self {
            asset,
            raster: Some(Arc::new(raster)),
            png_pixels,
        }
    }
    pub fn artifact(
        image: &Resolved,
        runtime: &Path,
        cache: &GeometryCache,
        remaining: u64,
    ) -> Result<Self> {
        use crate::provenance::stored::{PNG, SVG};
        let key = format!("artifact/{}", image.digest);
        let mut prepared = match image.media_type.as_str() {
            PNG => {
                let raster = bbt_render::image::decode_png(&image.bytes(runtime)?, remaining)?;
                Self::raster(Image {
                    name: key,
                    width: raster.width,
                    height: raster.height,
                    rgba: raster.rgba,
                })
            }
            SVG => {
                let bytes = image.bytes(runtime)?;
                let geometry = cache.scene(&image.digest, remaining, || Ok(bytes))?;
                let [width, height] = geometry.picture.size();
                Self {
                    asset: ImageAsset {
                        key,
                        width: width.ceil() as u32,
                        height: height.ceil() as u32,
                        revision: None,
                        retained: Some(geometry.picture),
                    },
                    raster: None,
                    png_pixels: geometry.png_pixels,
                }
            }
            _ => return Err(invalid("stored artifact is not a display image")),
        };
        prepared.asset.revision = Some(image.reference());
        Ok(prepared)
    }
}

#[cfg(feature = "canvas-image")]
#[derive(Default)]
pub struct Images {
    attempts: BTreeMap<String, Option<ImageAsset>>,
    rasters: BTreeMap<String, Arc<Image>>,
    pixels: u64,
}
#[cfg(feature = "canvas-image")]
impl Images {
    pub fn prepare(
        &mut self,
        target: &str,
        read: impl FnOnce(u64) -> Result<Prepared>,
    ) -> Option<ImageAsset> {
        if let Some(value) = self.attempts.get(target) {
            return value.clone();
        }
        if self.attempts.len() >= MAX_ATTEMPTS {
            return None;
        }
        self.attempts.insert(target.to_owned(), None);
        let remaining = MAX_PIXELS - self.pixels;
        let Prepared {
            asset,
            raster,
            png_pixels,
        } = read(remaining).ok()?;
        if png_pixels > remaining {
            return None;
        }
        self.pixels += png_pixels;
        if let Some(raster) = raster {
            self.rasters.insert(raster.name.clone(), raster);
        }
        self.attempts.insert(target.to_owned(), Some(asset.clone()));
        Some(asset)
    }
    pub fn canvas(
        &mut self,
        canvas: &bboxy450_canvas::Canvas,
        mut read: impl FnMut(&str, u64) -> Result<Prepared>,
    ) {
        for element in canvas.elements() {
            if let bboxy450_canvas::Content::Image { artifact, .. } = &element.content {
                self.prepare(artifact, |remaining| {
                    let mut prepared = read(artifact, remaining)?;
                    // A canvas image occupies only its authored bounds.
                    // The export still returns the resolved pin as metadata.
                    prepared.asset.revision = None;
                    Ok(prepared)
                });
            }
        }
    }

    pub fn assets(&self) -> BTreeMap<String, ImageAsset> {
        self.attempts
            .iter()
            .filter_map(|(target, result)| result.clone().map(|asset| (target.clone(), asset)))
            .collect()
    }
    pub fn into_rasters(self) -> Vec<Arc<Image>> {
        self.rasters.into_values().collect()
    }
}
