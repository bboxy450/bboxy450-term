// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pin the GitKB backend at compile time: canonical path, SHA-256 and
//! `--version`. `BBOXY_GITKB_BIN` selects it (a path or a name on PATH);
//! unset, `git-kb` on PATH is used. An explicit selection must resolve. With
//! none available the pin is empty and the plugin refuses `initialize`, so
//! workspace builds on machines without GitKB still compile.

// A build script has no UI thread; blocking file and process IO is its job.
#![allow(clippy::disallowed_methods)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

fn main() {
    println!("cargo:rerun-if-env-changed=BBOXY_GITKB_BIN");
    println!("cargo:rerun-if-changed=build.rs");
    let explicit = std::env::var("BBOXY_GITKB_BIN")
        .ok()
        .filter(|s| !s.is_empty());
    if explicit.is_none() {
        println!("cargo:rerun-if-env-changed=PATH");
    }
    let selected = explicit.clone().unwrap_or_else(|| "git-kb".into());
    let (path, sha256, version) = match locate(&selected).and_then(|p| p.canonicalize().ok()) {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            let sha256 = hash(&path);
            let output = Command::new(&path)
                .arg("--version")
                .output()
                .unwrap_or_else(|e| panic!("{} --version: {e}", path.display()));
            assert!(
                output.status.success(),
                "{} --version exited {}",
                path.display(),
                output.status
            );
            let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            (path.display().to_string(), sha256, version)
        }
        None if explicit.is_some() => panic!("BBOXY_GITKB_BIN={selected} does not resolve"),
        None => {
            println!("cargo:warning=git-kb not found; the GitKB plugin pin is empty");
            (String::new(), String::new(), String::new())
        }
    };
    println!("cargo:rustc-env=BBOXY_GITKB_PIN_PATH={path}");
    println!("cargo:rustc-env=BBOXY_GITKB_PIN_SHA256={sha256}");
    println!(
        "cargo:rustc-env=BBOXY_GITKB_PIN_VERSION={}",
        one_line(&version)
    );
}

/// `shutil.which` semantics: a name with a slash is a path; otherwise search PATH.
fn locate(selected: &str) -> Option<PathBuf> {
    if selected.contains('/') {
        let path = PathBuf::from(selected);
        return path.is_file().then_some(path);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(selected))
            .find(|candidate| executable(candidate))
    })
}

fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn hash(path: &Path) -> String {
    let mut file = std::fs::File::open(path).expect("open backend");
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buffer).expect("read backend");
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// rustc-env values are single lines: newlines travel as `\n` and the plugin
/// restores them.
fn one_line(version: &str) -> String {
    version.lines().collect::<Vec<_>>().join("\\n")
}
