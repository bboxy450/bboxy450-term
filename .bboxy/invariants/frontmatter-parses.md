---
id: 01M5JVQ5PH7488DHH8WKDK8F19
type: invariant
title: every .bboxy frontmatter parses
status: active
check: python3 -I scripts/frontmatter.py
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

**Check.** `python3 -I scripts/frontmatter.py` — standard library only — exit 0
held, exit 1 with `path:line` per error. Part of `scripts/invariants.sh`.
Replaced by the bbt-doc corpus test over `.bboxy/` when W3.5 lands, so the
reader and the gate parse with one parser.

**Cost of a violation.** A record the reader cannot parse is invisible to the
index, the viewer and every graph built on them; a task nobody can see is a
task nobody does.

**Evidence.** `python3 -I scripts/frontmatter.py — exit 0` after quoting the two
titles (2026-10-22).

Owner: Corvin Duskmantle <ARCHITECT>.
