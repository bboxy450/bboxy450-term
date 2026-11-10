---
type: report
title: 'QoS builds implemented checkpoint: blocked by inherited harness Clippy'
task: '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
author: seat:01M6S48V44QDK7SE9H4EDFPXVT
created: 2026-11-07T01:24:08.885Z
commits:
- 4d390d4fa3e12283c1ad9985b547c89a8c49b1fd
- dada2aa33a0faa66b1c175378412367c05adbaa9
- ace479183f70bd1fcf1f82d4a11df555191e3942
- f15fdd528738d5a7f1dd1a14c279e9ab156220d7
verify:
  command: BBOXY_CACHE_MODE=off make land
  exit_code: 2
id: 01M6SSP5W4B1AG4SBM55Q1D9B2
updated: 2026-11-07T01:24:08.885Z
number: 93377961366191
created_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
updated_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
authorship:
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  kind: seat
  public_key: 9eWyjSVdBKBGoA72B1nDgpObDHOHT0P0DKq9XWjgcwI=
  process_generation: '1'
provenance:
  digest: sha256:39d6a75034eb5d0abf31a8339086f90a5084ad2aa35a911307cf484d56502739
  frontmatter_digest: sha256:07da8ba835a003fe98872a3a5b0509734766846ad5e841774ce45251a76dab40
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  sig: eLu5nwpBF4DI75KmKOoOdkuCeTDS4gZkA10hDW4vLh0xD1Uiyy5SRXkIslz46w7eZEMVhkaT9/nGklzB/oQ2Cw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150076
  event:
    id: 01M6SSP6NJCVZB1T225H1VHNN3
    sequence: 150076
    hash: 9053e508a594eb95cbd3fc3340219be669cb561890e33f427e9cd175e3b48ea4
---
Implemented checkpoint; blocked before landing. The task is incomplete. Source is in the clean assigned worktree on `bboxy/01M6S48KXRY6YY8ZNHM5V5H8MG`, HEAD `f15fdd528738d5a7f1dd1a14c279e9ab156220d7`. Four local commits await landing; all four passed `git verify-commit` (exit 0, approved Boxy key).

## Blocking evidence and decision

The latest `BBOXY_CACHE_MODE=off make land` returned **2**; its inner landing gate returned **77**, and strict Clippy returned **101**. Inherited harness pin `6d9fc2e1869c1ce1333f959f36ed133824f825e0` fails at `crates/bboxy450-harness/src/frontends/gui/face.rs:2381`: `paint.clone()` triggers `clone_on_copy`; Clippy recommends `*paint`. This submodule file and publication of a harness fix are outside the assigned scope. A subsequent read of origin/main `3ff407090`, pin `f6a247b2be2ef21bb4d38ef805d404876d9e4adc`, found the same expression still present.

**Decision needed:** have the harness owner fix that lint and bump the parent pin, or amend this assignment with the necessary submodule file/worktree and push authority. No lint suppression or gate bypass was applied.

An earlier current-source gate hit two real watcher deadlines: `index.rs:1760` (“in-root edit was not observed”) and `knowledge/routing_tests.rs:69` (“replacement watcher did not deliver the edit”). Mux results were 389/391 passed, 2 failed, 11 skipped. Isolated diagnostics subsequently passed both cases at utility (1.740 s) and default policy (1.917 s), each exit 0. This does not establish a QoS cause or fix. The routing case shares its deadline with replacement reconciliation, so its failure does not prove 30 seconds of post-edit event loss. A separate scope/owner decision for watcher diagnosis was requested. No assertion, deadline or quarantine was changed.

## Implemented mechanisms

- `scripts/qos.sh:17`, `scripts/cache-env.sh`, `Makefile:8`, `scripts/landing.py:131` — inherited Darwin background compiler/link/clippy trees; utility test trees; non-macOS nice fallback. Cargo test selections prebuild at background policy. Rustdoc uses its test runtool to execute doctests at utility. The sccache namespace avoids legacy default-policy servers and starts new servers at background policy.
- `scripts/landing.py:161` — compile-only nextest invocation omits its execution-only `--no-fail-fast`; original execution arguments remain intact. Cargo/application argument boundaries are tested.
- `scripts/ci-local.sh`, `scripts/invariants.sh`, `scripts/landing.py:204`, `scripts/main-watch.sh`, `LAND.md` — ordinary interpreted leaves use utility. Explicit timing leaves and their named Make parents remain unclamped. The macOS watcher no longer nices all timed descendants; other hosts retain nice. Native fixtures demonstrate that clearing Darwin background state cannot undo an inherited utility clamp.
- `crates/bboxy450-term/src/main.rs:10008`, `src/qos.rs:8` — supervisor/logic/MCP startup requests initiated (25), desktop/UI interactive (33). `crates/bbt-rt/src/qos.rs:31`, `src/lib.rs:59`, mux `logic.rs:2293` and `mcp/bridge.rs:594` enforce the corresponding control/IO roles. Existing PTY reader/UI interactive roles remain. `logic.rs:2955` runs publication IO at utility and restores the pooled worker after internally caught panics.
- `scripts/qos-probe.py:52,91,123`, `crates/bbt-mux/src/lib.rs:59` — private native supervisor/mux/seat/socket/signing identity; real authenticated MCP self_status round trips; logical-CPU-count independent load children, all owned and reaped. Protocol notifications are consumed separately from matching responses.
- `scripts/land-fast.sh`, `scripts/landing.py:216` — per-step and total wall/child CPU/allocated target-byte deltas, preserving real failure exits. Rebase paths initialize the exact harness pin before building. Push retries rerun the current-source gate.
- `scripts/landing.py:184` — retain the waiting Cargo parent after prebuild. On this Darwin host, exec discarded accumulated prebuild child CPU: the regression's old implementation reported 0.156 s despite a 0.4 CPU-second child and failed exit 1. The corrected implementation passes the >=0.35 CPU-second assertion. Actual app-test cost now includes its compiler CPU.
- `crates/bboxy450-term/src/ctl/workspace.rs:503,520,534` — the expressly granted test-only channel holds the private peer alive through timeout restoration. The 100 ms budget, no-retry assertion and existing outer bound remain.
- `crates/bbt-supervisor/src/ring/writer/crash_tests.rs:166` — standalone fixture compiler honors nonempty RUSTC/RUSTC_WRAPPER. Its release-only crash case remains ignored as before; its compiler branch was compiled, not executed in the ordinary test run.
- `verification/contracts.toml`, `verification/coverage.toml` — QoS implementation/performance boundary, bounds, source mapping and deterministic negative controls. This is not an OS-scheduler TLA refinement claim.
- `scripts/test-landing.py:2910,2997` — actual child/grandchild, Make, rustdoc, interpreted CI, recursive timing-parent, watcher-launch and CPU accounting witnesses. Existing attribution policy was retained; the new owned Rust notice was normalized to the repository's exact standard.

