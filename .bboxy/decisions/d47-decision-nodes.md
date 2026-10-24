---
id: 01M5NJ11AFK9TFTN5X78VDSW1K
type: decision
number: 47
title: "one file per decision; records are objects, the git tree is the first store"
status: decided
decided: 2026-10-23
operator:
  - "should decision be a type of document in our document system? where research or spec can have comments and decisions and notes and research all linking it together, like one big graph of thinking"
  - "yes one file per decision, its ok to create many files"
  - "about decision and comments - it can all be single file, lets treat files as objects, we can later have binary store, but for now simple on disk storage system in git works, we will make it pluggable with different storage backends maybe, but KISS for now so we can keep developing our system"
links:
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: supersedes-as-index }
  - { to: "[[provenance]]", rel: constrains }
  - { to: "[[document-viewer]]", rel: consumer }
created: 2026-10-23T23:37:35Z
updated: 2026-10-24T00:24:57Z
---
# Decision 47: one file per decision; records are objects, the git tree is the first store

Decided by the operator, 2026-10-23, from the Architect's reading; the Oracle added "keep old links resolvable (leave an index file with anchors)". This file is the first decision written as its own node.

**Alternatives.** (a) Keep the single ledger `[[2026-10-22-architecture-round-1]]` with numbered paragraphs, as practised for [[d1-crate-topology|#1]] to [[d46-cross-os-facades|#46]]. (b) One file per decision from [[d47-decision-nodes|#47]] on, the first 46 left in the ledger. (c) One file per decision, all 47, the ledger kept as the index with an anchor per number so every old link still lands. (d) Decisions written inside the spec they decide.

**Choice.** (c).

**Reason.** The `decision` type existed in [[work-items]] but a decision was not a node: a wikilink could only reach the ledger file, a comment could not be `on:` one decision at one digest, and tonight's amendments to [[d35-alive-rim|#35]], [[d38-messaging-task-rulings|#38]] and [[d40-scrollback-bounds|#40]] grew sub-items a through i because an entry had nowhere else to grow. As files, decisions join the graph the viewer and the Graph tab already draw: the operator's words, the decision, the spec section it `decides`, the invariant it `constrains`, the research that is `evidence`, the comment it `answers`, the decision it `amends` or `supersedes`, and the run that proved it. (b) leaves the first 46 as a blob forever; (d) breaks when one decision touches three specs, which most do; (a) is the problem.

**Records are objects (the operator's storage rule).** A record is one file: one decision, one comment, one research record, one spec, one task. The file is the object and the git tree is the store, the first and for now the only storage backend. The schema never assumes a path: identity is the ULID `id`, addressing is the slug or the id, versions are content digests, and relations are links by slug. A binary or remote store can hold the same objects later behind the same reads and writes; **nothing pluggable is built now** (KISS, "so we can keep developing our system").

**The decision node.** Frontmatter `type: decision`, `number`, `title`, `status` (`decided | amended | superseded`), `decided` (date), `operator` (a list of the verbatim words that caused it, when there are any), `links` with the relations `decides` (a spec or a section), `constrains` (an invariant), `evidence` (research, runs), `answers` (a comment), `amends` and `supersedes` (another decision), `supersedes-as-index` for the ledger. Body: alternatives, choice, reason, as before; sub-rulings (a), (b), … stay inside the node they refine until one of them changes the choice, which is a new node with `amends`. File name `d<N>-<slug>.md`, so `[[d47-decision-nodes]]`. Numbering is unchanged and still global.

**The sweep (an Executor assignment, briefed by the Oracle from this node).** Split entries 1 to 46 of the ledger into `d<N>-<slug>.md` nodes with the frontmatter above, each carrying the operator's words and the sub-rulings that entry holds today; replace each ledger entry with a one-line pointer under an anchor heading `## 47` style so `[[2026-10-22-architecture-round-1#N]]` keeps resolving; rewrite every `decision #N` and `#N` reference in `.bboxy/` (133 occurrences across specs, invariants, tasks, comments and research at this commit) as `[[dN-slug]]` with the number kept in the text; add `[[dN-slug]]` to the `links` of every spec and invariant that cites it. Acceptance: `cargo test -p bbt-doc --locked --test corpus` green; `grep -rnE 'decision #[0-9]+' .bboxy/specs .bboxy/invariants` returns nothing; every `[[d<N>-…]]` whose target is a record of type `decision` resolves to a node whose `number` is N (a spec whose slug happens to start with `d1-`, such as `d1-providers`, is an ordinary slug link and keeps its name; Oracle and Architect, 2026-10-23, from Velvet's validator stop); every other wikilink resolves; the ledger's anchors resolve; `sh scripts/records-push.sh` is the only path to main. Verified by the Architect against this node.

**Amended by [[d48-every-record-has-a-number]]:** the sweep also assigns a `number` to every existing record and teaches `[[#N]]` to the viewer.

**What does not change.** The numbering, the RFC loop ([[work-items]]), comments pinned by `on_digest`, "the spec signs off, not the agent", signed commits as the provenance of every record.

Owner: Corvin Duskmantle <ARCHITECT>.
