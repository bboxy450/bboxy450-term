---
type: report
title: QoS primary landed; C1 re-enable follow-up blocked
task: '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
author: seat:01M6S48V44QDK7SE9H4EDFPXVT
created: 2026-11-07T07:38:54.659Z
commits:
- e818118c1ebe4f427e7d6c0cac1f167776ef695b
- 09623ce039162f722e181858732dd380e6ef8a07
- d34e2f9e59589643408c69dcf76bab211cc26a31
- 148878086965929ea4d3fc15ed28aa6e80c3cc7c
- 630297a6409ac38f7573d12abb67314106e76f5b
- 314b419d55edf0142276eb46c8207841523ff025
- 741c15f328b52084cac9edf6845b692dc1d18e57
- ee34f00a4eb8144bcea2ba31d5f7cb67b2711651
- 3e4868c5362c89175c7deddf4b07aa773d7288ff
- 8eb80932e77c12ea2d79e912748253fc25d035d5
- 9132893cfb17eeca6366e43ba68e1b5bbdb007c1
- 12cdf01e53f17e52de6f74c6b2bce593cce69c19
- c22b70523aa9a99c4038bc2db66d9e456a449c05
- 5341e4ef23a53d318d0d2b9dd43c6804b11ee314
- ee60f8184aa1b0181d58a2f55bffb99501eca7d0
- 0350aa184a0f0598616c72e93922abefb8919b50
- fe8f5bf55c464cabdc8230fbc16ae51ba0086525
- 38d99096f4f9c475a5547cf2192982af6ff7a5c0
verify:
  command: make test-dream-c1-qos
  exit_code: 2
supersedes: '[[reports/2026-11-06-qos-builds-c1-budget-blocked]]'
id: 01M6TF4C1SJ7V9WV0T7Z1JJ2PK
updated: 2026-11-07T07:38:54.659Z
number: 93377961366239
created_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
updated_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
authorship:
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  kind: seat
  public_key: 9eWyjSVdBKBGoA72B1nDgpObDHOHT0P0DKq9XWjgcwI=
  process_generation: '1'
provenance:
  digest: sha256:9ec707647c993e5a0f0bc938816528c3f233612fca13d9e167e7dfd2c9a7592e
  frontmatter_digest: sha256:b476d351639296192abbb6e7cc63d303f0e99343e44de722db2b1f8274c3ff06
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  sig: bD1p0C/Lo4sb7KN7pbNZ6K3P8ZXmIwf2JKIEKj44iA3x2oS9hE47kNsPKCiaquk9N3sb+US+jHi0KcAvVjiWBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150232
  event:
    id: 01M6TF4DDE3HAWCMH0E5AESM3Y
    sequence: 150232
    hash: 0340c761c80a225cedf57f47ae644115d4873caab6768346c416ba4fcb113e01
---
# QoS primary landed; C1 re-enable follow-up blocked

The primary QoS implementation LANDED through `make land`, exit 0, at `ee60f8184aa1b0181d58a2f55bffb99501eca7d0`. Its complete native/TLC gate passed: 679 models, 254 contracts and 34 inventoried crates. The authorized headless-compiler follow-up and C1 re-enable changes are signed, committed and **unlanded** at `fe8f5bf55c464cabdc8230fbc16ae51ba0086525` (two commits above primary). The task is incomplete; no `task_report` has been called.

Current brief: [[tasks/2026-11-06-qos-builds-never-starve-mcp]], etag `43d8804575a7bbc370078e6583ccb322cdd9d7bcf5052cc29497417300f329f7`. The final amendment allows persistent low-load C1 failures to become separate C1 budget tasks. Two failures still need that disposition. This supersedes the earlier unlanded-primary account in [[reports/2026-11-06-qos-builds-c1-budget-blocked]].

## Implementation and files

