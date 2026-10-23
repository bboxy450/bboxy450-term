---
id: 01M5NJETTJ655HDRTBVXPR4H2B
type: decision
number: 48
title: every record carries a number, one global sequence continuing from the decisions
status: decided
decided: 2026-10-23
operator:
  - "everything needs to have a # to it"
links:
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[d47-decision-nodes]]", rel: amends }
  - { to: "[[document-viewer]]", rel: consumer }
created: 2026-10-23T23:45:07Z
updated: 2026-10-23T23:45:07Z
---
# Decision 48: every record carries a number

Decided by the operator, 2026-10-23, the minute after [[d47-decision-nodes]]: "everything needs to have a # to it". This decision is the first object allocated under its own rule.

**Alternatives.** (a) A number per type with a letter prefix (D47, S12, C203). (b) One global sequence for every record, with the decisions keeping 1 to 47 as the first 47 handles and everything after continuing from 48. (c) A global sequence that restarts the decisions too.

**Choice.** (b).

**Reason.** A `#` is a handle you can type and say: "#52" in a message, a commit, a comment, a brief. The ULID stays the identity and the slug stays the file, but neither can be spoken. The decisions are the only numbered records today and are cited by number everywhere, so the sequence starts where they stop and nothing renumbers; (c) breaks every citation, (a) makes the reader learn an alphabet and still collides with the decisions' bare numbers. One counter is how issues and pull requests already work in every forge the operator uses; the type is read from the record, and in prose we keep writing "decision #52" or "comment #53" as we do now.

**The rule.** Every record in `.bboxy/` carries `number: N` in frontmatter, unique across all types, allocated at creation as one more than the highest number in the tree (`scripts/next-number.sh` prints it; a collision between two writers surfaces at the records push, where the corpus test requires every number to be present and unique, and the later writer takes the next free one). The number is a handle, not a timestamp: records that existed before this rule are numbered by the sweep in `created` order from 49, so an old spec may carry a higher number than a newer decision, and nobody reads order into it. `[[#N]]` is a wikilink that resolves to the record whose `number` is N, beside `[[slug]]`; the viewer shows `#N` with the type chip on every record and in every link preview; messaging and the task tools accept `#N` wherever they accept a slug. File names do not change: the slug is the file, the number lives inside it ([[d47-decision-nodes]], files are objects).

**The sweep (folds into the one briefed from [[d47-decision-nodes]]).** Assign `number` to every existing record (154 files under `.bboxy/` at this commit, minus the two decision nodes already numbered) in `created` order starting at 49; add `scripts/next-number.sh`; extend the corpus test so a missing or duplicate `number` fails; teach `[[#N]]` to `bbt-doc`'s link resolver and the Graph tab. Acceptance: corpus green with the new check; `sh scripts/next-number.sh` prints max + 1; `[[#47]]` opens this folder's `d47-decision-nodes.md` in the viewer; the ledger's pointers read `47. [[#47]] …`.

**What does not change.** ULIDs, slugs, file names, the RFC loop, digests as versions.

Owner: Corvin Duskmantle <ARCHITECT>.
