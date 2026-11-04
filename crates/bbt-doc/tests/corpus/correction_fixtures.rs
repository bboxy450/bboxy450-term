// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::{
    Scratch,
    reports::{commit_errors, sources},
};

const REPORT: &str = ".bboxy/reports/2026-10-31-work.md";

struct Fixture {
    root: Scratch,
    base: String,
}
impl Fixture {
    fn new() -> Self {
        let root = Scratch::new();
        git(&root.0, &["init", "-q"]);
        for (path, body) in sources().into_iter().chain([(
            ".bboxy/decisions/d3-corrections.md".into(),
            "---\nid: decision\nnumber: 3\ntype: decision\n---\nAllow exact legacy repairs.\n"
                .into(),
        )]) {
            fs_write(&root.0, &path, &body);
        }
        let mut fixture = Self {
            root,
            base: String::new(),
        };
        fixture.commit("Original report");
        fixture.base = fixture.sha();
        fixture
    }
    fn sha(&self) -> String {
        String::from_utf8(git(&self.root.0, &["rev-parse", "HEAD"]))
            .unwrap()
            .trim()
            .into()
    }
    fn commit(&self, message: &str) {
        git(&self.root.0, &["add", "-A"]);
        git(
            &self.root.0,
            &[
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Report Fixture",
                "-c",
                "user.email=fixture@bboxy450.invalid",
                "commit",
                "-qm",
                message,
            ],
        );
    }
    fn errors(&self) -> Vec<String> {
        commit_errors(&self.root.0, "HEAD^!")
    }
    fn correct(&self, suffix: &str) -> String {
        let from = String::from_utf8(git(&self.root.0, &["rev-parse", &format!("HEAD:{REPORT}")]))
            .unwrap();
        let body = std::fs::read_to_string(self.root.0.join(REPORT)).unwrap() + suffix;
        fs_write(&self.root.0, REPORT, &body);
        let to = String::from_utf8(git(&self.root.0, &["hash-object", REPORT])).unwrap();
        format!(
            "\n[[correction]]\npath = {REPORT:?}\nfrom_blob = {:?}\nto_blob = {:?}\nreason = \"Repair legacy metadata\"\ndecision = \"[[decisions/d3-corrections]]\"\n",
            from.trim(),
            to.trim()
        )
    }
    fn append(&self, entry: &str) {
        let path = self.root.0.join(LEDGER);
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        fs_write(&self.root.0, LEDGER, &(old + entry));
    }
    fn reject(&self, expected: &str) {
        let errors = self.errors();
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "{errors:?}"
        );
    }
}
fn fs_write(root: &Path, path: &str, body: &str) {
    std::fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    std::fs::write(root.join(path), body).unwrap();
}

#[test]
fn exact_corrections_append_across_commits_and_use_committed_blobs() {
    let fixture = Fixture::new();
    let first = fixture.correct("\nCorrection one");
    fixture.append(&first);
    fixture.commit("Exact correction without a Report trailer");
    assert!(fixture.errors().is_empty(), "{:?}", fixture.errors());
    let second = fixture.correct("\nCorrection two");
    fixture.append(&second);
    fixture.commit("Another exact correction");
    assert!(commit_errors(&fixture.root.0, &format!("{}..HEAD", fixture.base)).is_empty());
    // Neither a dirty report nor a dirty ledger can alter historical results.
    fs_write(&fixture.root.0, REPORT, "malformed working report");
    fs_write(&fixture.root.0, LEDGER, "malformed working ledger");
    assert!(fixture.errors().is_empty());
}

