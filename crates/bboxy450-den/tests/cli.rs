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
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "bboxy450-den-cli-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(path)
}
fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bboxy450-den"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn checks_directory_recursively_and_ignores_other_extensions() {
    let temp = Temp::new();
    let nested = temp.0.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::copy(fixture("sound/outbox.den"), nested.join("outbox.den")).unwrap();
    fs::write(temp.0.join("not-den.txt"), "this is not Den").unwrap();
    let result = cli(&["check".as_ref(), temp.0.as_os_str()]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}

#[test]
fn unsound_file_reports_path_line_column_and_exit_one() {
    let path = fixture("unsound/X101-missing-brace.den");
    let result = cli(&["check".as_ref(), path.as_os_str()]);
    assert_eq!(result.status.code(), Some(1));
    let output = String::from_utf8(result.stdout).unwrap();
    let diagnostic = output
        .strip_prefix(&format!("{}:", path.display()))
        .unwrap();
    let fields: Vec<_> = diagnostic.splitn(3, ':').collect();
    assert!(fields[0].parse::<u32>().unwrap() > 0);
    assert!(fields[1].split(' ').next().unwrap().parse::<u32>().unwrap() > 0);
    assert!(diagnostic.contains("X101"));
    assert!(diagnostic.contains("Repair:"));
}

#[test]
fn usage_and_io_errors_exit_two() {
    for args in [vec![], vec!["check".as_ref()], vec!["other".as_ref()]] {
        let result = cli(&args);
        assert_eq!(result.status.code(), Some(2));
        assert!(!result.stderr.is_empty());
    }
    let temp = Temp::new();
    let missing = temp.0.join("missing.den");
    let result = cli(&["check".as_ref(), missing.as_os_str()]);
    assert_eq!(result.status.code(), Some(2));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("missing.den")
    );
}

#[test]
fn json_is_one_stable_object_per_file_and_escapes_paths() {
    let temp = Temp::new();
    let path = temp.0.join("escaped\"\\\n.den");
    fs::copy(fixture("sound/outbox.den"), &path).unwrap();
    let args = ["check".as_ref(), "--json".as_ref(), temp.0.as_os_str()];
    let first = cli(&args);
    let second = cli(&args);
    assert_eq!(first.status.code(), Some(0), "{:?}", first);
    assert_eq!(first.stdout, second.stdout);
    let output = String::from_utf8(first.stdout).unwrap();
    assert_eq!(output.lines().count(), 1);
    assert!(output.starts_with("{\"path\":"));
    assert!(output.contains("escaped\\\"\\\\\\n.den\""));
    assert!(output.contains("\"report\":{\"records\":"));
    assert!(output.contains("\"diagnostics\":[]"));
    assert!(output.ends_with("}}\n"));
}

#[cfg(unix)]
#[test]
fn refuses_directory_symlink_cycles_without_hanging() {
    let temp = Temp::new();
    std::os::unix::fs::symlink(&temp.0, temp.0.join("cycle")).unwrap();
    let result = cli(&["check".as_ref(), temp.0.as_os_str()]);
    assert_eq!(result.status.code(), Some(2));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("symbolic links")
    );
}
