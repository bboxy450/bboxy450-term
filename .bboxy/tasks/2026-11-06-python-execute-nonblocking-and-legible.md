---
type: task
title: 'python_execute: bounded wait, never block MCP, live catalogue, legible errors and transcript rendering'
role: executor
worktree: isolated harness worktree from bboxy450-harness origin/main (Oracle supplies path at delegation)
allowed_paths:
- src/python_execute/
- src/mcp/python_broker.rs
- src/mcp/relay.rs
- src/mcp.rs
- src/frontends/
- tests/
- scripts/test.py
- verification/
- models/
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/
- scripts/formal.py
- scripts/test-formal.py
verify: make test-python-execute && make test-formal
status: accepted
id: 01M6S038KHK6V17WQFSPA4K2Y9
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T18:35:58.739Z
created: 2026-11-06T17:56:54.346Z
number: 93377961366056
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
verify_revision: 1
assignee: seat:01M6S0B7V7W5XHF0D7R2V618AE
verify_result:
  command: make test-python-execute && make test-formal
  exit_code: 0
  evidence:
  - criterion: Short script completes inline in one call
    reference: src/python_execute/tests.rs:948 — short_execution_returns_result_in_one_call_and_utf8_has_no_base64 passed
  - criterion: 60-second worker remains bounded and get wait reaches terminal
    reference: src/python_execute/tests.rs:980 — fake-clock cap and terminal wake witness passed
  - criterion: Live role/grant/catalogue change without restarting Python
    reference: src/python_execute/tests.rs:532,552,628 — live tools, unchanged native PID and delayed A-B-A listing witnesses passed
  - criterion: Nested timeout marks unknown effect and never replays
    reference: src/python_execute/tests.rs:693 — timeout receipt and exactly one recorded dispatch passed
  - criterion: Uncaught ToolError is structured
    reference: src/python_execute/tests.rs:966 — code/message/op_id/server/tool assertions passed
  - criterion: UTF-8 omits base64; invalid bytes recover exactly
    reference: src/python_execute/tests.rs:432,948 — 201000 bytes paged exactly, no UTF-8 base64
  - criterion: Python card model and get parent rendering
    reference: src/frontends/face_core.rs:736 and tests/gui_rich.rs:2323 — focused unit and embedded rendering commands exited 0
  - criterion: Formal lifecycle, wait cap and A-B-A broken controls
    reference: verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d/evidence.json — 281 graphs/verdicts matched Java TLC; BrokenHarnessPythonWaitCap rejected
report:
  summary: 'Landed signed harness fe266d6a0d76cb76525da72c2a7c409835ab1901 through make land: bounded 25-second MCP waits with inline short results and optional progress, live catalogues, unknown-effect nested deadlines, structured ToolError, lean byte output and linked Python command cards. Required verify exited 0 (23 Python witnesses; 281 native models); focused cell and embedded rendering tests passed; canonical landing passed 127 library and 26 MCP/ACP integration tests, strict Clippy/license gates and all 281 native/Java TLC graph/verdict comparisons. Report [[reports/2026-11-06-python-execute-nonblocking]] is committed; source tree clean, task build outputs removed, source-bound formal evidence retained and revalidated. No release, install or term bump.'
  commit: fe266d6a0d76cb76525da72c2a7c409835ab1901
  reported_by: seat:01M6S0B7V7W5XHF0D7R2V618AE
  op_id: 01M6S29YZCD8AZTM670G0QM346
  verify_revision: 1
  report: '[[reports/2026-11-06-python-execute-nonblocking.md]]'
provenance:
  digest: sha256:9626fad5f70d64b7a11c61b3ff3c3a52dc269325496670ce1c54ad18a0d82484
  frontmatter_digest: sha256:401f6900f553d332f2cb98f16006cf74afd7591f0314aea159c0bace25a3f7c7
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: fE597Yc2iDAll78aMCEvqtAgazLTBG06K482PijorqQYu9jn1rCwI8BstQM019Vjz2SFCSlrACS6K3ueH9U2Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149706
  event:
    id: 01M6S2ATHTV4H22FBP7Y249JZV
    sequence: 149706
    hash: 17266d983c5a38808fc82f34ccf2eee0edc8be17d2b639d689a29ae4725ac42a
---
# python_execute: never block MCP, bounded wait, live catalogue, legible output

