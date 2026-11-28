---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term/tests/dream_c1.rs
- scripts/test-dream-c1.py
- verification
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
created: 2026-11-28T12:19:23.378Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8H1H2484QHCM888EQ8SW5H7
number: 1794292861628057
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream C1 gate command 18: status and control latency while a source read is held'
type: task
updated: 2026-11-28T12:56:47.268Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test-dream-c1
provenance:
  digest: sha256:b9290ceb18be0d688eec4b5959899e80c2a218b6d8280e52c4fb1f04b0bf9991
  frontmatter_digest: sha256:16f5f208f684905404e2a963544c522556b3f68c7c751c3bb5915ed336a9fbb5
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: o4j0rYesD+Ka/SJlQdRabMgCt5TWP5TK2Yb7J6vz0p2RDaiLGXpw4k8teNyVbJjSUbRVI6VQRV67/BgdflfcDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2492
  event:
    id: 01M8H3NJ3WB5FNZMS7WE0PGTR9
    sequence: 2492
    hash: 8b521a38235c69cd9ca9e2a2c13d9e1bdb074affcc22764fe2876d6349cfb5e7
---
Our stand: subtract before you add; report what you removed. Split from `tasks/2026-11-26-dream-c1-store-pacing.md` (Oracle rulings 2026-11-28, fourth and fifth). That task lands the pacing fix, the explicit seat launcher and the clean-exit seal. With those in, `make test-dream-c1` commands 1–17 exit 0, and command 18 exits 100. This task owns **all seven** command 18 failures, not only the held-read one in the title.

## Evidence: `bboxy-dream-c1-gate-bd9ol8_k/command-17.log` on signed `0449db5b1`, read by the Oracle
The gate directory is under `/private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/`. The run: 13 tests, 6 passed, 7 failed on both tries, 874 s, host load about 4.5. All assertions are in `crates/bboxy450-term/tests/dream_c1.rs`.

| Failing test | First assertion |
|---|---|
| `dream_c1_actual_source_read_hold_keeps_controls_and_cancel_owner_until_release` | `:1489` status `not_found` for its own live run; `:1045` p99 155–167 ms > 100; `:703` timeout "source owner blocked on private FIFO" |
| `dream_c1_production_cpu_and_progress_keep_actual_controls_responsive` | `:1265` turn end admitted Dream: `storage_unavailable`; then `:1489`/`:1045` p99 165–169 ms |
| `dream_c1_protocol_fake_progress_flood_keeps_controls_and_cancel_independent` | `:1489` `not_found`; `:1045` p99 163–228 ms, **with a fake worker** |
| `dream_c1_four_live_real_jobs_keep_independent_controls_and_exact_settlement` | `:703` timeout "owned open source catalogue" |
| `dream_c1_real_closed_job_owns_controls_dedup_and_native_frame_overlap` | `:385` `ctl seat ls` refused: "UI requests use the ctl operator door" |
| `dream_c1_real_daemon_loss_quarantines_predecessor_without_replay_or_adoption` | `:703` timeout "owned Hello child receipt before Start"; `:1998` predecessor child not retained |
| `dream_c1_retained_same_generation_turn_pauses_real_worker_with_legacy_read_held` | read the log |

Passing: hot-owner mutant, page tamper, ctl single-seat, owned blob over limit, multi-MiB Store blob, omit-rest control.

## Do, in this order
1. **Classify first, then fix.** For each of the seven tests, give the first failure as `file:line` and the root cause as `file:line — mechanism`. Group the tests that share a cause. Expect a few shared causes, not seven:
   - status `not_found` and `storage_unavailable` for a live run look like one lookup or root defect;
   - the `ctl` operator-door refusal looks like the fixture using a door that seats are refused (`AGENTS.md`: "The shell command `bboxy450-term ctl` is the operator's door and refuses seats"). The fixture must use the seat-admitted path the product offers. Do not reopen the operator door to seats.
2. **Correctness before latency.** Fix `not_found`, `storage_unavailable`, the operator-door use and the catalogue/Hello/predecessor timeouts in the product or fixture, wherever the cause lives. Each fix gets a Rust witness. A stateful publish/lookup/lifecycle race also updates its Dream C1 model and adds a broken control that reintroduces it.
3. **Latency with a baseline.** The p99 also fails with a fake worker, so measure before assigning blame. Run the same three 200-sample streams (input, messaging, status) in the same fixture on the same host twice: once with no Dream work, once held. Report p50, p99 and max for both.
   - If the baseline also exceeds 100 ms p99, the test is timing the machine (store-pacing brief, step 2). Change the assertion to compare against the baseline: held p99 ≤ baseline p99 plus a stated margin, and max ≤ 250 ms. Record the margin and the reason in `verification/contracts.toml`, next to the Dream C1 timing contract.
   - If the baseline passes and the held run fails, a held read blocks a control path. Find the shared owner, lock or queue it waits on (`file:line`) and remove the wait. Keep every existing limit, and add a broken control that reintroduces the wait.
4. Do not change the producer, the 250 ms lease, the 30 s deadline, the pinned worker tuple or the pacing debt. No ignore, skip, serialisation or extra retry.

Verify: `make test-dream-c1` exits 0 on the final commit, then `make land`. Report once, with the classification table, the baseline and held numbers, and the gate directory.

Disk is shared and tight: delete your own `target/formal` scratch after formal evidence is recorded, and run one gate at a time.
