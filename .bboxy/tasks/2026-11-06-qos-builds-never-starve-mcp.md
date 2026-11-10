---
type: task
title: Builds/tests run at background QoS; mux, MCP and UI keep the performance cores; land prints per-step cost
role: executor
allowed_paths:
- crates/bboxy450-term/tests/headless.rs
- scripts/land-fast.sh
- scripts/landing.py
- scripts/cache-env.sh
- scripts/quarantine-test.sh
- scripts/test-landing.py
- scripts/qos-probe.py
- Makefile
- LAND.md
- crates/bboxy450-term/src/main.rs
- crates/bboxy450-term/src/qos.rs
- crates/bboxy450-term/Cargo.toml
- crates/bbt-supervisor/src/
- crates/bbt-mux/src/main.rs
- crates/bbt-mux/src/lib.rs
- crates/bbt-mux/src/mcp/bridge.rs
- verification/
- ATTRIBUTION.md
- .bboxy/reports/
- crates/bbt-rt/src/qos.rs
- crates/bbt-rt/src/lib.rs
- crates/bbt-mux/src/logic.rs
- crates/bboxy450-term/src/cockpit/logic.rs
- crates/bboxy450-term/src/cockpit/supervisor.rs
- crates/bboxy450-term/src/cockpit/desktop.rs
- scripts/qos.sh
- scripts/qos-probe/
- crates/bboxy450-term/src/ctl/workspace.rs
- scripts/ci-local.sh
- scripts/
- crates/bbt-mux/src/knowledge/routing.rs
- crates/bbt-mux/src/registry/dream_c1.rs
verify: python3 -I scripts/test-landing.py && python3 -I scripts/qos-probe.py --check && make test-formal-contracts
status: accepted
id: 01M6S3M8WTNZ32TMZ7YX9VTEJR
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T10:41:23.245Z
created: 2026-11-06T18:58:37.431Z
number: 93377961366081
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6S48V44QDK7SE9H4EDFPXVT
verify_result:
  command: python3 -I scripts/test-landing.py && python3 -I scripts/qos-probe.py --check && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Builds and tests started through Make or land show background/utility QoS
    reference: Makefile:8; scripts/qos.sh; scripts/cache-env.sh; scripts/landing.py:139,240; scripts/qos-policy.py. Final 204 landing fixtures and eighteen QoS/cost tests exit0; both private probes report ten native self background flags=1. reports/2026-11-06-qos-builds.md, Changed files and Final executable results.
  - criterion: App processes set their QoS at startup, with helper units and real process witnesses
    reference: crates/bboxy450-term/src/qos.rs:8; src/main.rs:10007,10153; crates/bbt-rt/src/qos.rs:31,50,59; crates/bbt-mux/src/mcp/bridge.rs:594; src/knowledge/routing.rs:84. Final helper units2+2 exit0 and real startup requests[25,25,25,33,33]; restoration witness passed. verify-qos-landed-final.log in committed report archive.
  - criterion: qos-probe.py --check passes on final source and before/after p95 is recorded
    reference: Final exact-source chain exit0 at11b57036170c94a42f7faf5d4162718f517fa940; verify-qos-landed-final.json. Both measured comparisons, including the standalone regression, are recorded in reports/2026-11-06-qos-builds.md and qos-final-latencies.json; background p95 values22.571583/1.632417ms, threshold250ms.
  - criterion: Land prints per-step wall time, CPU and target disk delta plus a final total
    reference: 'scripts/landing.py:289 and scripts/land-fast.sh; land-qos-ten-transfers.log/json: make land exit0, total wall1836.639s, CPU2319.434s, target growth282415104B. Every step has actual exit and cost. Primary cold formal/land costs also retained.'
  - criterion: QoS and responsiveness contract has deterministic broken controls and current-source formal evidence
    reference: verification/contracts.toml BuildQosResponsiveness; final default-QoS control exit1 with qos_ok=false; delay control exit1 with qos_ok=true/latency_ok=false and p95271.880667ms. Final source-bound formal receipt6cfc9a37895ceebd0439ddc3d94a2e60f0efdbe2211e495aaef7f73b6da4b845:679 validated models,254 contracts,34 crates,dirty=false.
  - criterion: Authorized C1 re-enable/defect-transfer disposition, headless compiler fix, signed landing and retained report
    reference: Ruling01M6TGEZ0HX7V1MFKX47FGW4F6 authorizes further C1 land-time failures to transfer. scripts/qos-policy.py and LAND.md:16 inventoried/six active/ten transferred; five wake-up tests plus custody executed in final canonical gates. All ten ignore/task mappings and unchanged assertions/budgets/guards are documented. Headless compiler wrapper fixture and strict Clippy passed. Eighteen signature checks exit0; git status clean and git cherry empty. Committed report01M6TSB0X2CVHTHBHRTR8KRQVF / KB3605333debcfd3427929daaa2f1dfba6762a63e5.
