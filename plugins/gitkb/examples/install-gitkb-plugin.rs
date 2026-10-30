// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `install-gitkb-plugin <plugin> --workspace <project>`: install only the
//! built artifact and its notices into an explicit local GitKB project.
//! Translated from bboxy450-term `scripts/install-gitkb-plugin.py`.
//! Discovery only; host trust, check, enable and root selection stay explicit.

// A command-line tool with no UI thread.
#![allow(clippy::disallowed_methods)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const LICENSE: &[u8] = include_bytes!("../LICENSE");
const NOTICE: &[u8] = include_bytes!("../NOTICE");
const ATTRIBUTION: &[u8] = include_bytes!("../ATTRIBUTION.md");

fn refuse(message: &str) -> ! {
    eprintln!("install_scope_refused: {message}");
    std::process::exit(2);
}

fn symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
}

/// Write through a private temporary file in the same directory, then rename.
fn atomic_write(target: &Path, data: &[u8], mode: u32) -> std::io::Result<()> {
    if symlink(target) {
        refuse("symlinked destination");
    }
    let parent = target.parent().expect("destination has a parent");
    let mut attempt = 0u32;
    let (temporary, mut file) = loop {
        let candidate = parent.join(format!(".gitkb-install-{}-{attempt}", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => attempt += 1,
            Err(e) => return Err(e),
        }
    };
    let written = file
        .write_all(data)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::set_permissions(&temporary, fs::Permissions::from_mode(mode)))
        .and_then(|()| fs::rename(&temporary, target));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (source, workspace) = match args.as_slice() {
        [source, flag, workspace] if flag == "--workspace" => (source, workspace),
        [flag, workspace, source] if flag == "--workspace" => (source, workspace),
        _ => {
            eprintln!("usage: install-gitkb-plugin <plugin> --workspace <project>");
            std::process::exit(2);
        }
    };
    let source = PathBuf::from(source);
    let workspace = Path::new(workspace)
        .canonicalize()
        .unwrap_or_else(|_| refuse("supply an explicit local GitKB project"));
    let home = std::env::var_os("HOME").and_then(|h| Path::new(&h).canonicalize().ok());
    if !workspace.is_dir() || !workspace.join(".kb").is_dir() || Some(&workspace) == home.as_ref() {
        refuse("supply an explicit local GitKB project");
    }
    let directory = workspace.join(".bboxy/plugins");
    let notices = workspace.join(".bboxy/plugin-notices/gitkb");
    for path in [
        workspace.join(".bboxy"),
        directory.clone(),
        notices.clone(),
        workspace.join(".bboxy/plugin-notices"),
    ] {
        if symlink(&path) {
            refuse("symlinked installation path");
        }
    }
    let name = source
        .file_name()
        .unwrap_or_else(|| refuse("plugin path has no file name"))
        .to_owned();
    let plugin = fs::read(&source).unwrap_or_else(|e| refuse(&format!("read plugin: {e}")));
    fs::create_dir_all(&directory)
        .and_then(|()| fs::create_dir_all(&notices))
        .unwrap_or_else(|e| refuse(&format!("create directories: {e}")));
    for path in [&directory, &notices] {
        if symlink(path) {
            refuse("symlinked installation path");
        }
    }
    let installed = directory.join(&name);
    for (target, data, mode) in [
        (installed.clone(), plugin.as_slice(), 0o755),
        (notices.join("LICENSE"), LICENSE, 0o644),
        (notices.join("NOTICE"), NOTICE, 0o644),
        (notices.join("ATTRIBUTION.md"), ATTRIBUTION, 0o644),
    ] {
        atomic_write(&target, data, mode)
            .unwrap_or_else(|e| refuse(&format!("write {}: {e}", target.display())));
    }
    let sha256: String = Sha256::digest(&plugin)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    println!(
        "{}",
        serde_json::json!({"installed": installed.display().to_string(), "sha256": sha256,
            "activation": "Discovery only; host trust, check, enable and root selection remain explicit"})
    );
}
