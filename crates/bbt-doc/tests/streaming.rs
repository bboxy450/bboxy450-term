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
];
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
        "[x][ref]\n",
        "[ref]: https://example.com\n",
        "*em* ",
        "\t",
        "  ",
    ];
    for seed in 1..=96_u64 {
        let mut rng = seed;
        let mut source = String::new();
        for _ in 0..12 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            source.push_str(pieces[(rng >> 32) as usize % pieces.len()]);
        }
        let mut stream = StreamingMarkdown::default();
        let mut prefix = String::new();
        for ch in source.chars() {
            let part = ch.to_string();
            prefix.push(ch);
            assert_eq!(
                stream.append(&part),
                &parse(&prefix),
                "seed {seed} prefix {prefix:?}"
            );
        }
    }
}
