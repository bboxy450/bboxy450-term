---
id: 01M5JVQ5PH7488DHH8WKDK8F19
type: invariant
title: every .bboxy frontmatter parses
status: active
check: cargo test -p bbt-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact
cost: a record the reader cannot parse is invisible to the index, the viewer and every graph built on them
links:
  - { to: "[[work-items]]", rel: constrains }
  - { to: "[[document-viewer]]", rel: constrains }
created: 2026-10-22T22:29:15Z
updated: 2026-10-22T22:29:15Z
---
# Every .bboxy frontmatter parses

**Property.** Every Markdown file under `.bboxy/` that opens with `---` closes
it, carries `id:` and `type:`, and every line is `key: value` or `- item` with
no unquoted scalar containing `: ` or ` #` (the two ways a plain YAML value
turns into a mapping or a comment). Found by the operator's own document
reader on 2026-10-22 in two spec titles; the records are the product's data,
so a parse failure is a product bug.

**Check.** `cargo test -p bbt-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact`
(`crates/bbt-doc/tests/corpus.rs:53`): the document reader's own parser walks
every tracked Markdown file under `.bboxy/` and fails with `path:line` on a
frontmatter it cannot parse or that lacks `id:` or `type:`. Exit 0 held. Part of
`scripts/invariants.sh`. One parser for the reader and the gate (W3.5b,
2026-10-22); the stand-in `scripts/frontmatter.py` retired the same day.

**Cost of a violation.** A record the reader cannot parse is invisible to the
index, the viewer and every graph built on them; a task nobody can see is a
task nobody does.

**Evidence.** The switched check run by the Architect in a clean worktree with a private target: see the commit that made the switch; Nyx's recorded run in `.bboxy/runs/w3-5b-native-figures.log`: 88 files, 0 errors.

Owner: Corvin Duskmantle <ARCHITECT>.