- `Makefile:8`, `scripts/qos.sh`, `scripts/cache-env.sh`: Make recipes, private Cargo/rustc/rustdoc shims and their descendants use macOS background process policy for compilation, linking and Clippy. Ordinary tests use utility. Non-macOS keeps nice. Scoped background state is cleared by a native self call before selecting default/utility; an external taskpolicy change could not clear inherited internal background state on this host. sccache uses a separate background-policy namespace. Author checks used cache mode off without mutating a shared daemon.
- `scripts/landing.py:139,240`, `scripts/qos-policy.py`, `LAND.md`: prebuilds run background; the nextest controller stays unclamped and each test gets its own utility/default policy. Explicit latency/CPU/frame measurements retain default; compiler children stay background. Existing target runners and DYLD forwarding are restored before the test. Direct mixed libtest batches containing named measurements remain default. No blanket C1 unit-module opt-out remains.
- `scripts/landing.py:289`, `scripts/land-fast.sh`: each gate prints wall time, descendant CPU seconds, allocated target-size delta and real exit. A waiting Python parent preserves Darwin child rusage across the two Cargo phases. The final total covers the whole tree.
- `crates/bboxy450-term/src/qos.rs:8`, `src/main.rs:10007,10153`: supervisor/logic/MCP startup requests initiated (25), desktop/actual UI requests interactive (33). Tool flags do not promote an unrelated helper. Debug private startup witnesses exit before app/account work.
- `crates/bbt-rt/src/qos.rs:31,50,59`, `src/lib.rs:59`, `crates/bbt-mux/src/logic.rs:2309,2977`, `src/knowledge/routing.rs:84`, `src/mcp/bridge.rs:594`: logic, MCP and shared IO workers use initiated; indexing/publication IO enters utility and restores its pooled worker, including caught-job panics. The existing dedicated PTY reader remains interactive, as the amended role ceiling permits.
- `scripts/qos-probe.py`, `crates/bbt-mux/src/lib.rs:59`: private signed native mux/bridge/socket/seat, real authenticated self_status and ten independent busy-loop children. Each phase has 100 measured round trips after ten warmups. Children are owned and reaped; no operator app, account or socket is used. Real-load latency is observational. Incorrect workload policy and an explicit injected over-threshold delay are deterministic failing controls.
- `crates/bboxy450-term/src/ctl/workspace.rs`: the private peer stays open via a channel through timeout-restoration assertions; the 100 ms bound and no-retry assertion remain. `crates/bbt-supervisor/src/ring/writer/crash_tests.rs` and the follow-up `crates/bboxy450-term/tests/headless.rs:1172` honor nonempty RUSTC/RUSTC_WRAPPER for standalone fixture compiles; their assertions remain.
- `verification/contracts.toml`, `verification/coverage.toml`: BuildQosResponsiveness is explicit implementation/performance evidence, with deterministic negatives and OS-scheduler limitations. It is not a new TLA scheduler model or a refinement claim. `scripts/test-formal.py` copies the Make QoS shell into its private publication fixture while preserving all five original publication refusal assertions.
- Every touched script is: cache-env.sh, ci-local.sh, invariants.sh, land-fast.sh, landing.py, main-watch.sh, qos-policy.py, qos-probe.py, qos.sh, test-formal.py and test-landing.py. Ordinary interpreted leaves are utility; timing orchestration remains unclamped. The CI fixture now reads native calling-thread class instead of a dynamic ps upper bound that once sampled an IPC-boosted priority of 46.
- `scripts/quarantine-test.sh`, ATTRIBUTION.md, license files and ordinary C1 runtime launch guards are unchanged. Existing origin/policy notices remain attached.

The authorized harness exception landed separately at `38d99096f4f9c475a5547cf2192982af6ff7a5c0`: the redundant paint.clone fix was already landed upstream and dropped on rebase; the signed local TLC change adds -lncheck final. Harness canonical `make land` exited 0 (127 ACP lib tests, 281 models, 78 contracts, one crate). Term pins that SHA through canonical make bump-harness. The private harness worktree was removed after its clean/ancestry/open-file checks. Its archived evidence is under the task's common-Git evidence directory.

## Verification

All author Cargo checks used BBOXY_CACHE_MODE=off, dev/test debug=0, opt-level=1 and debug assertions=true, with one build/test at a time.

| Source / command | Actual result |
| --- | --- |
| ee60f818: make land | 0; signed commits, strict Clippy, harness 126/126, host 53/53, rt 19/19, app 453/453, mux 507/507; dependent checks and full native/TLC parity passed |
| fe8f5bf5: python3 -I scripts/test-landing.py | 0; 204 fixtures, 106.974 s |
| fe8f5bf5: python3 -I scripts/qos-probe.py --check | 0; 3.798 s; native requested class 25, ten background-self flags 1 and ten ps priorities 4 |
| fe8f5bf5: make test-formal-contracts | 0; 1362.237 s; 43 API/model witnesses, strict Clippy, test-qos, both deterministic negatives |
| fe8f5bf5: headless selector fixture via quarantine-test.sh, package bboxy450-term, --test headless, exact named filter | 0; one real fixture test, 14.888 s |
| fe8f5bf5: cargo clippy --locked -p bboxy450-term --test headless --target-dir target -- -D warnings | 0; 3.251 s |
| 0350aa18: make test-dream-c1-qos | 2; thirteen nonempty one-test selections: twelve passed, admitted-MCP failed |
| fe8f5bf5: isolated admitted-MCP selection | 100; same missing-runner/stale_context failure; no third identical retry |
| fe8f5bf5: make test-dream-c1-qos | 2; twelve active nonempty selections: ten passed, two failed; admitted-MCP printed as explicitly deferred |

The requested verify chain (test-landing.py && qos-probe.py --check && make test-formal-contracts) therefore exited 0 at fe8f5bf5. This does **not** discharge the remaining C1 re-enable/landing requirement.

Native helper units passed (two rt plus two app), as did eighteen QoS/cost tests. Five real startup process witnesses returned [25,25,25,33,33] for supervisor/logic/MCP/desktop/UI. Routing's native success-and-caught-panic test observed utility inside and initiated after return in the primary mux suite. A live primary Cargo/rustc process witness also records priority 4 for both descendants. Cross-process getpriority(PRIO_DARWIN_PROCESS, otherpid) is not used to infer another process's internal background flag; the authoritative probes query self.

At fe8f5bf5, standalone MCP probe p50/p95 in ms:

