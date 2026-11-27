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
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8D617JACMD66KCFWK9D97KY
created: 2026-11-26T01:55:26.981Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8AS14QV5G47YBBF8FQ0W9SG
number: 1794292861627975
report:
  commit: 38a9fbdd50ab066d6d873464b19f5d97be13b0ed
  op_id: 01M8D60DZ598FYAAVKJ0GJDCB6
  report: '[[reports/2026-11-26-dream-c1-gate-landed.den]]'
  reported_by: seat:01M8BTWWB85M3BAVRWQ9NSM7KQ
  summary: '[[Merged]] Dream C1 repair landed at signed `38a9fbdd5`. `make land` exited 0 with a clean worktree, `origin/main` at the same commit, and a formal receipt covering 1008 models and 309 contracts. The gate runs the intended ignored exporter against a private admitted worker. The repair removes duplicate prebirth rests and replaces the hand-edited Python tuple and mirrored Rust hash with one measured tuple staged for review by `make bump-harness`. C1 commands 1–6 passed and command 7 remains with Store pacing. `make test` passed all 33 producer Dream tests before exiting 2 at the separately assigned native harness failures.'
  verify_revision: 3
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream C1: make test-dream-c1 runs green on main (witness selection, image custody, Deadline before birth, tuple drift)'
type: task
updated: 2026-11-27T00:21:09.838Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make land
verify_result:
  command: make land
  evidence:
  - criterion: Signed landed source and final formal receipt
    reference: target/dream-c1-repair/land.log:1801 and :1805, digest 00be4dd1b950c2733bac54665be2f164f16cabb4f6498e86625b892aa20656b1
  - criterion: Full CI passes producer Dream suite and stops at moved native cases
    reference: target/dream-c1-repair/full-test-timed-shell.log:391 and target/dream-c1-repair/ci-local-aa5f24427.log:2294
  - criterion: C1 commands 1–6 green and command 7 known red
    reference: target/dream-c1-repair/final-c1.log:22 through :28
  - criterion: Bump stages approved tuple for review without running worker
    reference: target/dream-c1-repair/bump.log, Makefile:444, crates/bbt-mux/src/logic/dream-c1-worker-build.txt
  exit_code: 0
verify_revision: 3
provenance:
  digest: sha256:b2b86609f3ec5fb4428ba6b7a815c205032110d0c88648a233f4adfe440bd970
  frontmatter_digest: sha256:10a15a165f52b29d1c887917b85db782272fcf90d8a471f40a3a25bca9a706d6
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: YPJktx6gu5gtp+Mgz1JTX2Xop7qhshe0RvfRaO43k5IUiUfd94FA83ituP8Ad3vfXCrngNaFredrNIlvaGNGAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2290
  event:
    id: 01M8D618775HB6MVZKTQK4FX8E
    sequence: 2290
    hash: 4123bd7090153b3cb7aad18c4c4c1e75b7c9361ecf9a46b11822ac0dd2d6b757
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
5. Superseded by the third ruling below.

## Oracle ruling 2026-11-26: the fb4bcfa tuple is approved
I checked each field against the tree myself:
- source `fb4bcfa8bd7371e488cba919f71c6c02228f2301`: the harness gitlink on main at 62318feea, from the signed harness landing of tasks/2026-11-24-harness-test-debt;
- compiler `408acb044ad9f92dcbaeb61386ef4dc1c39dd1c92d20b02c82b694a783c4b809`: the `rustc -vV | shasum -a 256` of the pinned toolchain, unchanged from the Q3b approval;
- executable `a1cbcd6f547e785bb01e2cb1d6ecaed01e1757401800aad05ecf456c3813c592`: `shasum -a 256 target/release/bboxy450-harness` in your worktree, matching your `target/dream-c1-repair/gate.log`.

Set `APPROVED_NATIVE_BUILD` in `scripts/test-dream-c1.py` to exactly this tuple, with a comment pointing at this ruling. Do not approve any other tuple without a new ruling.

This is the drift that step 4 is meant to remove: each harness bump needs a new ruling like this one. If your step 4 mechanism re-measures and stages the tuple in `make bump-harness`, this ruling becomes the last hand-approved literal. Say so in the report.

On the utility-QoS numbers (32.28 s with two rests per chunk against 15.46 s with one, across 576 chunks): if step 3 depends on them, report the decision you need with that measurement. Do not change a budget yourself.

## Oracle ruling 2026-11-26 (second): main is red; the full `make test` is now part of verify
My full `make test` on main 62318feea (log `~/mydev/bboxy450-term-oracle-ci.make-test.out`, exit 2) failed at `test-harness-source`. Thirteen harness Dream tests failed with "explicit preapproved ordinary read worker required" (`crates/bboxy450-harness/tests/dream_read.rs:1699`) or "gate requires explicit preapproved ordinary worker":
- seven in `dream_c1`, including `actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass`;
- six in `dream_read`.

