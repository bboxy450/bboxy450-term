// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! stdin -> d4947 `Sanitizer::text` -> stdout. Secret values come one per line from the file named by argv[1], if any.
use std::io::Read;
fn main() {
    let secrets = std::env::args()
        .nth(1)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).expect("stdin");
    print!(
        "{}",
        bboxy450_log_converter::sanitize::Sanitizer::new(secrets).text(&text)
    );
}
