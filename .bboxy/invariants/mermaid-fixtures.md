---
id: 01M5KQ81BHKCA8HCJDJSQFNTKA
number: 148
type: invariant
title: every .bboxy Mermaid block has a fixture the engine lays out
status: active
check: cargo test -p bboxy450-mermaid --locked --test fixtures every_bboxy_mermaid_block_has_a_fixture -- --exact
cost: a diagram the engine cannot lay out is a blank card in the viewer, and the spec it explains loses its picture
links:
  - { to: "[[d14-native-document-viewer]]", rel: decision }
  - { to: "[[document-viewer]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
created: 2026-10-23T06:30:19Z
updated: 2026-10-23T06:30:19Z
---
# Every .bboxy Mermaid block has a fixture

**Property.** Each ```mermaid block in a tracked `.bboxy` Markdown file matches,
byte for byte, a fixture `crates/bbt-mermaid/tests/fixtures/bboxy/<spec>-<n>.mmd`
that the engine parses and lays out (decision [[d14-native-document-viewer|#14]]: the subset the engine
supports is what `.bboxy` uses). A diagram edit and its fixture travel in one
commit.

**Check.** `cargo test -p bboxy450-mermaid --locked every_bboxy_mermaid_block_has_a_fixture -- --exact`
(`crates/bbt-mermaid/tests/fixtures.rs:123`) — exit 0 held, exit 1 naming the
block without a fixture. Part of `scripts/invariants.sh`.

**Cost of a violation.** A diagram the engine cannot lay out is a blank card in
the viewer, and the spec it explains loses its picture; found the hard way on
2026-10-22 when the Architect edited the D1 diagram without its fixture and
turned main red.

**Evidence.** Passed by the Architect in a clean worktree with a private target
after the Oracle refreshed `d1-providers-2.mmd` (2026-10-22).

Owner: Corvin Duskmantle <ARCHITECT>.
