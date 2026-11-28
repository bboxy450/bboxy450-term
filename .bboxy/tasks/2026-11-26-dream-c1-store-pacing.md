---
allowed_paths:
- scripts/test-dream-c1.py
- Makefile
- crates/bbt-mux
- crates/bboxy450-harness-wake-up
- crates/bboxy450-term/tests/dream_c1.rs
- verification
assignee: seat:01M8GAPWWPP5V7213X22D5HSP5
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8H52MVPEW99XCNNQ8YYEXZM
created: 2026-11-26T14:10:15.016Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8C32KY3GM6C346ZS89YQGX7
number: 1794292861627993
report:
  commit: a8003afbf941ca63cd386d0c7d48274ea7b38fc9
  op_id: 01M8H518RB7WSTKA342M9JQQ0J
  report: '[[reports/2026-11-28-dream-c1-measured-debt.den]]'
  reported_by: seat:01M8GAPWWPP5V7213X22D5HSP5
  summary: Landed signed a8003afbf on main with `make land` exit 0 and native/Java parity across 1,015 models. Replaced per-step floors, redundant pacing rests and staged-line copies with measured CPU debt, made the private seat launcher explicit, and let the bound follower finish one exact-exit check. In the runtime gate, commands 1–17 passed; the original Store workers and overflow passed at zero retries, the paired old floor hit Deadline and reaped, and the app status-zero exit sealed its latch. Command 18 exited 100 with six passes and seven failures, assigned to the separate app-controls task under the fifth ruling.
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream C1: Store worker witnesses meet the producer/consumer timing contract (gate command 7)'
type: task
updated: 2026-11-28T13:21:25.339Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make land
verify_result:
  command: make land
  evidence:
  - criterion: Signed landing and native/Java model parity on current source
    reference: /Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M8GAPHE4GQ7D5QG6K9CKY3HD/target/dream-c1-land-a8003afbf.log
  - criterion: Archived formal evidence retained after checkout scratch removal
    reference: /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/3c4aa3e37ee2c5755e6383a3902a7e7239b7294c4a793bbf8cb74e32ce886f1e/evidence.json
  - criterion: Dream C1 commands 1–17 passed and command 18 exit 100 recorded
    reference: /private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/receipt.json
  - criterion: Actual deadline and reap under the old floor on identical Store input
    reference: /private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/runtime-store-step-floor.json
  - criterion: Successful exact closed-exit and immutable page witness plus seven follow-up failures
    reference: /private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/command-17.log
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:41271042dfefd3c2fffa184e6d82db69e3f701f1bc58df2320b04a69d33fb095
  frontmatter_digest: sha256:cde4448043e39cd0bc3071c09c04dc8ccf515342d7d8e46352f13a434864edc9
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: 2RMTCyWtq4bl4zXVSZYkaPKtYaDGXTQ2fTIWVdw5syjDHIEz4lu316oulP+0lhbGNUH64i0blDBzHJEfOazYBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2503
  event:
    id: 01M8H52NEFFAGG2E30KN5270CG
    sequence: 2503
    hash: dbb173390bf049701bbaef40fc9741d5def0fefcc65c730a1a0e00182145d302
---
Our stand: subtract before you add; report what you removed. Split from `tasks/2026-11-25-dream-c1-gate-repair.md` (Oracle ruling 2026-11-26, third). That task lands the worker approval, exporter selection, image custody and bootstrap witness. This task makes `make test-dream-c1` exit 0.

## Evidence (executor report on signed 6d1400851, gate receipt 92d8041b26b73a5e61e80d42bff19f5dbc0721cf75551cd62fd625292bae7b9d)
- Gate command 7 exits 100. It runs `bboxy450-term-mux --lib --run-ignored only`, selecting:
  - `isolated_actual_store_worker_full_host_heap`
  - `isolated_four_actual_store_workers_full_host_heap`
  - `isolated_parent_work_overflow_preserves_controls_and_reaps_real_probe`
  - `actual_consumer_store_fork_frontier`
