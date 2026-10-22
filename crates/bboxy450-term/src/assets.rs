//! Build-time baked icons and the existing identity portrait; no runtime files.
use bbt_render::chrome::icons::Image;
use std::io::Cursor;
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

pub fn images() -> Result<Vec<Image>, String> {
    let mut images: Vec<_> = BAKED_ICONS
        .iter()
        .map(|(name, width, height, rgba)| Image {
            name: (*name).into(),
            width: *width,
            height: *height,
            rgba: rgba.to_vec(),
        })
        .collect();
    let bytes = include_bytes!("../../../lab/web/bboxy.png");
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut data = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or("identity image too large")?
    ];
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
        _ => return Err("unsupported identity PNG format".into()),
    };
    images.push(Image {
        name: "bboxy".into(),
        width: info.width,
        height: info.height,
        rgba,
    });
    Ok(images)
}
