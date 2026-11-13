// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Embedded SVG sources and the existing identity portrait; no runtime files.
use bbt_render::chrome::icons::{Assets, Image};
use std::io::Cursor;
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// PNG bytes to straight-alpha RGBA: the identity mark and terminal pictures.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut data = vec![0; reader.output_buffer_size().ok_or("image too large")?];
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
        png::ColorType::Grayscale => data.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return Err("unsupported PNG format".into()),
    };
    Ok((info.width, info.height, rgba))
}

pub fn images() -> Result<Assets, String> {
    let mut images = Vec::new();
    let bytes = include_bytes!(concat!(env!("BBOXY_INPUT_ROOT"), "/lab/web/bboxy.png"));
    let (width, height, rgba) = decode_png(bytes)?;
    images.push(Image {
        name: "bboxy".into(),
        width,
        height,
        rgba,
    });
    let mut assets = Assets::with_svgs(images, BAKED_ICONS);
    // A kept canvas uses the suite's existing document mark.
    let (_, book) = assets
        .svgs
        .iter()
        .find(|(name, _)| name == "book")
        .ok_or("missing document mark")?;
    assets.svgs.push(("canvas".into(), book.clone()));
    // The linked harness keeps its native icon bytes without replacing term's
    // same-named marks. rich_pane namespaces only the embedded Frame references.
    let rich = bboxy450_harness::frontends::gui::assets::icons();
    assets
        .bitmaps
        .extend(rich.bitmaps.into_iter().map(|mut image| {
            image.name = format!("harness-rich/{}", image.name);
            image
        }));
    assets.svgs.extend(
        rich.svgs
            .into_iter()
            .map(|(name, svg)| (format!("harness-rich/{name}"), svg)),
    );
    Ok(assets)
}
