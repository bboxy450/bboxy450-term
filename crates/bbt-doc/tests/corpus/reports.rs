// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use bbt_doc::links::{Record, resolve};
use std::{
    io::Write,
    process::{Command, Stdio},
};

pub(super) fn report_errors(sources: &[(String, String)], message: &str) -> Vec<String> {
    let docs: Vec<_> = sources.iter().map(|(_, s)| bbt_doc::parse(s)).collect();
    let records: Vec<_> = sources
        .iter()
        .zip(&docs)
        .map(|((path, _), markdown)| Record { path, markdown })
        .collect();
    let mut errors = Vec::new();
    for record in &records {
        if record.markdown.frontmatter.string("type").as_deref() != Some("report") {
            continue;
        }
        let metadata = serde_json::to_value(&record.markdown.frontmatter.fields).unwrap();
        if let Err(e) = bbt_doc::reports::validate(record.path, &metadata) {
            errors.push(format!("{}: {e}", record.path));
        }
        let task = metadata["task"].as_str().unwrap_or_default();
        match resolve(records.iter().copied(), task) {
            Ok(target)
                if matches!(
                    target.record.markdown.frontmatter.string("type").as_deref(),
                    Some("task" | "bug" | "epic")
                ) => {}
            _ => errors.push(format!(
                "{}: report task does not resolve to a task: {task}",
                record.path
            )),
        }
        if let Some(previous) = metadata["supersedes"].as_str() {
            let valid = resolve(records.iter().copied(), previous)
                .ok()
                .filter(|r| {
                    r.record.path != record.path
                        && r.record.markdown.frontmatter.string("type").as_deref() == Some("report")
                })
                .and_then(|r| r.record.markdown.frontmatter.string("task"))
                .and_then(|t| resolve(records.iter().copied(), &t).ok())
                .zip(resolve(records.iter().copied(), task).ok())
                .is_some_and(|(a, b)| a.record.path == b.record.path);
            if !valid {
                errors.push(format!(
                    "{}: supersedes must resolve to another report for the same task",
                    record.path
                ));
            }
        }
    }
    match bbt_doc::reports::trailers(message) {
        Err(e) => errors.push(e),
        Ok(trailers) => {
            for trailer in trailers {
                match resolve(records.iter().copied(), trailer) {
                    Ok(target)
                        if target.record.markdown.frontmatter.string("type").as_deref()
                            == Some("report") => {}
                    _ => errors.push(format!(
                        "Report: {trailer} does not resolve to a report in the same tree"
                    )),
                }
            }
        }
    }
    errors
}

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

