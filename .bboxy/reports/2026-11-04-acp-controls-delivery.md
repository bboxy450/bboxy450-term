---
type: report
title: Embedded profiles and negotiated ACP controls delivery
task: '[[01M6KF628K0H7DHN4R8BCN9WVZ]]'
commits:
- 9501b79845cd7486738e6bf948a09813e70d87cf
verify:
  command: make verify-acp-controls
  exit_code: 0
author: seat:01M6KF1JD34QA47CQP34FRJN5Z
created: 2026-11-04T14:55:07.320Z
id: 01M6KGWYYSF1W37CJ3T7ETB898
updated: 2026-11-04T14:55:07.320Z
number: 93377961359837
created_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
updated_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
authorship:
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  kind: seat
  public_key: El6VqSdMLzX3+r6uiKxbH22bBB8VwVCtourrWmlf8Yk=
  process_generation: '1'
provenance:
  digest: sha256:84217da171f07b5500bc813789742543721e751867f320f107082534ad976517
  frontmatter_digest: sha256:cdbeda89db74e91dc03a600876d11e96136fbb343e42fd66b0e361df68db4ac6
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  sig: TYtZzVDL2wkLXTeCycQYrGKNrWM3+A1ZGQyFtG6MUe4lUg39LKyP3DjIkHpPeVEmmOSt17BaUPo0Qz0AMkN1BQ==
  log: .bboxy/log/2026-11-04.jsonl#116712
---
Implemented in `/Users/feral/mydev/bboxy450-harness-land`, branch `codex-acp-controls-20261104`. Signed implementation commit: `9501b79845cd7486738e6bf948a09813e70d87cf` (verified G, Boxy/bboxy450). Its parents are published main `f2a0fd1304dec97b427f562f207f1b7f705635f6` and shipped producer `d48b941e24fba5aab48dbecfc4bc91db3312405a`. This is the authorized local reconciliation, with both histories and original records retained.

Acceptance evidence:

- Catalogue discovery: `src/frontends/gui/bridge.rs:1205` selects the machine HOME catalogue; `:1223` returns safe ProfileInfo plus explicit unavailable/incomplete diagnostics. Both Unix connect entry points and standalone startup use this loader. `src/frontends/gui/window.rs:688` shares the same path selection and rejects absent/empty HOME. `tests/acp_controls.rs:372` invokes the actual default connect entry point in private child processes with present, absent, empty, missing and unreadable HOME catalogues; it checks default marker and env-count projection without displaying values. The standalone path unit test and existing private profile/context/MCP process witnesses pass.
- Advertised model/effort controls: `src/protocol/agent_config.rs:24` retains only model/thought select fields; grouped values and legacy session/new.models are supported. `src/frontends/gui/view.rs:1349` renders current values, advertised choices and explicit unavailable labels in the existing scrollable profile popover, shared by standalone and embedded. `:1850` sends the captured gateway envelope. No vendor model IDs, shell model commands or global profile edits were introduced.
- Real ownership and acknowledgements: `src/protocol/mod.rs:90` adds SetAgentConfig with epoch, child incarnation, complete-option revision, config ID and advertised value. `src/backend/acp_agent.rs:607` validates these before sending session/set_config_option or advertised legacy session/set_model. `:1199` correlates acknowledgements, retains confirmed values on refusal and prevents an older reply overwriting a newer notification. `:1701` consumes complete config_option_update lists without creating a turn. A five-second local deadline labels an unanswered outcome unknown and never replays it; the outstanding request stays fenced until it resolves or its child ends. Bootstrap model intent has the same revision fence while preserving older v1 advertisement compatibility. `src/frontends/gui/model.rs:254` reconstructs confirmed control state from the existing durable ContextChanged namespace on reconnect.
- Process/native witnesses: `tests/acp_controls.rs:213` covers actual dynamic model calls, refreshed effort choices, refusal, A→B→A stale selections and two Unix view reconnects without new requests. `:298` tests a newer update before an older response plus a wrong-session notification; `:345` tests no replay after unknown outcome; `:545` tests bootstrap reordering; `:561` tests session/epoch/child rejection before wire; `:682` restarts a real private ACP child and rejects the previous incarnation. Every completed fixture log runs check_authored and counts the explicitly requested turns/prompts, with no extra prompt or client fs/terminal effect.
- Shipped GUI behavior retained: d48's face_core, face, face_rows, rich, smoke, MCP, gui_render, gui_rich and acp_launch_context files are byte-identical in this tree. The view's fixed transcript extents and retained prepared rows are preserved. The stronger `tests/gui_render.rs:2079` raster witness passes and checks that prior-history pixel regions remain unchanged while the gray queued tail is present. MCP completion, prepared history and centered portrait-loader witnesses also pass.