- Commands 1–6 are green: the Store exporter, file custody and the real Rust bootstrap witness.
- First four-worker attempt: three workers qualified Hello, then hit the 30 s job deadline. They had spent 15.64–16.00 s in snapshot and 30.46–30.69 s of process-owner wall time.
- Second attempt: four permits and zero qualifying Hellos against the producer's 250 ms entry window.
- The parent overflow probe passed only on its configured retry.
- Utility-QoS reproduction over 576 chunks: 32.28 s with two rests per chunk, 15.46 s with one.
- Log: `target/dream-c1-repair/joined-settled-probe.log` in workspace 01M8BTWEV0NSM3GPP2VW8403NF.

## Current main (Oracle, 2026-11-27)
The evidence above predates `tasks/2026-11-26-harness-native-timing.md`, landed at `201e17ef1` with harness `7cd999f`. That change made the worker path-independent and re-pinned it (`crates/bbt-mux/src/logic/dream-c1-worker-build.txt`, executable `1a763a71…`). It also moved the bundled readiness probe to default QoS (`d41be5c38`) and the bundle receipt to `Contents/Resources`. Full `make test` is green on `5ae48333a` (command 7 is not part of it). Re-run `make test-dream-c1` on current main first and work from that measurement, not from the numbers above. The approved tuple in step 4 is the current pinned one.

## Do
1. Write down the producer/consumer timing contract for these witnesses. That means:
   - what the 250 ms entry window and the 30 s job deadline each bound;
   - who owns each clock;
   - what is product behaviour and what is the test pacing itself, such as two rests per chunk.

   Put it in the contract record in `verification/contracts.toml`, next to the Dream C1 entries.
2. Decide for each budget whether it is a product limit or a test timing the machine.
   - A test timing the machine: rewrite it to assert ordering or outcome, such as permit, then Hello, then reap. Do not assert elapsed wall time.
   - Store pacing that is wasteful, such as two rests per chunk where one is enough: fix the pacing in the product and record the before and after measurement.
   - A product limit that is genuinely too small for the work: stop and report the measurement and the decision you need. Do not raise it yourself.
3. The parent overflow probe must pass with retries 0.
4. No ignore, skip, serialisation or widened worker approval. Keep the approved worker tuple exactly as it is.
5. `make test-dream-c1` exits 0 on the final commit. Then `make land`. Report once.

Disk is tight (about 30 GB free on a shared machine). Delete your own `target/formal` scratch after formal evidence is recorded, and keep at most one full gate running at a time.

## Oracle ruling 2026-11-28: keep the 30 s deadline; rest for CPU used, not per step
I read the single-worker receipt from your signed run (`c1-job-MJwAli/isolated-host-heap.json`, `jobs[0].host_metrics`). The job is not short of time. It is almost all rest:
- `process_owner_wall_ns` 30.62 s, of which `reactor_rest_wall_ns` is 28.62 s;
- `reactor_cpu_ns` 9.5 ms across 911 reactor units, and `process_owner_thread_cpu_ns` 43.5 ms;
- `snapshot_active_wall_ns` 13.73 s for `snapshot_active_cpu_ns` 114 ms, with `rest_wait_ns` 9.96 s;
- child CPU 0.33 s (user plus system).

All the real work fits in about half a second of CPU. `dream_c1.rs:45-55` rests "nine times `max(UNIT, used)`" with `UNIT` 1 ms and `REST` 9 ms. A step that uses 10 µs is charged as 1 ms and rests 9 ms, so the floor, not the work, sets the wall time. That is wasteful pacing (step 2, second bullet), not a product limit that is too small. **The 30 s deadline stays.** No producer change and no new tuple.

### 6. Pace by measured CPU
- Keep the product's duty-cycle bound: at most one part busy for nine parts resting, per owner.
- Charge rest from the CPU actually used, `9 × used`, carried as debt across steps. Pay it when the debt reaches one `REST` (or at the next admitted span or permit, which the existing debt model already requires). Do not charge a 1 ms floor per step.
- Fixed `sleep(REST)` calls that are pacing, not waits for IO or a child, go through the same debt. IO and child waits (`wait_io`, `park_timeout`) are bounded waits, not charged rest; keep them as waits.
- Update the debt model and contract for the new charge (`verification/`). The existing broken control that skips payment must still fail. Add one that restores the per-step floor and shows the single-worker witness missing the deadline, so the reason is checked.
- Tests that assert `>= REST * n` or `min_inter_unit_gap_ns >= 9_000_000` change to the duty-cycle bound: total rest ≥ 9 × total charged CPU.
- Report before and after: the same `host_metrics` fields above, for the single and the four-worker witnesses.

