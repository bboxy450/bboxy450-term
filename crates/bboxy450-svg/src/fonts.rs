// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{Diagnostic, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Deserialize)]
pub struct Face {
    file: String,
    pub weight: u16,
    style: String,
    pub postscript_name: String,
    pub sha256: String,
}
#[derive(Deserialize)]
struct License {
    file: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Manifest {
    family: String,
    faces: Vec<Face>,
    license: License,
}

/// One immutable active font resource, initialized explicitly by library callers.
/// Every name and digest comes from the sole resource manifest.
#[derive(Debug, Clone)]
pub struct FontSet {
    directory: PathBuf,
    identity: Arc<str>,
    family: Arc<str>,
    faces: Arc<[Face]>,
}
impl FontSet {
    #[allow(
        clippy::disallowed_methods,
        reason = "immutable font initialization runs on the preparation worker"
    )]
    pub fn load(directory: &Path) -> Result<Self> {
        let unavailable = |s: String| Diagnostic::refused("font_unavailable", s);
        if !cfg!(target_os = "macos") {
            return Err(unavailable("native chart text requires macOS".into()));
        }
        let directory = directory
            .canonicalize()
            .map_err(|e| unavailable(e.to_string()))?;
        let read = |name: &str| -> Result<Vec<u8>> {
            if name.is_empty() || Path::new(name).file_name().and_then(|s| s.to_str()) != Some(name)
            {
                return Err(unavailable("font resource path must be a leaf".into()));
            }
            let path = directory.join(name);
            let metadata = path
                .symlink_metadata()
                .map_err(|e| unavailable(e.to_string()))?;
            if !metadata.is_file() || metadata.len() > 16_777_216 {
                return Err(unavailable(
                    "font resource is not a bounded regular file".into(),
                ));
            }
            let file = std::fs::File::open(path).map_err(|e| unavailable(e.to_string()))?;
            let mut bytes = Vec::new();
            file.take(16_777_217)
                .read_to_end(&mut bytes)
                .map_err(|e| unavailable(e.to_string()))?;
            if bytes.len() > 16_777_216 {
                return Err(unavailable("font resource grew past byte ceiling".into()));
            }
            Ok(bytes)
        };
        let raw = read("manifest.json")?;
        let manifest: Manifest =
            serde_json::from_slice(&raw).map_err(|e| unavailable(e.to_string()))?;
        if manifest.faces.len() != 2
            || manifest.family.is_empty()
            || manifest.faces.iter().map(|f| f.weight).collect::<Vec<_>>() != [400, 700]
        {
            return Err(unavailable(
                "manifest must name the two approved static faces".into(),
            ));
        }
        let check = |name: &str, digest: &str| -> Result<()> {
            if format!("{:x}", Sha256::digest(read(name)?)) != digest {
                return Err(unavailable(format!("resource digest mismatch: {name}")));
            }
            Ok(())
        };
        check(&manifest.license.file, &manifest.license.sha256)?;
        for face in &manifest.faces {
            if face.style != "normal" {
                return Err(unavailable("unsupported face style".into()));
            }
            check(&face.file, &face.sha256)?;
        }
        for face in &manifest.faces {
            let path = directory.join(&face.file);
            // A repeated library initialization can encounter an already registered
            // process-local face. It is accepted only after the same actual-file check.
            let registration = bbt_render::text::register_font_file(&path);
            if let Err(identity) =
                bbt_render::text::verify_font_file(&path, &manifest.family, &face.postscript_name)
            {
                return Err(unavailable(format!(
                    "{identity}; registration: {registration:?}"
                )));
            }
        }
        Ok(Self {
            directory,
            identity: format!("sha256:{:x}", Sha256::digest(raw)).into(),
            family: manifest.family.into(),
            faces: manifest.faces.into(),
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn face(&self, weight: u16) -> Result<&Face> {
        self.faces
            .iter()
            .find(|f| f.weight == weight)
            .ok_or_else(|| {
                Diagnostic::refused("font_unavailable", format!("unsupported weight {weight}"))
            })
    }
    pub fn admit(&self, css_family: &str, weight: u16, text: &str) -> Result<&Face> {
        let families = svgtypes::parse_font_families(css_family)
            .map_err(|e| Diagnostic::refused("font_family", e.to_string()))?;
        if families != [svgtypes::FontFamily::Named(self.family.to_string())] {
            return Err(Diagnostic::refused(
                "font_family",
                "text must select the manifest family without fallback",
            ));
        }
        let face = self.face(weight)?;
        if !bbt_render::text::font_covers(&face.postscript_name, text)
            .map_err(|e| Diagnostic::refused("font_unavailable", e.to_string()))?
        {
            return Err(Diagnostic::unsupported("glyph outside the approved face"));
        }
        Ok(face)
    }
}
