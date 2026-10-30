// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded PNG snapshots from the captured workspace's one images folder.
use bbt_doc::{Block, BlockKind, Inline, Markdown, markdown::ImageAsset};
use bbt_render::chrome::icons::Image;
use std::{collections::BTreeMap, path::Path, sync::Arc};

fn visit(values: &mut [Inline], load: &mut impl FnMut(&str) -> Option<ImageAsset>) {
    for value in values {
        match value {
            Inline::Image { target, asset, wiki, .. } => {
                let normalized=target.strip_prefix(".bboxy/").unwrap_or(target);
                let local=normalized.strip_prefix("images/");
                *asset=if *wiki || local.is_some() { load(local.unwrap_or(normalized)) } else {None};
            },
            Inline::Emphasis(v) | Inline::Strong(v) | Inline::Strike(v) | Inline::Link { content: v, .. } => visit(v,load),
            _ => {}
        }
    }
}
fn blocks(values: &mut [Block], load: &mut impl FnMut(&str) -> Option<ImageAsset>) {
    for block in values {
        match &mut block.kind {
            BlockKind::Paragraph(v) | BlockKind::Heading { content: v, .. } => visit(v,load),
            BlockKind::Quote(v) | BlockKind::Footnote { blocks: v, .. } => blocks(v,load),
            BlockKind::List { items, .. } => { for item in items { blocks(item,load); } }
            BlockKind::Table { header, rows, .. } => { for cell in header.iter_mut().chain(rows.iter_mut().flatten()) { visit(cell,load); } }
            _ => {}
        }
    }
}

pub fn clear(markdown: &mut Markdown) { blocks(&mut markdown.blocks,&mut |_| None); }

pub fn load(markdown: &mut Markdown, root: &Path) -> Vec<Arc<Image>> {
    bbt_rt::assert_not_ui_thread();
    let mut images = BTreeMap::<String,Arc<Image>>::new();
    let mut pixels = 0;
    let mut attempts = BTreeMap::<String,Option<ImageAsset>>::new();
    blocks(&mut markdown.blocks,&mut |target| {
        if let Some(asset)=attempts.get(target) { return asset.clone(); }
        if attempts.len() >= 16 { return None; }
        attempts.insert(target.into(),None);
        let image=decode(root,target).ok()?;
        if pixels+image.width*image.height > 4_000_000 { return None; }
        pixels += image.width*image.height;
        let asset=ImageAsset {key:image.name.clone(),width:image.width,height:image.height};
        images.insert(target.into(),Arc::new(image));
        attempts.insert(target.into(),Some(asset.clone()));
        Some(asset)
    });
    images.into_values().collect()
}

#[allow(clippy::disallowed_methods, reason = "bounded image IO on the preparation worker; load asserts not UI") ]
fn decode(root: &Path, name: &str) -> Result<Image, String> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err("image must name a file in .bboxy/images".into());
    }
    let base = root.join(".bboxy/images");
    if base.canonicalize().map_err(|e|e.to_string())? != base {
        return Err("image folder must remain in the captured workspace".into());
    }
    let path = base.join(name);
    let metadata = path.symlink_metadata().map_err(|e|e.to_string())?;
    if !metadata.is_file() || metadata.len()>16*1024*1024 || path.canonicalize().map_err(|e|e.to_string())?.parent()!=Some(base.as_path()) {
        return Err("image must be a bounded regular local file".into());
    }
    let mut options=std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file=options.open(path).map_err(|e|e.to_string())?;
    use std::io::Read;
    let mut bytes=Vec::new();
    file.take(16*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>16*1024*1024 { return Err("image grew past its read bound".into()); }
    // Adapted from bboxy450-term src/assets.rs PNG decoding; bounded local snapshots.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let mut decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e|e.to_string())?;
    let (w,h) = (reader.info().width,reader.info().height);
    if w==0 || h==0 || w>=2048 || h>=2048 { return Err("image dimensions exceed the color atlas".into()); }
    let mut data = vec![0;reader.output_buffer_size().ok_or("PNG output unavailable")?];
    let info=reader.next_frame(&mut data).map_err(|e|e.to_string())?;
    let data=&data[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.as_chunks::<3>().0.iter().flat_map(|p|[p[0],p[1],p[2],255]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|v|[*v,*v,*v,255]).collect(),
        png::ColorType::GrayscaleAlpha => data.as_chunks::<2>().0.iter().flat_map(|p|[p[0],p[0],p[0],p[1]]).collect(),
        _ => return Err("unsupported PNG color format".into()),
    };
    use sha2::Digest;
    Ok(Image {name:format!("document/{:x}",sha2::Sha256::digest(&bytes)),width:w,height:h,rgba})
}
