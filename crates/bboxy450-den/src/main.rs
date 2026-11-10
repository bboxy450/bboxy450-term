// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![expect(
    clippy::disallowed_methods,
    reason = "Standalone checker and private CLI fixtures perform synchronous filesystem and process I/O; no UI thread exists here."
)]
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn collect(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: symbolic links are not supported", path.display()),
        ));
    }
    if metadata.is_dir() {
        let mut children = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<io::Result<Vec<_>>>()?;
        children.sort();
        for child in children {
            if child.is_dir() || child.extension() == Some(OsStr::new("den")) {
                collect(&child, files)?;
            }
        }
    } else if metadata.is_file() {
        files.push(path.to_owned());
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: expected a file or directory", path.display()),
        ));
    }
    Ok(())
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c < ' ' => write!(result, "\\u{:04x}", c as u32).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new("check")) {
        return Err("usage: bboxy450-den check [--json] PATH...".into());
    }
    let mut json = false;
    let mut paths = Vec::new();
    let mut options = true;
    for arg in args {
        if options && arg == OsStr::new("--") {
            options = false;
        } else if options && arg == OsStr::new("--json") {
            json = true;
        } else if options && arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", arg.to_string_lossy()));
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    if paths.is_empty() {
        return Err("usage: bboxy450-den check [--json] PATH...".into());
    }
    let mut files = Vec::new();
    for path in paths {
        collect(&path, &mut files).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    files.sort();
    files.dedup();
    let mut sound = true;
    for path in files {
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let report = bboxy450_den::check(&source);
        sound &= report.is_sound();
        if json {
            println!(
                "{{\"path\":{},\"report\":{}}}",
                json_string(&path.to_string_lossy()),
                report.to_json()
            );
        } else {
            for line in report.to_text().lines() {
                println!("{}:{line}", path.display());
            }
        }
    }
    Ok(sound)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
