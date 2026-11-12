// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export `protocol!` theorems to Lean 4 for `make test-lean`. Lean checks them; this only writes.
// Standalone development tool, outside the UI.
#![allow(clippy::disallowed_methods)]
use std::path::Path;

#[path = "../tests/protocol_runtime.rs"]
#[allow(dead_code)]
mod protocol_language;
#[path = "../../bboxy450-term-remote/src/delivery.rs"]
#[allow(dead_code)] // Only the declarations' Lean export is used here.
mod remote_delivery;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(proved), Some(controls)) = (args.next(), args.next()) else {
        return Err("usage: yarn_lean PROVED_DIRECTORY CONTROLS_DIRECTORY".into());
    };
    // Every type of the language: bool, domain, option and set.
    protocol_language::ProtocolLanguage::lean().export(
        Path::new(&proved),
        include_str!("../tests/lean/protocol_language.lean"),
        "pass",
    )?;
    // The proof travels with the declaration it is about.
    remote_delivery::RemoteDelivery::lean().export(
        Path::new(&proved),
        include_str!("../../bboxy450-term-remote/src/delivery.lean"),
        "pass",
    )?;
    // The volatile remote applies twice: no proof exists, and `sorry` is not one.
    remote_delivery::BrokenRemoteDelivery::lean().export(
        Path::new(&controls),
        include_str!("../../bboxy450-term-remote/src/delivery_broken.lean"),
        "fail",
    )?;
    Ok(())
}
