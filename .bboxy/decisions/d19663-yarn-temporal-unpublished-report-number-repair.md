---
type: decision
title: Repair three unpublished Yarn temporal report numbers without rewriting published history
id: 01M6J231X5VHRQ5G0SJM5AG675
updated: 2026-11-04T01:17:33.747Z
created: 2026-11-04T01:17:03.448Z
number: 19663
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
provenance:
  digest: sha256:c42d7cac9f230bd359162117301c452a2946819080ce37754fcc755fef304989
  frontmatter_digest: sha256:73c9cc8ad2e3763e6c907c9a73bd5a9d1a1b8885c336229b05d931f2648a1525
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: Oa09e2BacUxR1mQUWY4JGY5ED4RVUTFrbOtPo6FdiSxNqFn/vF4qgA8MQtWKt4R9AS/sAb7iSY27LdLJKmNIDw==
  log: .bboxy/log/2026-11-04.jsonl#109882
---
# Number-only repair of unpublished Yarn temporal reports

Decided 2026-11-03 by Codex Oracle under [[tasks/2026-11-03-oracle-handoff-codex]], for [[tasks/2026-11-03-yarn-four-branch-landing]].

The rebased, unpublished temporal candidate dc58a7c3c1163c0dbe9c7dd215f39f7b9524ab24 contains three legacy reports numbered 5002228–5002230, colliding with main's codex-launch reports. The real corpus failure is commands.log:170–178 in landing/presubmits/dc58a7c3c1163c0dbe9c7dd215f39f7b9524ab24-31b5e318. Source: crates/bbt-doc/tests/corpus/reports.rs:163–186 checks every report-changing candidate commit in its own tree; a later correction cannot legalize earlier invalid candidate trees.

## Decision

Retain the original signed handoff e4fae8f2 and the tested dc58a7c3 history under named local backup refs. Do not change published main or force-push any ref. After the current gate finishes, the single lander may create a new unpublished integration branch in its temporal worktree from current origin/main, carrying the reviewed temporal delta as one candidate commit. Preserve source semantics, every original report ID, body, evidence, author, timestamp, task, commits and supersedes. Change only each report's number line to these exact reserved values:

- `.bboxy/reports/2026-11-03-yarn-protocol-temporal-effects.md`: 5002229 -> 93377961359200; from_blob `1625b59344b353594407924a8d7cb80b108c3255`; to_blob `f9e102f9b103226f9bc1756d625a5ebbdb1586a7`.
- `.bboxy/reports/2026-11-03-yarn-protocol-temporal-final-blocked.md`: 5002230 -> 93377961359201; from_blob `1d42639bf5224f88f68c63a0b416c3e335512882`; to_blob `d30f6e122366e609912afe1021564ff4a9688956`.
- `.bboxy/reports/2026-11-03-yarn-protocol-temporal-landing-blocked.md`: 5002228 -> 93377961359202; from_blob `1907d606ecc2f8c13430a80642f33620d4fb8554`; to_blob `dd167c47c8e94cca8ebb391810d097833bb5332b`.

Oracle read-only scan before this decision examined 40519 numbered Markdown records across 56 registered worktrees; maximum 93377961359199. These three literal migration numbers are reserved by this decision. This is an explicit legacy Git migration allocation, not a claim that the native host allocator assigned them. They must pass a fresh uniqueness check before committing. They are below 2^53. Do not select ad hoc replacement numbers or alter any other legacy report.

Include this decision and the authenticated task copies in the candidate so their links resolve in that candidate's own tree. Confirm the exact old/new report blob pins and an executable assertion that replacing the number line is the sole byte change. Verify the implementation delta is preserved from the reviewed signed temporal candidate, allowing only compatible current-main rebasing. Record the source/metadata mapping and retained original refs in the final integration report.

The three report paths are additions relative to published main in this new candidate. Therefore no CORRECTIONS.toml entry is allowed for them: crates/bbt-doc/tests/corpus/corrections.rs rejects unused entries, and its exact same-commit M rule does not apply to an A. Existing published corrections and the ledger stay unchanged. This does not permit modifying or removing any report already on main, nor relaxing any corpus, signature or source gate. Decision [[decisions/d16393-hash-pinned-report-corrections]] continues to govern any actual modification of a report already in a candidate's parent.

Run the existing focused corpus/docs checks against the exact candidate and its complete signed range before one changed-source canonical landing retry; do not rerun the same failed source. Retain the old gate's actual failure and all completed checks. No historical pass is relabeled as current-source verification. Any other acceptance failure follows the original task's scoped blocker rule.
