---
type: report
title: 'QoS builds checkpoint: C1 journal CPU-budget fixtures block primary landing'
task: '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
author: seat:01M6S48V44QDK7SE9H4EDFPXVT
created: 2026-11-07T05:13:13.874Z
commits:
- 5341e4ef23a53d318d0d2b9dd43c6804b11ee314
- c22b70523aa9a99c4038bc2db66d9e456a449c05
- 12cdf01e53f17e52de6f74c6b2bce593cce69c19
- 9132893cfb17eeca6366e43ba68e1b5bbdb007c1
- 8eb80932e77c12ea2d79e912748253fc25d035d5
- 3e4868c5362c89175c7deddf4b07aa773d7288ff
- ee34f00a4eb8144bcea2ba31d5f7cb67b2711651
- 741c15f328b52084cac9edf6845b692dc1d18e57
- 314b419d55edf0142276eb46c8207841523ff025
- 630297a6409ac38f7573d12abb67314106e76f5b
- 148878086965929ea4d3fc15ed28aa6e80c3cc7c
- d34e2f9e59589643408c69dcf76bab211cc26a31
- 09623ce039162f722e181858732dd380e6ef8a07
- e818118c1ebe4f427e7d6c0cac1f167776ef695b
- 38d99096f4f9c475a5547cf2192982af6ff7a5c0
verify:
  command: env BBOXY_CACHE_MODE=off CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true sh -c '. scripts/cache-env.sh; sh scripts/quarantine-test.sh --locked -p bboxy450-term-mux --lib --target-dir target -E "test(=registry::dream_c1::registry_tests::dream_c1_registration_commits_close_cursor_original_op_and_source_key_once_after_reopen)"'
  exit_code: 100
supersedes: '[[reports/2026-11-06-qos-builds-blocked]]'
id: 01M6T6SM4J1VEQAW0PS6ZF1NTY
updated: 2026-11-07T05:13:13.874Z
number: 93377961366226
created_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
updated_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
authorship:
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  kind: seat
  public_key: 9eWyjSVdBKBGoA72B1nDgpObDHOHT0P0DKq9XWjgcwI=
  process_generation: '1'
provenance:
  digest: sha256:fb9db87f967bce4435db46272d90c0d2100e9af200946bb599ddb284b688ee53
  frontmatter_digest: sha256:c9765bcb272126fb4018ce0ea9838fd983b8a8aef270be6163a49165bc910565
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  sig: ug9eqnlvL6AqS2L19zPPfY4OltDdqgSQtGYwnBArtoT6gzWYXGwo+D0GR8kofxncKVhDFi61bcs8ukIWbN0vCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150182
  event:
    id: 01M6T6SNH58RR9ZPTMXAZMAXE3
    sequence: 150182
    hash: 8a09aef043a9fbce3fce6591683e1c8b994e0f01d47e49675a02326bff7a156f
---
# QoS builds: C1 native-budget blocker

The QoS implementation is signed and committed in the assigned worktree at `5341e4ef23a53d318d0d2b9dd43c6804b11ee314`, with a clean working tree. The term changes are **not landed**: the canonical gate remains red on existing C1 journal CPU-budget fixtures. The separate authorized harness checker fix is landed. This checkpoint supersedes [[reports/2026-11-06-qos-builds-blocked]]; it is not a completion report or a request to accept the task.

## Blocking evidence and decision

Three existing witnesses fail with `ResourceLimit` inside the real 1 ms C1 journal budget:

- `dream_c1_final_actual_authority_rechecks_row_head_grant_inode_aba_after_debt`: journal initialization at registry/dream_c1.rs:347 measured **1,045,125 ns**, then the fixture's register unwrap failed at logic.rs:14040.
- `dream_c1_final_actual_authority_held_clock_cpu_refusals_keep_owned_child`: the same initialization measured **1,360,250 ns** and failed at the same unwrap.
- `dream_c1_registration_commits_close_cursor_original_op_and_source_key_once_after_reopen`: its focused run, with its explicit default-QoS opt-out, measured **2,150,125 ns** at registry/dream_c1.rs:892; the replay operation unwrap failed at :3135.

