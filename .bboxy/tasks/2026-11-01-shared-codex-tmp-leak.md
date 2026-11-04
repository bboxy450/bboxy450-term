---
allowed_paths:
- scripts/e2e/shared_codex.py
- scripts/
- Makefile
- .bboxy/runs/
- .bboxy/reports/
artifacts:
- '[[01M6BNQBADF1Q5NVPZ4S76FY1Z]]'
assignee: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T11:11:21.464Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6BCX3AZFSSZS5GQVATE2Y2J
number: 1796
provenance:
  digest: sha256:80b35e1d3905de700f5dbf5bae2d337e53fc73026f863198e4d2103aed3ef756
  frontmatter_digest: sha256:d91fc7e1fb749ad9f555e501bb2da0e1b4ae2db533cb3cdc463b39148fcf66ab
  log: .bboxy/log/2026-11-01.jsonl#9420
  sig: 9VRnOvK30hCioOdB8rXTLMKZ+CwuUVWe/PjEnFjQPtp2BbIHv604hDyk+b4UyKnqmztXguaRtKZ5xtHwXU+ECQ==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
report:
  commit: 9030e565bd38fb7439f8c28523f35e39e35c5bd9
  op_id: 01M6BNVZ6309MGYXBWHRDF1VCY
  reported_by: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
  summary: 'LANDED on main: 9030e565bd38fb7439f8c28523f35e39e35c5bd9 (remote independently verified), signed Boxy/key844ECC0587DE7AB0; worktree clean/aligned. Full committed report #5000: [[reports/2026-11-01-shared-codex-tmp-leak]], linked by the signed Report trailer; artifact01M6BNQBADF1Q5NVPZ4S76FY1Z. Success removes the fixture root after owned-process shutdown; stopped failures retain only the documented 64KiB-capped diagnostics/owner manifest; marked all-dead roots are reclaimed and live/unproven custody is preserved. Native capture handoff copies only frames/ and codex-stream.bin to caller-owned temporary storage, retired on success/failure. Focused nine tests pass, including 32 owner filename/liveness cases, actual child reaping, removed-cleanup control and both handoff outcomes; licenses/diff checks0. Full normal make test passed0 on intermediate75df9ebf; the subsequent all-producer-record guard amendment passed focused tests and, per ruling01M6BJD6WTWB9H1YN50046C8W0, plain scoped landing validated0788883e and then rebased9030e565 with current formal/app tests/strict Clippy/corpus/145 extended landing fixtures/recovery/report provenance gates all green. Plain landing exited0, no fallback attempt needed. Final formal134 models/31crates/57contracts, receipt5c49c6e5. Initial missing-number report failure and pre-/post-rebase log mismatch are retained in the report/run summary; no log was rewritten. Unmarked legacy roots and live/unreadable ownership are deliberately untouched; actual Codex/Metal was not run, as the authorized fixture regression covers the handoff. Live task_report has no optional report parameter, so its qualified locator is in this summary.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: shared_codex e2e must not leak its /tmp fixture root
