#![allow(
    clippy::disallowed_methods,
    reason = "headless storybook tool, not the app UI thread"
)]
pub mod atoms;
pub mod components;
pub mod fixtures;
pub mod gallery;
pub mod graphs;
pub mod documents;
pub mod launchers;
pub mod materials;
pub mod modules;
pub mod overlays;
pub mod panes;
pub mod parity;
pub mod paths;
pub mod seat_rows;
pub mod perf;
pub mod settings;
pub mod streams;
pub mod styles;
pub mod swarms;
pub mod tabs;
pub mod title_bars;
pub mod widgets;
pub mod window;
use bbt_render::chrome::icons::{Assets, Image};
use std::{io::Cursor, path::Path};
include!(concat!(env!("OUT_DIR"), "/icons.rs"));
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn images() -> Result<Assets> {
    let mut images = Vec::new();
    let (width, height, rgba) = read_png_bytes(include_bytes!("../../../lab/web/bboxy.png"))?;
    images.push(Image {
        name: "bboxy".into(),
        width,
        height,
        rgba,
    });
    Ok(Assets::with_svgs(images, BAKED_ICONS))
}
pub fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>)> {
    read_png_bytes(&std::fs::read(path)?)
}
pub fn read_png_bytes(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut data = vec![0; reader.output_buffer_size().ok_or("PNG too large")?];
    let info = reader.next_frame(&mut data)?;
    let data = &data[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|c| [*c, *c, *c, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        _ => return Err("unexpected PNG colour type".into()),
    };
    Ok((info.width, info.height, rgba))
}
