---
type: note
title: 'Den feedback from its first outside use: what must change (2026-11-14)'
id: 01M7ED0ZM9SMVVFQMYP8AAWRYJ
updated: 2026-11-15T01:26:59.982Z
created: 2026-11-15T01:26:59.982Z
number: 1794292861627509
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:739886ddc49d198532ba9228ad3804d0a654d99d2735dcaec66d789e691fd862
  frontmatter_digest: sha256:58d4c6af824e340198e7c74fdd2478ffb8ae1478615f9800b8f687ecb6a28a20
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: iJDa6WG8Jvo33aRtUOLk9vytM4ugJcLC+oBHRZyaKwvJn7G9pP/m6WreXDJS9YZyvQB1R/f6iCbB6gE4HC81Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151683
  event:
    id: 01M7ED17MQ70J57GEBXVBKJ0Y6
    sequence: 151683
    hash: fb49e232a58dd20c14d273c1e644ef827f33fc85a03a1ed80bc27d90fc26d27d
---
# Den feedback from its first outside use: what must change

Source: a harness seat that wrote a game design in Den on 2026-11-14 (concern `void-tarot-game`, another repository): 14 specs, 1,033 lines, `bboxy450-den check .` exit 0, three deliberately broken copies each refused (X601, X801, X601). Checker built from this repository at `ba2ba219`. The operator sent the report here three times with: "crucial feedback from harness that tried to use the .den ... make sure you adopt this".

This note is the adoption list. Every row is owed; none is optional. Direction: [[d1794292861627507-rust-lean-tla-first-tools]], [[spec-language]].

## What the seat hit

**Finding the language**

- Nothing in a seat points at Den: `knowledge_orient`, `knowledge_types` and the pinned skills never mention it. An earlier seat searched, found nothing, and stopped to ask what ".den" meant.
- No checker in reach: the only binary was a stale debug build inside this repository's `target/`.
- The knowledge tools refuse `.den`: the specs were written as plain files with no identity, signature or provenance; `knowledge_search` cannot see them and `knowledge_commit` cannot checkpoint them.
- The README was enough to write from: ten of fourteen files passed on the first run.

**Syntax and diagnostics**

- Reserved words are undocumented. `by` as a field name and `Path` as an enum name were refused with X190, whose repair text ("use a standalone spec with Bool, bounded Int, enum and Set[Enum] types") pointed at a type error. `done` is reserved yet accepted in `case done if`.
- One diagnostic per run for a whole directory: `check .` over six files printed one X190 and stopped.
- Success prints nothing: fourteen sound files look like zero files found.
- Counterexample traces print raw values: `Gain([Scalar(0)]) -> Speak([Scalar(1)])` should read `Gain(Element.Fire) -> Speak(Element.Water)`; X801 prints state with no field names.
- Guards cannot be named, so a condition and its negation are written out twice; a typo in one copy becomes an overlap (X401) or a hole (X402) far from its cause.

**Fit for design work**

Worked: totality forces every gap into the open; reachability is a free playthrough check; scenarios read as worked examples; invariants state intent precisely; `render --markdown` gives a readable page per spec.

Fought back:

- No composition: `include` is X190, imports are parsed but not resolved, an unused import is X203. `enum Element` was declared four times in sibling files with nothing checking the copies agree.
- The 65,536 bound is on the whole state universe times one event's arguments, not on reachable states; one natural machine became two.
- No constants, functions or tables: a lookup table is an event with a case per row (22 cases for the trumps).
- No multiplication or division.
- Every spec needs a falsifiable invariant, so a field was added mainly to have something to assert.
- Counting is expensive, and nothing separates a design number from a checking bound.
- No home for prose-only design: tone, a rejected idea and open questions have only `intent`, `assume` and comments.

## Adoption list

| # | Change | Where | State |
|---|---|---|---|
| 1 | Seats are told Den exists: `knowledge_orient`, the document-type dictionary and a `den` skill name the format, the checker and where specs live | mux knowledge guidance, `.bboxy/skills/den` | owed |
| 2 | The checker is in reach: `den_check` in the tool catalogue | `bboxy_code` | built, landing |
| 3 | `knowledge_write`, `knowledge_fetch`, `knowledge_search` and `knowledge_commit` accept `.den`; records have identity, signature and provenance | mux index and knowledge | owed |
| 4 | Imports resolve: an enum is declared once and used by sibling specs; an unresolved import says which file it looked for | den crate | owed |
| 5 | Traces and observed state print names (`Element.Fire`, field names), never `Scalar(n)` | den crate | owed |
| 6 | A clean run prints one summary line with the count of sound specs; `--json` unchanged | den CLI | owed |
| 7 | Diagnostics are reported per file: one failing file does not hide the others | den CLI | owed |
| 8 | Reserved words are listed in the README; using one says "`by` is a reserved word here; rename it"; `Path` and other accidental refusals are fixed or listed | den crate | owed |
| 9 | `open ID "text"` declares an undecided question, is carried into the rendered page, and is never an error | den crate | owed |
| 10 | `source` links a rule to the knowledge record it came from | den crate | owed |
| 11 | `define NAME = EXPR` at spec level, usable in guards, so a condition is written once | den crate | owed |
| 12 | `table NAME: ENUM -> ENUM { ... }` usable in expressions | den crate | owed |
| 13 | A bound can be marked as a model bound, distinct from a design number, and the rendered page says which is which | den crate | owed, needs design |
| 14 | Multiplication and division on bounded integers | den crate, checker | owed, needs design |
| 15 | The size bound counts reachable states, not the declared universe | checker | owed, needs design |
| 16 | One spec's effect binds to another spec's event (composition beyond shared enums) | den crate | owed, needs design |

Rows 13 to 16 change what the checker explores or what a spec means, so each gets its own decision before code.

## Check

A row is done when the seat's own failing case passes: the fourteen design specs still check, the four copies of `enum Element` collapse into one import, a broken copy prints names in its trace, and a seat that has never heard of Den finds it from `knowledge_orient` alone.
