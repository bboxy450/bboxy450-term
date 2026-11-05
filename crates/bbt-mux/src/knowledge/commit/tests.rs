// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::{
    index::Index,
    store::{Events, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;
use serde_json::json;

#[test]
fn explicit_private_commit_is_the_artifact_boundary_and_later_edits_are_drafts() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("source");
    std::fs::create_dir_all(&repository).unwrap();
    let run = |root: &Path, args: &[&str]| {
        checked(root, args, Instant::now() + Duration::from_secs(15)).unwrap()
    };
    run(
        &repository,
        &["init", "--quiet", "--initial-branch=main", "--template="],
    );
    run(&repository, &["config", "user.name", "Fixture"]);
    run(
        &repository,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(repository.join(".gitignore"), ".bboxy/\n").unwrap();
    run(&repository, &["add", ".gitignore"]);
    run(
        &repository,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "source fixture",
        ],
    );
    let source_head = run(&repository, &["rev-parse", "HEAD"]);
    let root = repository.join(".bboxy");
    let index = Index::open(
        &root,
        SqliteStore::open(&temp.path().join("index")).unwrap(),
    )
    .unwrap();
    let key = SigningKey::from_bytes(&[62; 32]);
    let id = crate::Ulid::new();
    index
        .write(
            "notes/checkpoint.md",
            json!({"id":id,"type":"note"}),
            "first draft",
            crate::Ulid(1),
            &key,
        )
        .unwrap();
    index
        .write(
            "notes/unselected.md",
            json!({"id":crate::Ulid::new(),"type":"note"}),
            "unselected",
            crate::Ulid(1),
            &key,
        )
        .unwrap();
    let document = index.current("notes/checkpoint.md").unwrap().unwrap();
    let reference = d::RecordRef {
        id: id.to_string(),
        number: None,
        slug: "notes/checkpoint".into(),
        path: document.path,
        digest: document.digest,
        external_id: None,
        updated_at: None,
    };
    assert_eq!(
        state(&index, &reference.id, &reference.digest).unwrap()["state"],
        "draft"
    );
    assert!(
        index
            .activity()
            .after(0, 128)
            .unwrap()
            .iter()
            .all(|e| !e.event.subject.starts_with("artifact/"))
    );
    let signing = temp.path().join("signing-key");
    assert!(
        Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&signing)
            .stdin(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    run(&repository, &["config", "gpg.format", "ssh"]);
    run(
        &repository,
        &["config", "user.signingkey", signing.to_str().unwrap()],
    );
    let result = local(
        &root,
        d::CommitRequest {
            refs: vec![reference.clone()],
            message: "Deliberate checkpoint".into(),
        },
    )
    .unwrap();
    let anchor = record(
        &index,
        &result,
        &crate::knowledge::WriteContext::seat(crate::Ulid(1), Some(1), &key),
    )
    .unwrap();
    assert_eq!(
        state(&index, &reference.id, &reference.digest).unwrap()["state"],
        "committed"
    );
    assert_eq!(
        state(&index, &reference.id, &reference.digest).unwrap()["checkpoint"]["id"],
        anchor["id"]
    );
    assert_eq!(
        run(&repository, &["rev-parse", "HEAD"]),
        source_head,
        "private KB commit changed source history"
    );
    assert_eq!(
        run(&root, &["show", "--pretty=format:", "--name-only", "HEAD"]).trim(),
        "notes/checkpoint.md"
    );
    assert!(run(&root, &["cat-file", "commit", "HEAD"]).contains("BEGIN SSH SIGNATURE"));
    let allowed = temp.path().join("allowed-signers");
    std::fs::write(
        &allowed,
        format!(
            "fixture@example.invalid {}\n",
            std::fs::read_to_string(signing.with_extension("pub"))
                .unwrap()
                .trim()
        ),
    )
    .unwrap();
    run(
        &root,
        &[
            "config",
            "gpg.ssh.allowedSignersFile",
            allowed.to_str().unwrap(),
        ],
    );
    run(&root, &["verify-commit", "HEAD"]);
    index
        .write(
            "notes/checkpoint.md",
            json!({"id":id,"type":"note"}),
            "next draft",
            crate::Ulid(1),
            &key,
        )
        .unwrap();
    let edited = index.current("notes/checkpoint.md").unwrap().unwrap();
    assert_eq!(
        state(&index, &reference.id, &edited.digest).unwrap()["state"],
        "draft"
    );
    assert_eq!(
        run(&root, &["show", "HEAD:notes/checkpoint.md"])
            .lines()
            .last(),
        Some("first draft")
    );
    assert_eq!(
        index
            .activity()
            .after(0, 128)
            .unwrap()
            .iter()
            .filter(|e| e.event.subject.starts_with("artifact/"))
            .count(),
        1
    );
}