Protocol inspection: pinned `agent-client-protocol =3.4.0` was read locally. Installed `@agentclientprotocol/codex-acp` is 2.2.2; its dist source implements session config options and the legacy model method (dist/index.js:43655 and :43750). SHA256: package.json `517931a5786327ec60e877a992b7872f2d6a1c6d588e60834ff1a794d1b9f6c5`; dist/index.js `6b1bb4e7e9caaf4aeca1042ae4aec2dc2433b0b0e4b1614b38a4d41699902aaa`. The complete-list replacement and dependent effort behavior follow the current [ACP v1 config specification](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/session-config-options.mdx). These reads did not launch a live vendor probe.

Verification: `make verify-acp-controls` exited 0. Its 15 logged commands all exited 0: three compile configurations, 11 new top-level process/native tests (including private HOME children and raster), 20 existing ACP tests, seven launch-context tests, ten bridge tests, three pending-core tests, the retained history raster, two preserved GUI tests, the standalone path unit test, embedded/all-feature lint, five schema tests, and the two selected native formal test functions. Full commands, individual output-log SHA256 values, 88 exact source input hashes, tool pins and three capture hashes are committed in `.bboxy/runs/codex-acp-controls.json` (SHA256 `f46be4ea94b79fbe57bf8a1a8917b0482ef2996a59318d008e49097c5c1310b2`). All 88 input hashes were compared against git-show bytes of the signed implementation commit after committing; all matched. `git diff --check`, `git diff --cached --check` and `python3 -I scripts/checks.py headers` exited 0.

Selected native/reference formal evidence: `tests/formal/models.rs:3952`, contract `verification/contracts.toml:431`. Full reachable states/transitions and verdicts match pinned Java TLC for HarnessAcpControls (153/402), stale control (189/510), replay control (252/645), retained pending model (995/2532), pending-ack control (1245/3143) and pending-stale control (1283/3812). Good models exit TLC 0; all four deliberate safety controls exit TLC 12 as expected. Bounds are child births 0..2 (A/B/A), option revisions 0..2 and one admitted selection; only the local deadline is weakly fair. This is bounded contract evidence, not arbitrary Rust refinement or a claim of complete TLC replacement.

Initial checks found and corrected two compatibility regressions: dropping a legacy reported usage model, and refusing the existing model bootstrap from incomplete older advertisements. Their failed gate logs remain under target/acp-controls/attempt-{1,2}-*.log. An initial new raster assertion incorrectly expected a whole label in one text primitive; it now checks normalized painted text runs and distinct real pixel buffers (attempt-3 logs retained). The sandbox's private-socket restriction was resolved by the task-scoped unattended test execution; no gate was skipped. Earlier failures are not counted as passing evidence.

Scope deliberately omitted: whole test/formal/parity/runtime corpora, paid/live vendor probes, live desktop input, installation, activation, push, publication and shared/main merges. Original remote context proof was not rerun as a live vendor experiment; unchanged source inputs and the private scripted context/MCP tests were used. Oracle owns the parent pin/build/activation and independent acceptance. Worktree was clean after the signed implementation commit; the only subsequent change is this indexed report.
