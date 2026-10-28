// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_doc::{
    BlockKind, Inline,
    links::{Record, ResolveError, resolve},
};

// keeps: decision #48 — numeric handles coexist with legacy links, ambiguity is refused
#[test]
fn numeric_wikilinks_keep_their_target_and_resolve_like_slugs_ids_and_paths() {
    let source = "---\nid: 01J00000000000000000000047\ntype: decision\nnumber: 47\n---\n# Decision\nRead [[#47|the decision]].\n";
    let markdown = bbt_doc::parse(source);
    let record = Record {
        path: ".bboxy/decisions/d47-decision-nodes.md",
        markdown: &markdown,
    };
    for target in [
        "#47",
        "[[#47|the decision]]",
        r"[[#47\|the decision]]",
        r"[[d47-decision-nodes\|#47]]",
        "d47-decision-nodes",
        "decisions/d47-decision-nodes.md",
        record.path,
        "01J00000000000000000000047",
    ] {
        let resolved = resolve([record], target).unwrap();
        assert_eq!(resolved.record.path, record.path);
        assert_eq!(
            resolved.record.markdown.document(record.path).number,
            Some(47)
        );
        assert!(resolved.anchor.is_none());
    }
    assert!(markdown.blocks.iter().any(|block| matches!(&block.kind, BlockKind::Paragraph(values) if values.iter().any(|value| matches!(value, Inline::WikiLink { target, display, .. } if target=="#47" && display=="the decision")))));
    for target in ["", "#0", "#047", "#-47", "#47x", "#48", "missing"] {
        assert_eq!(
            resolve([record], target).unwrap_err(),
            ResolveError::Missing,
            "{target}"
        );
    }
}

// keeps: decision #48 — numeric handles coexist with legacy links, ambiguity is refused
#[test]
fn escaped_table_aliases_parse_to_the_same_target_and_display() {
    let markdown = bbt_doc::parse("| Reference |\n|---|\n| [[d47-decision-nodes\\|#47]] |\n");
    let BlockKind::Table { rows, .. } = &markdown.blocks[0].kind else {
        panic!("table missing")
    };
    assert!(rows[0][0].iter().any(|value|matches!(value,Inline::WikiLink {target,display,..} if target=="d47-decision-nodes" && display=="#47")),"{:?}",rows[0][0]);
}

// keeps: decision #48 — numeric handles coexist with legacy links, ambiguity is refused
#[test]
fn duplicate_number_id_or_slug_never_selects_the_first_record() {
    let one = bbt_doc::parse("---\nid: same\nnumber: 47\n---\nOne\n");
    let two = bbt_doc::parse("---\nid: same\nnumber: 47\n---\nTwo\n");
    let records = [
        Record {
            path: "a/record.md",
            markdown: &one,
        },
        Record {
            path: "b/record.md",
            markdown: &two,
        },
    ];
    for target in ["#47", "same", "record"] {
        assert_eq!(
            resolve(records, target).unwrap_err(),
            ResolveError::Ambiguous,
            "{target}"
        );
    }
    assert_eq!(
        resolve(records, "b/record").unwrap().record.path,
        "b/record.md"
    );
}

// keeps: decision #48 — numeric handles coexist with legacy links, ambiguity is refused
#[test]
fn numbered_record_handles_do_not_consume_legacy_heading_anchors() {
    let markdown = bbt_doc::parse("---\nid: ledger\nnumber: 99\n---\n## 47\n47. [[#47]]\n");
    let record = Record {
        path: "decisions/ledger.md",
        markdown: &markdown,
    };
    let resolved = resolve([record], "[[ledger#47|old citation]]").unwrap();
    assert_eq!(resolved.record.path, record.path);
    assert_eq!(resolved.anchor.as_deref(), Some("47"));
    assert_eq!(resolve([record], "#99").unwrap().anchor, None);
}

// keeps: decision #48 — numeric handles coexist with legacy links, ambiguity is refused
#[test]
fn qualified_root_paths_disambiguate_duplicate_readme_slugs() {
    let markdown = bbt_doc::parse("Readme\n");
    let records = [
        Record {
            path: ".bboxy/README.md",
            markdown: &markdown,
        },
        Record {
            path: ".bboxy/specs/README.md",
            markdown: &markdown,
        },
    ];
    assert_eq!(
        resolve(records, "README").unwrap_err(),
        ResolveError::Ambiguous
    );
    assert_eq!(
        resolve(records, ".bboxy/README").unwrap().record.path,
        ".bboxy/README.md"
    );
    assert_eq!(
        resolve(records, "specs/README").unwrap().record.path,
        ".bboxy/specs/README.md"
    );
}