Changed scripts: cache-env.sh, ci-local.sh, invariants.sh, land-fast.sh, landing.py, main-watch.sh, qos-probe.py, qos.sh, test-landing.py. No global Cargo configuration changed.

## Executed verification

These checkpoints precede the latest inherited-main rebase; they are not a final-source green receipt:

| Command/checkpoint | Actual result |
| --- | --- |
| `python3 -I scripts/test-landing.py` at the 44b696e7 checkpoint | exit 0; 196 fixtures, 93.368 s |
| `python3 -I scripts/test-landing.py QosTests CostTests` after watcher/compiler fixes | exit 0; 15 tests, 10.403 s |
| `BBOXY_CACHE_MODE=off python3 -I scripts/qos-probe.py --check` at 44b696e7 | exit 0; policy and latency checks true |
| `BBOXY_CACHE_MODE=off make test-capture-storage-pressure` | exit 0; 16 passed, 53 skipped |
| Earlier `make test-formal-contracts` | exit 0; 24 API/model witnesses, strict Clippy, two RT class tests, one app class test, five actual startup processes, policy/cost tests and both probe controls |
| `make test-licenses` after correcting the owned notice order | exit 0; license fixtures and owned-source notices held |
| Most recent land | make exit 2; inner gate 77; inherited harness Clippy 101 |

The earlier startup process witness returned supervisor/logic/MCP=25 and desktop/UI=33. Native compiler process observations showed priority 4. The controls returned their required specific failures: default-qos had qos_ok=false, latency_ok=true; delayed-reply had qos_ok=true, latency_ok=false. Their Make checks required exit 1 and the corresponding JSON field, not a generic failure.

The latest successful probe used N=10, M=100 per phase after ten warmups:

| Phase | p50 ms | p95 ms |
| --- | ---: | ---: |
| Unloaded | 1.946375 | 4.539166 |
| Before-policy baseline: default load | 4.358834 | 9.802333 |
| After-policy load: background | 2.074209 | 3.003542 |

Threshold was 250 ms. All ten background workers reported Darwin background flag 1; default workers reported 0; the private MCP thread requested 25. This paired same-source policy comparison is observational, not a historical old-app comparison or a hard latency guarantee.

The failed 51b6334 land printed, among other steps: app build wall=631.394 s / CPU=1094.611 s / disk=-499712 B; app tests wall=488.985 s / CPU=817.564 s / disk=+106496 B; total wall=1724.507 s / CPU=2706.528 s / disk=+140394496 B / inner exit=77. The latest inherited-Clippy failure printed total wall=94.183 s / CPU=51.502 s / disk=-389120 B / inner exit=77. CPU comes from waited children; detached cache-server CPU and shared-cache growth are outside that accounting. Author checks used cache off.

## Evidence locations and remaining work

Raw logs remain outside Git under this worktree's target/. Selected SHA256 values:

- land-qos-next.log: `4e55779c8b5046bedc8bcf27e7c61684588f40c87004ecf6cf10bcaa198656eb`
- land-qos-final.log: `09b2b070518b0f30dea878c78b1707f03a6d9132df287535773d58b8a8087cb1`
- verify-watch-qos.log: `2363759f572d097bc8f394eba36e6f9144f09f789c73ee20a7ec3d7aab4e495e`
- verify-landing-latest.log: `b4d65a0ba6df49b98ef7b2329145d75bcd3df5dcab35b0a63e5e0ff2f81bc14c`
- verify-probe-latest.json: `3a608b3a950af9772c9522dfbc995dbb08490e35375febb0d551076068f7896b`

No commit has pushed. Full formal verification has not yet been reached by land. The exact required verification chain must be refreshed on final source. No full make test or reinstall ran. The toolchain also emitted a nonfatal rust-objcopy/libLLVM.dylib warning; it was not suppressed or repaired globally. Native QoS witnesses were run on macOS; the nice fallback was not exercised on another host.

The expressly after-landing headless.rs compiler-wrapper change is still pending. The assigned C1 ignored test remains absent from the checked source; its re-enable duty is pending upstream landing and the corresponding task amendment. No test was fabricated or silently re-enabled elsewhere. The requested completion report and single task_report remain due after these requirements and the gates are satisfied.
