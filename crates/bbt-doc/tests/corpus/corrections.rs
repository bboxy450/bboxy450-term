// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Git-only legacy migration gate. Native report writers remain immutable.
use super::{git, tree_sources};
use bbt_doc::links::{Record, resolve};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const LEDGER: &str = ".bboxy/reports/CORRECTIONS.toml";

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Correction {
    path: String,
    from_blob: String,
    to_blob: String,
    reason: String,
    decision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    correction: Vec<Correction>,
}

// Python's stdlib TOML parser is already required by Make and the landing gate.
// Parse the committed bytes, never a path or Python supplied by the ledger.
fn parse(bytes: &[u8]) -> Result<Vec<Correction>, String> {
    if bytes.is_empty() {
        return Ok(vec![]);
    }
    let mut child = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import sys,json,tomllib; json.dump(tomllib.loads(sys.stdin.read()),sys.stdout)",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ledger parser: {e}"))?;
    let mut input = child.stdin.take().unwrap();
    let owned = bytes.to_vec();
    let writer = std::thread::spawn(move || input.write_all(&owned));
    let output = child
        .wait_with_output()
        .map_err(|e| format!("ledger parser: {e}"))?;
    let written = writer.join().unwrap();
    if !output.status.success() {
        return Err("malformed CORRECTIONS.toml".into());
    }
    written.map_err(|e| format!("ledger parser input: {e}"))?;
    let ledger: Ledger = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("malformed CORRECTIONS.toml: {e}"))?;
    let mut seen = BTreeSet::new();
    for entry in &ledger.correction {
        let valid_hash = |s: &str| {
            matches!(s.len(), 40 | 64)
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && s.bytes().any(|b| b != b'0')
        };
        let valid_path = entry.path.strip_prefix(".bboxy/reports/").is_some_and(|s| {
            s.ends_with(".md") && !s.contains('/') && !s.contains('\\') && !s.contains("..")
        });
        if !valid_path
            || !valid_hash(&entry.from_blob)
            || !valid_hash(&entry.to_blob)
            || entry.from_blob == entry.to_blob
            || entry.reason.trim().is_empty()
            || bbt_doc::reports::locator(&serde_json::Value::String(entry.decision.clone()))
                .is_err()
        {
            return Err(format!("malformed correction entry for {}", entry.path));
        }
        if !seen.insert((&entry.path, &entry.from_blob, &entry.to_blob)) {
            return Err(format!("correction entry reuse for {}", entry.path));
        }
    }
    Ok(ledger.correction)
}

fn ledger_bytes(root: &Path, tree: &str) -> Result<Vec<u8>, String> {
    let listing = git(root, &["ls-tree", tree, "--", LEDGER]);
    if listing.is_empty() {
        return Ok(vec![]);
    }
    if !listing.starts_with(b"100644 blob ") {
        return Err("CORRECTIONS.toml must be a regular non-executable file".into());
    }
    Ok(git(root, &["show", &format!("{tree}:{LEDGER}")]))
}

pub(super) fn commit_errors(root: &Path, sha: &str) -> (bool, Vec<String>) {
    let parents = String::from_utf8(git(root, &["rev-list", "--parents", "-n1", sha])).unwrap();
    let parent = parents.split_whitespace().nth(1);
    let mut args = vec![
        "diff-tree",
        "--root",
        "--no-commit-id",
        "--raw",
        "--no-abbrev",
        "--no-renames",
        "-r",
        "-z",
    ];
    if let Some(parent) = parent {
        args.push(parent);
    }
    args.extend([sha, "--", ".bboxy/reports"]);
    let changes = git(root, &args);
    if changes.is_empty() {
        return (false, vec![]);
    }
    let check = || -> Result<(), String> {
        let old_bytes = parent.map_or(Ok(vec![]), |parent| ledger_bytes(root, parent))?;
        let new_bytes = ledger_bytes(root, sha)?;
        if !new_bytes.starts_with(&old_bytes) {
            return Err("correction ledger is append-only: prior bytes mutated or deleted".into());
        }
        let old = parse(&old_bytes)?;
        let new = parse(&new_bytes)?;
        if !new.starts_with(&old) {
            return Err(
                "correction ledger is append-only: prior entries mutated or deleted".into(),
            );
        }
        let appended = &new[old.len()..];
        let mut used = BTreeSet::new();
        let fields: Vec<_> = changes
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .collect();
        let (pairs, remainder) = fields.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err("invalid report diff field count".into());
        }
        for pair in pairs {
            let raw = std::str::from_utf8(pair[0]).map_err(|_| "invalid report diff")?;
            let parts: Vec<_> = raw.split_whitespace().collect();
            let path = std::str::from_utf8(pair[1]).map_err(|_| "invalid report path")?;
            if path == LEDGER {
                if !matches!(parts[4], "A" | "M") || appended.is_empty() {
                    return Err(
                        "correction ledger is append-only: change requires newly appended entries"
                            .into(),
                    );
                }
                continue;
            }
            // --no-renames deliberately exposes a rename as deletion + addition.
            if parts[4] == "A" {
                continue;
            }
            if parts[4] != "M" || parts[0] != ":100644" || parts[1] != "100644" {
                return Err(format!(
                    "reports are append-only: deletion, rename or type change: {path}"
                ));
            }
            let matches: Vec<_> = appended
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.path == path && entry.from_blob == parts[2] && entry.to_blob == parts[3]
                })
                .collect();
            if matches.len() != 1 {
                return Err(format!(
                    "reports are append-only: {path} needs one newly appended same-commit correction matching actual parent/commit blobs"
                ));
            }
            used.insert(matches[0].0);
        }
        if used.len() != appended.len() {
            return Err("every newly appended correction must match one same-commit report modification; no reuse or unused entries".into());
        }
        if !appended.is_empty() {
            let sources = tree_sources(root, sha);
            let docs: Vec<_> = sources
                .iter()
                .map(|(_, source)| bbt_doc::parse(source))
                .collect();
            let records: Vec<_> = sources
                .iter()
                .zip(&docs)
                .map(|((path, _), markdown)| Record { path, markdown })
                .collect();
            for entry in appended {
                if !resolve(records.iter().copied(), &entry.decision)
                    .ok()
                    .is_some_and(|r| {
                        r.record.markdown.frontmatter.string("type").as_deref() == Some("decision")
                    })
                {
                    return Err(format!(
                        "correction decision must resolve in its commit tree: {}",
                        entry.decision
                    ));
                }
            }
        }
        Ok(())
    };
    (true, check().err().into_iter().collect())
}

#[path = "correction_fixtures.rs"]
mod fixtures;