#[test]
fn correction_broken_controls_refuse_unlisted_wrong_blobs_deletion_and_rename() {
    for fault in ["unlisted", "wrong-from", "wrong-to", "deletion", "rename"] {
        let fixture = Fixture::new();
        let mut entry = fixture.correct("\nRepair");
        match fault {
            "unlisted" => (),
            "wrong-from" | "wrong-to" => {
                let field = if fault == "wrong-from" {
                    "from_blob"
                } else {
                    "to_blob"
                };
                let line = entry.lines().find(|line| line.starts_with(field)).unwrap();
                entry = entry.replace(line, &format!("{field} = \"{}\"", "a".repeat(40)));
                fixture.append(&entry);
            }
            "deletion" => {
                std::fs::remove_file(fixture.root.0.join(REPORT)).unwrap();
            }
            "rename" => {
                std::fs::rename(
                    fixture.root.0.join(REPORT),
                    fixture.root.0.join(".bboxy/reports/2026-10-31-renamed.md"),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        fixture.commit(fault);
        fixture.reject("append-only");
    }
}

#[test]
fn correction_broken_controls_refuse_prior_ledger_mutation_removal_and_reuse() {
    for fault in [
        "mutation",
        "entry-removal",
        "file-removal",
        "reuse",
        "duplicate",
        "append-to-comment",
    ] {
        let fixture = Fixture::new();
        let first = fixture.correct("\nFirst repair");
        fixture.append(&first);
        fixture.commit("Valid repair");
        assert!(fixture.errors().is_empty());
        let second = fixture.correct("\nSecond repair");
        match fault {
            "mutation" => fs_write(
                &fixture.root.0,
                LEDGER,
                &first.replace("Repair legacy metadata", "Changed history"),
            ),
            "entry-removal" => fs_write(&fixture.root.0, LEDGER, "# removed\n"),
            "file-removal" => std::fs::remove_file(fixture.root.0.join(LEDGER)).unwrap(),
            "reuse" => (),
            "duplicate" => fixture.append(&first),
            "append-to-comment" => {
                // A byte prefix alone is insufficient when an append changes a prior TOML value.
                fs_write(
                    &fixture.root.0,
                    LEDGER,
                    &(first.trim_end().to_owned() + " # suffix"),
                );
            }
            _ => unreachable!(),
        }
        if matches!(fault, "mutation" | "entry-removal" | "append-to-comment") {
            fixture.append(&second);
        }
        fixture.commit(fault);
        assert!(!fixture.errors().is_empty(), "accepted {fault}");
    }
}

#[test]
fn correction_broken_controls_refuse_malformed_unused_and_unresolved_entries() {
    for fault in [
        "toml",
        "missing",
        "unknown",
        "empty-reason",
        "bad-hash",
        "path",
        "decision",
        "non-decision",
        "unused",
    ] {
        let fixture = Fixture::new();
        let mut entry = fixture.correct("\nRepair");
        entry = match fault {
            "toml" => "[[invalid".into(),
            "missing" => entry
                .lines()
                .filter(|line| !line.starts_with("reason"))
                .collect::<Vec<_>>()
                .join("\n"),
            "unknown" => entry + "extra = true\n",
            "empty-reason" => entry.replace("Repair legacy metadata", "  "),
            "bad-hash" => entry.replace("from_blob = \"", "from_blob = \"short"),
            "path" => entry.replace(REPORT, ".bboxy/reports/../escape.md"),
            "decision" => entry.replace("[[decisions/d3-corrections]]", "[[missing]]"),
            "non-decision" => entry.replace("[[decisions/d3-corrections]]", "[[tasks/work]]"),
            "unused" => {
                fs_write(&fixture.root.0, REPORT, &sources()[1].1);
                entry
            }
            _ => unreachable!(),
        };
        fixture.append(&entry);
        fixture.commit(fault);
        assert!(!fixture.errors().is_empty(), "accepted {fault}");
    }
}

#[test]
fn corrections_cannot_repair_an_earlier_unlisted_commit_or_bypass_metadata() {
    let fixture = Fixture::new();
    let first = fixture.correct("\nUnlisted repair");
    fixture.commit("Missing same-commit ledger");
    fixture.reject("same-commit");
    fixture.append(&first);
    fixture.commit("Too late");
    fixture.reject("same-commit");
    assert!(!commit_errors(&fixture.root.0, &format!("{}..HEAD", fixture.base)).is_empty());
    for (from, to, expected) in [
        (
            "author: seat:01M6AGAA0G0436ZEJAFB708CF1",
            "author: unknown",
            "author must name a seat",
        ),
        ("type: report", "type: note", "type: report"),
        ("number: 2", "number: 1", "duplicate record number"),
        ("id: report", "", "requires valid frontmatter"),
        ("[[tasks/work]]", "[[missing]]", "task does not resolve"),
    ] {
        let fixture = Fixture::new();
        let body = sources()[1].1.replace(from, to);
        fs_write(&fixture.root.0, REPORT, &body);
        let entry = fixture.correct("");
        fixture.append(&entry);
        fixture.commit("Exact blobs, invalid report, no trailer");
        fixture.reject(expected);
    }
}

#[test]
fn historical_invalid_metadata_and_reused_a_b_a_entries_remain_errors() {
    let fixture = Fixture::new();
    let original = sources()[1].1.clone();
    fs_write(
        &fixture.root.0,
        REPORT,
        &original.replace("number: 2", "number: 1"),
    );
    let bad = fixture.correct("");
    fixture.append(&bad);
    fixture.commit("Invalid correction metadata");
    fixture.reject("duplicate record number");
    fs_write(&fixture.root.0, REPORT, &original);
    let restored = fixture.correct("");
    fixture.append(&restored);
    fixture.commit("Restore valid report with a new exact correction");
    assert!(fixture.errors().is_empty());
    let errors = commit_errors(&fixture.root.0, &format!("{}..HEAD", fixture.base));
    assert!(
        errors.iter().any(|e| e.contains("duplicate record number")),
        "{errors:?}"
    );
    // Repeating A -> B after A -> B -> A must append a reused triplet and fail.
    fs_write(
        &fixture.root.0,
        REPORT,
        &original.replace("number: 2", "number: 1"),
    );
    fixture.append(&bad.replace(
        "Repair legacy metadata",
        "A different reason cannot reuse an old entry",
    ));
    fixture.commit("Reuse old A to B");
    fixture.reject("reuse");
}
