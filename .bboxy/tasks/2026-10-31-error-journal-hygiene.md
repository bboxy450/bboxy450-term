---
artifacts:
- '[[01M6AMG85MXGY0E78QTJ4WA3GW]]'
- '[[01M6AWBRGFFXFZ75GME7HT1DD2]]'
assignee: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T01:31:38.218Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6ABQK94JATCJFWMSWQQSN54
number: 509
provenance:
  digest: sha256:f76dbf129bae4e78174495bbe2faec74a4cfaa01e36e26b0b448243a4e000f91
  frontmatter_digest: sha256:ce8eb76b758555ba238113b176f78485ebb4087af3ae5c6be8eba7c700f74883
  log: .bboxy/log/2026-11-01.jsonl#4948
  sig: Q/nXX+rjL5v6fPr4DJ3hoMm71FJAzl15cIpmEhjlAjbakMHAhoK8R5BjdVtsIbPiKrZomwW3F3h7y4n1Ai7bDg==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
rejection:
  by: seat:01M6ABH71TVWGZ621NTD701PSH
  reason: 'This report and my ruling 01M6ARH3NBYDS4NXQVP1TSMA39 crossed in transit, so here it is again. The test-landing.py:698 failure is a load-sensitive fixture and not yours. The same test passed in the full green make test on e7c6dd6e, and Lumen owns the fix under #572. Rebase onto origin/main e7c6dd6e now; red CI on the old base is no reason to hold the rebase. Then run make test once on the new HEAD with the ordinary cache, then sh scripts/land.sh --gate-log target/ci-local.log. If only that same deadline case fails again, stop and report the log line, with no third run.'
report:
  commit: 2e7a6791b1d5727757687333b91a604a7082ad65
  op_id: 01M6AX0G2TPFJG7W3E9CF1XR2T
  reported_by: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
  summary: |-
    LANDED on main: 2e7a6791b1d5727757687333b91a604a7082ad65, independently confirmed by git ls-remote. Includes signed e9f8cff0 and 2e7a6791; approved Boxy identity/key; worktree clean and aligned with origin/main. Committed report: .bboxy/runs/error-journal-hygiene-20261031.log.

    Private fixture homes and authoritative account/bootstrap routing isolate journals; unknown optional MCP probes retain -32601 without journal rows, while real failures remain recorded. Missing-capability ctl status reproduces the redacted category privately; exact historical masked reason remains unrecoverable. Rebase retained both DiagnosticRouting and upstream TaskBytePage coverage/source mappings.

    Full make test on exact landed SHA exited 0 with ordinary cache: all133 landing fixtures (including deadline case), workspace build/strict Clippy/tests/doctests, formal, smoke and27 invariants passed (11 designated slow checks skipped). Formal112 models/31 crates/46 contracts; receipt65043216. Earlier focused diagnostics/control/boundaries/license checks exited0 as recorded in the committed report.

    After admission refusals75, waited for the Oracle-specified PIDs68871/76506; when one exited, ran sh scripts/land.sh --gate-log target/ci-local.log ONCE: exit0, reused exact candidate full normal gate and landed without another rebase. Log /tmp/rook-journal-land-after-slot.log, exit file matching0. make check-formal-evidence then exited0. No slots bypassed or thresholds altered. The earlier unlanded report operation01M6AWC299FDR5AKSNJE1AXM4T remains unknown/missing and was not replayed; this is the new landed result.
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Error journal: keep test fixtures out of the operator journal; do not journal server/discover probes'
type: task
updated: 2026-11-01T06:33:45.204Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test-boundaries
verify_result:
  command: make test-boundaries
  evidence:
  - criterion: 1 — isolation and broken control
    reference: .bboxy/runs/error-journal-hygiene-20261031.log; main.rs:9726,9769; diagnostic tests:64,219; fixture helpers ctl/ctl_records/headless/e2e_night/plugins/r5_stall/recovery. Prior make test-error-journal0 (5 passed incl actual provider + failing ctl with untouched decoy journal); control0 (guard removed ->2, restored ->0); boundaries0 (37 passed). Oracle already verified implementation.
  - criterion: 2,3 — probe classification and redacted failure
    reference: crates/bbt-mux/src/mcp/bridge.rs:218 and tests/diagnostics.rs:126 verify -32601 probes omitted and real tool/-32602 errors retained. ctl.rs:168,691 and bbt-rt/src/error_journal.rs:150 explain capability redaction; diagnostics.rs:219 reproduces private redacted app/status exit1. Exact historical reason unrecoverable; operator journal untouched/unread.
  - criterion: Exact landed-source full CI/formal
    reference: target/ci-local.log:1 source2e7a6791, :4903 normal result0. Stable copy /tmp/rook-journal-rebased-ci-full.log sha256 e036b96a95f5a14acc1743777cf4cbe3ff61fc6ba37a80abc1a3a60d5077689f (artifact01M6AWBRGFFXFZ75GME7HT1DD2). Formal receipt65043216a3d3a44a8a57025187e51e85eff6320d0e1fb94362a19c20a1264935:112models/31crates/46contracts, clean2e7a6791; make check-formal-evidence0 after landing.
  - criterion: Landed main and signed provenance
    reference: '/tmp/rook-journal-land-after-slot.log: reused full normal exact candidate2e7a6791 and LANDED; /tmp/rook-journal-land-after-slot.exit=0. git ls-remote origin refs/heads/main=2e7a6791b1d5727757687333b91a604a7082ad65. git status clean/aligned. Both commits G with key844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>.'
  exit_code: 0
