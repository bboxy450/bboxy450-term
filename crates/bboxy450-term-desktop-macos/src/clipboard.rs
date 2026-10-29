// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
//! AppKit pasteboard snapshots; image decoding/encoding is a separate worker operation.
use bboxy450_term_desktop_core::{
    ClipboardContent, ClipboardImageFormat, Error, MAX_CLIPBOARD_IMAGE_BYTES,
    MAX_CLIPBOARD_IMAGE_PIXELS,
};
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSPasteboard};
use objc2_foundation::{MainThreadMarker, NSData, NSDictionary, ns_string};

pub fn read_clipboard() -> Result<Option<ClipboardContent>, Error> {
    MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
    snapshot(&NSPasteboard::generalPasteboard())
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
    const PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4, 0, 9,
        251, 3, 253, 251, 94, 107, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    #[test]
    fn clipboard_private_pasteboard_snapshots_png_tiff_text_without_general_clipboard() {
        let board = NSPasteboard::pasteboardWithUniqueName();
        board.clearContents();
        assert!(board.setData_forType(Some(&NSData::with_bytes(PNG)), ns_string!("public.png")));
        assert_eq!(
            snapshot(&board).unwrap(),
            Some(ClipboardContent::Image {
                bytes: PNG.into(),
                format: ClipboardImageFormat::Png
            })
        );
        let image =
            NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &NSData::with_bytes(PNG))
                .unwrap();
        let tiff = image.TIFFRepresentation().unwrap();
        board.clearContents();
        assert!(board.setData_forType(Some(&tiff), ns_string!("public.tiff")));
        let Some(ClipboardContent::Image { bytes, format }) = snapshot(&board).unwrap() else {
            panic!("missing TIFF snapshot")
        };
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
        assert_eq!(
            snapshot(&board).unwrap(),
            Some(ClipboardContent::Text("fixture text".into()))
        );
        board.clearContents();
        assert_eq!(snapshot(&board).unwrap(), None);
    }
}