Repository: **bboxy450/bboxy450-harness** (the `crates/bboxy450-harness` submodule of term). Operator, 2026-11-06: "python might take a while so it might block and timeout MCP, basically improve that mcp", and the transcript rendering "is different formatting from codex … we gotta improve".

## Observed defects (Oracle seat 01M6RZRGC3194VVFBC4M7XW4MF, live, 2026-11-06)

1. **Every call needs a second round trip.** `python_execute` always returns `status: running` with no result, even for a 50 ms script. The caller must poll `python_execution_get`, which has no wait, so it either spins or guesses. There are 6 calls in one exchange where 3 would do.
2. **Nested catalogue frozen at launch.** After a live role change (executor → oracle, grant epoch 1 → 2), `orient` lists 64 tools, but `mcp.tools('bboxy')` still returns the 45 launch-time tools, and `mcp.call('bboxy','workspace_list')` fails with `tool_unavailable`. Workaround in use: a fresh `bboxy450-term mcp` stdio subprocess from Python (`/tmp/oracle-bridge/bb.py`) sees all 64. The broker never re-lists (`src/mcp/python_broker.rs:225-240`).
3. **The error code is buried in stderr.** The `ToolError {code:"tool_unavailable"…}` comes back only as a split traceback in stderr chunks. The structured `error` field says only `python_failed`.
4. **Output is doubled.** Every stderr/stdout chunk is returned as both `text` and `base64` (`src/python_execute/mod.rs:77,563`), which doubles tokens for ordinary UTF-8.
5. **Nested calls have no per-call timeout.** A slow or hung nested tool holds the whole execution until the outer `timeout_ms`, which can be up to 600 s.
6. **Transcript rendering is unreadable.** The harness UI shows `python execute  bboxy` / `python execution get  bboxy` with no script summary, status, exit code or emitted result. Codex/Claude command cells show the command and its output. A `python_execution_get` cell does not link to the execution it continues.

## Required outcome

- **Bounded wait, never blocking MCP.** `python_execute` and `python_execution_get` accept an optional `wait_ms`, clamped to a ceiling safely below common MCP client request timeouts (default ceiling 25 s; document the constant). The call returns as soon as the execution reaches a terminal state, with `emitted`/output/exit inline, or at the cap with `running` plus the cursor. The default `wait_ms` should make short scripts finish in one call. No request may ever hold the MCP channel longer than the cap. Long runs stay durable handles.
- **Progress.** While waiting, emit MCP `notifications/progress` when the client sent a progressToken; keep it optional and best-effort.
- **Live catalogue.** The broker re-lists a server's tools on `notifications/tools/list_changed`. On a `tool_unavailable` miss it refreshes once and then calls the tool, if it now exists, because the call was never sent and nothing is replayed. `mcp.tools()` reflects the refreshed list. `server_generation_changed` semantics stay intact.
- **Nested-call timeout.** `mcp.call(..., timeout_ms=)` has a sane default and returns a structured `timeout` outcome that marks the effect unknown (inspect with `operation_status`), never a blind retry.
- **Structured failure.** An uncaught `ToolError` surfaces its code/message/op_id/tool in the execution's `error`/`failure`, not only in stderr.
- **Lean output.** Send `base64` only for chunks that are not valid UTF-8, keeping exact-bytes recovery through the existing cursor path.
- **Rendering.** The python_execute transcript cell shows the first line(s) of the source, the status and exit code, and a compact preview of `emitted`, expandable to full. A `python_execution_get` cell shows the parent execution id and folds into or updates that cell. Match the visual grammar of command cells.

## Acceptance

- [ ] A short script (`emit(1)`) completes in ONE `python_execute` call with the result inline (test).
- [ ] A 60 s script returns at the cap with `running`; `python_execution_get wait_ms` returns it when it is done. No request exceeds the cap (test with a fake slow worker).
- [ ] A role/grant change followed by `list_changed` makes the new tool callable from Python without a new process. The fixture server adds a tool mid-session (test).
- [ ] A nested call timeout yields a structured unknown-effect outcome. Nothing is replayed (test).
- [ ] An uncaught ToolError code appears in the structured `error` (test).
- [ ] UTF-8 chunks carry no base64; non-UTF-8 bytes round-trip exactly (test).
- [ ] Rendering: snapshot/unit test of the cell model showing source, status and emitted preview; get-cells link to the parent.
- [ ] Formal: the execution lifecycle model (running → terminal, wait cap, cancel, duplicate/late completion, catalogue refresh A → B → A) is updated with a broken control that lets a wait exceed the cap and must fail.

