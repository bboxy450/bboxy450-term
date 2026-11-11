// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Reports are machine-local KB records (.bboxy/.gitignore): this checks the ones
// present, never that a commit carries or links one.
use super::*;
use bbt_doc::links::{Record, resolve};

pub(super) fn report_errors(sources: &[(String, String)]) -> Vec<String> {
    let docs: Vec<_> = sources.iter().map(|(_, s)| bbt_doc::parse(s)).collect();
    let records: Vec<_> = sources
        .iter()
        .zip(&docs)
        .map(|((path, _), markdown)| Record { path, markdown })
        .collect();
    let mut errors = Vec::new();
    for record in &records {
        if record.markdown.frontmatter.string("type").as_deref() != Some("report")
            && !record.path.starts_with(".bboxy/reports/")
        {
            continue;
        }
        if record.markdown.frontmatter.error.is_some()
            || !record.markdown.frontmatter.fields.contains_key("id")
            || record.markdown.frontmatter.number().is_none()
        {
            errors.push(format!(
                "{}: report requires valid frontmatter, id and positive number",
                record.path
            ));
        }
        if let Some(number) = record.markdown.frontmatter.number()
            && records
                .iter()
                .filter(|r| r.markdown.frontmatter.number() == Some(number))
                .count()
                != 1
        {
            errors.push(format!("{}: duplicate record number {number}", record.path));
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
    errors
}

fn sources() -> Vec<(String, String)> {
    vec![(".bboxy/tasks/work.md".into(), "---\nid: task\nnumber: 1\ntype: task\n---\nBrief".into()),
        (".bboxy/reports/2026-10-31-work.md".into(), "---\nid: report\nnumber: 2\ntype: report\ntask: '[[tasks/work]]'\ncommits: []\nverify: {command: 'true', exit_code: 0}\nauthor: seat:01M6AGAA0G0436ZEJAFB708CF1\ncreated: '2026-10-31T00:00:00Z'\n---\nFull evidence".into())]
}

#[test]
fn report_gate_rejects_dangling_task_and_malformed_records() {
    let sources = sources();
    assert!(report_errors(&sources).is_empty());
    let mut missing = sources.clone();
    missing.remove(0);
    assert!(
        report_errors(&missing)
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
        assert!(!report_errors(&bad).is_empty(), "missing {field}");
    }
}
