// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private signed knowledge-root fixture.

use bbt_mux::{
    Ulid,
    index::{GraphKind, Index},
    store::sqlite::SqliteStore,
};
use ed25519_dalek::SigningKey;
use petgraph::visit::EdgeRef;
use serde_json::json;

#[test]
fn research_tree_and_downstream_origins_survive_signed_writes_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    let store = SqliteStore::open(&temp.path().join("index")).unwrap();
    let mut index = Index::open(&root, store.clone()).unwrap();
    let researcher = Ulid::new();
    let oracle = Ulid::new();
    let research_key = SigningKey::from_bytes(&[61; 32]);
    let oracle_key = SigningKey::from_bytes(&[62; 32]);
    let topic = Ulid::new();
    let evidence = Ulid::new();
    let spec = Ulid::new();
    let task = Ulid::new();
    let dream = Ulid::new();
    let source = "https://example.org/primary-source";
    for (path, metadata, body, author, key) in [
        (
            "research/topic.md",
            json!({"id":topic,"type":"research","children":["[[research/topic/source]]"],"derived_from":[format!("[[{evidence}]]")]}),
            "# Question and evolving findings\n[[research/topic/source]]\n",
            researcher,
            &research_key,
        ),
        (
            "research/topic/source.md",
            json!({"id":evidence,"type":"research","parent":"[[research/topic]]","links":[{"to":source,"rel":"used"}]}),
            "# Source evidence\nRetrieved 2026-11-04; observed results and limitations.\n",
            researcher,
            &research_key,
        ),
        (
            "specs/result.md",
            json!({"id":spec,"type":"spec","derived_from":[format!("[[{topic}]]")]}),
            "# Specification grounded in research\n",
            oracle,
            &oracle_key,
        ),
        (
            "tasks/implementation.md",
            json!({"id":task,"type":"task","derived_from":["[[specs/result]]"]}),
            "# Task grounded in the specification\n",
            oracle,
            &oracle_key,
        ),
        // A future kind uses the same generic lineage contract; this adds no dream workflow.
        (
            "ideas/dream.md",
            json!({"id":dream,"type":"dream","derived_from":[format!("[[{task}]]")]}),
            "# Idea and its recorded origins\n",
            oracle,
            &oracle_key,
        ),
    ] {
        index.write(path, metadata, body, author, key).unwrap();
    }
    index.scan().unwrap();
    for document in index.documents.values() {
        assert!(document.error.is_none(), "{:?}", document.error);
        assert!(document.metadata["provenance"]["sig"].is_string());
        let expected = if document.metadata["type"] == "research" {
            researcher
        } else {
            oracle
        };
        assert_eq!(document.metadata["created_by"], format!("seat:{expected}"));
    }
    let assert_graph = |index: &Index| {
        let graph = &index.graphs[&GraphKind::Provenance];
        for (from, to, relation) in [
            (dream.to_string(), task.to_string(), "derived_from"),
            (task.to_string(), spec.to_string(), "derived_from"),
            (spec.to_string(), topic.to_string(), "derived_from"),
            (topic.to_string(), evidence.to_string(), "derived_from"),
            (evidence.to_string(), source.to_owned(), "used"),
        ] {
            assert!(
                graph
                    .edge_references()
                    .any(|edge| graph[edge.source()] == from
                        && graph[edge.target()] == to
                        && edge.weight().relation == relation),
                "missing {from} --{relation}--> {to}"
            );
        }
        let tree = &index.graphs[&GraphKind::Markdown];
        assert!(
            tree.edge_references()
                .any(|edge| tree[edge.source()] == evidence.to_string()
                    && tree[edge.target()] == topic.to_string()
                    && edge.weight().relation == "parent")
        );
    };
    assert_graph(&index);
    drop(index);
    let mut reopened = Index::open(&root, store).unwrap();
    reopened.scan().unwrap();
    assert_graph(&reopened);
    assert_eq!(reopened.documents.len(), 5);
}
