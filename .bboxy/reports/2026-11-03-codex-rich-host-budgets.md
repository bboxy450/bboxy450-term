---
type: report
title: 'Rich host four-view budget row: actual UI measurements, whole-frame budget unproven'
task: '[[tasks/2026-11-03-codex-rich-host]]'
supersedes: '[[reports/2026-11-03-codex-rich-host-cancel-fence]]'
commits:
- 37b8f2f8e2579bc0e720a269be92427abed12c16
author: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
created: 2026-11-03T21:52:16.445Z
verify:
  command: BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-harness-native
  exit_code: 0
id: 01M6HPC2150C7XRW5ASEAXPSHV
updated: 2026-11-03T21:52:16.445Z
number: 18874
created_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
updated_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
authorship:
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  kind: seat
  public_key: YKv54MXoFa4rl83lF7g/hJHRLfs8fsSGu2+nb6F1XqY=
  process_generation: '1'
provenance:
  digest: sha256:f1f93e0e6968b4c16cf3b2d176a08424adb4fb3392eac4bc4e1984ee75d72550
  frontmatter_digest: sha256:bdbf450584da3150c2e4a7295a50b3e4a478d5d05ffa1316c0012ab013cc93eb
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  sig: X1ZlwJ/pDqKWRK/MyFnDHhBAtZ2jHhESElw74NMewdBaUYam04JjEGUJy1j0SB6gD6RYOeiV+WF0ifwD5TmIDg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#104276
---
# Rich host: actual four-view budget measurements

