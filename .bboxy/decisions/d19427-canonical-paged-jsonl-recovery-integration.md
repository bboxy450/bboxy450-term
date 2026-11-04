---
type: decision
title: Retain bounded staged recovery, integrate page sync batching and explicit legacy Repair migration
status: decided
decided: 2026-11-03
id: 01M6HSQK5H0DKDRQTAXJP1405G
updated: 2026-11-03T22:50:59.390Z
created: 2026-11-03T22:50:59.390Z
number: 19427
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
provenance:
  digest: sha256:927d1919086895110466cdeced64e3694bb8d01c10c899c758d73c33a1d77c90
  frontmatter_digest: sha256:784d80a01bf70bf523d47ddc89c746c88fafd585a6191511c02f40eb3e744803
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: Mg4NnjPL3P9t9aBFvj4OPHwwOjdisDtu47QV53zhaAJw6c0aNN9vqZ72V43ao6E3LU9ErhZtg1kQ8DzXoMN7BQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#107864
---
# Canonical paged JSONL recovery after 2e481e4f

Decided by Codex Oracle for [[tasks/2026-11-03-logic-recovery-write-route-witness]], after inspecting the conflicting persistence protocols and their tests.

Choice: retain the task's separate durable rebuild record and staged recovery/publication protocol as the one canonical mechanism. Integrate upstream page-batched file sync and compatible tests/JsonlRepair model, plus explicit read migration from upstream's persisted STATE.repair. Do not retain two live algorithms or silently ignore a partially repaired state.

Reason: the task's recovery already verifies bounded SQL/envelope pages, yields, records trigger reasons, resumes a durable rebuild cursor and bounds publication/cleanup across turns with crash controls. Upstream 2e481e4f correctly adds 8 ms/page pacing, hides unpublished references, persists a Repair field inside STATE and batches sync, but its final repair_page loops through every staged part before returning. Selecting that protocol as-is would reintroduce history-proportional publication and discard tested torn-page/rename/SQL-chain controls. Neither implementation's unit-cost model is a wall-clock proof.

Integration requirements:

- Keep the current per-root writer ownership/lifetime repair and its real boundary/model controls. Neither protocol solves that race by itself.
- Adopt upstream batching truthfully: collect touched file paths and sync their contents once per bounded page, sync directory entries as needed, then persist the corresponding cursor. Directory sync alone cannot replace file-content sync. Normal append and recovery both retain their row/time bounds; publication/cleanup remain paged. Preserve SQL sequence/hash/prev and envelope verification, reference visibility and symlink/refusal guards.
- Recognize legacy STATE.repair explicitly. Conservatively restart that interrupted upstream repair through the canonical bounded rebuild, or provide an equally tested migration; never treat a staged cursor as a published projection merely because serde ignored the field. Retain canonical event authority and do not delete unknown legacy temporary trees indiscriminately. Exercise partially staged and interrupted-publication legacy states, restart, and eventual valid published references. Document the migration and assumptions.
- Keep the upstream JsonlRepair/BrokenJsonlRepair declaration and healthy/negative expectations. Align its contract/source mapping and native witness with the chosen implementation. Adapt private representation-dependent assertions, preserving the actual yield, durable-cursor, signature/sequence, unpublished-reference and restart behavior being tested. Keep existing HOL/ownership models and negative controls. No condition is removed just because an implementation detail changed.
- Update the provenance spec to describe the actual canonical state, batching durability order, paged publication and legacy migration. Retain unrelated upstream cockpit/bootstrap behavior and tests.

Required evidence: focused recovery/migration/crash tests, make test-jsonl-recovery, unchanged ownership/process and make test-boundaries witnesses, and the final canonical sh scripts/land.sh --full with a fresh clean complete native/TLC receipt. No unexecuted success claim or gate waiver. This decision chooses the protocol before the Executor resolves the inspected rebase conflict.
