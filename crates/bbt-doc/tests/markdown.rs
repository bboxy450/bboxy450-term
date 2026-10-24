use bbt_doc::{BlockKind, Inline, TableAlign, parse};

#[test]
fn commonmark_builds_nested_blocks_and_keeps_original_utf8_offsets() {
    let source = "---\ntitle: Fox\nunknown: [one, two]\n---\n# Café\n\nA **bold** and *slanted* ~~old~~ line\ncontinues.\n\n> 3. first\n> 4. second\n\n- [x] done\n- [ ] todo\n";
    let doc = parse(source);
    assert_eq!(doc.frontmatter.fields["title"].as_str(), Some("Fox"));
    assert!(doc.frontmatter.fields.contains_key("unknown"));
    assert!(matches!(
        doc.blocks[0].kind,
        BlockKind::Heading { level: 1, .. }
    ));
    assert_eq!(&source[doc.blocks[0].source.clone()], "# Café\n");
    let BlockKind::Paragraph(p) = &doc.blocks[1].kind else {
        panic!("paragraph")
    };
    assert!(p.iter().any(|i| matches!(i, Inline::Strong(_))));
    assert!(p.iter().any(|i| matches!(i, Inline::Emphasis(_))));
    assert!(p.iter().any(|i| matches!(i, Inline::Strike(_))));
    assert!(p.iter().any(|i| matches!(i, Inline::SoftBreak)));
    let BlockKind::Quote(q) = &doc.blocks[2].kind else {
        panic!("quote")
    };
    assert!(matches!(&q[0].kind, BlockKind::List { start: Some(3), items } if items.len()==2));
    let BlockKind::List { items, .. } = &doc.blocks[3].kind else {
        panic!("task list")
    };
    assert!(
        matches!(&items[0][0].kind, BlockKind::Paragraph(p) if p.contains(&Inline::Task(true)))
    );
    assert!(
        matches!(&items[1][0].kind, BlockKind::Paragraph(p) if p.contains(&Inline::Task(false)))
    );
    assert!(
        doc.blocks
            .iter()
            .all(|b| source.get(b.source.clone()).is_some())
    );
}

#[test]
fn wikilinks_retain_labels_weights_and_do_not_activate_inside_code() {
    let doc = parse("[[events|Bus]]{derived_from:0.7} then [[vision]] and `[[not-a-link]]`.");
    let BlockKind::Paragraph(p) = &doc.blocks[0].kind else {
        panic!("paragraph")
    };
    assert!(
        matches!(&p[0], Inline::WikiLink { target, display, relation:Some(rel), weight:Some(w) }
        if target=="events" && display=="Bus" && rel=="derived_from" && *w==0.7)
    );
    assert!(
        p.iter()
            .any(|i| matches!(i, Inline::Code(s) if s=="[[not-a-link]]"))
    );
    let malformed = parse("[[x]]{derives:NaN}");
    let BlockKind::Paragraph(p) = &malformed.blocks[0].kind else {
        panic!("paragraph")
    };
    assert!(matches!(&p[0], Inline::WikiLink { relation: None, .. }));
    assert!(Inline::plain_text(p).contains("{derives:NaN}"));
}

#[test]
fn gfm_tables_footnotes_links_and_mermaid_are_semantic_records() {
    let doc = parse(
        "| Name | Cost |\n| :--- | ---: |\n| **a** | `2` |\n\n[site](https://example.test \"Title\")[^n]\n\n[^n]: a note\n\n```mermaid\nflowchart TD\n A --> B\n```\n",
    );
    let BlockKind::Table {
        align,
        header,
        rows,
    } = &doc.blocks[0].kind
    else {
        panic!("table")
    };
    assert_eq!(align, &[TableAlign::Start, TableAlign::End]);
    assert_eq!(Inline::plain_text(&header[0]), "Name");
    assert!(matches!(&rows[0][0][0], Inline::Strong(_)));
    assert!(matches!(&rows[0][1][0], Inline::Code(s) if s=="2"));
    let BlockKind::Paragraph(p) = &doc.blocks[1].kind else {
        panic!("paragraph")
    };
    assert!(
        matches!(&p[0], Inline::Link { target, title, .. } if target=="https://example.test" && title=="Title")
    );
    assert!(p.contains(&Inline::Footnote("n".into())));
    assert!(
        doc.blocks
            .iter()
            .any(|b| matches!(&b.kind, BlockKind::Footnote { label, .. } if label=="n"))
    );
    assert!(doc.blocks.iter().any(|b| matches!(&b.kind, BlockKind::Code { info, text } if info=="mermaid" && text.contains("A --> B"))));
}

