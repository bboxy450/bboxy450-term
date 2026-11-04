// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Materialize the SDK transport example as a private Make test input.
#![allow(clippy::disallowed_methods)]
fn main() {
    let destination = std::path::PathBuf::from(std::env::args_os().nth(1).expect("destination"));
    let source = bbt_plugin_fixture::sdk_example("bboxy450-term-transport-example");
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let staged = destination.with_extension(format!("copy-{}", std::process::id()));
    std::fs::copy(source, &staged).unwrap();
    std::fs::rename(staged, destination).unwrap();
}
