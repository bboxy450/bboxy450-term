---
id: 01M5V9RQM5V6MDD2M9N2DZ9C4X
type: decision
number: 341
title: "three wastes cut: raw logs stay out of the record, which keeps a hash, a command and a short summary; a contract and its first implementation land together when one author does both and the golden proves old bytes; handoffs are at most eight lines with detail in the evidence file"
status: decided
decided: "2026-10-25"
operator:
  - "investigate the work in progress, make sure we are on a right track"
  - "make sure we are doing things in an optimal way"
links:
  - { to: "[[landing]]", rel: amends }
  - { to: "[[resource-ledger]]", rel: builds-on }
  - { to: "[[d321-out-of-integration-hell]]", rel: amends }
created: "2026-10-26T05:08:41Z"
updated: "2026-10-26T05:08:41Z"
---
# Decision 341: three wastes cut

Ruled at the Oracle's three findings, on the operator's words quoted above. All three proposals taken as made.

1. **Evidence volume.** On 2026-10-25 `.bboxy/runs` gained 233 767 lines in 340 files against 31 112 lines of crate code: seven and a half times more log than code, committed, re-read by agents and pushed through the records gate. **Raw logs stay in the landing state directory** (and the scratch directories of whoever ran them); the record keeps, per run, the command, its exit, the sha256 of the raw log, and a summary of five to twenty lines. The night's daily snapshot of the landing logs ([[landing]] §0a) follows the same rule: counts and outliers, not the logs. Existing committed logs stay as history; nothing is deleted.
2. **Contract-first landing.** Landing every contract alone and first doubled or tripled the landings per feature on three slots, each five to thirty minutes. **A contract and its first implementation may land as one cut** when the same author does both and the wire golden proves every old byte unchanged; a contract lands alone and first only when other agents need it before the implementation is ready.
3. **Handoff length.** Executor handoffs ran 1 500 to 3 000 characters, each a full wake for the Oracle, with inference dominated by re-reading. **A handoff is at most eight lines**: the sha and how it landed, the verdict, the test counts, the evidence path, and anything left undone; every other detail lives in the evidence file the handoff points at.

Owner: Corvin Duskmantle <ARCHITECT>.
