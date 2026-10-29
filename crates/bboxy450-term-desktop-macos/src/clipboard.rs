// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! AppKit pasteboard snapshots; image decoding/encoding is a separate worker operation.
use bboxy450_term_desktop_core::{
    ClipboardContent, ClipboardImageFormat, Error, MAX_CLIPBOARD_IMAGE_BYTES,
    MAX_CLIPBOARD_IMAGE_PIXELS,
};
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSPasteboard};
use objc2_foundation::{MainThreadMarker, NSData, NSDictionary, NSString, ns_string};

pub fn read_clipboard() -> Result<Option<ClipboardContent>, Error> {
    MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
    snapshot(&NSPasteboard::generalPasteboard())
}
/// Write through the platform clipboard port. Tests use the same writer with a unique board.
pub fn write_clipboard_text(text: &str) -> Result<(), Error> {
    MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
    write_text(&NSPasteboard::generalPasteboard(), text)
}
fn write_text(board: &NSPasteboard, text: &str) -> Result<(), Error> {
    if text.len() > 1024 * 1024 {
        return Err(Error::Native("clipboard_text_too_large".into()));
    }
    let text = NSString::from_str(text);
    board.clearContents();
    if board.setString_forType(&text, ns_string!("public.utf8-plain-text")) {
        Ok(())
    } else {
        Err(Error::Native("clipboard_write_failed".into()))
    }
}
fn snapshot(board: &NSPasteboard) -> Result<Option<ClipboardContent>, Error> {
    for (kind, format) in [
        (ns_string!("public.png"), ClipboardImageFormat::Png),
        (ns_string!("public.tiff"), ClipboardImageFormat::Tiff),
    ] {
        if let Some(data) = board.dataForType(kind) {
            if data.length() > MAX_CLIPBOARD_IMAGE_BYTES {
                return Err(Error::Native("clipboard_image_too_large".into()));
            }
            return Ok(Some(ClipboardContent::Image {
                bytes: data.to_vec(),
                format,
            }));
        }
    }
    if let Some(text) = board.stringForType(ns_string!("public.utf8-plain-text")) {
        if text.len() > 64 * 1024 {
            return Err(Error::Native("clipboard_text_too_large".into()));
        }
        return Ok(Some(ClipboardContent::Text(text.to_string())));
    }
    Ok(None)
}
/// Call on a blocking worker, never in the window callback. Only byte snapshots cross threads.
pub fn clipboard_image_png(bytes: &[u8], format: ClipboardImageFormat) -> Result<Vec<u8>, Error> {
    objc2::rc::autoreleasepool(|_| image_png(bytes, format))
}
fn image_png(bytes: &[u8], format: ClipboardImageFormat) -> Result<Vec<u8>, Error> {
    if bytes.is_empty() || bytes.len() > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(Error::Native("clipboard_image_too_large".into()));
    }
    let image =
        NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &NSData::with_bytes(bytes))
            .ok_or_else(|| Error::Native("clipboard_image_invalid".into()))?;
    let pixels = usize::try_from(image.pixelsWide())
        .ok()
        .zip(usize::try_from(image.pixelsHigh()).ok())
        .and_then(|(w, h)| w.checked_mul(h));
    if !pixels.is_some_and(|n| n > 0 && n <= MAX_CLIPBOARD_IMAGE_PIXELS) {
        return Err(Error::Native("clipboard_image_dimensions".into()));
    }
    if format == ClipboardImageFormat::Png && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(bytes.to_vec());
    }
    // SAFETY: the empty properties dictionary has no incorrectly typed values.
    let data = unsafe {
        image.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or_else(|| Error::Native("clipboard_image_conversion_failed".into()))?;
    if data.length() > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(Error::Native("clipboard_image_too_large".into()));
    }
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::rc::Retained;
    use std::sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
        mpsc,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static RUN: LazyLock<String> = LazyLock::new(|| {
        format!(
            "{}.{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    });
    static NEXT_BOARD: AtomicU64 = AtomicU64::new(0);
    const WAIT: Duration = Duration::from_secs(5);
    const TEXT: &str = "界e\u{301}  \nsecond";

    // Adapted from this module's private pasteboard fixtures. Explicit fixture
    // ownership replaces implicit allocation; it does not diagnose the old failure.
    // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    struct OwnedBoard {
        name: String,
        board: Retained<NSPasteboard>,
        cleanup: bool,
    }
    impl OwnedBoard {
        fn new(label: &str) -> Self {
            let instance = NEXT_BOARD.fetch_add(1, Ordering::Relaxed);
            let name = format!("bboxy450.fixture.{}.{}.{}", *RUN, instance, label);
            let board = NSPasteboard::pasteboardWithName(&NSString::from_str(&name));
            let owned = Self {
                name,
                board,
                cleanup: true,
            };
            owned.trace("allocated");
            owned
        }
        fn identity(&self) -> (String, usize) {
            (
                self.board.name().to_string(),
                std::ptr::from_ref(&*self.board) as usize,
            )
        }
        fn trace(&self, phase: &str) {
            let (actual, object) = self.identity();
            eprintln!(
                "clipboard fixture: phase={phase}, requested={:?}, actual={actual:?}, object={object:#x}, change_count={}",
                self.name,
                self.board.changeCount()
            );
            assert!(
                self.name
                    .starts_with(&format!("bboxy450.fixture.{}.", *RUN))
            );
            assert_eq!(
                actual, self.name,
                "private board returned a different identity"
            );
        }
    }
    impl Drop for OwnedBoard {
        fn drop(&mut self) {
            let actual = self.board.name().to_string();
            if self.cleanup && actual == self.name {
                eprintln!("clipboard fixture: phase=cleanup, name={actual:?}");
                self.board.clearContents();
            } else {
                eprintln!("clipboard fixture: cleanup skipped for unconfirmed or changed identity");
            }
        }
    }
    const PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4, 0, 9,
        251, 3, 253, 251, 94, 107, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    #[test]
    fn clipboard_private_text_writer_preserves_unicode_and_lines() {
        let owned = OwnedBoard::new("unicode");
        let board = &*owned.board;
        write_text(board, "界e\u{301}  \nsecond").unwrap();
        owned.trace("text_written_before_read");
        assert_eq!(
            snapshot(board).unwrap(),
            Some(ClipboardContent::Text("界e\u{301}  \nsecond".into()))
        );
        owned.trace("text_read");
        board.clearContents();
    }
    #[test]
    fn clipboard_private_pasteboard_snapshots_png_tiff_text_without_general_clipboard() {
        let owned = OwnedBoard::new("formats");
        let board = &*owned.board;
        board.clearContents();
        assert!(board.setData_forType(Some(&NSData::with_bytes(PNG)), ns_string!("public.png")));
        owned.trace("png_written_before_read");
        assert_eq!(
            snapshot(board).unwrap(),
            Some(ClipboardContent::Image {
                bytes: PNG.into(),
                format: ClipboardImageFormat::Png
            })
        );
        owned.trace("png_read");
        let image =
            NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &NSData::with_bytes(PNG))
                .unwrap();
        let tiff = image.TIFFRepresentation().unwrap();
        board.clearContents();
        assert!(board.setData_forType(Some(&tiff), ns_string!("public.tiff")));
        owned.trace("tiff_written_before_read");
        let Some(ClipboardContent::Image { bytes, format }) = snapshot(board).unwrap() else {
            panic!("missing TIFF snapshot")
        };
        owned.trace("tiff_read");
        let png = std::thread::spawn(move || clipboard_image_png(&bytes, format))
            .join()
            .unwrap()
            .unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        board.clearContents();
        assert!(board.setString_forType(
            ns_string!("fixture text"),
            ns_string!("public.utf8-plain-text")
        ));
        owned.trace("plain_text_written_before_read");
        assert_eq!(
            snapshot(board).unwrap(),
            Some(ClipboardContent::Text("fixture text".into()))
        );
        owned.trace("plain_text_read");
        board.clearContents();
        assert_eq!(snapshot(board).unwrap(), None);
        owned.trace("empty_read");
    }

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "bounded recv_timeout on joined fixture threads; never a UI callback"
    )]
    fn clipboard_private_owned_boards_keep_overlapping_text_and_image_writes_isolated() {
        #[derive(Debug)]
        enum Reply {
            Ready(String, usize),
            Written,
            Snapshot(Option<ClipboardContent>),
        }
        #[derive(Clone, Copy)]
        enum Command {
            Write,
            Read,
        }
        fn actor(
            image: bool,
        ) -> (
            mpsc::SyncSender<Command>,
            mpsc::Receiver<Reply>,
            std::thread::JoinHandle<()>,
        ) {
            let (command, commands) = mpsc::sync_channel(1);
            let (replies, reply) = mpsc::sync_channel(1);
            let handle =
                std::thread::spawn(move || {
                    let mut owned = OwnedBoard::new(if image {
                        "overlap-image"
                    } else {
                        "overlap-text"
                    });
                    // No mutation/cleanup until the parent confirms both actual
                    // names and handles differ; shared-name controls stop here.
                    owned.cleanup = false;
                    let (name, object) = owned.identity();
                    if replies.try_send(Reply::Ready(name, object)).is_err() {
                        return;
                    }
                    let Ok(Command::Write) = commands.recv_timeout(WAIT) else {
                        return;
                    };
                    owned.cleanup = true;
                    if image {
                        owned.board.clearContents();
                        assert!(owned.board.setData_forType(
                            Some(&NSData::with_bytes(PNG)),
                            ns_string!("public.png")
                        ));
                    } else {
                        write_text(&owned.board, TEXT).unwrap();
                    }
                    owned.trace("overlap_write_complete");
                    if replies.try_send(Reply::Written).is_err() {
                        return;
                    }
                    let Ok(Command::Read) = commands.recv_timeout(WAIT) else {
                        return;
                    };
                    owned.trace("overlap_before_read");
                    let actual = snapshot(&owned.board).unwrap();
                    owned.trace("overlap_read_complete");
                    let _ = replies.try_send(Reply::Snapshot(actual));
                });
            (command, reply, handle)
        }
        let (text, text_reply, text_thread) = actor(false);
        let (image, image_reply, image_thread) = actor(true);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let Reply::Ready(text_name, text_object) = text_reply.recv_timeout(WAIT).unwrap()
            else {
                panic!("text actor identity absent")
            };
            let Reply::Ready(image_name, image_object) = image_reply.recv_timeout(WAIT).unwrap()
            else {
                panic!("image actor identity absent")
            };
            assert_ne!(
                text_name, image_name,
                "owned private board names must be distinct"
            );
            assert_ne!(
                text_object, image_object,
                "live private handles must be distinct"
            );
            // Both actors are alive; dispatch both writes before waiting. Read
            // only after both real values coexist, so aliasing cannot pass by order.
            text.try_send(Command::Write).unwrap();
            image.try_send(Command::Write).unwrap();
            assert!(matches!(
                text_reply.recv_timeout(WAIT).unwrap(),
                Reply::Written
            ));
            assert!(matches!(
                image_reply.recv_timeout(WAIT).unwrap(),
                Reply::Written
            ));
            text.try_send(Command::Read).unwrap();
            image.try_send(Command::Read).unwrap();
            let Reply::Snapshot(text_actual) = text_reply.recv_timeout(WAIT).unwrap() else {
                panic!("text snapshot absent")
            };
            let Reply::Snapshot(image_actual) = image_reply.recv_timeout(WAIT).unwrap() else {
                panic!("image snapshot absent")
            };
            assert_eq!(text_actual, Some(ClipboardContent::Text(TEXT.into())));
            assert_eq!(
                image_actual,
                Some(ClipboardContent::Image {
                    bytes: PNG.into(),
                    format: ClipboardImageFormat::Png
                })
            );
        }));
        // Closing channels also releases actors after a parent assertion fails;
        // join both owned threads before restoring the original panic.
        drop((text, image, text_reply, image_reply));
        let joined = [text_thread.join(), image_thread.join()];
        let joined_cleanly = joined.iter().all(|result| result.is_ok());
        eprintln!("clipboard fixture: both actors joined, clean={joined_cleanly}");
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
        assert!(joined_cleanly, "private actor panicked");
    }
}
