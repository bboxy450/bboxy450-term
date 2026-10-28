// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "build script: compile-time IO, not the UI thread"
)]
use std::{env, fs, path::PathBuf};
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../..");
    // Development consumes the canonical workspace inputs. Published crates
    // carry the same bytes locally and do not read outside their package.
    let root = if workspace.join("crates/bboxybook/Cargo.toml").canonicalize().ok()
        == manifest.join("Cargo.toml").canonicalize().ok()
    {
        workspace
    } else {
        manifest.join("release-inputs")
    };
    println!("cargo:rustc-env=BBOXY_INPUT_ROOT={}", root.display());
    let source = root.join("lab/web/components.js");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("lab/web/bboxy.png").display()
    );
    let text = fs::read_to_string(source).unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let pattern = regex::Regex::new(r"(?m)^\s*(\w+):\s*'(<svg[^\n]+)</svg>',?\s*$").unwrap();
    let mut code = String::from("pub const BAKED_ICONS: &[(&str,&str)] = &[\n");
    let mut count = 0;
    for c in pattern.captures_iter(&text) {
        let svg = format!("{}</svg>", &c[2])
            .replace("<svg ", "<svg xmlns=\"http://www.w3.org/2000/svg\" ")
            .replace("currentColor", "#ffffff");
        code.push_str(&format!("({:?},{:?}),\n", &c[1], svg));
        count += 1;
    }
    assert!(count >= 15, "lab SVG dictionary disappeared");
    code.push_str("];\n");
    fs::write(out.join("icons.rs"), code).unwrap();
}
