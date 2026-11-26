---
allowed_paths:
- crates/bboxy450-harness
- crates/bbt-mux/src/knowledge/routing
- crates/bbt-mux/src/knowledge/routing.rs
- crates/bbt-mux/models/WorkspaceRetention.tla
- crates/bbt-mux/models/BrokenWorkspaceRetention.tla
- crates/bbt-mux/src/models/workspace_retention.rs
- verification/coverage.toml
- verification/contracts.toml
- verification/corpus.toml
assignee: seat:01M8ABFWBE9DFCF674S3TN2DB7
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8BTTZN0Z24CEMDQDVMJ7WQA
created: 2026-11-24T17:09:30.072Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M878HCFAH78PJHXBS5ZRHABC
number: 1794292861627912
report:
  commit: 62318feea5ff2b69ac503cffc81858767694d35a
  op_id: 01M8BTSZA189GM7QNPM2CQ1Q62
  report: '[[reports/2026-11-26-harness-test-debt-complete.den]]'
  reported_by: seat:01M8ABFWBE9DFCF674S3TN2DB7
  summary: '[[Implement]] Landed the signed harness fb4bcfa and term 62318feea, including pin 100b9a3d5. Removed 28 scoped ignores and unconditional mux pressure eviction. The held retirement witness passed 20 isolated and 20 loaded runs with zero retries. The harness gate, term formal gate and final term land all exited 0. Native/TLC matched 1,008 models and the WorkspaceRetention pressure mutant failed its named safety invariant. Both worktrees are clean.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Harness: clear the ignored-test debt (23 QoS-deferred dream tests, bill/group-proof race, 4 gui_rich, profile-switch SessionClosed)'
type: task
updated: 2026-11-26T11:46:17.106Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make -C crates/bboxy450-harness test
verify_result:
  command: make -C crates/bboxy450-harness test
  evidence:
  - criterion: Signed harness candidate passes the canonical gate and landed
    reference: target/harness-debt-verify-final-tree.json and target/harness-debt-land-harness.log:2111
  - criterion: Real GUI screenshot and named redaction control
    reference: target/harness-debt-verify-final-tree.log:2817,9427
  - criterion: Held-retirement negative witness and 20 + 20 green runs with retries 0
    reference: target/harness-debt-term-mux-held-control.log:24–31, target/harness-debt-term-mux-alone.log and target/harness-debt-term-mux-loaded.log
  - criterion: Named pressure invariant and complete native/TLC term parity
    reference: target/harness-debt-term-formal.log:1101–1102,1931–1933
  - criterion: Term pin and signed push
    reference: target/harness-debt-bump-term.log and target/harness-debt-term-land-after-rich-probe.log:1926–1931
  exit_code: 0
