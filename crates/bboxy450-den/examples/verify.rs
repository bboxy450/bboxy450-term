// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export Den fixtures to the differential corpus. Java TLC remains the reference.
// Standalone development tool, outside the UI.
#![allow(clippy::disallowed_methods)]
use bboxy450_tlc::corpus::export;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: verify OUTPUT_DIRECTORY")?;
    let root = Path::new(&output);
    let mut manifest = std::fs::read_to_string(root.join("manifest.tsv"))?;
    // A broken control keeps its spec's name in Den; the corpus needs a distinct one.
    for (name, source, expected) in [
        ("DenOutbox", include_str!("../tests/sound/outbox.den"), "pass"),
        (
            "BrokenDenOutbox",
            include_str!("../tests/unsound/X651-unfair-progress.den"),
            "liveness",
        ),
        (
            "DenRemoteDelivery",
            include_str!("../tests/sound/remote_delivery.den"),
            "pass",
        ),
        (
            "BrokenDenToggle",
            include_str!("../tests/unsound/X601-reachable-violation.den"),
            "safety",
        ),
    ] {
        let mut model = bboxy450_den::model(source).map_err(|r| r.to_text())?;
        model.name = name.into();
        export(model, expected, root, &mut manifest)?;
    }
    std::fs::write(root.join("manifest.tsv"), manifest)?;
    Ok(())
}