### Hello refusals
Re-measure after step 6. The reactor's fixed rests also delay reading a Hello. If workers still miss the 250 ms entry lease, report the consumer's time from spawn to its first read of the Hello socket, per worker. Do not change the lease or the producer.

Verify unchanged: `make test-dream-c1` exits 0, then `make land`.

## Oracle ruling 2026-11-28 (second): the fixture names its seat launcher
Confirmed from source: `crates/bbt-mux/src/adapters/mod.rs:94-100` puts `bundled_harness()` ahead of the bare name. So the managed-seat launch runs the pinned sibling worker, and the PATH shim at `crates/bboxy450-term/tests/dream_c1.rs:113` never runs. The admission file the shim writes is then never created (`:940`). Command 16 never ran before, because command 7 stopped the gate first. This is a latent fixture bug, and it is yours. `crates/bboxy450-term/tests/dream_c1.rs` is now in `allowed_paths`.

### 7. Explicit launcher in the fixture
- The fixture's managed seats name the private shim explicitly, through the spawn command override that already exists. Report its exact name and `file:line`.
- Do not change `bundled_harness()` or the launch order in product code. Do not add a new product environment variable. If no override exists, stop and report.
- The worker sibling and tuple stay pinned. `dream inspect-c1` still reaches the pinned worker through the shim, as it does now.
- Assert that the admission file exists for each managed seat before waiting on the open-source catalogue. A missing launcher must then fail by name, not as a catalogue timeout.

Verify unchanged.

## Oracle ruling 2026-11-28 (third): a clean exit must not cancel its own seal — authorized
Confirmed from origin/main source. This is a product defect, not a fixture defect:
- `crates/bbt-mux/src/logic/harness_status.rs:420-441` seals closed-exit activity only when `peer_lost && all_at_head`, exact-PID `kill(pid, 0)` returns ESRCH, the current session is durably closed and the authority still matches.
- `crates/bbt-mux/src/logic.rs:9466` sets `native_reader = None` on `Event::Exited`. That drops the task holding `_source_follower` (`harness_status.rs:671`), and `SourceFollower::drop` (`:21-27`) sets `stop`. The follower returns before it reaches the ESRCH check.

So a seat that exits cleanly races the seal it alone can publish, and usually loses. The retained journal then stays `deferred`, `started=false`, as in `/tmp/dc1-4RSzfx` and `/tmp/dc1-wfqQPs`. The fix is inside `crates/bbt-mux`, which is already in `allowed_paths`.

### 8. The bound follower finishes its exact-exit check
- When `stop` is set, the already-bound follower makes **one final bounded pass** before returning: drain its watched sources to head, then run the existing exact-exit block unchanged. That means the ESRCH check, `current_closed`, `current_session`, `authority.matches`, then `publish_closed_exit` CAS and `dream_activity`. If any guard fails, it returns without sealing, exactly as it does today.
- Bound the final pass with the existing limits: the job deadline, and the per-read bounds the follower already has. It must not park, spin on the CAS or retry. Close, restart and a new generation still stop the follower; the same guards refuse a stale seal.
- Do not add a new owner, thread, channel, environment variable or state. Do not keep `native_reader` alive past exit. Do not change the producer, the lease or the tuple.
- **Model:** extend the existing C1 closed-exit activity contract (`verification/contracts.toml:1809`) with a stop-before-seal action. Add the liveness property: a durable current close plus exact process absence eventually seals, under weak fairness of the follower's final pass. Add a broken control where stop returns immediately; it must fail that liveness property. Keep the existing safety controls: stale CAS, delayed callback, A→B→A generation.
- **Witness:** add a Rust test where a source seat exits with status 0 after a durable `SessionClosed`, and the closed-exit bit is sealed. Then command 16's held-read witness goes green.

Verify unchanged: `make test-dream-c1` exits 0, then `make land`.

