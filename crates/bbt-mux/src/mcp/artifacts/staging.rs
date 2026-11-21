// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    knowledge::BboxySource,
    provenance::stored::{MAX_FILE, PNG, SVG},
    store::{Result, invalid},
};
use std::{io::Read, os::unix::fs::MetadataExt, path::Path};

#[allow(
    clippy::disallowed_methods,
    reason = "staged artifact input reads run on the existing IO owner"
)]
pub(crate) fn read(root: &Path, path: &str) -> Result<Vec<u8>> {
    bbt_rt::assert_not_ui_thread();
    let root = root.canonicalize()?;
    let path = Path::new(path);
    let relative = if path.is_absolute() {
        path.strip_prefix(&root).map_err(|_| {
            invalid("artifact_source_forbidden: staged file is outside the captured workspace")
        })?
    } else {
        path
    };
    let open = || {
        BboxySource::open_resource(&root, relative)
            .map_err(|e| invalid(format!("artifact_source_unavailable: {e}")))
    };
    let (file, _, size) = open()?;
    if size > MAX_FILE as u64 {
        return Err(invalid(
            "artifact_too_large: staged content exceeds 16777216 bytes",
        ));
    }
    let metadata = |file: &std::fs::File| -> Result<_> {
        let stat = file.metadata()?;
        Ok((
            stat.dev(),
            stat.ino(),
            stat.size(),
            stat.mtime(),
            stat.mtime_nsec(),
            stat.ctime(),
            stat.ctime_nsec(),
        ))
    };
    let before = metadata(&file)?;
    let mut bytes = Vec::new();
    (&file)
        .take((MAX_FILE + 1) as u64)
        .read_to_end(&mut bytes)?;
    let (current, _, _) = open()?;
    if bytes.len() > MAX_FILE {
        return Err(invalid(
            "artifact_too_large: staged content exceeds 16777216 bytes",
        ));
    }
    if before != metadata(&file)? || before != metadata(&current)? || bytes.len() as u64 != size {
        return Err(invalid(
            "artifact_source_changed: staged file changed during its copy",
        ));
    }
    Ok(bytes)
}

/// A displayed revision reaches the same bounded preparation as a viewed image.
pub(crate) fn validate_image(media_type: &str, bytes: &[u8]) -> Result<()> {
    match media_type {
        SVG => bboxy450_svg::prepare_image(bytes, 4_000_000)
            .map(|_| ())
            .map_err(|reason| invalid(format!("artifact_image_refused: {reason}"))),
        PNG => bbt_render::image::decode_png(bytes, 4_000_000)
            .map(|_| ())
            .map_err(|reason| invalid(format!("artifact_image_refused: {reason}"))),
        _ => Ok(()),
    }
}
