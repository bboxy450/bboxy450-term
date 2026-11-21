// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::store::sqlite::SqliteStore;
use ed25519_dalek::SigningKey;
use tempfile::TempDir;

fn fixture() -> (TempDir, Index) {
    let work = TempDir::new().unwrap();
    let docs = work.path().join("knowledge");
    std::fs::create_dir(&docs).unwrap();
    let index = Index::open(
        &docs,
        SqliteStore::open(&work.path().join("index")).unwrap(),
    )
    .unwrap();
    (work, index)
}

fn request<'a>(
    op: Ulid,
    owner: Ulid,
    media: &'a str,
    expected: Option<Address>,
    inputs: &'a [Address],
) -> Write<'a> {
    Write {
        operation: op,
        request_digest: format!("fixture-{op}"),
        expected,
        media_type: media,
        inputs,
        execution: None,
        reader: Some(owner),
    }
}

#[test]
fn artifact_recovery_keeps_original_capture_and_rejects_changed_op() {
    let (work, index) = fixture();
    let owner = Ulid(42);
    let key = SigningKey::from_bytes(&[23; 32]);
    let author = Author::seat(owner, Some(3), &key);
    let writer = index
        .clone()
        .owned_projection(index.try_projection_writer().unwrap().unwrap())
        .unwrap();
    let pending = Store::new(&writer, "root", work.path());
    let op = Ulid::new();
    let mut request = request(op, owner, "text/plain", None, &[]);
    let captured = pending
        .capture(&request, &author, owner, b"first staged bytes")
        .unwrap();
    drop(writer);
    let writer = index
        .clone()
        .owned_projection(index.try_projection_writer().unwrap().unwrap())
        .unwrap();
    let reopened = Store::new(&writer, "root", work.path());
    let revision = reopened
        .write(&request, &author, Some(b"later bytes"))
        .unwrap();
    assert_eq!(revision.address.id, captured.content.revision.address.id);
    let (_, bytes) = reopened
        .read(revision.address.id, Some(1), Some(owner))
        .unwrap();
    assert_eq!(bytes, b"first staged bytes");
    assert_eq!(
        reopened.write(&request, &author, None).unwrap().digest,
        revision.digest
    );
    request.request_digest = "changed arguments".into();
    assert!(
        reopened
            .write(&request, &author, None)
            .unwrap_err()
            .to_string()
            .contains("op_id_reused")
    );
    assert!(
        reopened
            .read(revision.address.id, None, Some(Ulid(44)))
            .is_err()
    );
    assert!(
        reopened
            .read(revision.address.id, Some(2), Some(owner))
            .is_err()
    );
}

#[test]
fn artifact_cas_checks_the_owner_keeps_dense_history_and_pins_historical_inputs() {
    let (work, index) = fixture();
    let owner = Ulid(42);
    let key = SigningKey::from_bytes(&[24; 32]);
    let author = Author::seat(owner, Some(3), &key);
    let reader = Store::new(&index, "root", work.path());
    let create = request(Ulid::new(), owner, SVG, None, &[]);
    assert!(reader.write(&create, &author, Some(b"A")).is_err());
    let writer = index
        .clone()
        .owned_projection(index.try_projection_writer().unwrap().unwrap())
        .unwrap();
    assert!(index.try_projection_writer().unwrap().is_none());
    let store = Store::new(&writer, "root", work.path());
    let a = store.write(&create, &author, Some(b"A")).unwrap();
    let evidence = store
        .write(
            &request(Ulid::new(), owner, "application/json", None, &[]),
            &author,
            Some(br#"{"old":11}"#),
        )
        .unwrap();
    let evidence_pin = [evidence.address];
    let update = request(Ulid::new(), owner, SVG, Some(a.address), &evidence_pin);
    let b = store.write(&update, &author, Some(b"B")).unwrap();
    assert_eq!(b.address.revision, 2);
    assert_eq!(a.address.id, b.address.id);
    assert_eq!(b.capture.declared[0].digest, evidence.digest);
    let conflict = request(Ulid::new(), owner, SVG, Some(a.address), &[]);
    assert!(
        store
            .write(&conflict, &author, Some(b"stale"))
            .unwrap_err()
            .to_string()
            .contains("artifact_conflict")
    );
    let update = request(Ulid::new(), owner, SVG, Some(b.address), &[]);
    let c = store.write(&update, &author, Some(b"A")).unwrap();
    assert_eq!(c.address.revision, 3);
    assert_eq!(c.digest, a.digest);
    assert!(c.capture.declared.is_empty());
    assert_eq!(
        store.read(a.address.id, Some(2), Some(owner)).unwrap().1,
        b"B"
    );
    assert_eq!(
        store.read(a.address.id, Some(1), Some(owner)).unwrap().1,
        b"A"
    );
    let immutable = request(Ulid::new(), owner, SVG, Some(evidence.address), &[]);
    assert!(
        store
            .write(&immutable, &author, Some(b"update evidence"))
            .unwrap_err()
            .to_string()
            .contains("artifact_immutable")
    );
}
