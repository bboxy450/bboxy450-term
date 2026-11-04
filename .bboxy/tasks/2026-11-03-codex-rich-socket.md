---
type: task
title: 'Recover rich-pane B1/B2: seat socket lifecycle and Unix bridge'
status: accepted
role: executor
assignee: seat:01M6HCVV32KS845WHVD5FCX7QE
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
worktree: /Users/feral/mydev/bboxy450-harness-pane-b12
allowed_paths:
- src/frontends/**
- src/gateway/**
- src/main.rs
- tests/**
- verification/**
- scripts/**
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-rich-socket.json
verify: RUSTC_WRAPPER= make test
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYCQB66JTBKPHE1YE4QJB
updated: 2026-11-03T22:05:21.993Z
created: 2026-11-03T19:07:30.619Z
number: 15249
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
artifacts:
- '[[01M6HPB4DCT2G54E011RY6GFZW]]'
verify_result:
  command: RUSTC_WRAPPER= make test
  exit_code: 0
  evidence:
  - criterion: Preserve interrupted source and original tips; integrate the published standalone recovery
    reference: /tmp/codex-rich-socket-recovery/preservation-audit.json — all20 hashes equal signed433107e; integrated-signatures-and-preservation.json — af65c6c ancestor, nine approved signatures and six protected files byte-identical; recovery/rich-socket-{original,interrupted,verified}-2026-11-03 retained.
  - criterion: Seat binds0600 alias and per-session sockets; switch/ABA/resume/exit fence identities and clean addresses
    reference: tests/support/seat_socket.rs:234,370,374,428 — private PTY/pipes and crash/reopen duplicate witness; final-make-test.log — canonical default/acp,test-fixtures process suites exit0; src/frontends/headless.rs:432 and src/frontends/seat.rs:23,57.
  - criterion: Unix and InProc share the off-UI rich face with honest admission and lifecycle ownership
    reference: tests/gui_bridge.rs:148 — all12 locked IR v1 fixtures including1.2 yield equal Transcript/layout; :224,293,389,487 — asynchronous dispatch, cancellation, A/B/A, ordered stale reply/detach. final-integrated-gui-gate.json — all5 rows exit0.
  - criterion: Stateful lifecycle, retries, cancellation, stale/duplicate completion, fairness and broken controls
    reference: tests/formal/models.rs:3219,3338,3495,3634; verification/contracts.toml — four owned contracts/nine controls with finite bounds and implementation-only limits. final-report-head-formal.log — exit0; verification/evidence/5a0fd20f72fe9cfe2c4b50a69c9050dc4aa0a8c4bd3a2c743bca2eca6ccc6eaa/evidence.json — current2fb78a0,112 complete native/Java graph/verdict matches,38 contracts,one crate,dirty=false.
  - criterion: 'Four concurrent actual views share one clock: aggregate drain<=300us/projection<=600us, no lost tails'
    reference: 'src/frontends/gui/budget.rs:72; tests/gui_budget.rs:239 — four actual Bridge/HarnessView instances,128 deltas/view,22 frames, zero duplicate/lost durable work. scripts/test-gui-budget-control.py — production guard removed: cargo101 at aggregate projection assertion; exact restoration green0 in final-make-test.log. Injected allocation only; CPU margins UNMEASURED.'
  - criterion: False-subscription idle socket/actual term consumer stays live>20s with fenced Status and no semantic activity; old-filter control fails
    reference: src/gateway/mod.rs:299; tests/gateway_unix.rs:182,280; /tmp/harness-heartbeat-control-om5xuq17/{red,green}.log — actual unchanged rich::Connection uses ephemeral=false,21.001s,epoch1/head2,transport age6.015s, unchanged JSONL. Old filter fails real consumer before20s and socket assertion; red101/restored green0. actual-term-consumer-source.json records checkout248d18a/source/probe hashes and no consumer writes.
  - criterion: Canonical unattended verification, actual exits and current source binding
    reference: /tmp/codex-rich-socket-recovery/final-canonical-result.json — RUSTC_WRAPPER= make test exit0,21:34:32Z–21:50:28Z,no other full gate at launch; final-integrated-input-audit.json — all273 input hashes equal,21 canonical rows/5 GUI rows0,private_HOME=true/operator_input=false; final-make-test.log — packaging and hidden native startup smoke0.
  - criterion: Signed indexed report, retained evidence, clean handoff and preserved metadata
    reference: .bboxy/reports/2026-11-03-codex-rich-socket-integrated.md — indexed report01M6HPAEC8BJKZ1VER3H6Y1P72/#18870,Report trailer on signed2fb78a0; artifact01M6HPB4DCT2G54E011RY6GFZW accepted. final-report-checks.json — KB/secrets/signature/doc-only0; final-retained-evidence-check.log — current archive verification0; final-clean-state.json and preserved-generated-final.json — clean status,273 hashes equal,five generated files preserved/restored,preliminary report unchanged.
report:
  summary: Implemented the amended B1/B2/§9 assignment in pane/b1-b2. Signed source49fced5 includes published af65c6c, preserved interrupted work/tips and recovered quiet/IR1.2/startup contracts, shared Unix/InProc Bridge, socket lifecycle/idempotence, four-view aggregate allocation and unconditional existing idle Status routing. RUSTC_WRAPPER= make test exited0 on49fced5; all273 inputs still match report-only signed HEAD2fb78a0. Report-HEAD native/Java parity and retained-evidence verification exited0 (112 complete graphs/verdicts,38 contracts,one crate). Four-view tail and real false-subscription/actual term consumer heartbeat controls returned red101/restored green0; the unchanged consumer remained live21.001s without durable semantic activity. Indexed append-only report18870 supersedes the preliminary report; artifact claim accepted. Worktree is clean, generated metadata/raw logs retained outside Git. Actual loaded-machine CPU/memory margins remain UNMEASURED and Side A owns its remaining integration/benchmark witnesses.
  commit: 2fb78a017992f73468b6fa9fe5edbc48ba27f05c
  reported_by: seat:01M6HCVV32KS845WHVD5FCX7QE
  op_id: 01M6HPHD2TABP6M2ZN624RH0D7
  verify_revision: 0
  report: '[[reports/2026-11-03-codex-rich-socket-integrated]]'
provenance:
  digest: sha256:c3ca95abb524eddef49f2abe542a50ceba8b099d7aca142bfe2a2219b848ae04
  frontmatter_digest: sha256:ecad3d7320c6f646eca4c9a7b3c2333e371974d1f96109928d44a6da5978a7ed
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: pi6iLuGyxiglnDvl0zHqeIx2BcABeh6ySIKO6vc/qgP2dcsYPVQsUpUUY/TX3tF2XKXfHvD6tL5PCfiVH1gTCA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#105029
---
ROLE: EXECUTOR. Direct operator continuation under the Codex Oracle: the harness pane is still the line-mode frontend; implement the shared native rich face. Work ONLY in /Users/feral/mydev/bboxy450-harness-pane-b12, current branch pane/b1-b2. The previous session was interrupted with 20 source files already changed. Preserve and inspect that work; no reset, stash, broad stage or deletion. Baseline hashes are /tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0/preservation-before.json. Read current AGENTS.md, ATTRIBUTION.md and the ICD FROM origin/main (the shared root checkout is old): .bboxy/interfaces/rich-pane.md, gateway.md, embed-seat.md and log-ir-v1 spec. Rebase/integrate current main safely after committing/recovering the interrupted source. Leave original branch tips reachable.

Outcome: Side B §2/§3/§9 of rich-pane: seat --socket binds mode 0600 plus the per-session socket; switch rebinds, resume/restart reconnects, exit unlinks; session/epoch fencing and deterministic op IDs prevent duplicate or misrouted turns. Bridge::connect feeds the same Mailbox/FrameState on bbt_rt; Unix and InProc produce the same frame/layout. Keep the standalone and embedded face shared.

Use the existing uncommitted tests/support/seat_socket.rs, seat_process and gui_bridge work as evidence, not as proof. Extend formal contracts/model plus broken controls for lifecycle, cancellation, stale/duplicate completion and A→B→A. Explicitly record timing/fairness bounds and implementation-only behavior. No operator click, real credential, Keychain prompt or OS input; scripted/private identities only. Readiness/admission must remain honest.

Do not recruit, push, merge, publish or install. Preserve metadata/raw logs outside Git; signed source/report commits and exact path staging. Full report .bboxy/reports/2026-11-03-codex-rich-socket.md with Report trailer, file:line mechanisms and actual exits. Register report through knowledge tools for task_report. Canonical verify is RUSTC_WRAPPER= make test; also run GUI/bridge/socket process witnesses and current native/Java formal parity. One full make test at a time machine-wide: inspect running canonical gates, continue other useful checks while one is active, then self-serve the free gate; never wait for operator approval or end the turn merely waiting on a background command.

Coordinate with the Codex rich-host task for term Side A and the standalone recovery task. Send a short socket schema/ready commit handoff when the consumer can test; it is feedback, not scope expansion. Report once done, or a real all-work blocker with evidence.

Final-source checkpoint, 2026-11-03: recovery is published through origin/main af65c6cf82d9dde6314b1771b9314061b0e24212 (ea75307 implementation plus indexed report/summary). The Oracle independently verified all 270 canonical input hashes, signed commits, native/Java receipts, signed installation and installed private startup smoke. Your 46190c6 source predates that recovery and is preliminary. Preserve the existing run/results, then integrate/rebase published main before final canonical verification; keep quiet Native/Usage geometry, phase heartbeats, IR1.2 origin/Quiet semantics, startup bitmap alias and all controls alongside B1/B2. Do not report/land the older code as the final combined candidate.

ICD §9's budget witness is included in this task's existing scope: extend tests/gui_budget.rs to four concurrent views on one injected clock; aggregate drain <=0.3 ms and projection <=0.6 ms with no dropped admitted work, including a deliberate budget-tail guard failure and restored green. Per-view timing alone is not this witness. Coordinate the term bench row with rich-host; label actual measurements and unmeasured margins accurately. A new Codex native-visual task (seat 01M6HHK04M27ANJ2YPTBSC4B0Y) owns only the two carried gui/face.rs presentation defects and two audit passes; no Bridge/seat/gateway/IR changes. Continue useful work and give a short ready signed source handoff; no operator wait.

Gateway idle contract clarification from the existing interface: gateway.md §4 flow/liveness requires a heartbeat Status every 15 seconds while idle; a silent client is retained, with only unread-buffer lag closing it. Term's private consumer correctly treats >20 seconds without a validated transport frame as stale. Implement/verify the existing 15-second Status transport heartbeat with current session/epoch fencing, not a new semantic progress event or a JSONL activity timestamp. Add a private >20-second silent-subscriber witness and a deliberate missing-heartbeat control, preserve last semantic activity timestamps, and test with the term consumer. This is existing gateway/rich-pane scope, not an extension of wire semantics or permission for unrelated features. No operator input; record timing bounds and unmodeled scheduler behavior honestly.

Oracle ruling on the concrete idle-liveness boundary: gateway.md §4 requires a Status heartbeat on every idle socket. The current Gateway fan-out (src/gateway/mod.rs:297) suppresses all seq=None envelopes when subscriber.ephemeral=false; term's delivery client (wake-up/rich.rs:280) uses that false flag. Therefore preserve the existing 15-second backend idle Status timers and correct the filter so existing transport Status reaches subscribers regardless of their item-ephemeral preference. Keep item deltas/snapshots filtered as requested, preserve session/epoch fences, and introduce no new frame variant, fake progress, duplicate timer or durable activity record. Treat this as implementing the existing unconditional heartbeat contract. Update the relevant bounded model/control/source mapping for this routing property and prove the real false-subscription consumer stays live past20s, plus a deliberate old-filter failure and restored green. Record OS/timer fairness limits and keep the distinction between transport freshness and semantic progress. Freeze the final canonical source after this correction/witness; d327b37 is verified budget-component evidence, not final whole-task acceptance.

## Report from seat:01M6HCVV32KS845WHVD5FCX7QE

Implemented the amended B1/B2/§9 assignment in pane/b1-b2. Signed source49fced5 includes published af65c6c, preserved interrupted work/tips and recovered quiet/IR1.2/startup contracts, shared Unix/InProc Bridge, socket lifecycle/idempotence, four-view aggregate allocation and unconditional existing idle Status routing. RUSTC_WRAPPER= make test exited0 on49fced5; all273 inputs still match report-only signed HEAD2fb78a0. Report-HEAD native/Java parity and retained-evidence verification exited0 (112 complete graphs/verdicts,38 contracts,one crate). Four-view tail and real false-subscription/actual term consumer heartbeat controls returned red101/restored green0; the unchanged consumer remained live21.001s without durable semantic activity. Indexed append-only report18870 supersedes the preliminary report; artifact claim accepted. Worktree is clean, generated metadata/raw logs retained outside Git. Actual loaded-machine CPU/memory margins remain UNMEASURED and Side A owns its remaining integration/benchmark witnesses.

```json
{
  "op_id": "01M6HPHD2TABP6M2ZN624RH0D7",
  "commit": "2fb78a017992f73468b6fa9fe5edbc48ba27f05c",
  "report": "[[reports/2026-11-03-codex-rich-socket-integrated]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER= make test",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Preserve interrupted source and original tips; integrate the published standalone recovery",
        "reference": "/tmp/codex-rich-socket-recovery/preservation-audit.json — all20 hashes equal signed433107e; integrated-signatures-and-preservation.json — af65c6c ancestor, nine approved signatures and six protected files byte-identical; recovery/rich-socket-{original,interrupted,verified}-2026-11-03 retained."
      },
      {
        "criterion": "Seat binds0600 alias and per-session sockets; switch/ABA/resume/exit fence identities and clean addresses",
        "reference": "tests/support/seat_socket.rs:234,370,374,428 — private PTY/pipes and crash/reopen duplicate witness; final-make-test.log — canonical default/acp,test-fixtures process suites exit0; src/frontends/headless.rs:432 and src/frontends/seat.rs:23,57."
      },
      {
        "criterion": "Unix and InProc share the off-UI rich face with honest admission and lifecycle ownership",
        "reference": "tests/gui_bridge.rs:148 — all12 locked IR v1 fixtures including1.2 yield equal Transcript/layout; :224,293,389,487 — asynchronous dispatch, cancellation, A/B/A, ordered stale reply/detach. final-integrated-gui-gate.json — all5 rows exit0."
      },
      {
        "criterion": "Stateful lifecycle, retries, cancellation, stale/duplicate completion, fairness and broken controls",
        "reference": "tests/formal/models.rs:3219,3338,3495,3634; verification/contracts.toml — four owned contracts/nine controls with finite bounds and implementation-only limits. final-report-head-formal.log — exit0; verification/evidence/5a0fd20f72fe9cfe2c4b50a69c9050dc4aa0a8c4bd3a2c743bca2eca6ccc6eaa/evidence.json — current2fb78a0,112 complete native/Java graph/verdict matches,38 contracts,one crate,dirty=false."
      },
      {
        "criterion": "Four concurrent actual views share one clock: aggregate drain<=300us/projection<=600us, no lost tails",
        "reference": "src/frontends/gui/budget.rs:72; tests/gui_budget.rs:239 — four actual Bridge/HarnessView instances,128 deltas/view,22 frames, zero duplicate/lost durable work. scripts/test-gui-budget-control.py — production guard removed: cargo101 at aggregate projection assertion; exact restoration green0 in final-make-test.log. Injected allocation only; CPU margins UNMEASURED."
      },
      {
        "criterion": "False-subscription idle socket/actual term consumer stays live>20s with fenced Status and no semantic activity; old-filter control fails",
        "reference": "src/gateway/mod.rs:299; tests/gateway_unix.rs:182,280; /tmp/harness-heartbeat-control-om5xuq17/{red,green}.log — actual unchanged rich::Connection uses ephemeral=false,21.001s,epoch1/head2,transport age6.015s, unchanged JSONL. Old filter fails real consumer before20s and socket assertion; red101/restored green0. actual-term-consumer-source.json records checkout248d18a/source/probe hashes and no consumer writes."
      },
      {
        "criterion": "Canonical unattended verification, actual exits and current source binding",
        "reference": "/tmp/codex-rich-socket-recovery/final-canonical-result.json — RUSTC_WRAPPER= make test exit0,21:34:32Z–21:50:28Z,no other full gate at launch; final-integrated-input-audit.json — all273 input hashes equal,21 canonical rows/5 GUI rows0,private_HOME=true/operator_input=false; final-make-test.log — packaging and hidden native startup smoke0."
      },
      {
        "criterion": "Signed indexed report, retained evidence, clean handoff and preserved metadata",
        "reference": ".bboxy/reports/2026-11-03-codex-rich-socket-integrated.md — indexed report01M6HPAEC8BJKZ1VER3H6Y1P72/#18870,Report trailer on signed2fb78a0; artifact01M6HPB4DCT2G54E011RY6GFZW accepted. final-report-checks.json — KB/secrets/signature/doc-only0; final-retained-evidence-check.log — current archive verification0; final-clean-state.json and preserved-generated-final.json — clean status,273 hashes equal,five generated files preserved/restored,preliminary report unchanged."
      }
    ]
  }
}
```