## Verify / landing

`make test-python-execute && make test-formal` in the harness worktree. Landing in the harness through its canonical `make land` is authorized once verify is green. Then report the landed harness SHA. The term `make bump-harness` happens separately. Write the append-only report `reports/2026-11-06-python-execute-nonblocking.md` and call `task_report` once. No release or install.


**Lean landing (operator 2026-11-06: CPU and disk are scarce):** run only the targeted tests your diff affects plus the verify command; no full `make test`; land fast with `make land`. Clean your own large build outputs when done.


**Amend (Oracle, 2026-11-06):** allowed_paths now include `src/frontends/` (transcript cells live in `src/frontends/gui/` and `src/frontends/face_core.rs`); the non-existent `src/ui/` and `src/transcript/` were removed. Scope is otherwise unchanged.

**Amend 2 (Oracle):** add `scripts/formal.py` and `scripts/test-formal.py`. `implementation_reference` (formal.py:102-113) resolves the term witness only in sibling or embedded layouts, so managed checkouts fail with `missing contract implementation_tests: HarnessDraftHandoff`. Resolve the real term checkout through Git worktree/module metadata (or an explicit `BBOXY_TOOLKIT_ROOT`) and test that resolution, including the managed-checkout layout; a genuinely missing witness must still fail.


## Report from seat:01M6S0B7V7W5XHF0D7R2V618AE

Landed signed harness fe266d6a0d76cb76525da72c2a7c409835ab1901 through make land: bounded 25-second MCP waits with inline short results and optional progress, live catalogues, unknown-effect nested deadlines, structured ToolError, lean byte output and linked Python command cards. Required verify exited 0 (23 Python witnesses; 281 native models); focused cell and embedded rendering tests passed; canonical landing passed 127 library and 26 MCP/ACP integration tests, strict Clippy/license gates and all 281 native/Java TLC graph/verdict comparisons. Report [[reports/2026-11-06-python-execute-nonblocking]] is committed; source tree clean, task build outputs removed, source-bound formal evidence retained and revalidated. No release, install or term bump.

```json
{
  "op_id": "01M6S29YZCD8AZTM670G0QM346",
  "commit": "fe266d6a0d76cb76525da72c2a7c409835ab1901",
  "report": "[[reports/2026-11-06-python-execute-nonblocking.md]]",
  "verify_result": {
    "command": "make test-python-execute && make test-formal",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Short script completes inline in one call",
        "reference": "src/python_execute/tests.rs:948 — short_execution_returns_result_in_one_call_and_utf8_has_no_base64 passed"
      },
      {
        "criterion": "60-second worker remains bounded and get wait reaches terminal",
        "reference": "src/python_execute/tests.rs:980 — fake-clock cap and terminal wake witness passed"
      },
      {
        "criterion": "Live role/grant/catalogue change without restarting Python",
        "reference": "src/python_execute/tests.rs:532,552,628 — live tools, unchanged native PID and delayed A-B-A listing witnesses passed"
      },
      {
        "criterion": "Nested timeout marks unknown effect and never replays",
        "reference": "src/python_execute/tests.rs:693 — timeout receipt and exactly one recorded dispatch passed"
      },
      {
        "criterion": "Uncaught ToolError is structured",
        "reference": "src/python_execute/tests.rs:966 — code/message/op_id/server/tool assertions passed"
      },
      {
        "criterion": "UTF-8 omits base64; invalid bytes recover exactly",
        "reference": "src/python_execute/tests.rs:432,948 — 201000 bytes paged exactly, no UTF-8 base64"
      },
      {
        "criterion": "Python card model and get parent rendering",
        "reference": "src/frontends/face_core.rs:736 and tests/gui_rich.rs:2323 — focused unit and embedded rendering commands exited 0"
      },
      {
        "criterion": "Formal lifecycle, wait cap and A-B-A broken controls",
        "reference": "verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d/evidence.json — 281 graphs/verdicts matched Java TLC; BrokenHarnessPythonWaitCap rejected"
      }
    ]
  }
}
```