// Use the commit's blobs, never today's working files. One batch process reads
// the complete Markdown catalogue so ID/#N/slug ambiguity has the normal meaning.
fn tree_sources(root: &Path, sha: &str) -> Vec<(String, String)> {
    let listing = String::from_utf8(git(root, &["ls-tree", "-rz", sha, "--", ".bboxy"])).unwrap();
    let entries: Vec<_> = listing
        .split('\0')
        .filter_map(|line| {
            let (object, path) = line.split_once('\t')?;
            if !path.ends_with(".md") {
                return None;
            }
            Some((
                object.split_whitespace().nth(2)?.to_owned(),
                path.to_owned(),
            ))
        })
        .collect();
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let ids: Vec<_> = entries.iter().map(|(id, _)| id.clone()).collect();
    let writer = std::thread::spawn(move || {
        for id in ids {
            writeln!(input, "{id}").unwrap();
        }
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success());
    let mut bytes = output.stdout.as_slice();
    entries
        .into_iter()
        .map(|(_, path)| {
            let end = bytes.iter().position(|b| *b == b'\n').unwrap();
            let header = std::str::from_utf8(&bytes[..end]).unwrap();
            let len: usize = header.split_whitespace().nth(2).unwrap().parse().unwrap();
            let body = String::from_utf8(bytes[end + 1..end + 1 + len].to_vec()).unwrap();
            bytes = &bytes[end + 2 + len..];
            (path, body)
        })
        .collect()
}

pub(super) fn commit_errors(root: &Path, range: &str) -> Vec<String> {
    let log = String::from_utf8(git(root, &["log", "--format=%H%x00%B%x00", range])).unwrap();
    let mut fields = log.split('\0');
    let mut errors = Vec::new();
    while let (Some(sha), Some(message)) = (fields.next(), fields.next()) {
        if !message.lines().any(|s| s.starts_with("Report:")) {
            continue;
        }
        let sha = sha.trim();
        for error in report_errors(&tree_sources(root, sha), message) {
            errors.push(format!("{sha}: {error}"));
        }
    }
    let changes = String::from_utf8(git(
        root,
        &[
            "log",
            "--format=%H",
            "--name-only",
            "--no-renames",
            "--diff-filter=MD",
            range,
            "--",
            ".bboxy/reports",
        ],
    ))
    .unwrap();
    if !changes.trim().is_empty() {
        errors.push(format!(
            "reports are append-only; create a correction with supersedes instead:\n{changes}"
        ));
    }
    errors
}

fn commit_range(range: Option<&str>) -> &str {
    // Presubmit supplies the complete candidate range. Standalone records checks
    // validate the current commit; published history predating report records
    // used plain-path Report fields and cannot be retroactively rewritten.
    range.filter(|s| !s.is_empty()).unwrap_or("HEAD^!")
}

pub(super) fn repository_commit_errors(root: &Path) -> Vec<String> {
    let range = std::env::var("SIGNED_RANGE").ok();
    commit_errors(root, commit_range(range.as_deref()))
}

#[test]
fn committed_report_trailers_resolve_in_their_own_trees() {
    let root = corpus_root();
    if std::env::var_os("BBOXY_DOC_CORPUS_ROOT").is_some() {
        return;
    }
    let errors = repository_commit_errors(&root);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn sources() -> Vec<(String, String)> {
    vec![(".bboxy/tasks/work.md".into(), "---\nid: task\nnumber: 1\ntype: task\n---\nBrief".into()),
        (".bboxy/reports/2026-10-31-work.md".into(), "---\nid: report\nnumber: 2\ntype: report\ntask: '[[tasks/work]]'\ncommits: []\nverify: {command: 'true', exit_code: 0}\nauthor: seat:01M6AGAA0G0436ZEJAFB708CF1\ncreated: '2026-10-31T00:00:00Z'\n---\nFull evidence".into())]
}

#[test]
fn report_gate_rejects_dangling_trailer_task_and_malformed_records() {
    let sources = sources();
    assert!(report_errors(&sources, "Work\n\nReport: [[reports/2026-10-31-work]]").is_empty());
    assert!(report_errors(&sources, "No trailer").is_empty());
    assert!(
        report_errors(&sources, "Work\n\nReport: [[missing]]")
            .iter()
            .any(|s| s.contains("same tree"))
    );
    assert!(!report_errors(&sources, "Work\n\nReport: invalid").is_empty());
    assert!(!report_errors(&sources, "Work\n\nReport: [[tasks/work]]").is_empty());
    let mut missing = sources.clone();
    missing.remove(0);
    assert!(
        report_errors(&missing, "")
            .iter()
            .any(|s| s.contains("task does not resolve"))
    );
    for field in [
        "commits: []",
        "verify: {command: 'true', exit_code: 0}",
        "author: seat:01M6AGAA0G0436ZEJAFB708CF1",
        "created: '2026-10-31T00:00:00Z'",
    ] {
        let mut bad = sources.clone();
        bad[1].1 = bad[1].1.replace(field, "");
        assert!(!report_errors(&bad, "").is_empty(), "missing {field}");
    }
}

#[test]
fn commit_gate_uses_committed_tree_not_later_report_files() {
    let root = Scratch::new();
    git(&root.0, &["init", "-q"]);
    git(
        &root.0,
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Report Fixture",
            "-c",
            "user.email=fixture@bboxy450.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "Published legacy report field\n\nReport: old.log",
        ],
    );
    let legacy = String::from_utf8(git(&root.0, &["rev-parse", "HEAD"])).unwrap();
    for (path, content) in sources().into_iter().take(1) {
        fs::create_dir_all(root.0.join(&path).parent().unwrap()).unwrap();
        fs::write(root.0.join(path), content).unwrap();
    }
    git(&root.0, &["add", ".bboxy"]);
    git(
        &root.0,
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Report Fixture",
            "-c",
            "user.email=fixture@bboxy450.invalid",
            "commit",
            "-qm",
            "Work\n\nReport: [[reports/2026-10-31-work]]",
        ],
    );
    let (path, content) = sources().remove(1);
    fs::create_dir_all(root.0.join(&path).parent().unwrap()).unwrap();
    fs::write(root.0.join(path), content).unwrap();
    assert!(
        commit_errors(&root.0, commit_range(None))
            .iter()
            .any(|s| s.contains("same tree"))
    );
    git(&root.0, &["add", ".bboxy"]);
    git(
        &root.0,
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Report Fixture",
            "-c",
            "user.email=fixture@bboxy450.invalid",
            "commit",
            "-qm",
            "Add full report\n\nReport: [[reports/2026-10-31-work]]",
        ],
    );
    assert!(commit_errors(&root.0, "HEAD^..HEAD").is_empty());
    assert!(commit_errors(&root.0, commit_range(None)).is_empty());
    assert_eq!(commit_range(Some("")), "HEAD^!");
    assert_eq!(commit_range(Some("HEAD^..HEAD")), "HEAD^..HEAD");
    assert!(
        commit_errors(&root.0, &format!("{}..HEAD", legacy.trim()))
            .iter()
            .any(|s| s.contains("same tree"))
    );
    assert!(!commit_errors(&root.0, "HEAD").is_empty());
    let report_path = root.0.join(".bboxy/reports/2026-10-31-work.md");
    fs::write(
        &report_path,
        fs::read_to_string(&report_path).unwrap() + "\nChanged",
    )
    .unwrap();
    git(&root.0, &["add", ".bboxy"]);
    git(
        &root.0,
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Report Fixture",
            "-c",
            "user.email=fixture@bboxy450.invalid",
            "commit",
            "-qm",
            "Attempt to rewrite an existing report",
        ],
    );
    assert!(
        commit_errors(&root.0, "HEAD^..HEAD")
            .iter()
            .any(|e| e.contains("append-only"))
    );
}