#[test]
fn malformed_metadata_and_html_remain_visible_source() {
    let empty = parse("---\n---\nBody");
    assert!(empty.frontmatter.fields.is_empty() && empty.frontmatter.error.is_none());
    let doc = parse("---\ntitle: [broken\n---\nKept body.");
    assert!(doc.frontmatter.error.is_some());
    assert!(doc.frontmatter.source.contains("[broken"));
    assert!(
        matches!(&doc.blocks[0].kind, BlockKind::Paragraph(p) if Inline::plain_text(p)=="Kept body.")
    );
    let open = parse("---\ntitle: never closed");
    assert!(open.frontmatter.error.is_some());
    assert!(!open.blocks.is_empty());
    let html = parse("<script>do_not_run()</script>\n");
    assert!(matches!(&html.blocks[0].kind, BlockKind::Html(s) if s.contains("do_not_run()")));
}

#[test]
fn metadata_errors_use_markdown_lines_including_delimiter_bom_and_crlf() {
    for (bom, newline) in [("", "\n"), ("\u{feff}", "\r\n")] {
        let source = format!(
            "{bom}---{newline}id: fox{newline}type: spec{newline}title: live seats: cockpit{newline}---{newline}Kept body."
        );
        let doc = parse(&source);
        assert!(doc.frontmatter.error.is_some());
        assert_eq!(doc.frontmatter.error_line, Some(4));
        assert_eq!(doc.source, source);
        assert!(
            matches!(&doc.blocks[0].kind, BlockKind::Paragraph(p) if Inline::plain_text(p)=="Kept body.")
        );
        let serialized = serde_json::to_string(&doc.frontmatter).unwrap();
        assert_eq!(
            serde_json::from_str::<bbt_doc::Frontmatter>(&serialized).unwrap(),
            doc.frontmatter
        );
    }
    assert_eq!(
        parse("---\ntitle: never closed").frontmatter.error_line,
        Some(1)
    );
    assert_eq!(
        parse("---\ntitle: Fox\n---\nBody").frontmatter.error_line,
        None
    );
    // Older serialized records have no location field.
    let old = r#"{"fields":{},"source":"","error":null}"#;
    assert_eq!(
        serde_json::from_str::<bbt_doc::Frontmatter>(old)
            .unwrap()
            .error_line,
        None
    );
}
#[test]
fn invalid_explicit_record_numbers_keep_their_source_and_show_a_diagnostic() {
    for value in ["0", "-1", "1.5", "'47'", "null", "true", "[]"] {
        let source = format!("---\ntitle: Kept\nnumber: {value}\n---\nRetained body.\n");
        let doc = bbt_doc::parse(&source);
        assert_eq!(doc.frontmatter.number(), None, "{value}");
        assert!(doc.frontmatter.error.as_deref().unwrap().contains("number"));
        assert_eq!(doc.frontmatter.error_line, Some(3));
        assert_eq!(doc.source, source);
        assert!(!doc.blocks.is_empty());
        assert!(doc.frontmatter.fields.contains_key("number"));
    }
    for source in ["Plain Markdown\n", "---\ntitle: Generic\n---\nBody\n"] {
        let doc = bbt_doc::parse(source);
        assert!(doc.frontmatter.error.is_none());
        assert_eq!(doc.frontmatter.number(), None);
    }
    let doc = bbt_doc::parse("---\nnumber: 47\n---\nBody\n");
    assert!(doc.frontmatter.error.is_none());
    assert_eq!(doc.frontmatter.number(), Some(47));
}
