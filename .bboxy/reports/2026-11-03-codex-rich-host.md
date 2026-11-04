---
type: report
title: 'Rich host candidate: socket delivery and shared native pane; published pin required'
task: '[[tasks/2026-11-03-codex-rich-host]]'
commits:
- 5db409e5afcdcb1c896f9cc4484ca0b0bfa32c94
- 26084f2bd62e3aa23182e9f2c70c085d3d9e90a5
- dd7727373b74a6898737e939f87dc0676d4674f6
- e82b1784355f2158828c5387ce23409695843e7e
author: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
created: 2026-11-03T20:45:31.303Z
verify:
  command: BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity
  exit_code: 2
id: 01M6HJHVDSVP2WXS2XX3WCA8MR
updated: 2026-11-03T20:45:31.303Z
number: 17675
created_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
updated_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
authorship:
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  kind: seat
  public_key: YKv54MXoFa4rl83lF7g/hJHRLfs8fsSGu2+nb6F1XqY=
  process_generation: '1'
provenance:
  digest: sha256:a64156904b160b6c9d258e179aa8445a28bd52a97301c7143511e7d0bb029acc
  frontmatter_digest: sha256:f6e642e8398030c6e49b30dfc2e42a78559c2bacbae0828558fb82a622588619
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  sig: yXMYKQsTismoO6gepa4fbX0u9UXdqv9rea56h6rM6lB9lXQiZ1FxI1nldCfzhEYxDXmYVxvCfMhJ9vIqsxbTDw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100188
---
# Rich host candidate — blocked on the published combined harness pin