---
# Error journal hygiene from `ctl debug errors` triage

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-journal`, branch `journal-hygiene` (from origin/main 42320d93). Only this worktree.

## Evidence (Oracle, 2026-10-31, `bboxy450-term ctl debug errors --since 0`, 188 rows)
- Fixture rows in the operator's real journal: `source:provider op:request "LinkDown: Cloud { provider: \"nebula\", host: \"worker-east\" }"` and `worker-west` (pid 34930, at_ms 1793292533697), `LinkDown: Ssh { host: \"build-box\" }` (pid 88923, 1793333121608). These hostnames exist only in `crates/bbt-proto/src/hosts.rs`, `crates/bbt-mux/tests/provider*.rs`, `crates/bboxy450-term/tests/hosts.rs`. Same run wrote `source:ctl op:app/status "[redacted sensitive diagnostic line]"` (pids 34892/34903/34931, ms apart) and later pid 20911; plus `ctl command "usage: app verbs: …"` (pid 77247). An earlier fix 6d8e49e8 isolated malformed ctl diagnostics but tests still reach the operator journal.
- `source:mcp op:server/discover "Method not found"` from a live Claude seat (at_ms 1793496490746): a client capability probe journaled as an error.
- Stale, no action: 113 `hook input bound` codex-notify rows all before #80 fix fe183086; 66 MCP refusals are correct refusals of agent misuse.

## Required outcome
1. Every test that spawns ctl/app/provider code resolves its error journal inside its private test home; none can write the operator's journal. Add a guard test that runs the fixture provider + `ctl app status` failure path and asserts the operator-journal path is untouched (broken control: remove isolation → fails).
2. `server/discover` (and other unknown optional JSON-RPC methods that are pure probes) answers per MCP/JSON-RPC but is not written as an error row; real tool errors still journal. Test both.
3. Find the cause of the `app/status` redacted rows; if it is the same leak, item 1 covers it; if it is a real product failure, fix it or report the exact cause.

## Verify
`make test-boundaries`, the new tests, strict clippy on touched crates, `make test` on committed source. Land with `scripts/land.sh` if green.

## Communication (operator order: no token burn)
No progress DMs, no channel posts, no acknowledgements. One `task_report` at the end: files+mechanism, commands+exit codes, evidence per item, gaps, commit. Message the Oracle only for a blocker that stops all work, in ≤3 lines.


## Report from seat:01M6ABQRB8SJNZHP6NRHQD3BC8

Implemented; NOT LANDED. Clean journal-hygiene tree, two signed Boxy commits: 81028084327464d6a2ab3849adcc7d292408923e and 50a9af354e1f14ff9985c4bf761779338c5c5445 (approved key 844ECC0587DE7AB0). Committed scoped report: .bboxy/runs/error-journal-hygiene-20261031.log; artifact 01M6AMG85MXGY0E78QTJ4WA3GW. Current knowledge_write schema has no report type, so the requested .bboxy/reports record/Report trailer could not be produced through the admitted KB writer.

Private HOME/run fixture helpers and authoritative private-account/bootstrap journal selection prevent inherited operator-run leakage. MCP method-not-found probes retain -32601 responses without error rows; real tool/malformed-request failures remain journaled. The new real provider + failing ctl status guard preserves a decoy operator journal. Missing-capability endpoint rejection reproduces the redacted status category; historical redaction cannot distinguish that from the capability-refusal EOF reason, so the exact old reason remains unproven.

make test-error-journal, make test-error-journal-control, make test-boundaries and make test-licenses all exited 0; focused target includes strict all-target Clippy on mux/app. Full make test exited 0 on signed a9ca91de4409dbed64cb319b18c938c62d5a94a3 before rebasing (target/ci-local.log). Private per-command Java tmp resolved an earlier TLC standard-module temp-file failure; no global configuration changed. Current-source formal verification then passed 110 models / 31 crates / 45 contracts on 50a9af35.

Landing blocker: direct scoped scripts/land.sh exited 77 (presubmit 124, 900s cap; one unrelated conformance assertion had failed). Canonical make land RUST_TEST_THREADS=2 with private build directory retried on current main: that case and all 271 mux unit tests passed; 57 test groups completed, no assertion failure in the retry log. It still exhausted RUN 900.073/900s (592.149 test + 307.924 other), BUILD 1169.145/1800s; stopped during hook_gate, so remaining checks are unproven. make logged recipe/script exit 77, presubmit 124; outer Make exit was not retained. No push; remote main remains ceaf24e03abb46ae91db374530afc86c3f08f93a. No third unchanged retry, budget override or unrelated code edits. The landing runtime accounting/budget needs disposition outside this journal brief.

```json
{
  "commit": "50a9af354e1f14ff9985c4bf761779338c5c5445",
  "op_id": "01M6APN3B4A3SJKC7V11WDR221",
  "verify_result": {
    "command": "make test-boundaries",
    "evidence": [
      {
        "criterion": "1 — private journal routing, real provider/ctl guard and broken control",
        "reference": "crates/bboxy450-term/src/main.rs:9726,9769; tests/{ctl,ctl_records,headless,e2e_night,plugins,r5_stall,recovery}.rs; tests/diagnostics.rs:64,219. make test-error-journal: 5 passed, exit 0 (/tmp/rook-journal-focused-fixed.log). make test-error-journal-control: exit 0, removed bootstrap guard -> nested exit 2/private-journal assertion, restored -> 0 (/tmp/rook-journal-control.log)."
      },
      {
        "criterion": "2 — optional probes omitted, real errors retained",
        "reference": "crates/bbt-mux/src/mcp/bridge.rs:218; crates/bboxy450-term/tests/diagnostics.rs:126. Actual MCP subprocess: server/discover before/after initialize and other optional probes return -32601; real forwarded tool error and -32602 malformed request produce exactly two private rows. Focused exit 0; both new ordinary diagnostic cases also passed in the latest landing log."
      },
      {
        "criterion": "3 — redacted status mechanism and historical limit",
        "reference": "crates/bboxy450-term/src/ctl.rs:168,691; crates/bbt-rt/src/error_journal.rs:150. Missing capability and capability-refused EOF are scrubbed. diagnostics.rs:219 reproduces missing-capability ctl app status exit 1 + redacted private row; exact historical reason cannot be recovered from redacted rows. Operator journal untouched/unread."
      },
      {
        "criterion": "Required checks and full committed-source CI",
        "reference": "make test-boundaries exit 0, 37 passed (/tmp/rook-journal-boundaries.log); strict mux/app all-target Clippy exit 0 in focused target; make test-licenses exit 0; make test normal exit 0 on a9ca91de4409dbed64cb319b18c938c62d5a94a3 (target/ci-local.log and /tmp/rook-journal-make-test-private.log). Full normal CI was not rerun on rebased HEAD; its scoped gate is incomplete."
      },
      {
        "criterion": "Current-source formal evidence and bounds",
        "reference": "make test-formal exit 0 on clean 50a9af354e1f14ff9985c4bf761779338c5c5445: 110 models, 31 crates, 45 contracts. /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/a363be75d69d1175e8bd15f5c6f4f0a47a301c88b08f639908a952c6fe19a8f5/evidence.json. DiagnosticRouting passes and BrokenDiagnosticRouting has the expected safety failure, both 5 states/6 edges; finite atomic-routing abstraction/IO limits recorded in verification/contracts.toml:315."
      },
      {
        "criterion": "Landing failed honestly; no publication",
        "reference": "/tmp/rook-journal-land-scoped.log: script 77/presubmit timeout124. /tmp/rook-journal-make-land.log and target/land-attempt-1.log: canonical retry recipe/script77, presubmit124, verdict red at runtime900.073s. Exact retry log: /Users/feral/.local/state/bboxy450-term/landing/presubmits/50a9af354e1f14ff9985c4bf761779338c5c5445-d44d7306/commands.log. Held-conformance case passes; no FAILED/error lines; run interrupted during hook_gate. git status clean/ahead2; git log signatures G; git ls-remote confirms main ceaf24e03abb46ae91db374530afc86c3f08f93a."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M6ABQRB8SJNZHP6NRHQD3BC8

Rework verification blocked; NOT LANDED. Source unchanged at signed 50a9af35, clean journal-hygiene worktree. Committed implementation/evidence: .bboxy/runs/error-journal-hygiene-20261031.log. Prior private-journal guards, MCP probe/tool-error test, broken control, boundary and strict Clippy results remain as reported.

Full make test on 50a9af35 first exited 2: I reused target/private-build from the temporary landing checkout; fixture artifacts retained its deleted CARGO_MANIFEST_DIR. Preserved log /tmp/rook-journal-head-ci-stale-cache.log. Corrected by unsetting CARGO_BUILD_BUILD_DIR and reran with ordinary worktree-local cache. This exited 2 in landing infrastructure fixtures before Rust: scripts/test-landing.py:698 expected build_cap, recorded presubmit_timeout; 133 fixture tests, one failure. Both budgets are 0.5s (scripts/landing.py:877-881), runner hardcodes eight workers; no supported serial knob. Current log target/ci-local.log; launcher/exit /tmp/rook-journal-head-ci-corrected.{log,exit}.

No source edits, threshold changes, skipped checks, green-log substitution or push. No exact-SHA normal green log exists, so scripts/land.sh was not invoked again. Blocker DM 01M6ARGJVWKNE6MGR7ASRB8RR6 gives the out-of-scope assertion for Oracle disposition. Origin/main advanced to e7c6dd6e; local branch has not been rebased while full CI is red.

```json
{
  "commit": "50a9af354e1f14ff9985c4bf761779338c5c5445",
  "op_id": "01M6ARKSHHJA4P6ET8C897A9MV",
  "verify_result": {
    "command": "make test-boundaries",
    "evidence": [
      {
        "criterion": "1,2,3 — implemented behavior retained",
        "reference": ".bboxy/runs/error-journal-hygiene-20261031.log committed in 81028084,50a9af35; diagnostics.rs:64,126,219; bridge.rs:218; main.rs:9726. Prior focused test/control/boundary/strict-Clippy exits 0. Exact historical redacted reason unrecoverable; reproduced missing-capability row isolated."
      },
      {
        "criterion": "Requested exact-SHA normal CI",
        "reference": "make test on 50a9af354e1f14ff9985c4bf761779338c5c5445 exited 2 twice: first stale artifact provenance (/tmp/rook-journal-head-ci-stale-cache.log); corrected run failed scripts/test-landing.py:698, PresubmitTests.test_deadline_is_enforced_and_does_not_kill_an_unrelated_process, expected build_cap, actual presubmit_timeout. target/ci-local.log; /tmp/rook-journal-head-ci-corrected.exit contains 2. No green proof."
      },
      {
        "criterion": "Landing and state",
        "reference": "No new landing invocation/push because full CI failed. Clean worktree, HEAD50a9af35, two signed local commits; origin/main e7c6dd6e. Current-source formal receipt a363be75d69d1175e8bd15f5c6f4f0a47a301c88b08f639908a952c6fe19a8f5 remains available (110 models,31 crates,45 contracts)."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M6ABQRB8SJNZHP6NRHQD3BC8

LANDED on main: 2e7a6791b1d5727757687333b91a604a7082ad65, independently confirmed by git ls-remote. Includes signed e9f8cff0 and 2e7a6791; approved Boxy identity/key; worktree clean and aligned with origin/main. Committed report: .bboxy/runs/error-journal-hygiene-20261031.log.

Private fixture homes and authoritative account/bootstrap routing isolate journals; unknown optional MCP probes retain -32601 without journal rows, while real failures remain recorded. Missing-capability ctl status reproduces the redacted category privately; exact historical masked reason remains unrecoverable. Rebase retained both DiagnosticRouting and upstream TaskBytePage coverage/source mappings.

Full make test on exact landed SHA exited 0 with ordinary cache: all133 landing fixtures (including deadline case), workspace build/strict Clippy/tests/doctests, formal, smoke and27 invariants passed (11 designated slow checks skipped). Formal112 models/31 crates/46 contracts; receipt65043216. Earlier focused diagnostics/control/boundaries/license checks exited0 as recorded in the committed report.

After admission refusals75, waited for the Oracle-specified PIDs68871/76506; when one exited, ran sh scripts/land.sh --gate-log target/ci-local.log ONCE: exit0, reused exact candidate full normal gate and landed without another rebase. Log /tmp/rook-journal-land-after-slot.log, exit file matching0. make check-formal-evidence then exited0. No slots bypassed or thresholds altered. The earlier unlanded report operation01M6AWC299FDR5AKSNJE1AXM4T remains unknown/missing and was not replayed; this is the new landed result.

```json
{
  "commit": "2e7a6791b1d5727757687333b91a604a7082ad65",
  "op_id": "01M6AX0G2TPFJG7W3E9CF1XR2T",
  "verify_result": {
    "command": "make test-boundaries",
    "evidence": [
      {
        "criterion": "1 — isolation and broken control",
        "reference": ".bboxy/runs/error-journal-hygiene-20261031.log; main.rs:9726,9769; diagnostic tests:64,219; fixture helpers ctl/ctl_records/headless/e2e_night/plugins/r5_stall/recovery. Prior make test-error-journal0 (5 passed incl actual provider + failing ctl with untouched decoy journal); control0 (guard removed ->2, restored ->0); boundaries0 (37 passed). Oracle already verified implementation."
      },
      {
        "criterion": "2,3 — probe classification and redacted failure",
        "reference": "crates/bbt-mux/src/mcp/bridge.rs:218 and tests/diagnostics.rs:126 verify -32601 probes omitted and real tool/-32602 errors retained. ctl.rs:168,691 and bbt-rt/src/error_journal.rs:150 explain capability redaction; diagnostics.rs:219 reproduces private redacted app/status exit1. Exact historical reason unrecoverable; operator journal untouched/unread."
      },
      {
        "criterion": "Exact landed-source full CI/formal",
        "reference": "target/ci-local.log:1 source2e7a6791, :4903 normal result0. Stable copy /tmp/rook-journal-rebased-ci-full.log sha256 e036b96a95f5a14acc1743777cf4cbe3ff61fc6ba37a80abc1a3a60d5077689f (artifact01M6AWBRGFFXFZ75GME7HT1DD2). Formal receipt65043216a3d3a44a8a57025187e51e85eff6320d0e1fb94362a19c20a1264935:112models/31crates/46contracts, clean2e7a6791; make check-formal-evidence0 after landing."
      },
      {
        "criterion": "Landed main and signed provenance",
        "reference": "/tmp/rook-journal-land-after-slot.log: reused full normal exact candidate2e7a6791 and LANDED; /tmp/rook-journal-land-after-slot.exit=0. git ls-remote origin refs/heads/main=2e7a6791b1d5727757687333b91a604a7082ad65. git status clean/aligned. Both commits G with key844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>."
      }
    ],
    "exit_code": 0
  }
}
```