Harness `fb4bcfa` enabled these previously ignored tests, but term still approves the old `f9f0b4a` tuple, so they find no approved worker and fail. The four `bboxy450-yarn` loom failures in the same log are broken controls that fail by design. They are not a defect.

The fix is your tuple approval above, plus step 4, so this cannot recur on the next bump. Quarantine also printed `unlisted or expired: actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass`. Make that test pass for real; do not quarantine it.

Verify was `make test-dream-c1 && make test`. The third ruling below replaces it.

## Oracle ruling 2026-11-26 (third): split command 7 out and land what is green
This is the second refusal at the same gate, so the premise changes. Gate command 7 is the four `bboxy450-term-mux --run-ignored only` Store witnesses: the 30 s job deadline after a 16 s snapshot, the 250 ms entry window with zero Hellos, and the parent overflow probe passing only on retry. That is a producer/consumer timing contract question, not this task. It moves to `tasks/2026-11-26-dream-c1-store-pacing.md`, with your evidence copied there. Do not work on command 7 here.

This task now lands what is already green on `6d1400851`:
- the worker approval;
- exporter selection;
- separate image custody;
- the bootstrap witness;
- step 4, if it removes the hand step.

Its `make test` verify is replaced by the fourth ruling below.

## Oracle ruling 2026-11-26 (fourth): land now; the native failures move out
I checked your blocker from the logs myself. `full-test-timed-shell.log` exits at `test-harness-native` Error 100, and every stage before it is green. That includes `test-harness-source`, which is where main 62318feea failed on the 13 Dream tests. So your diff fixes main's red.

The remaining failures are not caused by your diff:
- `mcp_seat_isolated_process`, with early expiry at `harness_status.rs:162` and no ACP exit notice at `mcp.rs:5132`;
- the spaced-`TARGET_DIR` cases.

Your diff (6 files: Dream C1, Makefile, contracts) touches none of those tests. Main never reached this stage, so those failures were already there, only hidden. They move to `tasks/2026-11-26-harness-native-timing.md` with your evidence. Do not work on them here, and do not keep the alias-target workaround in the Makefile unless it is part of your Dream C1 change.

**Verify is `make land` on the final rebased commit.** Run nothing else; do not rerun `make test`. Report once, with:
- the `make land` log path and exit;
- `target/dream-c1-repair/full-test-timed-shell.log` as the `make test` evidence (green through `test-harness-source`, red only at `test-harness-native` for the moved cases);
- the `make test-dream-c1` log, with commands 1–6 green and command 7 known red, owned by store-pacing;
- whether step 4 removed the hand-edited tuple.

Keep the approved tuple and all recorded budgets unchanged.


## Report from seat:01M8BTWWB85M3BAVRWQ9NSM7KQ

[[Merged]] Dream C1 repair landed at signed `38a9fbdd5`. `make land` exited 0 with a clean worktree, `origin/main` at the same commit, and a formal receipt covering 1008 models and 309 contracts. The gate runs the intended ignored exporter against a private admitted worker. The repair removes duplicate prebirth rests and replaces the hand-edited Python tuple and mirrored Rust hash with one measured tuple staged for review by `make bump-harness`. C1 commands 1–6 passed and command 7 remains with Store pacing. `make test` passed all 33 producer Dream tests before exiting 2 at the separately assigned native harness failures.

```json
{
  "op_id": "01M8D60DZ598FYAAVKJ0GJDCB6",
  "commit": "38a9fbdd50ab066d6d873464b19f5d97be13b0ed",
  "report": "[[reports/2026-11-26-dream-c1-gate-landed.den]]",
  "verify_result": {
    "command": "make land",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Signed landed source and final formal receipt",
        "reference": "target/dream-c1-repair/land.log:1801 and :1805, digest 00be4dd1b950c2733bac54665be2f164f16cabb4f6498e86625b892aa20656b1"
      },
      {
        "criterion": "Full CI passes producer Dream suite and stops at moved native cases",
        "reference": "target/dream-c1-repair/full-test-timed-shell.log:391 and target/dream-c1-repair/ci-local-aa5f24427.log:2294"
      },
      {
        "criterion": "C1 commands 1–6 green and command 7 known red",
        "reference": "target/dream-c1-repair/final-c1.log:22 through :28"
      },
      {
        "criterion": "Bump stages approved tuple for review without running worker",
        "reference": "target/dream-c1-repair/bump.log, Makefile:444, crates/bbt-mux/src/logic/dream-c1-worker-build.txt"
      }
    ]
  }
}
```
