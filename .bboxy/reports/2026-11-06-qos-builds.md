---
type: report
title: Build QoS and responsiveness landed with verified cost and explicit C1 transfers
task: '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
author: seat:01M6S48V44QDK7SE9H4EDFPXVT
created: 2026-11-07T10:37:18.502Z
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
- 11b57036170c94a42f7faf5d4162718f517fa940
- 38d99096f4f9c475a5547cf2192982af6ff7a5c0
verify:
  command: python3 -I scripts/test-landing.py && python3 -I scripts/qos-probe.py --check && make test-formal-contracts
  exit_code: 0
supersedes: '[[reports/2026-11-06-qos-builds-native-unit-budget-blocked]]'
id: 01M6TSB0X2CVHTHBHRTR8KRQVF
updated: 2026-11-07T10:37:18.502Z
number: 93377961366256
created_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
updated_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
authorship:
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  kind: seat
  public_key: 9eWyjSVdBKBGoA72B1nDgpObDHOHT0P0DKq9XWjgcwI=
  process_generation: '1'
provenance:
  digest: sha256:be1511f187380a6e9ae18b9cc2ed273c4241dffa08f0b5840dc54396f2b54180
  frontmatter_digest: sha256:c5778539cac689e1e1e054c5bdf1045cc21ced2d0aed1ef474eb5cd50a41cfa7
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  sig: bcZz92ghflCm2vng+Tqh5jgrlV6i/T8yWdkbkBJUxl3JfYT48nyUxTjXHbg3FX52KGjkkORS5sM2QhZg7PhcBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150285
  event:
    id: 01M6TSB2CF0T3B6EMV7XC3DTSY
    sequence: 150285
    hash: 5963e31120ce0572ca54d6e0cf809cb15bcc5190548e1140c110ad348de7f0c2
---
# QoS builds: landed and verified

Primary QoS landed at `ee60f8184aa1b0181d58a2f55bffb99501eca7d0`; the authorized follow-up landed at **11b57036170c94a42f7faf5d4162718f517fa940**. Both used canonical `make land`, exit 0. The exact required verify chain also exited **0 on that final SHA**, with unchanged HEAD throughout. The code worktree is clean and `git cherry origin/main HEAD` is empty. Seventeen term commits and the pinned harness commit passed signature verification.

The final implementation keeps compiler/linker trees at background policy, ordinary tests at utility, explicit measurements at default and app threads within their role ceilings. The C1 runtime was not installed or launched through its ordinary one-shot gate. C1 defects remaining after background builds are named transfers, not passing witnesses.

## Changed files and mechanisms

- **Build/test process trees:** `Makefile:8`, `scripts/qos.sh`, `scripts/cache-env.sh` dispatch macOS trees through taskpolicy -b for compile/link/Clippy and -c utility for ordinary tests. Private Cargo/rustc/rustdoc shims cover descendants and standalone fixture compilers. A native self call clears inherited internal background state before utility/default execution. Non-macOS retains nice. A separate background-policy sccache namespace avoids using an existing foreground daemon; author checks used cache-off without mutating shared daemons.
- **Honest measured processes:** `scripts/landing.py:139,240`, `scripts/qos-policy.py`, `LAND.md` prebuild at background and run tests separately. Nextest's controller stays unclamped; individual tests select utility or explicit default. Existing target runners and DYLD forwarding survive the protected taskpolicy exec. Mixed direct libtest batches containing named measurements stay default. Every timing opt-out is enumerated in LAND.md; no blanket C1 unit-module opt-out remains.
- **App ceilings:** `crates/bboxy450-term/src/qos.rs:8`, `src/main.rs:10007,10153`, `crates/bbt-rt/src/qos.rs:31,50,59`, `src/lib.rs:59`, `crates/bbt-mux/src/logic.rs:2309,2977`, `src/knowledge/routing.rs:84`, `src/mcp/bridge.rs:594`. Supervisor control, logic, MCP and shared IO workers request initiated (25); desktop/actual UI requests interactive (33). The existing dedicated PTY reader stays interactive, as the amended ceiling permits. Indexing/publication IO enters utility and restores its pooled thread, including caught-job panics. Tool flags do not promote unrelated helpers.
- **Observable landing cost:** `scripts/land-fast.sh`, `scripts/landing.py:289` print each step's wall time, child CPU, allocated target-size delta and actual exit, followed by the total. A waiting parent preserves Darwin child rusage across Cargo's prebuild/execution phases.
- **Private response measurement:** `scripts/qos-probe.py`, `crates/bbt-mux/src/lib.rs:59` use a private native supervisor/mux/socket/seat/signing key and actual authenticated self_status through the bridge. Each normal phase measures 100 requests after ten warmups, with ten logical-CPU busy-loop children. Owned children are reaped. No operator app, account or socket is used. Wrong workload QoS and an explicit injected post-reply measurement delay are deterministic negatives; real-load latency is observational under the amendment.
- **Fixture fixes, without weaker assertions:** `crates/bboxy450-term/src/ctl/workspace.rs` holds its private peer open through timeout-restoration assertions using a channel; the 100 ms bound and no-retry assertion remain. `crates/bbt-supervisor/src/ring/writer/crash_tests.rs:162` and `crates/bboxy450-term/tests/headless.rs:1172` honor nonempty RUSTC/RUSTC_WRAPPER for standalone fixture compiles.
- **Verification:** `verification/contracts.toml` adds BuildQosResponsiveness as implementation/performance evidence with explicit controls and OS-scheduler limits; `verification/coverage.toml` maps the QoS source. This is not a TLA scheduler/refinement claim. `scripts/test-formal.py` supplies qos.sh to its private Make/publication fixture while preserving the five publication-refusal assertions. `scripts/test-landing.py` verifies inherited policies, mixed nextest execution, runner/DYLD composition and real descendant cost; its CI fixture reads native requested class instead of an IPC-boosted dynamic ps upper bound.
- **Every touched script:** cache-env.sh, ci-local.sh, invariants.sh, land-fast.sh, landing.py, main-watch.sh, qos-policy.py, qos-probe.py, qos.sh, test-formal.py and test-landing.py. Ordinary interpreted leaves are utility; timing orchestration stays unclamped. `scripts/quarantine-test.sh`, ATTRIBUTION.md and license files are unchanged; existing policy/origin notices remain attached.

