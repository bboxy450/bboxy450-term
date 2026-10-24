---
id: 01M5NJETTJ655HDRTBVXPR4H2B
type: decision
number: 48
title: "every record carries a number, one global sequence continuing from the decisions"
status: decided
decided: 2026-10-23
operator:
  - "everything needs to have a # to it"
links:
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[d47-decision-nodes]]", rel: amends }
  - { to: "[[document-viewer]]", rel: consumer }
created: 2026-10-23T23:45:07Z
updated: "2026-10-24T02:17:51Z"
---
# Decision 48: every record carries a number

Decided by the operator, 2026-10-23, the minute after [[d47-decision-nodes]]: "everything needs to have a # to it". This decision is the first object allocated under its own rule.

**Alternatives.** (a) A number per type with a letter prefix (D47, S12, C203). (b) One global sequence for every record, with the decisions keeping 1 to 47 as the first 47 handles and everything after continuing from 48. (c) A global sequence that restarts the decisions too.

**Choice.** (b).

**Reason.** A `#` is a handle you can type and say: "[[d52-brand-and-face|#52]]" in a message, a commit, a comment, a brief. The ULID stays the identity and the slug stays the file, but neither can be spoken. The decisions are the only numbered records today and are cited by number everywhere, so the sequence starts where they stop and nothing renumbers; (c) breaks every citation, (a) makes the reader learn an alphabet and still collides with the decisions' bare numbers. One counter is how issues and pull requests already work in every forge the operator uses; the type is read from the record, and in prose we keep writing "decision [[d52-brand-and-face|#52]]" or "comment [[d53-evolve-and-restart|#53]]" as we do now.

**The rule.** Every record in `.bboxy/` carries `number: N` in frontmatter, unique across all types, allocated at creation as one more than the highest number in the tree (`scripts/next-number.sh` prints it; a collision between two writers surfaces at the records push, where the corpus test requires every number to be present and unique, and the later writer takes the next free one). The number is a handle, not a timestamp: records that existed before this rule are numbered by the sweep in `created` order from the next free number at sweep time (49 went to the next decision the same evening), so an old spec may carry a higher number than a newer decision, and nobody reads order into it. `[[#N]]` is a wikilink that resolves to the record whose `number` is N, beside `[[slug]]`; the viewer shows `#N` with the type chip on every record and in every link preview; messaging and the task tools accept `#N` wherever they accept a slug. File names do not change: the slug is the file, the number lives inside it ([[d47-decision-nodes]], files are objects).

**The sweep (folds into the one briefed from [[d47-decision-nodes]]).** Assign `number` to every existing record (154 files under `.bboxy/` at this commit, minus the two decision nodes already numbered) in `created` order starting at the next free number; add `scripts/next-number.sh`; extend the corpus test so a missing or duplicate `number` fails; teach `[[#N]]` to `bbt-doc`'s link resolver and the Graph tab. Acceptance: corpus green with the new check; `sh scripts/next-number.sh` prints max + 1; `[[#47]]` opens this folder's `d47-decision-nodes.md` in the viewer; the ledger's pointers read `47. [[#47]] …`.

**Runtime allocation (confirmed for NUM2, 2026-10-24, Velvet's reading).** The task tools and every other writer allocate at the Index's signed-write boundary: take the store's lock (one lock per `.bboxy` root, held from reading the maximum through writing the file, its signature and the atomic publication), read the fresh maximum across the `number` of every on-disk Markdown record of every type (an empty tree starts at 1), write max + 1, release. No separate counter anywhere; an external Git or editor collision is caught by the corpus test at the push, never by a runtime guess. A duplicate or invalid number is refused on write; an existing record keeps its number on every update and retry; a legacy record with an invalid number is refused for update and shown as broken in every reader, never skipped silently ([[work-items]] Index rule). A number once allocated is never reissued: records are never deleted ([[mantras]] 22), so the maximum only grows; an external removal that would let a number be reused is a store violation, visible in history, not a case the allocator handles. Numeric selectors (`#N`) resolve to exactly one record before any task or authority check runs; seat ids, operation ids and paging cursors stay ULIDs, the number is a handle for people.

**What does not change.** ULIDs, slugs, file names, the RFC loop, digests as versions.

Owner: Corvin Duskmantle <ARCHITECT>.
