// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_doc::{StreamingMarkdown, parse};
const CORPUS: &[&str] = &[
    "# Heading\n\nText **bold**, _italic_, é🙂.\n\n```rust\nfn main() {\n    println!(\"ok\");\n}\n```\n\nTail\n",
    "| name | value |\n| :--- | ---: |\n| a | b |\n| c | d |\n\nAfter\n",
    "---\nnumber: 7\ntitle: test\n---\n\n# Title\n\nText\n",
    "[late][r] and [^f]\n\nfirst\n\n[r]: https://example.com\n[^f]: footnote\n",
    "one\n---\n\n    indented\n    code\n\n# Next\n\n~~~py\nprint(1)\n~~~~\n",
    "- item\n\n  continuation\n\n- next\n\n> quote\n>\n> body\n\nend\n",
    "\u{feff}---\r\nnumber: 8\r\n---\r\n\r\n# BOM\r\n\r\nbody\r\n",
    "<div>\nraw\n</div>\n\nNext\n\n***\n\nlast\n",
    "[link][r]\n\n# Stable\n\nBefore\n\n> [r]: u\n\nAfter [link][r]\n",
    "[link][r]\n\n# Stable\n\nBefore\n\n- [r]: u\n\nAfter [link][r]\n",
    "footnote [^f]\n\n# Stable\n\nBefore\n\n- [^f]: x\n\nAfter [^f]\n",
    "[link][foo bar]\n\n# Stable\n\nBefore\n\n[foo\nbar]: u\n\nAfter [link][foo bar]\n",
];
#[test]
fn quoted_definition_seed_resolves_a_retained_prefix() {
    let mut s = StreamingMarkdown::default();
    s.append("[link][r]\n\n# Stable\n\nBefore\n\nTail\n");
    assert!(s.stable_blocks() > 0);
    s.append("\n> [r]: u\n\nAfter\n");
    assert_eq!(s.document(), &parse(&s.document().source));
}
#[test]
fn a_possible_multiline_label_can_close_without_disabling_prefix_reuse() {
    let mut s = StreamingMarkdown::default();
    s.append("# Stable\n\nBefore\n\n[open\n");
    assert_eq!(s.reparsed_range().start, 0);
    s.append("label](url)\n\nAfter\n");
    assert_eq!(s.document(), &parse(&s.document().source));
    s.append("\nTail\n");
    assert!(s.reparsed_range().start > 0);
    assert_eq!(s.document(), &parse(&s.document().source));
}
// Property witness: every prefix, every single split, and reproducible random
// multi-append partitions, including every byte boundary inside fences/tables.
#[test]
fn incremental_equals_full_at_every_append_and_random_split() {
    for source in CORPUS {
        let boundaries: Vec<_> = source
            .char_indices()
            .map(|(i, _)| i)
            .chain([source.len()])
            .collect();
        for &split in &boundaries {
            let mut stream = StreamingMarkdown::default();
            assert_eq!(stream.append(&source[..split]), &parse(&source[..split]));
            assert_eq!(
                stream.append(&source[split..]),
                &parse(source),
                "split {split} {source:?}"
            );
        }
        for seed in 1..=64_u64 {
            let mut rng = seed;
            let mut at = 0;
            let mut stream = StreamingMarkdown::default();
            while at + 1 < boundaries.len() {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let next = (at + 1 + (rng >> 32) as usize % 9).min(boundaries.len() - 1);
                let end = boundaries[next];
                let got = stream.append(&source[boundaries[at]..end]);
                assert_eq!(
                    got,
                    &parse(&source[..end]),
                    "seed {seed} end {end} {source:?}"
                );
                at = next;
            }
        }
    }
}
#[test]
fn stable_prefix_is_retained_and_only_open_tail_is_parsed() {
    let mut s = StreamingMarkdown::default();
    s.append("# Completed\n\nFirst paragraph\n\n```rust\nlet n = 1;");
    let prefix = s.document().blocks[..2].to_vec();
    assert_eq!(s.stable_blocks(), 2);
    let start = s.document().source.find("```rust").unwrap();
    s.append("\n```\n\nTail");
    assert_eq!(s.reparsed_range().start, start);
    assert_eq!(s.document().blocks[..2], prefix);
    assert_eq!(s.document(), &parse(&s.document().source));
    // Broken cache control: freezing an EOF paragraph before a table delimiter
    // does not equal a full parse. The production cache retains this open tail.
    assert_ne!(
        parse("| a | b |\n").blocks[0].kind,
        parse("| a | b |\n|---|---|\n").blocks[0].kind
    );
}