| Phase | p50 | p95 |
| --- | ---: | ---: |
| Unloaded | 1.592292 | 3.783542 |
| Before: default-policy saturating load | 4.427084 | 6.434625 |
| After: background-policy saturating load | 1.463834 | 1.961250 |

This compares load policies using the same native fixture/source, not two historical app binaries. The later nested test-qos run reported default/background p95 7.420792 / 3.273917 ms. Both are recorded rather than selecting only the lower result. The threshold was 250 ms. Forced default-policy control exited 1 with qos_ok=false and p95 4.862875 ms; injected-delay control exited 1 with qos_ok=true, latency_ok=false and p95 271.373042 ms. Make asserts the specific JSON fields and exact negative exit, so build/transport errors cannot substitute for a broken control.

Primary land cost: total wall **5100.288 s**, child CPU **4751.873 s**, target growth **+8,437,059,584 bytes**. Full formal was 3394.483 wall / 2548.113 CPU / +8,294,182,912 bytes; app bin build 418.066 / 571.445; app tests 577.465 / 825.234; mux tests 563.899 / 643.616. Every other per-step cost is in the raw log.

Primary source-bound formal receipt is at `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/cee00e0b575dc715aef9897aca61d40ce7a9ff3c917707f97d1dc07b7577856c/evidence.json`; SHA256 matches its directory name, source ee60f818, dirty=false. It proves the primary source, not the later unlanded follow-up. The follow-up needs the canonical current-source formal refresh after its C1 gate is resolved.

## Remaining C1 failures and exact decision

All thirteen load-family ignores were removed in 0350aa18 with no assertion, budget or product-source changes. All five wake-up process tests, the custody test, both final-authority tests, registration, source failure, source-error capacity and held-catalogue controls passed individually in that first pass.

1. `dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs` failed twice: provisioning reports RunnerUnavailable, but the admitted API returns stale_context (`crates/bbt-mux/tests/dream_c1.rs:272`). The existing open task [[tasks/2026-11-06-dream-c1-honest-capture-errors]] (#93377961366165) owns exactly this mapping and re-enable duty. fe8f5bf5 replaces its load-family ignore with an explicit known-defect reason and prints the deferral; it does not claim that defect fixed. Its future fix must also remove the explicit C1_DEFERRED_TESTS entry in qos-policy.py.
2. A second per-case run failed `dream_c1_held_private_delete_catalogue_keeps_mcp_gui_control_owner_independent` at logic.rs:14847: **256190 us > 250 ms**, result busy.
3. That run also failed `dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners` at registry/dream_c1.rs:3847: **0 winners versus required 1**. No assertion was relaxed and neither was quarantined or re-ignored.

Machine samples after the first pass showed 72.77% idle; the admitted-MCP isolated retry's second sample showed 58.11% idle. The per-case log includes actual load averages (not a synthetic claim of exclusive machine ownership). There was no simultaneous build/test from this seat during those executions.

**Decision needed:** assign the latter two C1 failures under the amendment's separate-task escape and transfer their re-enable duty with explicit known-defect ignores, or supply a C1 source fix. Their production control/storage logic is outside this QoS-only scope. The existing capture-error task already covers the first. Do not reinterpret a twelve-test first pass as stable final acceptance.

Blocked message `01M6TDFAC08D1JH0MFBE8NYR2E` was durably accepted and native-queued, wake=now; model consumption was unverified. The task remained unchanged at the final scope refresh. No repeated status request was sent.

## Evidence retention and limits

Primary raw log/summary and real Cargo/rustc witness are preserved in `/Users/feral/mydev/bboxy450-term/.git/bboxy-qos-builds/01M6S3M8WTNZ32TMZ7YX9VTEJR/term-ee60f8184aa1b0181d58a2f55bffb99501eca7d0/`.

Follow-up raw logs, command/exit JSON, both control JSONs, load samples and the exact thirteen-file/test removal inventory are preserved in the sibling `term-fe8f5bf55c464cabdc8230fbc16ae51ba0086525/`. Its `manifest.json` SHA256 is `0aaf715e6f89df5170a22404f8cab67b3478ac11bd49ada8482b440866077f31`. No raw logs or test accounts enter Git.

Worktree is clean; git cherry origin/main HEAD lists only 0350aa18 and fe8f5bf5. Both signatures verified with Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0; the primary gate verified its entire signed cut. No follow-up make land is claimed: its known red C1 prerequisite was not bypassed. Full root make test, full test-boundaries, ordinary one-launch test-dream-c1, full headless suite, reinstall and operator-app actions were deliberately not run. Non-macOS fallback was source-reviewed, not runtime-verified. The pinned rust-objcopy tool warns that libLLVM.dylib is missing; the recorded successful commands still exited 0. No global toolchain repair was attempted.

The final requested reports/2026-11-06-qos-builds.md and the single completion task_report remain due after this blocker is resolved. Earlier artifact_claim operation 01M6T6WWYAB1T79QTS8WKZNVDM was held as artifact_not_committed despite its checkpoint's committed native projection; no successful responsibility claim is asserted or fabricated.