Task [[tasks/2026-11-03-codex-rich-host]], Executor seat 01M6HCYVGE7CJ8KK6PB20SPF3D. This supplements and supersedes [[reports/2026-11-03-codex-rich-host-cancel-fence]] (#18711) for the latest source/measurement handoff. Its Cancel correction, source provenance and remaining integration work remain applicable. The Oracle's budget review 01M6HHRY83DRVCF82PE6H7FYN1 was correct: four conversions of one Frame were not an actual four-view App budget witness.

## Mechanism and witness

- `Makefile:2022` adds the canonical `make bench-budgets` term row, advertises it in `make help`, and includes it in `make test-harness-native`. The row prints p99 values, **under load UNMEASURED**, **whole-frame UNPROVEN**, and **heap UNMEASURED** with a 32 MiB allocation per view.
- `crates/bboxy450-term/src/rich_faces.rs:115` delegates production `rich_drain` to the same elapsed-clock loop exercised by the fixture (`:119`). Production still uses Instant. The test injects one shared clock across all four views, with a 300 us deadline and 100 us per visited drain. Exactly three publications are consumed on the first frame, one remains deferred, and round-robin consumes it on frame two. Every latest publication remains observable; the test asserts the counts rather than inferring success from timing.
- `crates/bboxy450-term/src/rich_pane.rs:209` provides a nonconsuming diagnostic publication counter. The worker stores it with Release **after** publishing Latest (`:176`); the fixture observes with Acquire before starting a simulated frame. It does not insert synthetic snapshots, read the socket on UI, consume Latest prematurely or change product admission policy.
- `src/rich_faces/budgets.rs:72` uses four production Pane/Unix Bridge/HarnessView actors in an actual headless App owner. One private Python process serves four independent Unix listeners and subscribers, with four session identities. The corpus is v1.1-all-items with its terminal SessionClosed omitted to keep these fixture sessions live. All four retain durable cursor 31, 13 items and zero duplicate durable events. The test drives 128 concurrent four-view Focus publication rounds, invokes actual `App::rich_drain` and `App::rich_paints`, and asserts final state retention. Hiding or occluding removes all native view actors; the fixture session process stays alive.
- Both UI spans include the actual production work, including cursor bookkeeping, old-snapshot retirement, pane-body composition and Terminal toggles. Worker projection duration is taken from actual snapshots consumed by a frame. Raw Arc addresses identify changed snapshots without retaining an extra Arc that would artificially move destruction outside the measured UI span (`budgets.rs:268`). No raw pointer is dereferenced.
- The old capture test is renamed `rich_native_parity_and_composition_microbench` and its JSON explicitly labels the four-copy conversion measurement and whole-frame UNPROVEN. Its native visual comparison remains separate.

## Observed measurements and limits

| Measurement, four visible views | p99 ms |
| --- | ---: |
| Actual App aggregate rich drain | 0.052250 |
| Actual App aggregate rich composition | 0.061334 |
| Measured UI rich slice (per-sample drain + composition) | 0.099709 |
| Latest consumed workers, summed projection elapsed | 4.018416 |

Cold worker projection elapsed sum: **1.495000 ms**. 128 driven rounds, 128 sampled UI frames; four cursors at 31, 13 items each, zero duplicate durable events.

The UI-rich-slice p99 is computed from each sample's drain + composition, not by adding independent p99s. Worker numbers are sums of **off-thread elapsed durations associated with snapshots drained by that UI frame**, not CPU time, UI blocking time or whole-frame wall time. They cover the latest consumed HarnessView::render plus Frame-to-Paint conversion; intermediate projections replaced by Latest are not counted. They exclude Bridge background preparation/Markdown, the rest of App projection, GPU encoding and presentation.

**The observed worker projection elapsed sum is above the 0.6 ms projection allocation. This does not demonstrate that the ICD's aggregate projection budget holds.** The UI measurements cannot substitute for that result. The JSON exposes `worker_projection_sum_p99_over_allocation=true`, and the overall result is UNPROVEN. Measurement/no-loss test completion exits 0; it is not an allocated-budget PASS. The owner must resolve the aggregate projection budget's measurement/scheduling disposition before anyone claims that allocation is met. The separate Side B four-view injected-clock/no-loss witness remains unadopted until its published result is integrated.

There is no controlled token-stream/system-load experiment here: under-load behavior is **UNMEASURED**. Heap usage is **UNMEASURED**; 32 MiB is an allocation, not an enforced or measured per-view bound. The cold worker projection sum is reported separately from the 128-round warm fixture. No full 8.33 ms frame or 4 ms CPU-path verdict is claimed.

## Verification and source binding

Signed source commit `37b8f2f8e2579bc0e720a269be92427abed12c16`, parent `248d18a79a071d1df69ef5e04e9df7b593a12e07`, worktree `/Users/feral/mydev/bboxy450-term-codex-rich`, branch `codex-rich-pane-20261103`. Tests use signed private harness `d5e7249a01bf4830c79979aa7a8e2f17753be83b`; the committed gitlink stays `0c403165e815c2f7bf77ccac293784b021f42f96`, deliberately unstaged. No dependency pin was published or invented.

| Actual command | Exit | Local log |
| --- | --- | --- |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-harness-native` | 0 | `/tmp/codex-rich-budget-native.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make check-rich-host` | 0 | `/tmp/codex-rich-budget-lint.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-formal` | 0 | `/tmp/codex-rich-budget-formal.log` |
| `make test-licenses` | 0 | `/tmp/codex-rich-budget-licenses.log` |
| `make help` | 0 | `/tmp/codex-rich-budget-help.log` |

The canonical `bench-budgets` goal ran inside the current native Make target and printed the term row above. Its exit 0 proves collection and the scripted deferral/no-loss assertions, not the whole-frame allocation.

Current-source native Rust/pinned Java TLC full reachable state, transition and verdict parity passed: **286 models, 34 crates, 117 contracts**; evidence SHA256 `c5ee070210611839caae43430f952d18b05bbaaa5e524980c6e0716ec5d01a5f`, source `37b8f2f8e2579bc0e720a269be92427abed12c16`, `dirty=true` because of the private gitlink.

Timing is specialized implementation evidence. The existing formal contracts still explicitly leave renderer/resource cost implementation-only; their implementation-test/source hashes now include the real four-view witness. The injected elapsed-clock extraction preserves the production drain policy, and the publication counter is diagnostic rather than a product scheduling decision. Finite model parity does not prove arbitrary Rust, wall-clock deadlines or heap bounds.

The current native run also rechecks socket delivery/zero PTY bytes, interrupted process/resume, captured session/epoch/generation Cancel, the three causal controls and offscreen native parity. The controls still require their specific assertion exit 101 and restore any changed source exactly. Offscreen row mismatch is 0.0 (required ≤0.01). The last generic boundary suite was 39 passed at d4199242, recorded in #18711; this measurement-only cut makes no new process-boundary claim.

The full parity gate and clean formal receipt were last actually run at d4199242 and exited 2 because of the same intentionally unstaged private gitlink. They were not repeated a third time against an unchanged prerequisite, and no fresh whole-task verification result is offered here. The current formal evidence is bound to the private checkout and remains `dirty=true`.

## Handoff

The canonical row and actual aggregate drain/composition witness are now present. **Overall acceptance remains blocked** on the published combined standalone+B1/B2 harness pin and a signed task amendment permitting its gitlink, plus disposition of the still-unproven aggregate projection budget. Final combined-pin capture Options/IR 1.2 migration, real rich-harness App detach/reattach, heartbeat, Side B budgets and live-load measurements remain as in #18711. Published standalone alone still lacks Bridge::connect and is not substituted for the combined cut.

The report/summary commit contains evidence only; task_report carries its signed SHA. `.bboxy/runs/codex-rich-host.json` preserves earlier results and appends current budget commands, exits, source/log/data hashes and measurements. Raw logs and fixture data remain outside Git. The only intended uncommitted source difference is the private harness checkout. No push, merge, publication, trusted install, global setting/credential change, recruitment or operator OS input/desktop capture occurred.
