---
id: 01M6C96PF7RRKZNZJN04V5FK4N
type: decision
number: 5002
title: "Presubmit run budget: allocate the 900 s by stage, remove duplicate work, keep the cap"
status: proposed
proposed: "2026-11-01"
links:
  - { to: "[[d321-out-of-integration-hell]]", rel: refines }
  - { to: "[[d365-mandatory-formal-verification]]", rel: constrained-by }
---
# Presubmit run budget: allocate the 900 s by stage, remove duplicate work, keep the cap

Proposed 2026-11-01 by the Architect, Nyx Copperwing, at the Oracle's request (message 01M6C9144XXNDY6BNPX7H588F4). This is bboxy450-term's budget, so the decision is recorded here.

## Context

The presubmit run bound is a hard 900 s (`scripts/landing.py:878-881`). It measures every second that is not in the compile phase (`TwoClocks`, `landing.py:518-552`). Nothing splits it further, so no stage owns a share and none has a margin.

PR #87, presubmit on 63499b1e, attempt `528de3fc`, after E4 (d6fa3fd5): run 900.08 s, exit 124 `presubmit_timeout`. Before E4, d32f5b62 used 879.4 s (98%). The stages below come from the run's `phases.jsonl`, cut at the `presubmit:` command boundaries in `commands.log`:

| Stage (`presubmit_worker`, `landing.py:812-850`) | Run s | Compile s | Runs when |
|---|---:|---:|---|
| `make test-formal` | 277.9 | 120.3 | every code-tier cut |
| doc corpus | 1.3 | 6.5 | `.bboxy/*.md` changed |
| `quarantine-test.sh` (affected crates) | 391.2 | 212.7 | affected crates |
| clippy | 0.2 | 37.3 | affected crates |
| `test-landing.py --all` + `test-resource-ledger.py` | 162.3 | 0 | **any** `scripts/` path changed (`landing.py:831`) |
| `invariants.sh --affected` | 67.8 | 0 | every code-tier cut |
| **total** | **900.7** | 377.4 | |

The "284 s unclassified" is almost all `make test-formal`: 273.6 s in phase `other`. That covers the steps after model export, namely `scripts/test-formal.py`, the TLC and Yarn `verify` examples, and `scripts/formal.py`'s Java TLC parity. Each of those prints its own results, but they are not timed one by one. Before E4 the same block took 252.0 s, so E4's 67 imported harness models (`harness: imported 67 native models`) added about 9 s. E4 did not cause the overrun; the budget had no margin to begin with.

There are two pieces of duplicate or over-broad work:

1. **Invariants re-run tests that already passed.** `invariants.sh --affected` runs a cargo test batch (`... -p bboxy450-harness-host -p bboxy450-term -p bboxy450-term-mux --lib --test ctl`). That batch ran `ctl-ae36e8742bdd313b` again, the same binary hash that `quarantine-test.sh` had passed minutes earlier, for 54.0 s against the first run's 54.4 s. That is 58 s of test time spent twice.
2. **Landing fixtures run on any script change.** `test-landing.py --all` (161.6 s) runs whenever any `scripts/` path changes. PR #87 changed `scripts/ci-local.sh` and `scripts/harness-submodule.py`, and neither is landing machinery.

Load average during the run was about 4-5 on 10 CPUs. The bound is wall time on a shared host, so load is part of the allocation, not noise.

## Alternatives

- (a) Raise the 900 s cap. Rejected, as in d667: a cap is a budget, not a knob.
- (b) Retry until a run fits. Rejected: a coin flip on every landing, and it hides the overrun.
- (c) Move whole stages to ci-local's postsubmit. Rejected for formal and the affected tests: those are what presubmit exists for (d321, d365).
- (d) **Allocate the bound by stage with an explicit margin, and remove the duplicate and over-broad work.** Chosen.

## Choice

**Allocation of the unchanged 900 s run bound** for code-tier cuts:

| Stage | Allocation s | PR #87 measured s | PR #87 after R1-R3, worst case s |
|---|---:|---:|---:|
| formal (`make test-formal`) | 290 | 277.9 | 277.9 (no model reused) |
| affected tests (`quarantine-test.sh`) | 400 | 391.2 | 391.2 |
| landing fixtures | 60 | 162.3 | 162.3 (touches `ci-local.sh`, R2) |
| invariants and misc | 60 | 69.5 | about 12 (R1) |
| **margin** | **90 (10%)** | -0.7 | about 56 |

PR #87 is the worst case: it touches models' inputs, landing machinery and bbt-mux all at once. Even after R1-R3 it fits under the bound, but its fixture stage is over allocation, which R4 records as a finding. A code cut that touches neither landing machinery nor model bytes spends only model export on formal, and fixtures 0. R0 measures that figure. This decision does not assume it.

**R0. Classify.** `make test-formal` emits a phase event per sub-step: export, `test-formal.py`, TLC and Yarn verify, Java parity. The presubmit record carries `stage_seconds` per stage of the table above, plus the 1-minute load average at start and end. `unclassified_seconds` above 15 s is a finding.

**R1. No double-run tests.** When `quarantine-test.sh` passed a package in the same attempt and target, `invariants.sh --affected` gets the cargo invariants for that package with `--no-cargo`. A cargo-backed invariant whose test was filtered out of, or `#[ignore]`d in, the quarantine run is still run. Coverage is unchanged; the saving is about 58 s.