provenance:
  digest: sha256:e14991e7b17c5124c5989974fff159b2cf4965e828ac6c66d85d423b456d9fea
  frontmatter_digest: sha256:e0f615c14abc2e3dcc7d343462ba77e8ae6f82678c0174410e4d369864883684
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: MFL8Q+EN/1A3sSV2Z868etrD3BcL53l53PT30nalc0AkTiL9pqxSXB7jLOUjIl/45/F3NaVSt0r4xSJ9BYfADw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2228
  event:
    id: 01M8BTV105ZV24A8945DKJNP5F
    sequence: 2228
    hash: 025d622719cf34fe34ed7a79bc5f56d22b6ed57e09f40031fe043019625698e0
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-08-reenable-qos-deferred-dream-tests.md`, `tasks/2026-11-08-harness-known-bill-group-proof-race.md`, `tasks/2026-11-08-harness-gui-test-known-defects.md`, `tasks/2026-11-08-gui-profile-switch-session-closed.md`.

Work in the harness submodule (its own repo, bboxy450/bboxy450-harness); land it there, then `make bump-harness` in term.

## Evidence (harness origin/main e9efa3b)
- 23 `load-sensitive ... re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp` ignores: `src/dream/resolve.rs` 9, `tests/dream_c1.rs` 7, `tests/dream_read.rs` 6, `fixtures/dream-bootstrap/process.rs` 1. That task is closed.
- Ignore at `src/dream/resolve.rs:4861`: known bill vs group-proof upgrade ordering race.
- 4 `gui_rich` ignores at `tests/gui_rich.rs:238,295,550,1658` (mermaid figure, reasoning prose, live tool tail, MCP raw cards).
- `gui_socket_start_switch_aba_resume_and_cleanup` (`tests/support/seat_socket.rs:669`, wait :588) intermittently misses the old session's `SessionClosed`; no fix commit.

## Do
1. QoS ignores: tests must not time the machine (`53a9e215f` stance). Rewrite each to assert ordering/outcome instead of wall time, or delete it if that leaves nothing. Remove all 23 ignores.
2. Fix the bill/group-proof race with a deterministic test for both orders; remove the ignore.
3. Fix the 4 gui_rich tests and remove their ignores.
4. Root-cause the missing `SessionClosed` (or prove it gone with a load run of 20 iterations, retries 0).
5. Land in the harness repo, `make bump-harness`, `make land` in term.
6. **Oracle ruling 2026-11-25 (amendment): the Dream replay gate failure is in scope; you fix it, nobody else is assigned.** Your report shows `tests/dream_replay.rs:471` and `:1048` failing with `dream replay: invalid or unavailable input/cache` (`src/dream/mod.rs:122`) only inside the parallel `cargo test --all-targets` step of `scripts/test.py`, passing alone. Two different tests failing at random under the full run is a shared-state defect (a cache, temp path, env var or origin shared across test processes/binaries), not load. Find which state is shared: print the cache/input path the failing call resolves and compare it with concurrently running tests (dream_c1/dream_read/lib dream tests in the same step). Give each test private state, or fix the product path if two real processes can corrupt each other (that is the `native_two_processes_race_*` contract — check that first). No ignore, no retry, no skip, no serialising the step as the fix. First check whether harness origin/main e9efa3b fails the same way (one run of the same step); say so in the report either way. Then `make -C crates/bboxy450-harness test` exit 0, and continue with step 5. Report once, landed or truly blocked.
7. **Oracle ruling 2026-11-25 (second amendment): the context recovery failure is in scope; you fix it, nobody else is assigned.** `sigkill_between_rounds_rebuilds_the_identical_model_request` failed intermittently at `tests/context_auto.rs:504` (frontier 122 vs 107). Fix it to rebuild from durable state or wait on the durable compaction event, not on time. Prove it with 20 iterations, retries 0, plus one full step. No ignore, retry, skip or serialisation. Done in 5870ddbe: it now reuses the live catalogue and waits for the durable compact event.
8. **Oracle ruling 2026-11-26 (third amendment): yes, fix the ACP quiet fixture as you proposed, and take standing authority for the rest.** A wall-time assertion that sees 198 ms next to a correct monotonic `Quiet { ms: 200 }` is a test timing the machine, which step 1 already forbids. Replace it with durable gauge order plus the recorded quiet reason.
   **Standing authority, so you do not stop again:** fix any further inherited failure in `make -C crates/bboxy450-harness test` yourself, without asking, if it is one of these:
   - a test asserting wall time, sleep length or a machine-speed threshold, which you rewrite to assert ordering or outcome;
   - a test sharing state with another test, which you give private state.

   Rules for each such fix:
   - Check it against upstream main once, and record whether it is inherited.
   - List it in the report with its `file:line`, the mechanism, and the before and after of the assertion.
   - No ignore, retry, skip, serialisation or raised budget.

   Stop and report only for a genuine product defect, meaning the product itself misbehaves rather than the test's clock or setup, or a failure that needs a change outside `crates/bboxy450-harness`.
   Harness main has moved: term is at harness `d63fee07` (term commit 89082b7e4). Rebase onto harness origin/main before the final gate. Then `make -C crates/bboxy450-harness test` exit 0, step 5, and one report, landed.
9. **Oracle ruling 2026-11-26 (fourth amendment): the GUI smoke layout defect is in scope; you fix it here, nobody else is assigned.** Verified from your logs: `target/harness-debt-gui-smoke-baseline.log` on unmodified harness main `d63fee07` exits 1 with `bboxy450-harness gui: tool text touches the card border`, raised at `src/frontends/gui/smoke.rs:323` by the inset check at `:319` (7.5 px horizontal, 5.5 px vertical). This is an inherited product defect: the tool card paints its text 5.25 px from the border.
   - **Fix the product layout, not the check.** Make the tool card's text inset meet the smoke's 5.5 px vertical and 7.5 px horizontal inset. Do it by correcting the padding or rounding where the card is laid out in `src/frontends/gui`. Do not lower, round or loosen the threshold at `smoke.rs:319`.
   - If `tests/gui_rich.rs` or a parity reference changes geometry because of this, update it in the same commit and name it in the report.
   - Prove it in three ways:
     - `make -C crates/bboxy450-harness test-gui-smoke` exits 0 and writes the screenshot.
     - The capability control `GuiSmokeRegistersCapabilityBeforeRendering` now fails at its own assertion.
     - The full `make -C crates/bboxy450-harness test` exits 0.
   - Then do step 5. Report once, landed. Disk is tight (14 GB free): do not create a second target directory.
10. **Oracle ruling 2026-11-26 (fifth amendment): the term mux retention failure is in scope; you fix it here, nobody else is assigned.** `allowed_paths` now also has `crates/bbt-mux/src/knowledge/routing`.
    - **Verified harness landing.** Harness `fb4bcfa` is signed (G Boxy) and is an ancestor of harness origin/main.
    - **Verified term failure.** `target/harness-debt-land-term.log:840–879` shows `workspace_pressure_evicts_the_least_recent_idle_root_and_caps_shared_bindings` failing on both tries at `crates/bbt-mux/src/knowledge/routing/workspaces_retention_tests.rs:109` (`assert_slots`: roots 3, expected 4), called from `:162`.
    - **Diagnosis.** The test came from index-bounds `972720fa8`. It passes alone and fails under the gate's load. That points at the fixture reading wall time or racing a background mount, not at the bound itself.
    - **Find the mechanism.** Find what drops or delays the 4th root under load. Candidates:
      - an idle or age-based eviction keyed to `Instant`;
      - an asynchronous mount the 2 ms poll loop at `:94–105` does not wait to be fully counted.
    - **Fix rule.** Make the fixture wait on the router's durable or observable state, or drive the clock explicitly, instead of real time.
    - **Product defect fallback.** If the cache really can hold 3 roots when 4 are within the limit, that is a product defect in the router. Fix it in `crates/bbt-mux/src/knowledge/routing` and keep the test as its witness.
    - No ignore, retry, skip, serialisation or raised budget.
    - **Prove it in four steps:**
      1. The fixed test passes 20 runs in a row with retries 0: `cargo nextest run -p bboxy450-term-mux -E 'test(workspace_pressure_evicts)' --retries 0`, in a loop.
      2. Run the same 20 runs while the full mux nextest runs alongside, to apply load.
      3. Name the mechanism in the report.
      4. Add the fix as a signed commit on top of `100b9a3d5`, then run `make land` in term until it exits 0.
    - Report once, landed. Disk is at 10 GB free: reuse this worktree's target directory only.
11. **Oracle ruling 2026-11-26 (sixth amendment): yes, fix the router.** It is a product defect, which is the step 10 fallback. `allowed_paths` now also has:
    - `crates/bbt-mux/src/knowledge/routing.rs`;
    - `crates/bbt-mux/models/WorkspaceRetention.tla` and `BrokenWorkspaceRetention.tla`;
    - `crates/bbt-mux/src/models/workspace_retention.rs`;
    - `verification/{coverage,contracts,corpus}.toml`.

    **Verified evidence:**
    - `routing.rs:514–520`: when `roots + mounting + retiring >= root_limit`, every pending retry calls `evict_idle_root()` again, even while `retiring` already holds a root. Each retry therefore evicts one more idle root.
    - `target/harness-debt-term-mux-held-control.log:24` reproduces this with retries 0. At `workspaces_retention_tests.rs:198`: "pressure retry evicted the recently used root before the retiring slot became free".

    **Fix.** Initiate an eviction only when no root is retiring: with `!self.retiring.is_empty()`, return Pending and wait for the slot already being retired. This is the smallest change. Keep the occupancy bound unchanged.

    **Model.**
    - Add the same guard to `WorkspaceRetention.tla`.
    - Keep or add a broken control that drops the guard, and confirm it fails a safety invariant you name. The invariant must say that no recently used root is evicted while a slot is retiring, or that at most one root is retiring per pressure cycle.
    - Update the inventory entries.
    - Run `make test-formal` and get exit 0.

    **Prove:**
    - the held-retirement witness is green;
    - the step 10 proof: 20 runs alone and 20 runs under load, retries 0;
    - `make land` in term exits 0.

    Signed commits go on top of `100b9a3d5`. Report once, landed.


## Report from seat:01M8ABFWBE9DFCF674S3TN2DB7

[[Implement]] Landed the signed harness fb4bcfa and term 62318feea, including pin 100b9a3d5. Removed 28 scoped ignores and unconditional mux pressure eviction. The held retirement witness passed 20 isolated and 20 loaded runs with zero retries. The harness gate, term formal gate and final term land all exited 0. Native/TLC matched 1,008 models and the WorkspaceRetention pressure mutant failed its named safety invariant. Both worktrees are clean.

```json
{
  "op_id": "01M8BTSZA189GM7QNPM2CQ1Q62",
  "commit": "62318feea5ff2b69ac503cffc81858767694d35a",
  "report": "[[reports/2026-11-26-harness-test-debt-complete.den]]",
  "verify_result": {
    "command": "make -C crates/bboxy450-harness test",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Signed harness candidate passes the canonical gate and landed",
        "reference": "target/harness-debt-verify-final-tree.json and target/harness-debt-land-harness.log:2111"
      },
      {
        "criterion": "Real GUI screenshot and named redaction control",
        "reference": "target/harness-debt-verify-final-tree.log:2817,9427"
      },
      {
        "criterion": "Held-retirement negative witness and 20 + 20 green runs with retries 0",
        "reference": "target/harness-debt-term-mux-held-control.log:24–31, target/harness-debt-term-mux-alone.log and target/harness-debt-term-mux-loaded.log"
      },
      {
        "criterion": "Named pressure invariant and complete native/TLC term parity",
        "reference": "target/harness-debt-term-formal.log:1101–1102,1931–1933"
      },
      {
        "criterion": "Term pin and signed push",
        "reference": "target/harness-debt-bump-term.log and target/harness-debt-term-land-after-rich-probe.log:1926–1931"
      }
    ]
  }
}
```
