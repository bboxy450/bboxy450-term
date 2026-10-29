// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private local image custody. Clipboard bytes are copied; arbitrary source paths are never opened.
use bboxy450_term_desktop::{ClipboardImageFormat, MAX_CLIPBOARD_IMAGE_BYTES};
use bbt_ui::modules::composer::Attachment;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Bytes from the clipboard or a file explicitly selected through the operator control door.
pub enum Source {
    Clipboard {
        bytes: Vec<u8>,
        format: ClipboardImageFormat,
    },
    File(PathBuf),
}

/// Only this ownership token can unlink a draft image. Drop queues IO off the window thread.
#[derive(Debug)]
pub struct OwnedImage {
    path: PathBuf,
    preserve: bool,
    committed: bool,
}
impl OwnedImage {
    /// Runs only on a blocking worker; never opens a source path in a window callback.
    #[allow(clippy::disallowed_methods)] // Bounded source read on the dedicated blocking IO path.
    pub fn from_source(source: Source) -> Result<Self, String> {
        match source {
            Source::Clipboard { bytes, format } => Self::save(&bytes, format),
            Source::File(path) => {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(path)
                    .map_err(|error| format!("image_source: {error}"))?;
                let metadata = file
                    .metadata()
                    .map_err(|error| format!("image_source: {error}"))?;
                if !metadata.is_file() || metadata.len() > MAX_CLIPBOARD_IMAGE_BYTES as u64 {
                    return Err("image_source_must_be_bounded_regular_file".into());
                }
                let mut bytes = Vec::new();
                (&mut file)
                    .take(MAX_CLIPBOARD_IMAGE_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| format!("image_source: {error}"))?;
                if bytes.len() > MAX_CLIPBOARD_IMAGE_BYTES {
                    return Err("clipboard_image_too_large".into());
                }
                let format = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    ClipboardImageFormat::Png
                } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
                    ClipboardImageFormat::Tiff
                } else {
                    return Err("image_source_requires_png_or_tiff".into());
                };
                Self::save(&bytes, format)
            }
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Runs on a blocking worker. tempfile creates a new owner-only file without following links.
    pub fn save(bytes: &[u8], format: ClipboardImageFormat) -> Result<Self, String> {
        let png =
            bboxy450_term_desktop::clipboard_image_png(bytes, format).map_err(|e| e.to_string())?;
        Self::save_png(&png)
    }
    #[allow(clippy::disallowed_methods)] // This private writer is called only on a blocking worker.
    fn save_png(png: &[u8]) -> Result<Self, String> {
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") || png.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err("clipboard_image_invalid".into());
        }
        let mut file = tempfile::Builder::new()
            .prefix("bboxy450-image-")
            .suffix(".png")
            .tempfile()
            .map_err(|e| e.to_string())?;
        file.write_all(png)
            .and_then(|()| file.as_file().sync_data())
            .map_err(|e| e.to_string())?;
        let (_, path) = file.keep().map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            preserve: false,
            committed: false,
        })
    }
}
impl Drop for OwnedImage {
    #[allow(clippy::disallowed_methods)] // The removal itself runs inside spawn_blocking.
    fn drop(&mut self) {
        if !self.preserve {
            let path = self.path.clone();
            drop(bbt_rt::handle().spawn_blocking(move || {
                let _ = std::fs::remove_file(path);
            }));
        }
    }
}
#[derive(Default)]
pub struct Images {
    owned: BTreeMap<String, OwnedImage>,
}
impl Images {
    pub fn ready(&mut self, id: &str, image: OwnedImage, draft: &[Attachment]) -> Option<String> {
        if !draft.iter().any(|a| a.kind == "image" && a.id == id) {
            return None;
        }
        let path = image.path.to_string_lossy().into_owned();
        self.owned.insert(id.into(), image);
        Some(path)
    }
    pub fn validate(&self, draft: &[Attachment]) -> bool {
        draft.iter().filter(|a| a.kind == "image").all(|a| {
            self.owned
                .get(&a.id)
                .is_some_and(|image| image.path == Path::new(&a.path))
        })
    }
    /// Transfer newly accepted files to message custody once, before clearing its draft.
    /// The bounded map contains only current draft/pending images; sent history is never scanned.
    pub fn committed(&mut self, paths: &[String]) {
        for image in self.owned.values_mut() {
            if paths.iter().any(|p| image.path == Path::new(p)) {
                image.committed = true;
                image.preserve = true;
            }
        }
    }
    /// Keep only current draft or pending/unknown tokens. Committed files remain
    /// readable after relinquishing the token; refused unsent tokens still unlink.
    pub fn reconcile(&mut self, draft: &[Attachment], protected: &[String]) {
        self.owned.retain(|id, image| {
            let pending = protected.iter().any(|p| image.path == Path::new(p));
            image.preserve = image.committed || pending;
            pending || draft.iter().any(|a| a.kind == "image" && a.id == *id)
        });
    }
}
/// This only formats the owned attachment; it never reads or grants a supplied path.
/// The window owner checks Images::validate before admitting a batch.
pub fn reference(image: &Attachment) -> Result<String, &'static str> {
    let path = Path::new(&image.path);
    if !path.is_absolute()
        || !path.components().all(|c| {
            matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
        || !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("bboxy450-image-") && n.ends_with(".png"))
        || image.path.contains(['[', ']', '|'])
        || image.path.chars().any(char::is_control)
        || !image.title.starts_with("[Image #")
        || !image.title.ends_with(']')
        || image.title[8..image.title.len() - 1]
            .parse::<usize>()
            .ok()
            .is_none_or(|n| n == 0)
    {
        return Err("image_attachment_invalid");
    }
    let value = format!(
        "[[{}|{}]]",
        image.path,
        &image.title[1..image.title.len() - 1]
    );
    if value.len() > 512 {
        return Err("attachment_reference_too_large");
    }
    Ok(value)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // Disposable files/processes; no UI callback runs here.
mod tests {
    use super::*;
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nprivate fixture bytes";
    fn attachment(image: &OwnedImage, id: &str) -> Attachment {
        Attachment {
            kind: "image".into(),
            id: id.into(),
            title: "[Image #1]".into(),
            path: image.path.to_string_lossy().into_owned(),
            ..Default::default()
        }
    }
    fn removed(path: &Path) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            !path.exists(),
            "draft file was not cleaned: {}",
            path.display()
        );
    }
    #[test]
    fn image_custody_deletes_cancelled_or_removed_drafts_and_never_arbitrary_paths() {
        let mut files = Images::default();
        let image = OwnedImage::save_png(PNG).unwrap();
        let path = image.path.clone();
        let a = attachment(&image, "one");
        files.ready("one", image, std::slice::from_ref(&a)).unwrap();
        assert!(files.validate(std::slice::from_ref(&a)));
        let mut forged = a.clone();
        forged.path = "/etc/passwd".into();
        assert!(!files.validate(&[forged.clone()]));
        assert!(reference(&forged).is_err());
        files.reconcile(&[], &[]);
        removed(&path);
        let cancelled = OwnedImage::save_png(PNG).unwrap();
        let path = cancelled.path.clone();
        assert!(files.ready("removed-before-save", cancelled, &[]).is_none());
        removed(&path);
    }
    #[test]
    fn image_custody_retains_admitted_unknown_and_stored_files_across_owner_restart() {
        let mut files = Images::default();
        let image = OwnedImage::save_png(PNG).unwrap();
        let path = image.path.clone();
        let a = attachment(&image, "one");
        files.ready("one", image, std::slice::from_ref(&a)).unwrap();
        files.reconcile(&[], std::slice::from_ref(&a.path));
        drop(files);
        assert_eq!(std::fs::read(&path).unwrap(), PNG);
        std::fs::remove_file(&path).unwrap();
    }
    #[test]
    fn image_custody_cleans_explicitly_refused_removed_draft_after_pending_preservation() {
        let mut files = Images::default();
        let image = OwnedImage::save_png(PNG).unwrap();
        let path = image.path.clone();
        let a = attachment(&image, "one");
        files.ready("one", image, std::slice::from_ref(&a)).unwrap();
        files.reconcile(&[], std::slice::from_ref(&a.path));
        assert!(path.exists());
        files.reconcile(&[], &[]);
        removed(&path);
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn image_custody_local_recipient_reads_png_from_signed_attachment_after_gui_and_mux_restart() {
        use crate::{
            cockpit::UiMsg,
            messaging::{Client, Reply, Results},
        };
        use bbt_mux::{
            Ulid,
            messaging::{Messaging, keys::LocalSeatKeys},
            registry::{Registry, SeatRecord},
            store::{object_dir, sqlite::SqliteStore},
        };
        use bbt_proto::messaging::{Request, Response};
        use bbt_ui::modules::composer::{Address, Command, Composer, Controller, Outcome};
        use std::sync::Arc;
        let root = tempfile::tempdir().unwrap();
        let swarm = Ulid::new();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let sender = SeatRecord::new(Ulid::new(), swarm, "fixture sender", "claude");
        let recipient = SeatRecord::new(Ulid::new(), swarm, "fixture recipient", "claude");
        registry.insert(&sender).unwrap();
        registry.insert(&recipient).unwrap();
        registry
            .set_attention(
                recipient.id,
                recipient.generation,
                bbt_proto::registry::AttentionRule {
                    destination: format!("seats/{}/inbox", recipient.id),
                    class: bbt_proto::messaging::MessageClass::Conversation,
                    mode: bbt_proto::messaging::WakeMode::Wake,
                },
            )
            .unwrap();
        let keys = Arc::new(LocalSeatKeys::new(ed25519_dalek::SigningKey::from_bytes(
            &[42; 32],
        )));
        let mut mux = Messaging::open(root.path(), keys.clone()).unwrap();
        let mut png = Vec::new();
        {
            let mut writer = png::Encoder::new(&mut png, 1, 1).write_header().unwrap();
            writer.write_image_data(&[123]).unwrap();
        }
        let image = OwnedImage::save(&png, ClipboardImageFormat::Png).unwrap();
        let path = image.path.clone();
        let mut editor = Controller::new(Composer {
            focus: true,
            to: vec![Address {
                kind: "seat".into(),
                destination: format!("seats/{}/inbox", recipient.id),
                ..Default::default()
            }],
            ..Default::default()
        });
        editor.handle(Command::Attach(Attachment {
            kind: "image".into(),
            id: "one".into(),
            ..Default::default()
        }));
        let mut images = Images::default();
        let owned = images.ready("one", image, &editor.view.attach).unwrap();
        editor.handle(Command::ImageReady {
            id: "one".into(),
            path: owned,
        });
        let Outcome::Submit(draft) = editor.handle(Command::Send) else {
            panic!("image-only message must send")
        };
        assert!(images.validate(&draft.attach));
        let mut client = Client::default();
        let commands = client.admitted(
            client
                .prepare(Some(bbt_proto::SwarmId::from_u128(swarm.0)), draft)
                .unwrap(),
        );
        let UiMsg::Messaging {
            id,
            request: Request::Send { message },
            ..
        } = &commands[0]
        else {
            panic!("send")
        };
        images.reconcile(&[], &client.protected_image_paths());
        let stored = mux.send(sender.id, 1, message.clone(), 1000).unwrap();
        assert!(stored.envelope().is_some());
        let reference = stored.view().attachments[0].clone();
        assert!(reference.contains(path.to_str().unwrap()));
        let mut results = Results::default();
        results.push(
            *id,
            Reply::Messaging(Response::Stored {
                message: Box::new(stored),
            }),
        );
        let (submitted, _, committed) = client.replies_with_images(&results);
        images.committed(&committed);
        editor.stored(&submitted.unwrap());
        assert!(editor.view.attach.is_empty());
        images.reconcile(&editor.view.attach, &client.protected_image_paths());
        assert!(
            images.owned.is_empty(),
            "successful send relinquishes its owned token"
        );
        assert!(client.protected_image_paths().is_empty());
        drop(images);
        drop(editor);
        drop(client);
        drop(mux);
        let mut mux = Messaging::open(root.path(), keys).unwrap();
        mux.ready(2000).unwrap();
        let work = mux.prepare(swarm, recipient.id, 2000).unwrap().unwrap();
        assert_eq!(work.messages[0].view().attachments[0], reference);
        assert!(work.text().contains(&reference));
        let output = std::process::Command::new("python3").args(["-I","-c","import pathlib,sys; b=pathlib.Path(sys.argv[1]).read_bytes(); assert b[:8] == bytes([137,80,78,71,13,10,26,10]); print(len(b))"]).arg(&path).output().unwrap();
        assert!(
            output.status.success(),
            "recipient read: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(&path).unwrap(), png);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn image_custody_many_completed_sends_bound_active_ownership_and_keep_prior_pngs_readable() {
        use crate::{
            cockpit::UiMsg,
            messaging::{Client, Reply, Results},
        };
        use bbt_proto::{SwarmId, messaging::Response};
        use bbt_ui::modules::composer::{Address, Submission};
        let mut png = Vec::new();
        {
            let mut writer = png::Encoder::new(&mut png, 1, 1).write_header().unwrap();
            writer.write_image_data(&[123]).unwrap();
        }
        let mut images = Images::default();
        let mut client = Client::default();
        let mut paths = vec![];
        let mut results = Results::default();
        let draft = |attach: Vec<Attachment>| Submission {
            delegate: false,
            to: vec![Address {
                destination: "seats/fixture/inbox".into(),
                ..Default::default()
            }],
            attach,
            text: "fixture message".into(),
            kind: "text".into(),
            priority: 0,
        };
        for index in 0..128 {
            let image = OwnedImage::save_png(&png).unwrap();
            let path = image.path.clone();
            let attachment = attachment(&image, &format!("fixture-{index}"));
            images
                .ready(&attachment.id, image, std::slice::from_ref(&attachment))
                .unwrap();
            let commands = client.admitted(
                client
                    .prepare(Some(SwarmId::from_u128(1)), draft(vec![attachment.clone()]))
                    .unwrap(),
            );
            let UiMsg::Messaging { id, .. } = commands[0] else {
                panic!("send")
            };
            images.reconcile(
                std::slice::from_ref(&attachment),
                &client.protected_image_paths(),
            );
            assert_eq!(images.owned.len(), 1);
            assert_eq!(client.protected_image_paths().len(), 1);
            results.push(
                id,
                Reply::Messaging(Response::Held {
                    id,
                    reason: "durably accepted fixture".into(),
                }),
            );
            let (completed, _, committed) = client.replies_with_images(&results);
            assert!(completed.is_some());
            assert_eq!(committed, vec![attachment.path]);
            images.committed(&committed);
            images.reconcile(&[], &client.protected_image_paths());
            assert!(
                images.owned.is_empty(),
                "completed sends must relinquish all active tokens"
            );
            assert!(client.protected_image_paths().is_empty());
            assert!(
                client.replies_with_images(&results).2.is_empty(),
                "replayed completion is idempotent"
            );
            paths.push(path);
        }
        let commands = client.admitted(
            client
                .prepare(Some(SwarmId::from_u128(1)), draft(vec![]))
                .unwrap(),
        );
        let UiMsg::Messaging { id, .. } = commands[0] else {
            panic!("text send")
        };
        results.push(
            id,
            Reply::Messaging(Response::Held {
                id,
                reason: "durably accepted text fixture".into(),
            }),
        );
        let (_, _, committed) = client.replies_with_images(&results);
        assert!(committed.is_empty());
        images.committed(&committed);
        images.reconcile(&[], &client.protected_image_paths());
        assert!(images.owned.is_empty());
        drop(images);
        drop(client);
        for path in paths {
            assert_eq!(std::fs::read(&path).unwrap(), png);
            std::fs::remove_file(path).unwrap();
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn image_file_source_copies_only_bounded_image_bytes_and_keeps_source_custody_external() {
        let source = tempfile::NamedTempFile::new().unwrap();
        let mut png = Vec::new();
        {
            let mut writer = png::Encoder::new(&mut png, 1, 1).write_header().unwrap();
            writer.write_image_data(&[123]).unwrap();
        }
        std::fs::write(source.path(), &png).unwrap();
        let image = OwnedImage::from_source(Source::File(source.path().into())).unwrap();
        let path = image.path.clone();
        assert_ne!(path, source.path());
        assert_eq!(std::fs::read(&path).unwrap(), png);
        drop(image);
        removed(&path);
        assert_eq!(std::fs::read(source.path()).unwrap(), png);
        source
            .as_file()
            .set_len(MAX_CLIPBOARD_IMAGE_BYTES as u64 + 1)
            .unwrap();
        assert!(OwnedImage::from_source(Source::File(source.path().into())).is_err());
        source.as_file().set_len(0).unwrap();
        std::fs::write(source.path(), "not an image").unwrap();
        assert!(OwnedImage::from_source(Source::File(source.path().into())).is_err());
    }
}