#[test]
fn generated_block_mixtures_match_full_parse_after_each_scalar_append() {
    let pieces = [
        "plain\n",
        "\n",
        "# title\n",
        "- item\n",
        "> quote\n",
        "    code\n",
        "```rust\n",
        "```\n",
        "~~~\n",
        "| a | b |\n",
        "|---|---|\n",
        "---\n",
        "é🙂\n",
        "<div>\n",
        "</div>\n",
        "[x](https://example.com)\n",
        "`a[i]` and `vec![]`\n",
        "- [ ] task\n",
        "*em* ",
        "\t",
        "  ",
    ];
    let mut incremental_seeds = 0;
    for seed in 1..=96_u64 {
        let mut rng = seed;
        let mut source = "## Stable\n\n[inline](url), `a[i]` and `vec![]`.\n\n".to_string();
        for _ in 0..12 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            source.push_str(pieces[(rng >> 32) as usize % pieces.len()]);
        }
        // Seven out of eight seeds contain no definitions: ordinary brackets
        // must exercise prefix reuse. Dedicated seeds still cover late binding.
        if seed % 8 == 0 {
            source.push_str("\n\n[x][ref] and [^f]\n\n");
            source.push_str(match seed % 32 {
                0 => "> [ref]: https://example.com\n",
                8 => "- [ref]: https://example.com\n",
                16 => "- [^f]: footnote\n",
                _ => "[multi\nline]: https://example.com\n[^f]: footnote\n",
            });
        }
        let mut stream = StreamingMarkdown::default();
        let mut prefix = String::new();
        let mut retained_appends = 0;
        for ch in source.chars() {
            let part = ch.to_string();
            prefix.push(ch);
            assert_eq!(
                stream.append(&part),
                &parse(&prefix),
                "seed {seed} prefix {prefix:?}"
            );
            retained_appends += usize::from(stream.reparsed_range().start > 0);
        }
        incremental_seeds += usize::from(retained_appends * 2 > source.chars().count());
    }
    assert!(
        incremental_seeds >= 80,
        "only {incremental_seeds}/96 seeds primarily used prefix reuse"
    );
    eprintln!("generated incremental coverage: {incremental_seeds}/96 seeds");
}

#[test]
fn ordinary_brackets_reparse_only_the_open_tail_and_count_bytes() {
    let mut stream = StreamingMarkdown::default();
    let mut parsed_bytes = 0;
    let mut full_bytes = 0;
    for i in 0..100 {
        let delta = format!(
            "## Section {i}\n\n[link](https://example.com) and `a[i]`, `vec![]`, {{\"a\": []}}.\n\n- [ ] task\n- [x] done\n\n```rust\nlet a = vec![1, 2];\nlet n = a[0];\n```\n\nTail {i}\n\n"
        );
        let before = stream.document().source.len();
        stream.append(&delta);
        let range = stream.reparsed_range();
        parsed_bytes += range.len();
        full_bytes += stream.document().source.len();
        if i > 0 {
            assert!(
                range.start >= before.saturating_sub(32),
                "lost prefix at append {i}: {range:?}"
            );
        }
        assert_eq!(stream.document(), &parse(&stream.document().source));
    }
    assert!(
        parsed_bytes * 10 < full_bytes,
        "tail {parsed_bytes} versus full {full_bytes}"
    );
    eprintln!("ordinary brackets: reparsed={parsed_bytes}, full={full_bytes}");
    stream.append("");
    assert!(stream.reparsed_range().is_empty());
}