Both authority witnesses are explicitly classified default. They passed a focused two-test run once (exit 0, 66.516 s), but failed in the 98-test native-C1 diagnostic (exit 100, 96 passed/2 failed). That diagnostic temporarily put all three native-C1 API modules at default; its broader module opt-outs were **removed**. The raw reproduction patch is `target/c1-native-budget-diagnostic.patch` against c22b70523aa9a99c4038bc2db66d9e456a449c05 (SHA256 `5bcd501d4b184d5e3442cb2c2e24f671b3d15d4cb9f482f36fbe9d53a007c1a2`). The isolated registration test also fails at default, exit 100. A scheduling opt-out alone has not made these fixtures reliable.

**Decision needed:** assign the C1 owner a journal/fixture follow-up that preserves the existing 1 ms production bound and makes these unattended witnesses valid. The current brief freezes QoS scope; product journal/fixture changes at these sites are outside it. No assertion, timeout, CPU limit, retry policy or ignore was weakened to make a gate pass. The tests remain enabled. No further identical full gate was run after the default-QoS diagnostic failed.

## Implemented mechanisms

- `scripts/qos.sh`, `cache-env.sh` and `landing.py:139,240` classify root Make/Cargo work: background compiler/linker/clippy trees, utility ordinary tests, explicitly listed default measurements. Compiler and rustdoc shims cover direct compiler children and doctest execution. Non-macOS retains nice. The cache namespace avoids legacy foreground sccache servers; author checks used cache-off.
- `scripts/qos-policy.py:80,88` supplies a stable command-scoped nextest target runner. Each actual test selects its policy; the controller stays unclamped. It preserves an existing environment runner, Darwin library paths and the previous runner environment. Private BBOXY-stripped environments can still resolve the Cargo shim. LAND.md lists the opt-outs; no all-tests default bypass was introduced.
- `crates/bboxy450-term/src/qos.rs:8` and main.rs:10007,10153 request supervisor/logic/MCP class 25 and desktop/UI class 33. Actual window entry claims interactive; headless measurements and tools retain the caller's class. Existing PTY-reader/UI interactive requests remain.
- `crates/bbt-rt/src/lib.rs:59`, mux logic.rs:2309 and mcp/bridge.rs:594 request initiated for shared IO, logic and MCP. Publication IO (logic.rs:2976) and workspace preparation/indexing (knowledge/routing.rs:83) run utility, catch job panics inside the scope and restore their worker class. The local routing witness checks success and caught-panic restoration.
- `scripts/landing.py:289` and land-fast.sh print per-command wall, descendant CPU, allocated target-size delta and real exits, plus total cost. The waiting parent retains prebuild child rusage across Darwin exec boundaries.
- `scripts/qos-probe.py` and mux lib.rs's private ignored `qos_probe::serve` use a throwaway repository, supervisor, seat, signing key and socket with the real native MCP Bridge/self_status. They do not use the operator's app or account.
- The workspace timeout fixture holds its private peer open through the unchanged restoration assertion. The ring crash fixture honors nonempty RUSTC/RUSTC_WRAPPER. The private publication-gate fixture now carries qos.sh and delegates only its policy bootstrap to real Python; all identity, clean-tree and exact normal-receipt assertions remain.
- `verification/contracts.toml:6` records BuildQosResponsiveness as implementation/performance evidence, with deterministic wrong-policy and delayed-reply controls; it does not claim OS scheduling is proved by TLA. RT coverage maps the helper.

Every touched script: `cache-env.sh`, `ci-local.sh`, `invariants.sh`, `land-fast.sh`, `landing.py`, `main-watch.sh`, `qos-policy.py`, `qos-probe.py`, `qos.sh`, `test-formal.py`, `test-landing.py`. Makefile, LAND.md, the runtime/startup files, the two fixture files and the verification maps are also changed. Existing ATTRIBUTION.md already supplies the canonical project/policy; origin notices were retained.

## Verification actually completed

Logs are under the assigned worktree's ignored `target/`; no raw logs were committed.

