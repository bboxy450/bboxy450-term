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
            BlockKind::Quote(v) | BlockKind::Callout { blocks: v, .. } | BlockKind::Footnote { blocks: v, .. } => blocks(v,load),
            BlockKind::List { items, .. } => { for item in items { blocks(item,load); } }
            BlockKind::Table { header, rows, .. } => { for cell in header.iter_mut().chain(rows.iter_mut().flatten()) { visit(cell,load); } }
            _ => {}
        }
    }
}

pub fn clear(markdown: &mut Markdown) {
    blocks(&mut markdown.blocks, &mut |_| None);
}

/// Bind the same local PNG resolver to a seat's admitted workspace.
pub(crate) fn transcript(
    root: std::path::PathBuf,
) -> Arc<bboxy450_harness::frontends::gui::rich::ImagePreparation> {
    bboxy450_harness::frontends::gui::rich::ImagePreparation::new(move |markdown| {
        load(markdown, &root)
    })
}

pub fn load(markdown: &mut Markdown, root: &Path) -> Vec<Arc<Image>> {
    bbt_rt::assert_not_ui_thread();
    let mut images = BTreeMap::<String,Arc<Image>>::new();
    let mut pixels = 0;
    let mut attempts = BTreeMap::<String, Option<ImageAsset>>::new();
    blocks(&mut markdown.blocks, &mut |target| {
        if let Some(asset) = attempts.get(target) {
            return asset.clone();
        }
        if attempts.len() >= 16 {
            return None;
        }
        attempts.insert(target.into(), None);
        let image = decode(root, target, 4_000_000 - pixels).ok()?;
        pixels += u64::from(image.width) * u64::from(image.height);
        let asset = ImageAsset {
            key: image.name.clone(),
            width: image.width,
            height: image.height,
        };
        images.insert(target.into(), Arc::new(image));
        attempts.insert(target.into(), Some(asset.clone()));
        Some(asset)
    });
    images.into_values().collect()
}

#[allow(
    clippy::disallowed_methods,
    reason = "bounded image IO on the preparation worker; load asserts not UI"
)]
fn decode(root: &Path, name: &str, remaining_pixels: u64) -> Result<Image, String> {
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("image must stay a regular file".into());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("image grew past its read bound".into());
    }
    let image =
        bbt_render::image::decode_png(&bytes, remaining_pixels).map_err(|e| e.to_string())?;
    use sha2::Digest;
    Ok(Image {
        name: format!("document/{:x}", sha2::Sha256::digest(&bytes)),
        width: image.width,
        height: image.height,
        rgba: image.rgba,
    })
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "private filesystem fixture runs outside the UI thread"
)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};

    fn png(path: &Path, width: u32, height: u32) {
        let mut encoder = png::Encoder::new(fs::File::create(path).unwrap(), width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![71; (width * height) as usize])
            .unwrap();
    }

    fn populated(doc: &Markdown) -> usize {
        let mut count = 0;
        for block in &doc.blocks {
            if let BlockKind::Paragraph(inlines) = &block.kind {
                count += inlines
                    .iter()
                    .filter(|inline| matches!(inline, Inline::Image { asset: Some(_), .. }))
                    .count();
            }
        }
        count
    }

    #[test]
    fn images_resolve_a_message_with_refusals_and_whole_message_limits() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().canonicalize().unwrap();
        let base = root.join(".bboxy/images");
        fs::create_dir_all(&base).unwrap();
        png(&base.join("ok.png"), 2047, 1);
        png(&base.join("side.png"), 2048, 1);
        fs::File::create(base.join("oversize.png"))
            .unwrap()
            .set_len(16_777_217)
            .unwrap();
        png(&base.join("large.png"), 2000, 2000);
        png(&root.join("outside.png"), 2, 2);
        symlink(root.join("outside.png"), base.join("linked.png")).unwrap();
        let source = [
            "first ![ok](images/ok.png) last",
            "![again](images/ok.png)",
            "![absent](images/absent.png)",
            "![oversize](images/oversize.png)",
            "![side](images/side.png)",
            "![link](images/linked.png)",
            "![escape](images/../outside.png)",
            "![pixel budget](images/large.png)",
        ]
        .join(
            "

",
        );
        let mut markdown = bbt_doc::parse(&source);
        let assets = load(&mut markdown, &root);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].width, 2047);
        assert_eq!(populated(&markdown), 2);
        assert!(markdown.source.contains("first ") && markdown.source.contains(" last"));

        let mut sixteen = (0..16)
            .map(|n| format!("![missing](images/missing-{n}.png)"))
            .collect::<Vec<_>>();
        sixteen.insert(1, sixteen[0].clone());
        sixteen.push("![refused](images/ok.png)".into());
        let mut markdown = bbt_doc::parse(&sixteen.join(
            "

",
        ));
        assert!(load(&mut markdown, &root).is_empty());
        sixteen.remove(2);
        let mut markdown = bbt_doc::parse(&sixteen.join(
            "

",
        ));
        assert_eq!(load(&mut markdown, &root).len(), 1);
    }
}