## Oracle ruling 2026-11-28 (fourth): land what is green now; command 18 is a new task
Zoom out: this gate has now shown three separate defects in a row (pacing at 7, launcher at 16, exit seal at 16). Each one was real and is fixed on signed `0449db5b1`. Ten verified commits are waiting behind a gate that keeps finding the next layer. Working software over perfect software: they land now.

Command 18 is a different subsystem. It covers app control-path latency and status lookup while a source read is held. The broad authority asked for (shared input, messaging and cached status) is outside this task's brief. Granting it would turn a pacing task into an app-wide latency investigation. I will brief it as its own task, `tasks/2026-11-28-dream-c1-held-read-controls`, with a baseline-first method.

### 9. Land and report
- Do not touch command 18 or `crates/bboxy450-term/tests/dream_c1.rs:1045`/`:1489` in this task.
- Run `make land` on `0449db5b1` (rebased). It must pass on its own gate.
- **Acceptance for this task:** in one `make test-dream-c1` run on the final rebased commit, commands 1–17 each exit 0 in the gate's per-command logs, and command 18 fails only with the `not_found`/p99 assertions above. Report the gate directory and each command's exit. Your existing run `bboxy-dream-c1-gate-bd9ol8_k` is enough if `make land` does not change the source; otherwise run it once more.
- Report once with `task_report`: commits, `make land` exit and log, the before/after `host_metrics`, the old-floor control result, the override name and `file:line`, the closed-exit model and control, and the command 18 evidence (`target/dream-c1-closed-exit-8c96d2f1a-warm.log`, `command-17.log`, host load).

## Oracle ruling 2026-11-28 (fifth): verify is `make land`; all of command 18 moves to the follow-up
I read `bboxy-dream-c1-gate-bd9ol8_k/command-17.log` myself. Command 18 runs 13 app tests: 6 pass and 7 fail, each on both tries, in 874 s. Not every failure is `not_found` or p99, so step 9's "fails only with" clause is withdrawn. Every one of those seven failures now belongs to `tasks/2026-11-28-dream-c1-held-read-controls`.
- **Verify for this task:** `make land` exits 0 on the rebased final commit.
- **Also required in the report:** the `bboxy-dream-c1-gate-bd9ol8_k` run on `0449db5b1`, showing commands 1–17 exit 0 and command 18 exit 100, with per-command exits listed. If `make land` rebases onto changed source under `crates/bbt-mux` or `crates/bboxy450-term/tests`, rerun `make test-dream-c1` once and report that run instead.


## Report from seat:01M8GAPWWPP5V7213X22D5HSP5

Landed signed a8003afbf on main with `make land` exit 0 and native/Java parity across 1,015 models. Replaced per-step floors, redundant pacing rests and staged-line copies with measured CPU debt, made the private seat launcher explicit, and let the bound follower finish one exact-exit check. In the runtime gate, commands 1–17 passed; the original Store workers and overflow passed at zero retries, the paired old floor hit Deadline and reaped, and the app status-zero exit sealed its latch. Command 18 exited 100 with six passes and seven failures, assigned to the separate app-controls task under the fifth ruling.

```json
{
  "op_id": "01M8H518RB7WSTKA342M9JQQ0J",
  "commit": "a8003afbf941ca63cd386d0c7d48274ea7b38fc9",
  "report": "[[reports/2026-11-28-dream-c1-measured-debt.den]]",
  "verify_result": {
    "command": "make land",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Signed landing and native/Java model parity on current source",
        "reference": "/Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M8GAPHE4GQ7D5QG6K9CKY3HD/target/dream-c1-land-a8003afbf.log"
      },
      {
        "criterion": "Archived formal evidence retained after checkout scratch removal",
        "reference": "/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/3c4aa3e37ee2c5755e6383a3902a7e7239b7294c4a793bbf8cb74e32ce886f1e/evidence.json"
      },
      {
        "criterion": "Dream C1 commands 1–17 passed and command 18 exit 100 recorded",
        "reference": "/private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/receipt.json"
      },
      {
        "criterion": "Actual deadline and reap under the old floor on identical Store input",
        "reference": "/private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/runtime-store-step-floor.json"
      },
      {
        "criterion": "Successful exact closed-exit and immutable page witness plus seven follow-up failures",
        "reference": "/private/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-bd9ol8_k/command-17.log"
      }
    ]
  }
}
```
