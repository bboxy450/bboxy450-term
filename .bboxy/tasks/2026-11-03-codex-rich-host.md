---
type: task
title: 'Host the shared rich harness view in term: socket delivery, resume and native pane'
status: accepted
role: executor
assignee: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
worktree: /Users/feral/mydev/bboxy450-term-codex-rich
allowed_paths:
- crates/bbt-mux/**
- crates/bboxy450-harness-wake-up/**
- crates/bbt-proto/**
- crates/bbt-ui-model/**
- crates/bboxy450-term/**
- crates/bbt-ui/**
- tests/**
- verification/**
- scripts/**
- Makefile
- Cargo.toml
- Cargo.lock
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-rich-host.json
- crates/bboxy450-harness
- .bboxy/tasks/**
- .bboxy/decisions/d16393-hash-pinned-report-corrections.md
- crates/bbt-doc/tests/corpus/**
verify: make verify-rich-delivery
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYDWBAX2FDYGMSPB12HRN
updated: 2026-11-04T02:46:45.815Z
created: 2026-11-03T19:07:32.055Z
number: 15252
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
artifacts:
- '[[01M6HMRXN8JZ9V0YM7RKG4DQ1P]]'
- '[[01M6HPCV8J7YXY3XB3JR049KHH]]'
verify_revision: 1
verify_result:
  command: make verify-rich-delivery
  exit_code: 0
  evidence:
  - criterion: Fresh canonical targeted verification on clean signed application source; final report metadata checked on clean signed report HEAD
    reference: a531338d930f6bed73747047e06c56883b5ce5ea; /tmp/codex-rich-verify-delivery-final.log exit0 SHA25640859d172cf878d046e15334589f81a030adfd6331942540ceda050ae18ec9ef; 4b8cd49c report-only production diff exit0; /tmp/codex-rich-final-report-corpus.log two selected cases exit0 SHA256b901db82c1c1f6aea5732b26477f3809ec0b42779b1ca23afad960f1b55b94bf
  - criterion: Published dependency and exact upstream report correction/canonical task closure
    reference: make bump-harness5c288d0bcbdaebc7249740220158829e87c85e5a exit0 /tmp/codex-rich-final-cfg-pin.log; exact d32803e2 startup-modal/CORRECTIONS/decision16393 bytes and authenticated task IDs/etags/hashes in .bboxy/runs/codex-rich-host.json
  - criterion: Shared native face, real App owner-preserving detach/replacement/rollback and launch parity
    reference: /tmp/codex-rich-verify-delivery-final.log; rich_gui_handoff_preserves_turn_and_rolls_back_failed_activation0; rich_app_detach_during_native_turn_retains_owner_and_replays0; launch_children_receive_workflow_and_call_their_own_admitted_mcp0; report19886 actual PIDs/session/cursors
  - criterion: Prior controls/formal/budget limits preserved with explicit operator-authorized omissions
    reference: reports/2026-11-04-codex-rich-host-delivery.md supersedes18874; archived prior-source receipt b19641f1c5dbed75b14eb7f9eda21c7d31d9dd28ccbd04ac35c962170dee31d4; current source/model/tool hashes in run JSON; no new whole-frame/load/CPU/heap or fresh full-suite/formal claim
report:
  summary: 'Scoped rich-host delivery is ready for Oracle verification/installation: clean signed report HEAD4b8cd49c, application sourcea531338d rebased onto published term f01428b8, published harness5c288d0bcbdaebc7249740220158829e87c85e5a (oracle/embedded-cfg-20261104). Exact make verify-rich-delivery exited0 on clean signeda531: strict affected embedded consumer lint, both real corpus cases, real GUI replacement/rollback, unexpected App detach/replay and production launch/context/MCP witnesses. Report-only4b leaves all production/Make/model inputs unchanged; both corpus cases were checked again against that committed report HEAD and pass0. Fresh App proofs retain supervisor/harness/session identity, contiguous1..19, actual new GUI source/instance and rollback; launch fixtures exercise real own-seat MCP at generations1,2 with production --yolo, no shim/vendor request. Indexed append-only report19886 supersedes18874 and preserves all original refs/reports/logs, exact d328 correction/task closure, prior316-model receipt and measured budget/visual observations labeled prior-source. Full test/parity/runtime/visual/formal corpora were omitted under authenticated speed override, with no fabricated fresh receipt. No trusted install/push/merge/publication performed; Oracle''s tested make install-reopen handoff and actual running installation outcome remain.'
  commit: 4b8cd49cdb64ccb440213db8617a334c8e2fdd87
  reported_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  op_id: 01M6J72N43PWQ8PJSAMKV69P37
  verify_revision: 1
  report: '[[reports/2026-11-04-codex-rich-host-delivery.md]]'
provenance:
  digest: sha256:63801af93bab65342d209a63ab3377a0a79a8a0480f6b7e8b40ec1fb2b94cda0
  frontmatter_digest: sha256:63a7be6165047af9051182a3407b9ff5e2c487f7d56952176eb6a2b0698c64df
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: 70/MyqAj9EU6V9M3w+XKpMWwyN7JemfYyTSEcIFsV9BtUrIoU8CHcPQoL2wOHfh4hKlJ1CwBFaB+Nm0hbvWTCA==
  log: .bboxy/log/2026-11-04.jsonl#112716
---
ROLE: EXECUTOR. Final focused delivery in /Users/feral/mydev/bboxy450-term-codex-rich, branch codex-rich-pane-20261103. Direct operator: "hurry up ... only run tests it needs, not entire test suites, if it compiles its likely correct!" This replaces earlier broad test wording. No full make test/test-parity, all-runtime tests, full visual/model/TLC corpus or repeated completed checks. Current signed2d1642d320397c16a9bcbf1cb7c8629fb093ca1c and old component reports/logs are preserved; the detailed prior brief is retained in Oracle records/history. No new features/refactors.

CFG IS NOW VERIFIED, ACCEPTED AND PUBLISHED: use make bump-harness REV=5c288d0bcbdaebc7249740220158829e87c85e5a. Approved repository https://github.com/bboxy450/bboxy450-harness, published ref oracle/embedded-cfg-20261104. This is a published SHA, not a private override; canonical bump-harness.py fetches explicit SHA and supports it. Main was concurrently receiving unrelated record-only updates, so Oracle published the verified dependency on a dedicated branch rather than chasing that moving ref. Do not wait for main or another report. Runtime implementation035e5b29 is the acp-agent guard plus strict three-configuration Make regression; your signed reported626fbcc and prior published log/research-only commits are retained through signed merge5c288d. Oracle checked compile input hashes, red make2/cargo101→green0, all signatures and report19676; cfg task19662 is accepted. Return to THIS term task now; stop writing harness-land. One writing worktree at a time.

Preserve your term tip/ref, then integrate/rebase published current term main (last inspected184b770f9f43094c518570f143e5eeaeb7e999ef). Its signed d32803e2/decision16393 exact CORRECTIONS.toml transaction already repairs duplicate5002227 and startup-modal metadata. Apply that published repair verbatim, retaining report bodies/provenance/ledger; no independent report rewrite, deletions, test weakening or waiver of compile errors. Rebase may import published upstream changes; author conflict fixes only within assigned implementation scope. Expanded task/decision/bbt-doc corpus paths authorize only that existing repair and exact canonical task closure. Copy necessary current own/parent/linked records from authenticated knowledge by ID, retaining IDs/numbers/provenance and a compact hash/locator manifest. Do not import the malformed duplicate-locator coordination records, transcripts or credentials.

Keep the already implemented shared GPU HarnessView/IR face, default rich pane after valid announcement, profile/exec/ACP launch context+workflow+MCP, real production --yolo intent without a test shim masking it, generation/session/Cancel/delivery/status/input/IME fencing, actual App detach/reattach and GUI replacement/rollback. Preserve all old source/model evidence. No real vendor/provider requests, global profile edits, role/grant widening, live capability reads, OS input, broad kills, push/merge/publication/trusted install by Executor.

Finish the minimal Make verify-rich-delivery target:
- check-rich-host for actual affected compile/lint configuration;
- only the two failing real corpus frontmatter/link cases;
- existing focused real GUI handoff/rollback witness if upstream integration changed its production inputs, plus App detach or launch/MCP witness only where production inputs changed.
Reuse exact-input prior focused logs/316-model receipt where valid; clearly label prior-source/stale broader proof. No whole-suite rerun. Compile/record failures remain real failures. Budget observations remain bounded: whole-frame/load/background CPU/heap are UNPROVEN/UNMEASURED,32MiB/view an allocation; no new performance investigation.

Clean signed committed source, make verify-rich-delivery exit0, compact scoped run JSON and one final append-only indexed report/task_report with this exact verify command. Name actual focused checks, reused evidence, explicit operator-authorized omissions, published dependency ref and source/remaining limits. Fresh report required because verify changed. Preserve historical reports as evidence; same-task supersedes is allowed. Oracle will independently check the focused logs/source, then build/install/activate through Make's tested GUI-only handoff, keeping current supervised agents alive. Do not forge a full-suite receipt to satisfy a publication script; local installed GPU delivery takes priority, publication may remain pending its normal broader gate. No operator restart, click, test input or permission prompt is required.


## Report from seat:01M6HCYVGE7CJ8KK6PB20SPF3D

Scoped rich-host delivery is ready for Oracle verification/installation: clean signed report HEAD4b8cd49c, application sourcea531338d rebased onto published term f01428b8, published harness5c288d0bcbdaebc7249740220158829e87c85e5a (oracle/embedded-cfg-20261104). Exact make verify-rich-delivery exited0 on clean signeda531: strict affected embedded consumer lint, both real corpus cases, real GUI replacement/rollback, unexpected App detach/replay and production launch/context/MCP witnesses. Report-only4b leaves all production/Make/model inputs unchanged; both corpus cases were checked again against that committed report HEAD and pass0. Fresh App proofs retain supervisor/harness/session identity, contiguous1..19, actual new GUI source/instance and rollback; launch fixtures exercise real own-seat MCP at generations1,2 with production --yolo, no shim/vendor request. Indexed append-only report19886 supersedes18874 and preserves all original refs/reports/logs, exact d328 correction/task closure, prior316-model receipt and measured budget/visual observations labeled prior-source. Full test/parity/runtime/visual/formal corpora were omitted under authenticated speed override, with no fabricated fresh receipt. No trusted install/push/merge/publication performed; Oracle's tested make install-reopen handoff and actual running installation outcome remain.

```json
{
  "op_id": "01M6J72N43PWQ8PJSAMKV69P37",
  "commit": "4b8cd49cdb64ccb440213db8617a334c8e2fdd87",
  "report": "[[reports/2026-11-04-codex-rich-host-delivery.md]]",
  "verify_result": {
    "command": "make verify-rich-delivery",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Fresh canonical targeted verification on clean signed application source; final report metadata checked on clean signed report HEAD",
        "reference": "a531338d930f6bed73747047e06c56883b5ce5ea; /tmp/codex-rich-verify-delivery-final.log exit0 SHA25640859d172cf878d046e15334589f81a030adfd6331942540ceda050ae18ec9ef; 4b8cd49c report-only production diff exit0; /tmp/codex-rich-final-report-corpus.log two selected cases exit0 SHA256b901db82c1c1f6aea5732b26477f3809ec0b42779b1ca23afad960f1b55b94bf"
      },
      {
        "criterion": "Published dependency and exact upstream report correction/canonical task closure",
        "reference": "make bump-harness5c288d0bcbdaebc7249740220158829e87c85e5a exit0 /tmp/codex-rich-final-cfg-pin.log; exact d32803e2 startup-modal/CORRECTIONS/decision16393 bytes and authenticated task IDs/etags/hashes in .bboxy/runs/codex-rich-host.json"
      },
      {
        "criterion": "Shared native face, real App owner-preserving detach/replacement/rollback and launch parity",
        "reference": "/tmp/codex-rich-verify-delivery-final.log; rich_gui_handoff_preserves_turn_and_rolls_back_failed_activation0; rich_app_detach_during_native_turn_retains_owner_and_replays0; launch_children_receive_workflow_and_call_their_own_admitted_mcp0; report19886 actual PIDs/session/cursors"
      },
      {
        "criterion": "Prior controls/formal/budget limits preserved with explicit operator-authorized omissions",
        "reference": "reports/2026-11-04-codex-rich-host-delivery.md supersedes18874; archived prior-source receipt b19641f1c5dbed75b14eb7f9eda21c7d31d9dd28ccbd04ac35c962170dee31d4; current source/model/tool hashes in run JSON; no new whole-frame/load/CPU/heap or fresh full-suite/formal claim"
      }
    ]
  }
}
```