**R2. Scope the landing fixtures.** `test-landing.py --all` and `test-resource-ledger.py` run when the cut touches landing machinery: `scripts/landing.py`, `landing_*`, `land.sh`, `presubmit.sh`, `quarantine-test.sh`, `invariants.sh`, `ci-local.sh`, `resource-ledger*`, `test-landing.py`, `test-resource-ledger.py`, or a fixture they read. Other `scripts/` changes keep their own script checks. ci-local's full run (`scripts/ci-local.sh:70`) still runs `test-landing.py` on every postsubmit. PR #87 touched `ci-local.sh`, so under this list it would still run the fixtures; that list entry is deliberate, because ci-local drives postsubmit. For PR #87 the fixtures therefore stay at 162 s, and R1 is what brings it under the bound.

**R3. Reuse formal verdicts by content, never by assumption.** A Java-TLC or native parity verdict depends only on a model's `.tla` bytes, its expectation and the pinned tools. Presubmit still compiles and exports every model in every run. A model whose `tla_sha256`, expectation, contract metadata and tool pins all equal a green model in the archived evidence store (`formal.py:259` `archive_evidence`) takes that verdict, recorded as `reused_from: <evidence_sha256>`. Only changed models are re-checked. The receipt still binds to the candidate SHA and current input hashes (`formal.py:218-257`). This meets d365's rule: "validate source/model/tool hashes and refresh evidence after rebasing". Postsubmit `ci-local.sh` re-checks every model in full and never reuses. A cut that changes a tool pin re-checks every model.

**R4. Hard gate and judgement stay separate.** The 900 s run bound is still the only presubmit block. A stage over its allocation is an advisory finding in `landings.log` with the stage, seconds and load, and it never fails a landing. Two findings in a row for the same stage go to the Architect for re-allocation.

**R5. A records-only rebase keeps a gate log valid.** Context: landing PR #87 by `--gate-log` froze bboxy450-term main for about 75 minutes. Any push during the full ci-local run would move `origin/main`, and `gate_log_source_mismatch` (`landing.py:1098`) would void the log. That freeze binds every pusher, which makes it a chokepoint. Rule: a gate log for candidate C is accepted for C′, C rebased onto a newer `origin/main`, when all of the following hold.
  1. `git diff --quiet C C′ -- . ':!.bboxy'` exits 0, so the tested tree and C′ differ only under `.bboxy/`.
  2. No changed `.bboxy/` path is named by code: `git grep -F -l <path> C′ -- crates scripts Makefile verification lab` is empty for each changed path. Example: `.bboxy/specs/e2e-acceptance.md` is read by a bbt-ui test.
  3. The records gate set from `scripts/records-push.sh` passes on C′.
  4. C′ is signed, and the landing record carries `gate_log_source: C`, `records_delta: [paths]` and `records_checks_exit: 0`.

If any condition fails, the landing refuses with `gate_log_source_mismatch`, as today. Records pushes then never wait on a landing, and no landing waits on records.

**Immediate path for PR #87 (no change needed).** `land --gate-log` already admits a validated full exact-SHA ci-local log for any tier (`landing.py:1511`; the d667 ruling, whose record is not yet on main). `land --full` runs that canonical check itself. Either lands 63499b1e with the cap unchanged, so a retry at lower load is not the only route.

## Verification

| Rule | Evidence | Broken control |
|---|---|---|
| R0 | `test-landing.py` case: a fixture `make test-formal` with four sub-steps gives `stage_seconds.formal` equal to their sum and `unclassified_seconds` < 1 | a sub-step without a phase event shows up as unclassified and raises the finding |
| R1 | `test-landing.py` case: a presubmit over package P runs the ctl test binary once; `invariants.sh` gets `--no-cargo` for P | an invariant test filtered out of the quarantine run still runs |
| R2 | `test-landing.py` case: a cut touching only `scripts/harness-submodule.py` runs no landing fixtures | a cut touching `scripts/landing.py` runs them |
| R3 | `scripts/test-formal.py` case: an unchanged model is reused with `reused_from` set; a one-byte `.tla` change re-checks only that model | a changed tool pin re-checks every model; postsubmit evidence never contains `reused_from` |
| R4 | `test-landing.py` case: a stage over allocation writes a finding and the exit stays 0 | a run over 900 s still exits 124 |
| R5 | `test-landing.py` case: a gate log for C lands C′ when C..C′ adds only an unreferenced `.bboxy/decisions/*.md`, and the landing record names `gate_log_source` | a C′ that also changes one byte under `crates/` refuses `gate_log_source_mismatch`; a C′ that changes `.bboxy/specs/e2e-acceptance.md` refuses, because the path is named by code |
| Budget | two consecutive code-tier presubmits touching bbt-mux show ≥ 90 s margin in `stage_seconds` | |

## Change log

- **2026-11-01, at the Oracle's request** (message 01M6CC26D3KMZNQY5GFKC027JR). Added R5 after the 75-minute main freeze for PR #87 (landed at `afda1b63`).

## Owner and status

Owner: Nyx Copperwing <ARCHITECT> (allocation). Implementation goes to an Executor under an Oracle brief (E1c, task #4900); landing machinery is in `scripts/landing.py`, `scripts/invariants.sh`, `scripts/formal.py` and `Makefile`. Status stays proposed until the Oracle accepts it, and the allocation is re-measured after R1-R3 land.
