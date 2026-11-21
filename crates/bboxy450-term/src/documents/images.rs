// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded PNG snapshots from the captured workspace's one images folder.
use bbt_doc::{Block, BlockKind, Inline, Markdown, markdown::ImageAsset};
use bbt_render::chrome::icons::Image;
use std::{path::Path, sync::Arc};

fn visit(values: &mut [Inline], load: &mut impl FnMut(&str) -> Option<ImageAsset>) {
    for value in values {
        match value {
            Inline::Image {
                target,
                asset,
                wiki,
                ..
            } => {
                let normalized = target.strip_prefix(".bboxy/").unwrap_or(target);
                let local = normalized.strip_prefix("images/");
                *asset = if *wiki || local.is_some() {
                    load(local.unwrap_or(normalized))
                } else {
                    None
                };
            }
            Inline::Emphasis(v)
            | Inline::Strong(v)
            | Inline::Strike(v)
            | Inline::Link { content: v, .. } => visit(v, load),
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

pub(crate) fn load_targets(
    markdown: &mut Markdown,
    mut load: impl FnMut(&str) -> Option<ImageAsset>,
) {
    blocks(&mut markdown.blocks, &mut load);
}

pub fn clear(markdown: &mut Markdown) {
    blocks(&mut markdown.blocks, &mut |_| None);
}

#[derive(Clone)]
pub struct Binding {
    pub port: Arc<Port>,
    pub scope: bbt_proto::gui::SwarmScope,
    pub epoch: u64,
}
impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.port, &other.port) && self.stamp() == other.stamp()
    }
}
impl Binding {
    pub fn stamp(&self) -> (bbt_proto::gui::SwarmScope, u64) {
        (self.scope, self.epoch)
    }
}

/// Bind the same resolver to a seat's admitted workspace.
pub(crate) fn transcript(
    root: std::path::PathBuf,
    artifacts: Option<Binding>,
) -> Arc<bboxy450_harness::frontends::gui::rich::ImagePreparation> {
    bboxy450_harness::frontends::gui::rich::ImagePreparation::new(move |markdown| {
        load_with_artifacts(
            markdown,
            &root,
            artifacts
                .as_ref()
                .map(|binding| (binding.port.as_ref(), binding.scope)),
        )
    })
}

pub fn load(markdown: &mut Markdown, root: &Path) -> Vec<Arc<Image>> {
    load_with_artifacts(markdown, root, None)
}

pub fn load_with_artifacts(
    markdown: &mut Markdown,
    root: &Path,
    artifacts: Option<(&Port, bbt_proto::gui::SwarmScope)>,
) -> Vec<Arc<Image>> {
    load_for_host(markdown, root, artifacts).0
}

pub(crate) fn load_for_host(
    markdown: &mut Markdown,
    root: &Path,
    artifacts: Option<(&Port, bbt_proto::gui::SwarmScope)>,
) -> (Vec<Arc<Image>>, bool) {
    use bbt_mux::artifact_images::{Images, Prepared};
    bbt_rt::assert_not_ui_thread();
    let mut images = Images::default();
    let mut follows_artifact = false;
    load_targets(markdown, |target| {
        if let Some(address) = target.strip_prefix("artifact:") {
            follows_artifact |= bboxy450_canvas::artifact_address(address)
                .is_some_and(|(_, revision)| revision.is_none());
        }
        images.prepare(target, |remaining| {
            if target.starts_with("artifact:") {
                let (port, scope) = artifacts
                    .ok_or_else(|| bbt_mux::store::invalid("artifact host unavailable"))?;
                port.image(target, scope, remaining)
            } else {
                decode(root, target, remaining)
                    .map(Prepared::raster)
                    .map_err(bbt_mux::store::invalid)
            }
        })
    });
    (images.into_rasters(), follows_artifact)
}

/// The document and rich transcript workers share one native read door and
/// a disposable geometry cache. They never derive a filesystem path from Markdown.
pub struct Port {
    runtime: std::path::PathBuf,
    sender: tokio::sync::mpsc::WeakSender<crate::cockpit::UiMsg>,
    geometry: bbt_mux::artifact_images::GeometryCache,
}

impl Port {
    pub fn new(
        runtime: std::path::PathBuf,
        sender: tokio::sync::mpsc::WeakSender<crate::cockpit::UiMsg>,
    ) -> Self {
        Self {
            runtime,
            sender,
            geometry: Default::default(),
        }
    }
    pub fn resolve(
        &self,
        target: &str,
        scope: bbt_proto::gui::SwarmScope,
    ) -> bbt_mux::store::Result<bbt_mux::artifact_images::Resolved> {
        use bbt_mux::store::invalid;
        bbt_rt::assert_not_ui_thread();
        let (id, revision) =
            bboxy450_canvas::artifact_address(target.strip_prefix("artifact:").unwrap_or(target))
                .ok_or_else(|| invalid("invalid artifact address"))?;
        let sender = self
            .sender
            .upgrade()
            .ok_or_else(|| invalid("artifact reader detached"))?;
        let (reply, receive) = tokio::sync::oneshot::channel();
        sender
            .try_send(crate::cockpit::UiMsg::ArtifactImage {
                id,
                revision,
                scope,
                reply,
            })
            .map_err(|_| invalid("artifact reader busy"))?;
        bbt_rt::handle().block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(3), receive)
                .await
                .map_err(|_| invalid("artifact read expired"))?
                .map_err(|_| invalid("artifact reader detached"))?
                .map_err(invalid)
        })
    }
    pub fn prepare(
        &self,
        found: &bbt_mux::artifact_images::Resolved,
        remaining: u64,
    ) -> bbt_mux::store::Result<bbt_mux::artifact_images::Prepared> {
        bbt_mux::artifact_images::Prepared::artifact(
            found,
            &self.runtime,
            &self.geometry,
            remaining,
        )
    }
    pub fn bytes(
        &self,
        found: &bbt_mux::artifact_images::Resolved,
    ) -> bbt_mux::store::Result<Vec<u8>> {
        found.bytes(&self.runtime)
    }
    pub fn image(
        &self,
        target: &str,
        scope: bbt_proto::gui::SwarmScope,
        remaining: u64,
    ) -> bbt_mux::store::Result<bbt_mux::artifact_images::Prepared> {
        let found = self.resolve(target, scope)?;
        self.prepare(&found, remaining)
    }
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