report:
  summary: 'Landed primary QoS at ee60f818 and the authorized follow-up at 11b57036170c94a42f7faf5d4162718f517fa940. Canonical land and the exact final-source verify chain exit 0; 679 models/254 contracts/34 crates, 506 mux tests, 16 wake-up tests, four headless tests, native QoS witnesses and deterministic controls passed. Final land cost: 1836.639 s wall, 2319.434 child CPU seconds, +282415104 target bytes. Both final MCP comparisons are reported honestly: default/background p95 8.978/22.572 ms and 9.938/1.632 ms, all below 250 ms. Six original C1 witnesses remain re-enabled; ten explicit defect transfers (including three additional land-time cases) follow ruling 01M6TGEZ. No reinstall or ordinary one-shot C1 runtime launch. Full committed report: [[reports/2026-11-06-qos-builds]], KB 3605333debcfd3427929daaa2f1dfba6762a63e5.'
  commit: 11b57036170c94a42f7faf5d4162718f517fa940
  reported_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
  op_id: 01M6TSHXYQB89A8KMABJBRMWYJ
  verify_revision: 0
  report: '[[reports/2026-11-06-qos-builds.md]]'
provenance:
  digest: sha256:4de28fce7f1691ed71c20728aa89423c4709d0c2f63279feb3177f761b48e9f7
  frontmatter_digest: sha256:a47bbae0d3f1d4b11f899333079b8080a7217bcca5edd09865c5512b2990348b
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: YZWVwXCCVKiGhp2TXLLwJ+0zanKTLOcthMnOMwdhoDwWTnGuj7lFo3Sc779TGQnkJGzKuHOKKn+JH8A4qr4ZCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150292
  event:
    id: 01M6TSJHD891PNEJDRRYWXG1KE
    sequence: 150292
    hash: ac7e046cb8083f1dec37e6ae34dc9bce508bb0e57bdd4d98e8403e1f83410c0f
---
# Builds must never starve the MCP or the UI

Operator, 2026-11-06: "when we are hitting cpu hard - bboxy MCP starts to time out, everything starts lagging, that's an issue"; "optimize make land … reducing cpu, disk and mem usage; cpu and disk being big problems".

## Evidence

- This machine has 4 performance and 6 efficiency cores (`sysctl hw.perflevel0/1.physicalcpu`). Load average was 13.5 while executors built.
- The only deprioritization anywhere is `nice -n 10` in `scripts/land-fast.sh:17` and `scripts/main-watch.sh:11`. `make test`, `make test-formal`, `make test-dream-c1`, executors' iterative `cargo` runs, rustc and the linker all run at default QoS. On Apple Silicon, `nice` barely affects scheduling; QoS class decides P-core versus E-core placement and IO throttling. So compiles compete with the supervisor, logic, MCP bridge and desktop on the 4 P-cores.
- The app processes (`bboxy450-term supervisor|logic|desktop|mcp|harness-exec`) set no QoS (repository grep for taskpolicy, qos_class and setpriority: none).

## Required

