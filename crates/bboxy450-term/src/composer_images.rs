// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App process boundaries, IO-owned terminal models and worker-prepared frames.
//! Private local image custody. Clipboard bytes are copied; arbitrary source paths are never opened.
use bboxy450_term_desktop::{ClipboardImageFormat, MAX_CLIPBOARD_IMAGE_BYTES};
use bbt_ui::modules::composer::Attachment;
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

/// Only this ownership token can unlink a draft image. Drop queues IO off the window thread.
#[derive(Debug)]
pub struct OwnedImage {
    path: PathBuf,
    preserve: bool,
}
impl OwnedImage {
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Runs on a blocking worker. tempfile creates a new owner-only file without following links.
    pub fn save(bytes: &[u8], format: ClipboardImageFormat) -> Result<Self, String> {
        let png =
            bboxy450_term_desktop::clipboard_image_png(bytes, format).map_err(|e| e.to_string())?;
        Self::save_png(&png)
    }
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
        })
    }
}
impl Drop for OwnedImage {
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
    /// Pending/ambiguous and durable sends preserve their files even after GUI detach.
    /// A definitive refusal releases only files whose marker has also been removed.
    pub fn reconcile(&mut self, draft: &[Attachment], protected: &[String]) {
        self.owned.retain(|id, image| {
            image.preserve = protected.iter().any(|p| image.path == Path::new(p));
            image.preserve || draft.iter().any(|a| a.kind == "image" && a.id == *id)
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
        let submitted = client.replies(&results).0.unwrap();
        editor.stored(&submitted);
        assert!(editor.view.attach.is_empty());
        images.reconcile(&editor.view.attach, &client.protected_image_paths());
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
}