The authorized harness exception landed separately at `38d99096f4f9c475a5547cf2192982af6ff7a5c0`. The redundant paint.clone fix was already upstream and dropped on rebase; the signed local change adds TLC -lncheck final so safety/liveness verdict selection is deterministic after complete exploration. Harness `make land` exited 0: 127 ACP lib tests, 281 models, 78 contracts, one crate. Term pins that SHA via canonical make bump-harness, with no Cargo.lock delta. The private harness worktree was removed after clean/ancestry/open-file checks.

## Final executable results

All author Cargo checks used cache-off, dev/test debug=0, opt-level=1 and debug assertions=true; one build/test at a time.

| Command / source | Actual result |
| --- | --- |
| make land / primary ee60f818 | 0; strict Clippy, harness 126/126, host 53/53, rt 19/19, app 453/453, mux 507/507, dependent checks and full formal parity |
| make land / final 11b570361 | **0**; strict Clippy; wake-up dream_c1 integration 16/16; headless 4/4 with 23 skipped; mux lib + dream_c1 integration 506/506 with 29 skipped; current-source formal gate |
| Required exact three-command verify chain / final 11b570361 | **0**, 772.624 s; 204 landing fixtures, private probe, 43 formal API/model witnesses, strict Clippy and test-qos |
| Native QoS tests within the final chain | rt helper 2/2, app helper 2/2; eighteen QoS/cost tests passed |
| Five real private startup process witnesses | supervisor/logic/MCP=25; desktop/UI=33 |
| Signature checks | all seventeen term commits and harness38d99096 exit 0; Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0 |

The headless selector's standalone fixture test and strict Clippy also passed separately before landing; final canonical headless execution includes that fixture. Skipped tests are not claimed executed. Routing's native success-and-caught-panic witness observed utility inside the job and initiated afterward in the primary mux suite. The actual Cargo/rustc process witness records background priority 4; policy tests query the child's native self background state. Cross-process getpriority of another PID is not treated as evidence of its internal background flag.

Final formal receipt: **679 models, 254 contracts, 34 crates**, source 11b570361, dirty=false. All 679 validated identical model/tool/native-graph results reused the primary's complete native/TLC parity evidence; the current source was re-exported and bound into a new receipt. This is validated reuse, not a claim to have launched Java again for every model.

Receipt retained outside the worktree:
`/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/6cfc9a37895ceebd0439ddc3d94a2e60f0efdbe2211e495aaef7f73b6da4b845/evidence.json`.
Its SHA256 equals the directory name. Primary cold receipt: cee00e0b575dc715aef9897aca61d40ce7a9ff3c917707f97d1dc07b7577856c.

## MCP measurements and controls

Both final-source measurements are recorded; the standalone result did not show a speedup, and it is not replaced by the better nested result.

| Final run / phase | p50 ms | p95 ms |
| --- | ---: | ---: |
| Standalone: unloaded | 1.667083 | 4.705209 |
| Standalone: before/default-policy load | 4.400333 | 8.977917 |
| Standalone: after/background-policy load | 6.011500 | 22.571583 |
| Nested test-qos: unloaded | 1.494917 | 3.877292 |
| Nested test-qos: before/default-policy load | 4.093208 | 9.937708 |
| Nested test-qos: after/background-policy load | 1.432000 | 1.632417 |

All ten after-load workers reported native self background=1, and the native bridge requested class25. Dynamic ps samples ranged 1–4 for background children; native self policy is authoritative. Both final p95 values were below the 250 ms threshold. These same-fixture comparisons isolate the requested load policies rather than compare historical app binaries, but shared-machine variance means they do not establish a universal latency reduction.