1. **Build and test work runs at low QoS.** Every Make entry point that builds or tests (`cache-env.sh` is sourced by all of them) and `make land` run cargo, rustc, linkers and test processes under macOS `taskpolicy`: compile, link and clippy at `-b` (background: E-cores, throttled IO), and test execution at `-c utility`. Replace `nice -n 10` with this. It must be inherited by children: wrap the process tree, not just cargo's argv. Non-macOS hosts keep `nice`.
2. **Timing-sensitive gates opt out explicitly.** Gates that measure latency or CPU budgets (`test-dream-c1`'s timed runs, perf gates such as `gates_seats`, and anything asserting the 1 ms, 250 ms or frame budgets) run their timed process at default QoS, so the measurement stays honest. Their builds still run at background QoS. List each opt-out explicitly in `LAND.md`; there is no blanket opt-out.
3. **App processes claim responsiveness.** At startup the supervisor, logic and desktop processes and the `mcp` bridge set their own QoS (`pthread_set_qos_class_self_np` or `setpriority(PRIO_DARWIN_THREAD)`): desktop and UI threads user-interactive; logic, supervisor and MCP user-initiated. Their own background workers (Dream C1 work, indexing) stay at utility or background as the existing specs say. Do not raise anything above its role.
4. **`make land` prints its cost.** Each gate step prints wall time and the target dir's size delta. The final line summarizes total wall time, CPU seconds (via `getrusage` of the children) and disk growth, so we can see where land spends.
5. **Measure the fix.** Add `scripts/qos-probe.py`. It starts a private, headless, saturating load (N = logical-CPU busy loops at the QoS the build would get), then times M MCP round trips (`self_status` through a private test mux and socket, never the operator's app) and reports p50/p95. `--check` fails if p95 under load exceeds 4× the unloaded p95 or 250 ms, whichever is larger. Record the before and after numbers in the report.

## Acceptance

- [ ] Builds and tests started through Make or land show background/utility QoS (verify with `taskinfo` or `ps -o pri`, or with a test that inspects the child's QoS).
- [ ] App processes set their QoS at startup (unit test of the helper, plus a process witness).
- [ ] `qos-probe.py --check` passes on the final source; the before/after p95 is in the report.
- [ ] The land output shows per-step wall time, CPU and disk.
- [ ] Contracts: add the QoS and responsiveness contract to `verification/contracts.toml` with a broken control (load at default QoS must fail the probe threshold).

Lean landing: run only `scripts/test-landing.py`, the probe and the touched crates' tests; no full `make test`; land with `make land`. No reinstall: the operator decides when. Write the report `reports/2026-11-06-qos-builds.md` and call `task_report` once.

**Amend (Oracle):** Correction to Evidence: the app already sets user-interactive QoS through `bbt_rt::qos::interactive_thread()` (bbt-supervisor/src/supervisor.rs:875, bbt-mux/src/logic.rs:2264, bbt-rt/src/lib.rs:59 Tokio workers); the Oracle grep missed it. Main lever stays: build/test work must leave the P-cores. Granted bbt-rt qos.rs/lib.rs, bbt-mux logic.rs, cockpit logic/supervisor/desktop.rs, scripts/qos.sh, scripts/qos-probe/ to correct role ceilings (UI/PTY reader interactive; logic/MCP user-initiated; background work utility/background). Broken controls: deterministic ones are accepted. A forced QoS refusal (builds left at default) and an injected over-threshold delay must each fail the probe. The real-load p95 stays observational in the report and is not a flaky gate. Live evidence 2026-11-06 11:13: an MCP read timed out (`Seat control transport unavailable`, read_response WouldBlock after 10001 ms) at load 15.3, with fseventsd at 102% CPU and the GUI process at 71%.

**Oracle amend (re-enable duty):** under the operator's "cut corners" order, the C1 landing marks `bboxy450-harness-wake-up::dream_c1::actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames` as `#[ignore]` (CPU 0.55 ms, but 207 ms wall under load 11–15). After the QoS change lands, remove that `#[ignore]` and show the test passing at default QoS for timed processes while builds run in the background. That is part of this task's acceptance.

**Amend (Oracle):** granted crates/bboxy450-term/src/ctl/workspace.rs for a TEST-FIXTURE-ONLY fix at ~526. The private peer closed at 150 ms, before the restoration assertion ran at 187 ms under load, and Darwin refuses timeout restoration after peer close. Hold the peer open with a channel until the assertion, keeping the 100 ms invariant and the no-retry assertion. No timeout relaxation, quarantine or product change.

**Amend (Oracle):** granted scripts/ci-local.sh. Apply utility QoS to ordinary leaf commands (~51–75), keeping explicit timed opt-outs. The C1 ignored-test re-enable stays pending until C1 lands on main (the test is not on main yet), and is then due as a follow-up under this task.

**Amend (Oracle):** BLANKET grant of `scripts/` (including invariants.sh at ~290), ONLY for QoS classification of ordinary CI leaf commands (utility/background), with Cargo/Make timing orchestration and explicit timed opt-outs unclamped. No gate, assertion or timeout changes. No need to ask per file; list every touched script in the report.

**Oracle amend (re-enable list, growing):** under the "cut corners" order, C1 marks `#[ignore]` on load-sensitive tests: wake-up `actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames`, `actual_private_process_clears_authority_environment_and_uses_fixed_argv`, and any further `actual_private_process_*` failing only its 250 ms window; mux `logic::dream_c1::tests::custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes`, plus any dream_c1 test failing only with ResourceLimit from the CPU-frame budget. Root cause: at load 18, default-QoS threads run on E-cores, so per-unit CPU and wall time inflate. ACCEPTANCE for this task now includes removing every one of these ignores (C1's report lists them) and showing they pass with builds at background QoS. They run under `make test-dream-c1` and the wake-up suite.

**Amend (Oracle):** follow-up granted after the main QoS landing: crates/bboxy450-term/tests/headless.rs (~1172 calls rustc directly, bypassing RUSTC_WRAPPER), so the fixture compiler honors the wrapper and background QoS. No assertion or gate changes.

**Amend (Oracle, ~01:25):** harness authority granted for ONE fix. Remove the redundant `paint.clone()` (Clippy -D warnings) at harness `src/frontends/gui/face.rs` ~2381, still present on harness origin/main 216cf0d and breaking every term landing that clippies the harness. Do it in a private detached harness worktree from harness origin/main, sign it, land through the harness's canonical `make land` (the harness has autonomous landing authority), then `make bump-harness REV=<that SHA>` in the term worktree and remove the harness worktree. No other harness change.

**Amend (Oracle, ~01:55):** a second harness change is granted in the same private harness worktree: `-lncheck final` in harness `scripts/formal.py` `tlc()` (~244), identical to term's landed fix, because the harness gate hit the same TLC liveness-before-safety race on BrokenHarnessDreamC1QuantumDebtAbaEffect. A source-pin or evidence refresh, if required, goes only through the harness's canonical Make target. No other harness edits.

**Amend (Oracle, ~02:00):** granted crates/bbt-mux/src/knowledge/routing.rs, ONLY for wrapping routing::io_result (~83) in bbt_rt::qos::with_utility_qos, with its panic catch inside the scope and a local class/restoration witness. No routing, lifecycle, assertion or timeout change. SCOPE FREEZE after this: land QoS now. Further audit findings go to a follow-up task, not this landing.

**Amend (Oracle, ~03:15):** C1's executor is closed and this task already owns re-enabling C1's load-ignored tests. Granted crates/bbt-mux/src/registry/dream_c1.rs (~347, 892, 1068) and crates/bbt-mux/src/logic.rs (~14040) ONLY to add #[ignore = "load-sensitive per-unit CPU budget (ResourceLimit 1.045/1.360/2.150 ms vs 1 ms at default QoS); re-enable via this task"] to exactly the C1 journal setup/replay tests failing with CPU-frame ResourceLimit. Assertions, budgets and source are unchanged. Add them to this task's re-enable list. Then land. Your acceptance still requires re-enabling ALL listed ignores once builds run at background QoS; if one still fails at low load, it becomes its own C1 budget task.

## Oracle ruling (re-stated ~07:55; first sent as DM 01M6TDG8…): the three remaining C1 re-enables become KNOWN-DEFECT ignores owned by their tasks
Keep each #[ignore] but set its reason to a known-defect pointer:
- admitted-MCP stale_context → "KNOWN DEFECT: capture() maps budget errors to stale_context; fix: tasks/2026-11-06-dream-c1-honest-capture-errors"
- held_private_delete_catalogue (256190 µs > 250 ms) → "KNOWN DEFECT: held-catalogue control latency; fix: tasks/2026-11-06-dream-c1-cold-mcp-latency"
- source_error_capacity (0 winners vs 1) → "KNOWN DEFECT: busy0 two-owner contention; fix: tasks/2026-11-06-dream-c1-two-owner-storage-race"
Those three tasks now own their re-enable (the Oracle amended each). Fold these changes into fe8f5bf5 (headless compiler fix plus the 10 passing re-enables), make land, then the ONE task_report. This task's acceptance is satisfied by the 10 re-enables plus these three transfers. artifact_claim: known bug, ignore.


## Report from seat:01M6S48V44QDK7SE9H4EDFPXVT

Landed primary QoS at ee60f818 and the authorized follow-up at 11b57036170c94a42f7faf5d4162718f517fa940. Canonical land and the exact final-source verify chain exit 0; 679 models/254 contracts/34 crates, 506 mux tests, 16 wake-up tests, four headless tests, native QoS witnesses and deterministic controls passed. Final land cost: 1836.639 s wall, 2319.434 child CPU seconds, +282415104 target bytes. Both final MCP comparisons are reported honestly: default/background p95 8.978/22.572 ms and 9.938/1.632 ms, all below 250 ms. Six original C1 witnesses remain re-enabled; ten explicit defect transfers (including three additional land-time cases) follow ruling 01M6TGEZ. No reinstall or ordinary one-shot C1 runtime launch. Full committed report: [[reports/2026-11-06-qos-builds]], KB 3605333debcfd3427929daaa2f1dfba6762a63e5.

```json
{
  "op_id": "01M6TSHXYQB89A8KMABJBRMWYJ",
  "commit": "11b57036170c94a42f7faf5d4162718f517fa940",
  "report": "[[reports/2026-11-06-qos-builds.md]]",
  "verify_result": {
    "command": "python3 -I scripts/test-landing.py && python3 -I scripts/qos-probe.py --check && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Builds and tests started through Make or land show background/utility QoS",
        "reference": "Makefile:8; scripts/qos.sh; scripts/cache-env.sh; scripts/landing.py:139,240; scripts/qos-policy.py. Final 204 landing fixtures and eighteen QoS/cost tests exit0; both private probes report ten native self background flags=1. reports/2026-11-06-qos-builds.md, Changed files and Final executable results."
      },
      {
        "criterion": "App processes set their QoS at startup, with helper units and real process witnesses",
        "reference": "crates/bboxy450-term/src/qos.rs:8; src/main.rs:10007,10153; crates/bbt-rt/src/qos.rs:31,50,59; crates/bbt-mux/src/mcp/bridge.rs:594; src/knowledge/routing.rs:84. Final helper units2+2 exit0 and real startup requests[25,25,25,33,33]; restoration witness passed. verify-qos-landed-final.log in committed report archive."
      },
      {
        "criterion": "qos-probe.py --check passes on final source and before/after p95 is recorded",
        "reference": "Final exact-source chain exit0 at11b57036170c94a42f7faf5d4162718f517fa940; verify-qos-landed-final.json. Both measured comparisons, including the standalone regression, are recorded in reports/2026-11-06-qos-builds.md and qos-final-latencies.json; background p95 values22.571583/1.632417ms, threshold250ms."
      },
      {
        "criterion": "Land prints per-step wall time, CPU and target disk delta plus a final total",
        "reference": "scripts/landing.py:289 and scripts/land-fast.sh; land-qos-ten-transfers.log/json: make land exit0, total wall1836.639s, CPU2319.434s, target growth282415104B. Every step has actual exit and cost. Primary cold formal/land costs also retained."
      },
      {
        "criterion": "QoS and responsiveness contract has deterministic broken controls and current-source formal evidence",
        "reference": "verification/contracts.toml BuildQosResponsiveness; final default-QoS control exit1 with qos_ok=false; delay control exit1 with qos_ok=true/latency_ok=false and p95271.880667ms. Final source-bound formal receipt6cfc9a37895ceebd0439ddc3d94a2e60f0efdbe2211e495aaef7f73b6da4b845:679 validated models,254 contracts,34 crates,dirty=false."
      },
      {
        "criterion": "Authorized C1 re-enable/defect-transfer disposition, headless compiler fix, signed landing and retained report",
        "reference": "Ruling01M6TGEZ0HX7V1MFKX47FGW4F6 authorizes further C1 land-time failures to transfer. scripts/qos-policy.py and LAND.md:16 inventoried/six active/ten transferred; five wake-up tests plus custody executed in final canonical gates. All ten ignore/task mappings and unchanged assertions/budgets/guards are documented. Headless compiler wrapper fixture and strict Clippy passed. Eighteen signature checks exit0; git status clean and git cherry empty. Committed report01M6TSB0X2CVHTHBHRTR8KRQVF / KB3605333debcfd3427929daaa2f1dfba6762a63e5."
      }
    ]
  }
}
```