Task [[tasks/2026-11-03-codex-rich-host]] (#15252). Executor seat 01M6HCYVGE7CJ8KK6PB20SPF3D; worktree `/Users/feral/mydev/bboxy450-term-codex-rich`, branch `codex-rich-pane-20261103`, base `1affb9865d75d775a9a5f306c043da8a05bf8f7c`.

This is an implemented and privately exercised candidate, **not an acceptance claim**. The committed harness gitlink remains `0c403165e815c2f7bf77ccac293784b021f42f96`. Tests used the signed private B1/B2 candidate `d5e7249a01bf4830c79979aa7a8e2f17753be83b`, checked out only in this worktree's submodule. The gitlink is deliberately unstaged. That older committed pin has no `Bridge::connect`; this term cut therefore requires a compatible published pin before it is buildable from its committed tree. The Oracle's integration message 01M6HESP3JDV3NND8SFKNK78ZD orders standalone recovery, B1/B2, then the term pin. No unpublished pin was committed.

## Changed mechanisms and clause evidence

| Requirement | Implementation and observed evidence |
| --- | --- |
| A3: bounded socket argv and session discovery | `crates/bboxy450-harness-wake-up/src/launch.rs:516` passes term's chosen `--socket`, validates resume and passes `--resume`. `src/rich.rs:175` refuses nonabsolute or >96-byte paths without truncation. `crates/bbt-mux/src/logic.rs:6620` retains the path across owner restarts; `:6315` creates the generation-owned async client, and `:10830` handles Hello without needing another PTY byte. The session is persisted through the existing native binding path. Private Unix tests assert path bounds; real native MCP tests observe the exact argv, Hello and persisted session. |
| A4: native admission before PTY arbitration | `crates/bbt-mux/src/logic.rs:5288` dispatches harness control delivery asynchronously using the call's stable op_id; `:9014` dispatches message delivery by kind before Operator/PTy routing. `:9136` is the synchronous no-PTY defense. `crates/bbt-mux/src/rich.rs:45` stores original session and exact op content before sending, with per-seat admission serialization on a disk worker. A changed-content reuse refuses. Retrying never retargets to a new Hello. |
| Correct outcomes and zero PTY bytes | `crates/bboxy450-harness-wake-up/src/rich.rs:133` exposes recent admission only; `:264` owns connection, framing, 2-second reconnect limit, correlated Ack, Busy, Closed/UnknownSession and the five-second unknown deadline. Drop retires retained clones (`:85`). Parsing and disposal of large IR payloads occur on blocking workers (`:199`). `crates/bbt-mux/src/mcp/seats.rs:595` marks Busy retryable. Four real Unix fixture tests cover duplicate, Busy, Closed, unknown, fragmentation, absent socket, future major, oversize, rebind A/B/A and retired generation. The two real native MCP fixtures at `tests/mcp.rs:5069` and `:5089` assert correlated duplicate Ack, exactly one causal queued turn with started/ended turn-id linkage, socket removal refusal and **all PTY writer receipt bytes**, not a proxy counter. |
| Resume and interrupted process | `crates/bbt-mux/src/mcp/seats.rs:413` admits only canonical nonzero harness session ULIDs. The native owner-close fixture starts a HUP-ignoring tool, closes the owning process group, requires every exact recorded group to become ESRCH, resumes with MCP `seat_spawn`, checks retained completed items and runs the real harness transcript checker (exit 0). Normal seat close and owner close are both exercised. |
| A1/A2: one shared view and renderer | `crates/bboxy450-term/Cargo.toml` links harness `embedded` without default features; root patches resolve all ten toolkit packages once to in-tree paths (native target's executable inventory). `src/rich_pane.rs:89` runs Unix Bridge, HarnessView, Markdown preparation and projection on workers. `:270` converts its existing Frame to existing term Paints; assets are namespaced in `src/assets.rs` instead of replacing term's icons. `src/main.rs:8175` fills rich panes and `:8222` excludes their PTY grid. No second harness renderer or PTY text parser was introduced. |
| Rich input routing | `src/rich_faces.rs:199` consumes rich keyboard press/repeat/release and `:274` handles IME. Paste, wheel and pointer paths route to the actor. The actor fences submitted input against its displayed session. Composer Send/Stop uses the shared composer's Regions hit logic; Stop sends Interrupt, approvals ResolveApproval, and `/profile` SetProfile. `/stop` is normalized to interrupt rather than terminating the owned session. `tests/rich_client.rs:59` drives the production actor against a separate scripted Unix process and asserts those four operations, distinct op ids and no Close. The normalized App key/IME witness asserts zero pending PTY commands even when the socket is unavailable. |
| Preference, replay and detach | Protocol WindowState v5 (`crates/bbt-proto/src/gui/app.rs:124`) saves per-pane faces and session/generation/epoch/sequence replay hints; journal readers accept v1–v5. `src/rich_faces.rs:12` defaults announced harness sessions to rich, renders a Terminal toggle, and creates only visible subscribers. `:116` drains Latest snapshots round-robin with a shared 0.3 ms cutoff and reschedules deferred work. Missing sockets retain an honest starting/unavailable face. The actor test drops and recreates the production view, retains the cursor, checks contiguous durable sequences and zero duplicate count, and proves its scripted session PID survives. `make test-boundaries` separately exercises real backend detach/reattach, retained owners and orderly/hard shutdown. **A real rich-harness turn across `ctl app detach` and reattach on the combined published pin is still outstanding; these are separate production-actor and generic-backend witnesses.** |
| Native visual equality | `src/rich_faces.rs` offscreen verification renders the same `v1.1-all-items.jsonl` Frame through the harness's native capture and term's actual offscreen GPU, 1120×820, animation time zero. Non-vacuous row profiles are asserted. Row structure mismatch **0.0**, required ≤0.01. Captures are local under `target/rich-pane-parity/{standalone,pane}.png`; this is the pane body, excluding term's outer chrome. |
| Layering and credit | `scripts/layering.sh:8` reuses the prepared signed dfb21967c0157aebcadec5b028d300abd86203ce ORDER fix (desktop trio before harness); the layering check exits 0, with no exemption. Root ATTRIBUTION already contains the canonical policy and remains byte-identical; generated source carries its complete origin/policy headers, and adapted socket framing, pointer routing and parity metrics carry adjacent harness credit. |

`GuiSeat.rich_socket`, WindowState fixture fields and the UI PaneBody face vocabulary required matching literal updates in the affected proto, mux and term fixtures. Navigation, clipboard and wheel changes prevent rich focus from entering the old terminal input paths. `scripts/harness-submodule.py` provides an explicit private-test candidate override: exact SHA, clean child and signature required, with both actual candidate and committed pin printed. Its default remains the committed pin. The clean-source/full-CI and formal evidence checks were not weakened.

## Formal contract and controls

`crates/bboxy450-term/tests/formal_contracts/rich_host.rs:14` names twelve bounded state variables: incarnation 1–3 (A/B/A are distinct IDs), originally bound session 0–3, connected, pending, detached, rich face, PTY-byte and stale-effect bits, two sends/effects and two process generations. Worker owns connection/replies, logic owns generation/admission and UI owns the disposable face. Admit, Send, AckOrDeadline, Switch, Reconnect, Restart, Detach, Attach and Key are atomic abstractions. Safety is one effect, no stale-session/generation effect, zero rich PTY bytes and detach preserving durable effects. Weak fairness of AckOrDeadline proves pending delivery retires; reconnection is possible but is not assumed fair.

Four strict good/broken contract pairs are inventoried: RichHost (fresh-id duplicate control), RichHostIdentity (retarget control), RichHostDetach (erase-effect control), RichHostRouting (PTY key control). Each broken control must fail safety. The mandatory independent corpus and per-crate contract checks are preserved. Native and pinned Java TLC compare full reachable state and transition sets and verdicts. Full gate exit 0: **284 models, 34 crates, 116 contracts**; evidence SHA256 `cca2509fa9222bc0d2648a95b43453ba43677c6c1b6768f4a0ec81b956fd262d`, source `e82b1784355f2158828c5387ce23409695843e7e`. Its dirty flag is **true** because the private harness checkout differs from the committed gitlink. The four good models each explore 336 states/1346 edges; duplicate, identity, detach and routing controls explore respectively 388/1528, 384/1518, 400/1638 and 672/2692 states/edges. TLC exits 0 for the good cases and 12 for the four safety controls; every graph matches.

The executable controls in `scripts/rich-host-controls.py` also deliberately produce two effects with a fresh resend id and introduce a PTY fallback into the real native fixture. Each must exit **101 at its specific causal assertion**; the outer control target exits 0 only when both are detected. No source mutation or credentials are involved. Initial formal runs found the strict pair inventory requirement and missing candidate socket/text-zoom coverage; these were fixed by valid paired records and all ten candidate models, without relaxing the gates.

Socket framing, real wall-clock bounds, SQLite durability, kernel ownership, arbitrary Rust interleavings, GPU pixels and runtime resource costs remain implementation witnesses, not claims of model refinement or complete TLC replacement.

## Commands, results and retained evidence

All private candidate Make invocations use `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b`. Build/test workflows were sequential except that the parity signature preflight overlapped the end of the eight window-journal tests; full CI refused before any Cargo build or test. No two Cargo builds were launched together. Logs remain outside Git; `.bboxy/runs/codex-rich-host.json` records commands, actual exit codes, source/candidate hashes and log hashes.

| Command | Exit | Log |
| --- | --- | --- |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-harness-native` | 0 | `/tmp/codex-rich-native-current.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-boundaries` | 0 | `/tmp/codex-rich-boundaries.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-formal` | 0 | `/tmp/codex-rich-formal-complete.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make check-rich-host test-licenses` | 0 | `/tmp/codex-rich-quality.log` |
| `. ./scripts/cache-env.sh; cargo nextest run --locked -p bboxy450-term-mux --lib -E 'test(window_journal)'` | 0 | `/tmp/codex-rich-window-journal.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity` | 2 | `/tmp/codex-rich-parity.log` |
| `make check-formal-evidence` | 2 | `/tmp/codex-rich-receipt.log` |
| `sh scripts/layering.sh` | 0 | `/tmp/codex-rich-layering-final.log` |
| `python3 -I scripts/source-notices.py --check` | 0 | `/tmp/codex-rich-notices-final.log` |
| `python3 -I scripts/test-formal.py` | 0 | `/tmp/codex-rich-formal-selftests.log` |

The native suite includes four Unix tests, two real native/MCP/process tests, the inherited-FD regression, the production rich actor witness, two App/capture witnesses and both deliberate controls. Boundaries passed 39 tests; one subprocess-only signing helper is deliberately ignored as a direct test and driven by its parent. Exploratory failures caught and corrected the causal turn-id expectation, an independent Hello metadata wake, fork ancestry admission, the required harness preparation permit, private GUI lease timing, and the native composer Send/Stop region absent from Frame.actions. Earlier partial/failing logs are preserved under `/tmp/codex-rich-*`; their results do not replace the final command evidence.

## Budgets and remaining limits

The UI drain allocation is **0.3 ms aggregate**, with round-robin deferral; worker-prepared composition has a **0.6 ms aggregate allocation**. Measured composition of four copies of the fixture: p99 **0.033417 ms**, 200 samples. Hidden/occluded views are dropped. This is a conversion/composition microbenchmark, not four live streams or a worst-case proof. Live aggregate drain/projection cost under load remains **UNMEASURED**; the single-step deadline can overrun, and a hard worst-case projection limit is not established. Transcript plus layout cache has a **32 MiB allocation, unmeasured and not an enforced heap bound**. The report labels these limits rather than marking them PASS. B's injected-clock four-view budget witness still needs the combined-pin integration.

Candidate d5e7249 has no 15-second server transport heartbeat. Term expires admission honestly after 20 seconds and reconnects; peer message 01M6HDJJDN4RSDVQ3JNGDK6PME requested the envelope heartbeat. Queued native delivery is storage evidence, not model receipt. The new IR 1.2/Quiet/origin constructor migrations, recovered standalone chrome/startup, final heartbeat behavior, full real App detach case and final integrated gates must be checked against the eventual combined published harness pin, not inferred from this IR 1.1 private candidate.

## Handoff and exact unblock decision

Signed implementation commits: `5db409e5afcdcb1c896f9cc4484ca0b0bfa32c94`, `26084f2bd62e3aa23182e9f2c70c085d3d9e90a5`, `dd7727373b74a6898737e939f87dc0676d4674f6`, `e82b1784355f2158828c5387ce23409695843e7e`. All signatures verify as Boxy <bboxy450@gmail.com>, approved key 9B07E849480C68328CA20684844ECC0587DE7AB0. This report's signed containing commit is bound by the Report trailer and task_report; its SHA is not self-referential frontmatter.

**Decision needed from the delegating Oracle:** provide the published combined standalone+B1/B2 harness SHA and amend this task's allowed paths to include the harness gitlink for that pin. Then migrate any additive constructors and run the outstanding combined-pin integration and clean canonical gates. Do not accept this task from the private candidate evidence. The current-source formal receipt is hash-bound to the private checkout with `dirty=true`; `make check-formal-evidence` refuses it (exit 2); full `make test-parity` is refused before full CI/visual parity because the source tree has the unstaged private gitlink (exit 2).

At handoff, the only intended source-tree difference outside committed files is the unstaged private harness checkout; the report/summary commit contains no source changes. No recruitment, push, merge, publication, trusted install, global setting or credential change, operator OS input or operator desktop capture was performed. All captures were private offscreen surfaces, and process cleanup targeted only fixture-owned PIDs/groups.

## Report registration transport failure

Authenticated `knowledge_fetch` failed twice with original operation `01M6HJ8VSRK143C0561S424Y27`; the required original-operation lookup also failed (`01M6HJ9VT9MSE8VN0GZE7KTWEG`). Each reports `Seat control transport unavailable`, read_response WouldBlock/os 35 after approximately 10,000 ms, status unknown. No effect is inferred from these errors. If registration remains unavailable, the second unblock is restoring the seat control transport so the host can assign the report id/number, register it and receive the task_report. No hand-written knowledge record or outside-scope backend restart is a substitute.
