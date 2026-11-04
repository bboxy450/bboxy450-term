// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "build-time asset generation runs outside the app UI thread"
)]
use std::{env, fs, path::PathBuf};
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../..");
    // Development consumes the canonical workspace inputs. Published crates
    // carry the same bytes locally and do not read outside their package.
    let root = if workspace
        .join("crates/bboxy450-term/Cargo.toml")
        .canonicalize()
        .ok()
        == manifest.join("Cargo.toml").canonicalize().ok()
    {
        workspace
    } else {
        manifest.join("release-inputs")
    };
    println!("cargo:rustc-env=BBOXY_INPUT_ROOT={}", root.display());
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    let mut revision = "unavailable".to_owned();
    if git(&["rev-parse", "--show-toplevel"])
        .as_deref()
        .map(PathBuf::from)
        .and_then(|p| p.canonicalize().ok())
        == root.canonicalize().ok()
    {
        if let Some(head) = git(&["rev-parse", "HEAD"]) {
            revision = head;
            if git(&["status", "--porcelain"]).is_none_or(|s| !s.is_empty()) {
                revision.push_str("-dirty");
            }
        }
        // Git refs AND source inputs invalidate the stamp. Ignored build output
        // is excluded, including each submodule's target directory.
        for args in [
            ["rev-parse", "--git-path", "HEAD"],
            ["rev-parse", "--git-path", "index"],
        ] {
            if let Some(path) = git(&args) {
                println!("cargo:rerun-if-changed={}", root.join(path).display());
            }
        }
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
            && let Some(path) = git(&["rev-parse", "--git-path", &reference])
        {
            println!("cargo:rerun-if-changed={}", root.join(path).display());
        }
        if let Some(files) = git(&["ls-files", "--recurse-submodules"]) {
            for path in files.lines() {
                println!("cargo:rerun-if-changed={}", root.join(path).display());
            }
        }
    }
    println!("cargo:rustc-env=BBOXY_GUI_SOURCE={revision}");

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
