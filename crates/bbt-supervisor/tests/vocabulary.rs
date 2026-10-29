// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Source-file audit, never the UI thread.
use std::path::Path;

fn scan(directory: &Path, policy: &Path, forbidden: &[String], hits: &mut Vec<String>) -> usize {
    let mut texts = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != ".git" {
                texts += scan(&path, policy, forbidden, hits);
            }
        } else if kind.is_file() && path != policy {
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // The canonical, suite-wide legal inventory credits other projects; it is not
    // host implementation vocabulary. `make test-licenses` verifies this mirror.
    // Only this root policy is exempt: sources and nested names are still scanned.
    let texts = scan(root, &root.join("ATTRIBUTION.md"), &forbidden, &mut hits);
    assert!(texts > 0, "no host text files were scanned");
    assert!(hits.is_empty(), "program-specific vocabulary: {hits:#?}");
}

#[test]
fn legal_inventory_exception_does_not_hide_implementation_or_nested_notices() {
    let root = tempfile::tempdir().unwrap();
    let policy = root.path().join("ATTRIBUTION.md");
    let source = root.path().join("src");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(&policy, "fixture-specific-program").unwrap();
    std::fs::write(source.join("lib.rs"), "fixture-specific-program").unwrap();
    std::fs::write(source.join("ATTRIBUTION.md"), "fixture-specific-program").unwrap();
    let mut hits = Vec::new();
    assert_eq!(
        scan(
            root.path(),
            &policy,
            &["fixture-specific-program".into()],
            &mut hits
        ),
        2
    );
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|hit| hit.contains("lib.rs")));
    assert!(hits.iter().any(|hit| hit.contains("src/ATTRIBUTION.md")));
}
