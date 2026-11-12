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

const USAGE: &str = "usage: bboxy450-den check [--json] PATH...\n       bboxy450-den render --markdown [--out DIR] [--overwrite] [--no-mermaid] PATH...\n       bboxy450-den lean --out DIR [--expect-fail] PATH...\n       bboxy450-den replay SPEC TRACE...";

/// `replay SPEC TRACE...`: each trace file is a recorded run in scenario-step
/// syntax; exit 1 when the spec does not allow one of them.
fn replay(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<bool, String> {
    let read = |path: &Path| {
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let spec = PathBuf::from(args.next().ok_or(USAGE)?);
    let source = read(&spec)?;
    let traces: Vec<_> = args.map(PathBuf::from).collect();
    if traces.is_empty() {
        return Err(USAGE.into());
    }
    let mut conforms = true;
    for trace in traces {
        let report = bboxy450_den::replay(&source, &read(&trace)?);
        conforms &= report.is_sound();
        for line in report.to_text().lines() {
            println!("{}:{line}", trace.display());
        }
    }
    Ok(conforms)
}

/// `lean --out DIR`: for each sound spec that declares a `theorem`, write
/// `MODULE.lean` and `MODULECheck.lean`, copy the proof beside the spec
/// (`outbox.den` -> `outbox.lean`) to `MODULEProof.lean`, and append the row
/// `scripts/lean.py --check DIR` reads. Nothing here runs Lean.
fn lean(args: impl Iterator<Item = std::ffi::OsString>) -> Result<bool, String> {
    let mut expected = "pass";
    let mut out: Option<PathBuf> = None;
    let mut paths = Vec::new();
    let mut options = true;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if options && arg == OsStr::new("--") {
            options = false;
        } else if options && arg == OsStr::new("--expect-fail") {
            expected = "fail";
        } else if options && arg == OsStr::new("--out") {
            out = Some(PathBuf::from(args.next().ok_or("--out needs a directory")?));
        } else if options && arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", arg.to_string_lossy()));
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    let (Some(out), false) = (out, paths.is_empty()) else {
        return Err(USAGE.into());
    };
    let mut files = Vec::new();
    for path in &paths {
        collect(path, &mut files).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    files.sort();
    files.dedup();
    let mut sound = true;
    for path in files {
        if path.extension() != Some(OsStr::new("den")) {
            continue;
        }
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let lean = match bboxy450_den::lean(&source) {
            Ok(Some(lean)) => lean,
            Ok(None) => continue,
            Err(report) => {
                sound = false;
                for line in report.to_text().lines() {
                    println!("{}:{line}", path.display());
                }
                continue;
            }
        };
        let proof = path.with_extension("lean");
        let Ok(proof_text) = fs::read_to_string(&proof) else {
            sound = false;
            println!(
                "{}:1:1 X905 theorems {} have no proof. Repair: write {} proving each ID_proved : ID_statement in namespace {}",
                path.display(),
                lean.theorems().join(", "),
                proof.display(),
                lean.module
            );
            continue;
        };
        lean.export(&out, &proof_text, expected)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        println!("{} -> {}", path.display(), out.join(&lean.module).display());
    }
    Ok(sound)
}

/// `render --markdown`: one `.md` beside each sound `.den` (or under `--out DIR`,
/// mirroring the input path). A file that is not a previous lowering is kept
/// unless `--overwrite`. Unsound input writes nothing and prints diagnostics.
fn render(args: impl Iterator<Item = std::ffi::OsString>) -> Result<bool, String> {
    let mut markdown = false;
    let mut overwrite = false;
    let mut mermaid = true;
    let mut out: Option<PathBuf> = None;
    let mut paths = Vec::new();
    let mut options = true;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if options && arg == OsStr::new("--") {
            options = false;
        } else if options && arg == OsStr::new("--markdown") {
            markdown = true;
        } else if options && arg == OsStr::new("--overwrite") {
            overwrite = true;
        } else if options && arg == OsStr::new("--no-mermaid") {
            mermaid = false;
        } else if options && arg == OsStr::new("--out") {
            out = Some(PathBuf::from(args.next().ok_or("--out needs a directory")?));
        } else if options && arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", arg.to_string_lossy()));
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    if !markdown || paths.is_empty() {
        return Err(USAGE.into());
    }
    let mut files = Vec::new();
    for path in &paths {
        collect(path, &mut files).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    files.sort();
    files.dedup();
    let mut sound = true;
    for path in files {
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let target = match &out {
            Some(dir) => {
                // Mirror the path under DIR: relative to the current directory
                // when inside it, else from the filesystem root.
                let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
                let relative = path
                    .strip_prefix(&cwd)
                    .or_else(|_| path.strip_prefix("/"))
                    .or_else(|_| path.strip_prefix("./"))
                    .unwrap_or(&path);
                dir.join(relative)
            }
            None => path.clone(),
        }
        .with_extension("md");
        match bboxy450_den::render::render_with(&source, mermaid) {
            Err(report) => {
                sound = false;
                for line in report.to_text().lines() {
                    println!("{}:{line}", path.display());
                }
            }
            Ok(rendered) => {
                if let Ok(existing) = fs::read_to_string(&target) {
                    let lowering = bboxy450_den::render::embedded(&existing).is_some();
                    if !lowering && !overwrite {
                        return Err(format!(
                            "{}: exists and is not a lowering; pass --overwrite to replace it",
                            target.display()
                        ));
                    }
                    if existing == rendered.markdown {
                        println!("{} unchanged", target.display());
                        continue;
                    }
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| format!("{}: {error}", parent.display()))?;
                }
                fs::write(&target, &rendered.markdown)
                    .map_err(|error| format!("{}: {error}", target.display()))?;
                println!(
                    "{} <- {} {}",
                    target.display(),
                    rendered.module,
                    &rendered.source_sha256[..12]
                );
            }
        }
    }
    Ok(sound)
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        Some(verb) if verb == OsStr::new("check") => {}
        Some(verb) if verb == OsStr::new("render") => return render(args),
        Some(verb) if verb == OsStr::new("lean") => return lean(args),
        Some(verb) if verb == OsStr::new("replay") => return replay(args),
        _ => return Err(USAGE.into()),
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
        return Err(USAGE.into());
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