- Exact requested three-command chain reached exit 0 at the preceding 1ac5ca6b83daed61ff986b5b4c6dedf92719536d checkpoint, before a source rebase. It ran 204 script fixtures, the probe and 43 formal API tests, strict Clippy and the QoS target. Its following make land failed on a submodule rebase conflict, not those checks. This is **not** represented as a final-source pass.
- `make test-qos`: exit 0 at the earlier checkpoint: two RT tests, two app helper tests, five real startup process witnesses, and 18 native policy/cost tests. The latter include effective utility/default nextest children, descendant background inheritance, DYLD/runner restoration, private-environment shims, native rustdoc and retained child CPU.
- A rebased canonical gate ran all touched lib/bin tests successfully once: harness 126, host 53, RT 19, app 453 and mux 510, plus dependent checks and strict Clippy. It later stopped at the missing-QoS-shell formal fixture. The focused repair then passed all 33 formal-infrastructure tests.
- The next canonical gates exposed the C1 failures above. Latest full gate at c22b70523aa9a99c4038bc2db66d9e456a449c05: exit 2 (inner 77), mux 509/510 passed. The final single-test default-QoS check at the now-committed registration opt-out: exit 100.
- Current-source root full native/TLC formal evidence is **not complete**: the first whole-root formal attempt stopped at its self-test fixture; subsequent landings stop earlier at C1. The harness's separate 281-model proof below is not a substitute.

## Controlled latency and landing cost

Latest normal probe in the successful pre-rebase chain used 10 logical-CPU busy-loop processes and 100 measured round trips per phase (ten warmups), same native fixture binary:

| Workload | p50 ms | p95 ms |
| --- | ---: | ---: |
| Unloaded | 1.518792 | 3.507458 |
| Default-QoS load (before build-policy change) | 3.992333 | 4.924541 |
| Background load (after build-policy change) | 1.468333 | 1.783041 |

All ten loaded children reported internal Darwin background=1 and ps priority 4; MCP requested class=25; limit=250 ms. This compares load policies, not two whole application revisions. An earlier complete QoS-target run separately verified wrong-policy control exit 1 (qos_ok=false, latency_ok=true, p95 4.409291 ms) and injected-delay exit 1 (qos_ok=true, latency_ok=false, p95 267.751834 ms). Default-load latency is comparative evidence, not a required failing negative.

The failed 9132893cfb17eeca6366e43ba68e1b5bbdb007c1 landing printed total wall **3483.981 s**, child CPU **5230.024 s**, target growth **+1,645,162,496 B**. Its app-bin step alone was 635.077 s wall/1142.987 s CPU; its mux test step was 794.977 s wall/749.780 s CPU. The later c22b7052 gate printed **766.291 s wall**, **902.408 s CPU**, **+141,070,336 B**, exit 77. Per-step failures and totals are both recorded. Persistent formal archives and shared cache growth are outside the target delta.

## Authorized harness work and remaining duties

Upstream had already landed the equivalent paint Copy fix at 7764eea618d22b1f862790b34b0e69d2e8aceae8; canonical rebase dropped the duplicate local paint commit. The granted `-lncheck final` checker fix landed as signed Boxy commit **38d99096f4f9c475a5547cf2192982af6ff7a5c0** through harness make land, exit 0: strict Clippy, 127 library tests, and native/TLC full graph/verdict parity for **281 models / 78 contracts / one crate**. Receipt SHA256: `210f191020a7e0be4dda4c2370b946117d62cee682d007963106ce315f883200`.

The full 1.81 GiB source-bound archive and command receipt survive at `/Users/feral/mydev/bboxy450-term/.git/bboxy-qos-builds/01M6S3M8WTNZ32TMZ7YX9VTEJR/harness-38d99096f4f9c475a5547cf2192982af6ff7a5c0`. Root make bump-harness exited 0; only the gitlink changed. The private harness worktree was clean, on origin/main, had no open files (lsof exit 1), and was removed. Its final source/commit signature used approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0.

Primary term landing, the expressly after-primary headless compiler fixture fix, and removal/verification of the ten C1 load-family ignores remain pending. Known separate correctness-defect and real-producer/runner-required ignores remain untouched. No ordinary one-launch Dream C1 runtime gate, full root make test, reinstall or deployment was run. Native task_report has not been called; its single completion handoff remains due after these requirements are fulfilled.