type: task
updated: 2026-11-01T13:48:15.120Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test
verify_result:
  command: make test
  evidence:
  - criterion: 1 — success retirement with owned-process custody
    reference: scripts/e2e/shared_codex.py:101,134,200; scripts/test-shared-codex-cleanup.py:test_success_removes_root_after_owned_child_exits. make test-shared-codex-cleanup exit0 (9 tests); actual tracked subprocess is reaped before successful-root absence assertion. Source Python files unchanged0788883e..landed9030e565 (git diff --exit-code0).
  - criterion: 2 — failure retained set and byte bound
    reference: 'Report #5000 lists exact 13 diagnostic filenames, each tail-capped64KiB, plus cleanup-owner.json. test_failure_keeps_only_bounded_diagnostics checks exact small retained set, oversized-log cap, removed home/packages/frames and original error. Proven-live/unknown producers cause fail-closed root retention, documented explicitly.'
  - criterion: 3,4 — dead/live/foreign reclamation and controls
    reference: shared_codex.py:51,73 checks known daemon PID records and all run/*.owner.json, script/root/inode/UID/nonempty PID metadata. test_next_run_reclaims_dead_and_preserves_live_or_foreign_roots covers8 ownership states across4 producer filenames (32 cases); unknown/corrupt/symlink and live auxiliary guards pass. Removed finish control must fail the same successful-root witness. Native caller test checks explicit two-artifact handoff, source-root deletion and caller cleanup in both outcomes.
  - criterion: Verification and final landing
    reference: 'target/ci-local.log:4994 and /tmp/rook-tmpleak-ci-green-full.log: make test normal75df9ebf exit0 before final Python guard amendment. /tmp/rook-tmpleak-land-scoped.log and .exit: plain landing exit0; scoped9030e565 verdictgreen, runtime882.005s/compile625.651s. Current log /Users/feral/.local/state/bboxy450-term/landing/presubmits/9030e565bd38fb7439f8c28523f35e39e35c5bd9-65be1dd6/commands.log includes app tests, strict all-target Clippy,145 landing fixtures, recovery and committed report resolution.'
  - criterion: Current-source formal and signed main
    reference: '/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/5c49c6e582dfea133c0b00b75b4334f229257e0fb801a4c304fa38a3094ba766/evidence.json: clean9030e565,134models31crates57contracts. git ls-remote origin refs/heads/main=9030e565bd38fb7439f8c28523f35e39e35c5bd9; git status clean/aligned; signatureG approvedkey844ECC0587DE7AB0 author/committerBoxy. Report SHA25601d3fe51e97d9cbeefec19998c9926c962b0b63e241a250aa710210d7b151c26.'
  exit_code: 0
---
# shared_codex e2e must not leak its /tmp fixture root

ROLE: EXECUTOR. Create worktree `~/mydev/bboxy450-term-tmpleak`, branch `shared-codex-tmp-leak` from origin/main. Only this worktree.

## Problem
`scripts/e2e/shared_codex.py:78` creates `/tmp/bbt-shared-codex-*` (≈250 MB–1.2 GB each: private HOME, Codex packages, run dir). On 2026-11-01 the Oracle found 54 leftovers, 13 GB in total. They kept recovery tests under their 10 GiB free-disk floor and blocked landings (#572). The `finally` at :491 writes `cleanup-owner.json` and removes only `home/.codex/packages` (:523), so the root stays behind.

## Required outcome
1. A passing run removes its whole fixture root, after stopping only the PIDs it started and recorded.
2. A failing run keeps at most the diagnostics needed (logs, `cleanup-owner.json`) and drops the bulky package and home trees. Say what is kept.
3. A bound on accumulation: a new run reclaims earlier `bbt-shared-codex-*` roots whose recorded owner PIDs are all dead. Never touch a root whose owner is alive, nor anything not created by this script.
4. Regression: a fixture-level test showing that success leaves no root, that failure leaves only the small retained set, and that a stale root with dead owners is reclaimed while one with a live owner is kept. Include a broken control: with the cleanup removed, the test fails.

Allowed paths: `scripts/e2e/shared_codex.py`, new tests under `scripts/`, `Makefile` (a focused target only), `.bboxy/runs/`, `.bboxy/reports/`.

## Verify
The focused target and `make test` on the exact committed source. Land with `sh scripts/land.sh --gate-log target/ci-local.log`. Write the full report in `.bboxy/reports/` and add a `Report:` trailer.

## Communication
Quiet by default: one task_report at the end. Message the Oracle mid-task only for a blocker that stops all work, in three lines or fewer.


## Report from seat:01M6ABQRB8SJNZHP6NRHQD3BC8

LANDED on main: 9030e565bd38fb7439f8c28523f35e39e35c5bd9 (remote independently verified), signed Boxy/key844ECC0587DE7AB0; worktree clean/aligned. Full committed report #5000: [[reports/2026-11-01-shared-codex-tmp-leak]], linked by the signed Report trailer; artifact01M6BNQBADF1Q5NVPZ4S76FY1Z. Success removes the fixture root after owned-process shutdown; stopped failures retain only the documented 64KiB-capped diagnostics/owner manifest; marked all-dead roots are reclaimed and live/unproven custody is preserved. Native capture handoff copies only frames/ and codex-stream.bin to caller-owned temporary storage, retired on success/failure. Focused nine tests pass, including 32 owner filename/liveness cases, actual child reaping, removed-cleanup control and both handoff outcomes; licenses/diff checks0. Full normal make test passed0 on intermediate75df9ebf; the subsequent all-producer-record guard amendment passed focused tests and, per ruling01M6BJD6WTWB9H1YN50046C8W0, plain scoped landing validated0788883e and then rebased9030e565 with current formal/app tests/strict Clippy/corpus/145 extended landing fixtures/recovery/report provenance gates all green. Plain landing exited0, no fallback attempt needed. Final formal134 models/31crates/57contracts, receipt5c49c6e5. Initial missing-number report failure and pre-/post-rebase log mismatch are retained in the report/run summary; no log was rewritten. Unmarked legacy roots and live/unreadable ownership are deliberately untouched; actual Codex/Metal was not run, as the authorized fixture regression covers the handoff. Live task_report has no optional report parameter, so its qualified locator is in this summary.

```json
{
  "commit": "9030e565bd38fb7439f8c28523f35e39e35c5bd9",
  "op_id": "01M6BNVZ6309MGYXBWHRDF1VCY",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "1 — success retirement with owned-process custody",
        "reference": "scripts/e2e/shared_codex.py:101,134,200; scripts/test-shared-codex-cleanup.py:test_success_removes_root_after_owned_child_exits. make test-shared-codex-cleanup exit0 (9 tests); actual tracked subprocess is reaped before successful-root absence assertion. Source Python files unchanged0788883e..landed9030e565 (git diff --exit-code0)."
      },
      {
        "criterion": "2 — failure retained set and byte bound",
        "reference": "Report #5000 lists exact 13 diagnostic filenames, each tail-capped64KiB, plus cleanup-owner.json. test_failure_keeps_only_bounded_diagnostics checks exact small retained set, oversized-log cap, removed home/packages/frames and original error. Proven-live/unknown producers cause fail-closed root retention, documented explicitly."
      },
      {
        "criterion": "3,4 — dead/live/foreign reclamation and controls",
        "reference": "shared_codex.py:51,73 checks known daemon PID records and all run/*.owner.json, script/root/inode/UID/nonempty PID metadata. test_next_run_reclaims_dead_and_preserves_live_or_foreign_roots covers8 ownership states across4 producer filenames (32 cases); unknown/corrupt/symlink and live auxiliary guards pass. Removed finish control must fail the same successful-root witness. Native caller test checks explicit two-artifact handoff, source-root deletion and caller cleanup in both outcomes."
      },
      {
        "criterion": "Verification and final landing",
        "reference": "target/ci-local.log:4994 and /tmp/rook-tmpleak-ci-green-full.log: make test normal75df9ebf exit0 before final Python guard amendment. /tmp/rook-tmpleak-land-scoped.log and .exit: plain landing exit0; scoped9030e565 verdictgreen, runtime882.005s/compile625.651s. Current log /Users/feral/.local/state/bboxy450-term/landing/presubmits/9030e565bd38fb7439f8c28523f35e39e35c5bd9-65be1dd6/commands.log includes app tests, strict all-target Clippy,145 landing fixtures, recovery and committed report resolution."
      },
      {
        "criterion": "Current-source formal and signed main",
        "reference": "/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/5c49c6e582dfea133c0b00b75b4334f229257e0fb801a4c304fa38a3094ba766/evidence.json: clean9030e565,134models31crates57contracts. git ls-remote origin refs/heads/main=9030e565bd38fb7439f8c28523f35e39e35c5bd9; git status clean/aligned; signatureG approvedkey844ECC0587DE7AB0 author/committerBoxy. Report SHA25601d3fe51e97d9cbeefec19998c9926c962b0b63e241a250aa710210d7b151c26."
      }
    ],
    "exit_code": 0
  }
}
```
