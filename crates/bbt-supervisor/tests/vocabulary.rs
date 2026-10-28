// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Source-file audit, never the UI thread.
use std::path::Path;

fn scan(directory: &Path, forbidden: &[String], hits: &mut Vec<String>) -> usize {
    let mut texts = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != ".git" {
                texts += scan(&path, forbidden, hits);
            }
        } else if kind.is_file() {
            let bytes = std::fs::read(&path).unwrap();
            if let Ok(text) = std::str::from_utf8(&bytes) {
                texts += 1;
                let text = text.to_ascii_lowercase();
                for word in forbidden {
                    if text.contains(word) {
                        hits.push(format!("{}: {word}", path.display()));
                    }
                }
            }
        }
    }
    texts
}

// keeps: decision #273
#[test]
fn host_has_no_program_specific_vocabulary() {
    // Split the needles so this audit does not match its own source.
    let forbidden =
        [["cl", "aude"], ["co", "dex"], ["75", "01"], ["75", "02"]].map(|parts| parts.concat());
    let mut hits = Vec::new();
    let texts = scan(Path::new(env!("CARGO_MANIFEST_DIR")), &forbidden, &mut hits);
    assert!(texts > 0, "no host text files were scanned");
    assert!(hits.is_empty(), "program-specific vocabulary: {hits:#?}");
}
