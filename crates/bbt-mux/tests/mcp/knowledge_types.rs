// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The real MCP bridge discovers static type help and writes concerns with existing authority.
use super::*;

#[test]
fn mcp_knowledge_types_are_discoverable_and_readable_by_workers_and_oracles() {
    let f = Fixture::new();
    for seat in [f.own.id, f.oracle.id] {
        let mut bridge = f.bridge(seat);
        let initialized = rpc(
            &mut bridge,
            0,
            "initialize",
            json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}),
        );
        assert!(
            initialized["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("knowledge_types")
        );
        let list = rpc(&mut bridge, 1, "tools/list", json!({}));
        let tools = list["result"]["tools"].as_array().unwrap();
        let help = tools
            .iter()
            .find(|t| t["name"] == "knowledge_types")
            .unwrap();
        assert_eq!(help["annotations"]["readOnlyHint"], true);
        assert_eq!(help["annotations"]["destructiveHint"], false);
        assert!(
            help["inputSchema"]["required"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let write = tools
            .iter()
            .find(|t| t["name"] == "knowledge_write")
            .unwrap();
        assert!(
            write["description"]
                .as_str()
                .unwrap()
                .contains("knowledge_types")
        );

        let response = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"knowledge_types","arguments":{}}),
        );
        let dictionary = data(response["result"].clone());
        let entries = dictionary["types"].as_array().unwrap();
        let kinds: std::collections::BTreeSet<_> = entries
            .iter()
            .map(|entry| entry["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds.len(), 16);
        for kind in ["concern", "artifact", "report", "interface", "recipe"] {
            assert!(kinds.contains(kind));
        }
        for entry in entries {
            for key in ["purpose", "path", "writing"] {
                assert!(!entry[key].as_str().unwrap().is_empty(), "{entry}");
            }
        }
        assert!(
            dictionary["writing"]["update"]
                .as_str()
                .unwrap()
                .contains("etag")
        );
        assert!(
            dictionary["writing"]["custom_types"]
                .as_str()
                .unwrap()
                .contains("Oracle")
        );

        let filtered = rpc(
            &mut bridge,
            3,
            "tools/call",
            json!({"name":"knowledge_types","arguments":{"type":"concern"}}),
        );
        let concern = data(filtered["result"].clone());
        assert_eq!(concern["types"].as_array().unwrap().len(), 1);
        assert_eq!(concern["types"][0]["type"], "concern");
        assert_eq!(concern["types"][0]["path"], "concerns/<slug>.md");
        assert!(
            concern["types"][0]["purpose"]
                .as_str()
                .unwrap()
                .contains("what is at stake")
        );
        let recipe = data(f.call(
            seat,
            Ulid::new(),
            "knowledge_types",
            json!({"type":"recipe"}),
        ));
        assert!(
            recipe["types"][0]["writing"]
                .as_str()
                .unwrap()
                .contains("any tool or workflow")
        );
    }
}

#[test]
fn mcp_knowledge_types_refuse_unknown_types_and_invalid_arguments() {
    let f = Fixture::new();
    for args in [
        json!({"type":"unknown-kind"}),
        json!({"type":""}),
        json!({"type":42}),
        json!({"path":"concerns/example.md"}),
    ] {
        knowledge_denied(
            f.call(f.own.id, Ulid::new(), "knowledge_types", args),
            "rejected",
        );
    }
}

#[test]
fn mcp_knowledge_types_help_survives_an_unavailable_document_root() {
    let f = Fixture::new();
    std::fs::rename(
        f.config.repository.join(".bboxy"),
        f.config.repository.join("private-unavailable-root"),
    )
    .unwrap();
    let help = data(f.call(
        f.own.id,
        Ulid::new(),
        "knowledge_types",
        json!({"type":"concern"}),
    ));
    assert_eq!(help["types"][0]["path"], "concerns/<slug>.md");
}

#[test]
fn mcp_knowledge_types_concern_uses_its_prefix_and_existing_signed_write_policy() {
    let f = Fixture::new();
    let path = "concerns/session-continuity.md";
    knowledge_denied(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "knowledge_write",
            knowledge_record_args(path, "concern", "Session continuity matters to operators."),
        ),
        "knowledge_write_denied",
    );
    assert!(!f.config.repository.join(".bboxy").join(path).exists());
    let created = knowledge_create(
        &f,
        f.oracle.id,
        "concern",
        path,
        "What matters: session continuity. Evidence: sessions survive interruption.",
    );
    assert_eq!(created["path"], path);
    assert_eq!(created["frontmatter"]["type"], "concern");
    verify_knowledge_record(&created);
    let fetched = data(knowledge_fetch(&f, f.own.id, path));
    assert_eq!(fetched["id"], created["id"]);
    assert_eq!(fetched["body"], created["body"]);
    let found = data(knowledge_search(
        &f,
        f.own.id,
        json!({"query":"type:concern"}),
    ));
    assert_eq!(found["items"].as_array().unwrap().len(), 1);
    assert_eq!(found["items"][0]["ref"]["id"], created["id"]);
}
