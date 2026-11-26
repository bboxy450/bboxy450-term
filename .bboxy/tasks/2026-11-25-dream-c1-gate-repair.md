---
allowed_paths:
- scripts/test-dream-c1.py
- Makefile
- crates/bbt-mux
- crates/bboxy450-harness-wake-up
- verification
assignee: seat:01M8BTWWB85M3BAVRWQ9NSM7KQ
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
created: 2026-11-26T01:55:26.981Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8AS14QV5G47YBBF8FQ0W9SG
number: 1794292861627975
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream C1: make test-dream-c1 runs green on main (witness selection, image custody, Deadline before birth, tuple drift)'
type: task
updated: 2026-11-26T11:47:24.313Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test-dream-c1
provenance:
  digest: sha256:6dfdb212414165d22549786b6941da25132bad77743f173a39c51ae5616c68d7
  frontmatter_digest: sha256:1edac64590622bac5d674c002bb5fd7feded91e0c92824162bbbb2edb6ab163a
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: mqA3TNfXsKVuKa5v6Scc37PVLM5b3RKdtGajIzvcnKdetUDnRch8O5j46NCsayjRhBVt1X9dxIN9fYCXoyuABw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2230
  event:
    id: 01M8BTX2PC178QWHZVB7RWHA64
    sequence: 2230
    hash: 06888e080635e1ce12b9a8e9dad2ec838639d3799330c474c3a925d7ddfaf58f
---
Our stand: subtract before you add; report what you removed. Ask what each gate layer proves, and delete any layer that proves nothing.

## Evidence (Oracle, 2026-11-25, from tasks/2026-11-24-dream-q3b-read-prepare.md)
- `make test-dream-c1` could not run on main 972720fa8. `scripts/test-dream-c1.py:39-43` approved worker source `2ce4359`, but the harness gitlink was `f9f0b4a` on rustc 1.100.0, so `:1019` refused before any probe. Q3b approved the measured tuple (commit 8626973219).
- After approval: export command 3 (`scripts/test-dream-c1.py:3466-3493`) selects the pinned producer's Store exporter. That test is `#[ignore]` at harness `tests/dream_c1.rs:3629`, so nextest selects zero tests and exits 4. See the Q3b workspace log `target/dream-q3b/c1-approved-export.log`.
- The release test build replaces the provisioned worker image, so later steps run on a different inode than the one that was admitted.
- A Rust probe on a freshly restored, approved image returned `Deadline before birth`, exit 100 (`target/dream-q3b/native-probe-amended.log`). The gate's own standalone probe admitted the first launch and settled Hello and reap. Find why the two disagree.
- Drift: every harness bump changes the worker source, so the approval goes stale and the gate refuses again. In `--worker-only` mode (`:3424`, `Makefile:859`) a mismatch quietly turns native provisioning off.

## Do
1. Fix the witness selection so command 3 runs the exporter it means to run (include ignored, or select the right test). Assert that it selected at least one test.
2. Image custody: the release test build must not replace the admitted worker image. Give each its own path.
3. Diagnose `Deadline before birth` from its log and the code: product defect, wrong probe, or a budget that is too small for this machine. Fix the cause. Do not raise budgets or widen the approval to make it pass. If a budget is genuinely wrong, report the measurement and the decision you need.
4. Tuple drift: propose the smallest mechanism that keeps the approval current across harness bumps without a hand-edited literal (for example, `make bump-harness` re-measures and stages the tuple for review in the same commit). Implement it only if it removes the hand step, then report what it replaced.
5. `make test-dream-c1` exit 0 on the final commit, then `make land`. Report once.