Final wrong-policy control: expected exit **1**, qos_ok=false, latency_ok=true, p95 **5.109375 ms**. Final injected-delay control: expected exit **1**, qos_ok=true, latency_ok=false, p95 **271.880667 ms**. Make checks the exact negative exit and corresponding JSON field, so a build/transport failure cannot substitute for a broken control. The delay is explicitly injected into the measured interval after a reply. Normal load latency remains observational, as instructed.

## Cost printed by make land

| Land | Wall seconds | Child CPU seconds | Allocated target growth |
| --- | ---: | ---: | ---: |
| Primary, including cold formal | 5100.288 | 4751.873 | +8,437,059,584 B |
| Final follow-up | **1836.639** | **2319.434** | **+282,415,104 B** |
| Final formal step within follow-up | 418.921 | 360.946 | +158,539,776 B |

Final per-step examples: Clippy 82.028 wall / 138.182 CPU; headless 407.662 / 669.507; mux 910.300 / 1140.186. Every step's disk delta and actual exit appear in the raw log. Growth is the requested target-directory measurement; it is not whole-machine disk accounting. CPU is RUSAGE_CHILDREN, not a measurement of other seats or an existing daemon.

## C1 re-enables and authorized transfers

The primary landed first, then all thirteen load-family ignores were attempted without changing assertions or budgets. The Oracle's final ruling, signed message **01M6TGEZ0HX7V1MFKX47FGW4F6**, assigned persistent failures to known-defect tasks and explicitly authorized the same transfer for any further C1 family failure during canonical land. No more separate pre-landing C1 passes were run after that ruling.

Final accounting: **six of the original thirteen remain enabled**—the five wake-up process tests plus `custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes`. Seven original witnesses transferred, plus three additional land-time failures: sixteen inventoried witnesses, six active, ten explicit transfers. This is not a thirteen-test pass. The six active originals passed through final canonical testing.

| Owning task | Known-defect witnesses |
| --- | --- |
| [[tasks/2026-11-06-dream-c1-honest-capture-errors]] | admitted-MCP stale_context mapping |
| [[tasks/2026-11-06-dream-c1-cold-mcp-latency]] | held private catalogue control latency |
| [[tasks/2026-11-06-dream-c1-two-owner-storage-race]] | source-error capacity / both owners refusing |
| [[tasks/2026-11-07-dream-c1-native-unit-budget-overruns]] | both final-authority witnesses, source-error replay, registration, concurrent registration, expired-grant admission and busy-writer schema |

The native-unit task's seven witnesses include the two initially persistent low-load failures and five subsequently observed land-time failures. Actual additional CPU overruns were 1.179541 ms (expired grant), 1.279334 ms (schema), 1.125875 ms (concurrent registration), 1.037334 ms (registration) and 1.292875 ms (held-authority setup), each against the unchanged 1 ms unit budget. Their failing gate logs are retained. The separate native-unit task owns profiling the mandated optimized C1 fixture against the ordinary land profile before deciding on routing or source splitting.

`scripts/qos-policy.py` names every timed/deferred witness; LAND.md names their tasks. Each owner removes its source ignore and matching C1_DEFERRED_TESTS entry on repair. A final read-only consistency review at 11b570361 independently checked the sixteen/six/ten mapping and confirmed no assertion, budget, test body or one-launch guard changed in the transfer diff. Script exits remain the gate verdicts.

The additional ignore annotations are authorized defect transfers, not fixes or passing evidence. The earlier C1 two-owner start defect, real-producer requirements and ordinary runtime launch guards remain intact. No ordinary one-shot C1 runtime launch, reinstall or operator-app action occurred.

## Evidence and limits

Final raw logs, exact command/exit/source JSON, controls, signature results, C1 inventory, both latency measurements and the failed intermediate land logs are preserved at:

`/Users/feral/mydev/bboxy450-term/.git/bboxy-qos-builds/01M6S3M8WTNZ32TMZ7YX9VTEJR/term-final-11b57036170c94a42f7faf5d4162718f517fa940/`

Manifest SHA256: **7bf5d7b0e84b3c1a2b0f9d066a6561bd2f0faf6ae220c4fce4fc6558ec1200a5**. Key files: land-qos-ten-transfers.log/json; verify-qos-landed-final.log/json; qos-control-default.json; qos-control-delay.json; qos-final-signatures.json; qos-final-c1-inventory.json; qos-final-latencies.json. Prior primary/harness archives remain under the same task directory; raw logs and fixture accounts are outside Git.

No full root make test, full test-boundaries, visual parity, release, reinstall or operator interaction was run under this lean brief. Non-macOS fallback was source-reviewed, not runtime-verified. QoS is an OS request, not hard affinity or an absolute latency guarantee; the probe covers private MCP under CPU load, not every operator-app/IO scenario. The pinned rust-objcopy tool emits a missing-libLLVM.dylib stripping warning; successful commands still exited 0, and no global toolchain repair was attempted.

Artifact_claim is intentionally omitted under the Oracle's explicit known-bug ruling. This append-only final report supersedes the blocked reports; code and required verification are complete. Acceptance remains the Oracle's responsibility.
