# OSC 7501 live wire capture

2026-10-21 · Nyx Copperwing <EXECUTOR> · Task H live-capture addendum.

**PARTIAL: Claude is proven on the wire; Codex is unproven behind its startup trust gate.** The both-harness acceptance check exits 1. The operator must resolve that gate before the Codex experiment continues; no trust or hook-hash approval was accepted.

[[vision]] · [[ux-sidebar]] · [[iterm-harness-plugin]] · [[subagent-detection]] · [[harness-adapters]] · [[specs/README]]

## Retained evidence and method

- [Claude raw frames](../runs/osc7501-claude.bin): 482 bytes, SHA-256 `1614cad9e5c41558b9c1f0af74839b19ad20eb2cdbb39e66b23e6fc3903767eb`.
- [Claude timing/index](../runs/osc7501-claude-timing.json): UTC start `2026-10-21T20:54:36Z`; 13 harness-to-terminal frames, one probe and 12 status reports.
- [Codex capture](../runs/osc7501-codex.bin): **zero bytes**, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. This is an empty, startup-gated attempt, not a successful capture or evidence that Codex lacks the protocol.
- [Codex timing/index](../runs/osc7501-codex-timing.json): UTC start `2026-10-21T21:03:43Z`; no probe/report; bounded deadline and child exit `-15`.
- [Collector/reproducer](../runs/osc7501-capture.py): `.bboxy/runs/osc7501-capture.py:21` filters complete OSC frames; `.bboxy/runs/osc7501-capture.py:108` writes only unmodified 7501 frames; `.bboxy/runs/osc7501-capture.py:109` records byte offsets/lengths and elapsed monotonic time. Ordinary PTY text is never written to the captures. The Claude run discarded 8659 other PTY bytes.

The binaries are concatenations of exact **harness-to-terminal OSC 7501 frames only**, with no timestamps or terminal transcript bytes interleaved. Sidecars map every byte interval to receive time and record the reply event separately; the terminal's reply is not passed off as a harness report. Protocol title/msg fields remain inside their original frames; the table below decodes only state/id/title. All prompts and the child description are synthetic fixture inputs. No ordinary screen text, transcript files, credential values or hook payloads were exported. Diagnostic sidecars contain fixed flag names and shipped-binary offsets, not startup screen text.

`.bboxy/runs/osc7501-capture.py:85` starts t=0 before fork/exec; times are observer receive times, not producer timestamps. The loop polls at 50 ms, and frames read together receive nearby filter timestamps. UTC starts have one-second precision. PTY shape: 120 columns × 36 rows; `TERM=xterm-256color`. The collector handles the 7-bit `ESC ]` form, with BEL or ST termination. It does not establish 8-bit C1 behavior.

## Installed targets and launch scope

| Target | Installed artifact | Bytes | SHA-256 after capture |
|---|---|---:|---|
| Claude Code 2.1.304 | `/opt/homebrew/Caskroom/claude-code/2.1.304/claude` | 271161296 | `b9ac5374839f14abf3f1f45610637c332b5daee5ce0d1634726cef87aab78e6a` |
| Codex CLI 0.163.0 | `/opt/homebrew/Caskroom/codex/0.163.0/bin/codex` | 249952688 | `8813367285524ac798dd96d1b071aea30d58708c7226fd236cd275cf1cfdee31` |

`claude --version` / `codex --version`, both helps and `codex features list` were read-only commands, exit 0. The installed feature list names `hooks` as stable/enabled; later Codex launches used `--disable hooks` for this invocation only. No hook-trust bypass flag was used.

`.bboxy/runs/osc7501-capture.py:15` contains the complete fixture argv. Both processes ran from `/Users/feral/mydev/bboxy450-term`. Claude used a launch-scoped settings overlay with hooks disabled, no MCP servers, and only the Agent tool plus one tool-free custom child. It was asked to launch one synchronous child with description `OSC7501 child fixture` and then finish. The final Claude run used `TERM_PROGRAM=bboxy450-term`. Captured task IDs form a one-element set: `a9d4d919158900674`.

Codex used a fresh local backend, read-only sandbox and approval policy never. Attempts also tried the native iTerm environment (`TERM_PROGRAM=iTerm.app`, version 3.7.6), disabled hooks, removed inherited parent runtime/session identifiers and explicitly set this project's trust to **untrusted in argv only**. Those changes never selected trust. Native auth/configuration was not copied into Git or edited. The exact recorded argv is in each timing sidecar; the current reproducer stops at a recognized trust prompt instead of accepting it.

## Claude: observed handshake and reports

Native probe at +223.283 ms:

`ESC ] 7501 ; ? BEL`

Hex: `1b 5d 37 35 30 31 3b 3f 07`.

The collector replied at +223.293 ms with the same code/payload and an ST terminator:

`ESC ] 7501 ; ? ESC \`

Hex: `1b 5d 37 35 30 31 3b 3f 1b 5c`.

**The actual native probe was BEL-terminated, not the ST spelling in the original brief.** Claude accepted the ST-form reply and subsequently emitted status. All 13 received frames in this capture are BEL-terminated. The first main report arrived at +614.966 ms. This run proves that pairing works; it is not a no-reply control proving the reply is necessary.

| Receive t (ms) | Raw offset | Bytes | State/payload | Agent ID | Decoded title |
|---:|---:|---:|---|---|---|
| 223.283 | 0 | 9 | probe ? | main / all | — |
| 614.966 | 9 | 34 | idle | main / all | — |
| 746.192 | 43 | 37 | working | main / all | — |
| 3779.674 | 80 | 70 | working | main / all | — |
| 3779.751 | 150 | 93 | working | a9d4d919158900674 | OSC7501 child fixture |
| 5292.918 | 243 | 37 | working | main / all | — |
| 6652.330 | 280 | 40 | clear | a9d4d919158900674 | — |
| 7745.629 | 320 | 34 | done | main / all | — |
| 10758.925 | 354 | 37 | working | main / all | — |
| 10770.056 | 391 | 34 | done | main / all | — |
| 15759.594 | 425 | 19 | clear | main / all | — |
| 15761.889 | 444 | 19 | clear | main / all | — |
| 15762.776 | 463 | 19 | clear | main / all | — |

At +3779.751 ms, the child report was:

`ESC ] 7501 ; state=working:app=claude-code:id=a9d4d919158900674:title=T1NDNzUwMSBjaGlsZCBmaXh0dXJl BEL`

At +6652.330 ms it was removed with:

`ESC ] 7501 ; state=clear:id=a9d4d919158900674 BEL`

That is a 2872.579 ms **advertised child-row interval**, not a measurement of model/CPU execution time. The single distinct task ID and fixture title establish the requested native Agent-child lifecycle. The id-less main report became `done` at +7745.629 ms. No status setter, hook shim or iTerm API supplied these frames.

Reports are not synonymous with state transitions: multiple main `working` reports have other protocol metadata. The table omits msg/progress values. No blocked/error state was elicited.

**Cleanup boundary:** after the fixture's main `done`, this collector version tried `/exit` at +10746.336 ms while slash commands were disabled. The +10758.925 `working` and +10770.056 `done` belong to that cleanup input, not the original fixture. The collector then terminated its owned process group; child exit was 143 and the final three `clear` reports followed termination. These are not proof of a graceful application exit. The reproducer now uses Ctrl-D and retains a bounded termination fallback.

## Codex: startup gate, no protocol claim

All attempts were from the exact repository directory named in the brief. The current capture saw the static startup action `Trust and continue`. It did not see `Open restricted`, and no restricted-choice event occurred. No trust action was submitted.

The compiled Codex artifact at `0xb8d9464` lies inside the startup-action text `Trust and continue`. Adjacent shipped text says the trust decision will be saved and distinguishes restricted opening from changing saved trust. This is shipped-artifact/static evidence, correlated with the fixed runtime flag `trust_continue_visible`; no raw startup screen was retained. The configured project entry was absent before and after these experiments. That is not a bytewise audit of all global settings.

[[specs/README]]:84, S6: **“Per-launch settings overlay only. Never edit user settings files or trust hashes. If a harness demands trust approval, surface it to the operator; no bypass.”** Vesper's follow-up ICQ `244d2271-3514-4629-af32-fe8b50468660` explicitly says to stop and mark Codex unproven if the chooser persists in this repository. It persisted; the capture was stopped. The operator must approve the relevant trust in their own Codex pane or supply an already approved launch context before a new Codex run.

**Gap:** zero captured 7501 frames here do not establish emission, probe behavior, states, task IDs, or lack of support. The UI gate prevented the requested live Codex proof. Native Codex's static 7501 strings remain the evidence level reported in [[iterm-harness-plugin]]; they were not promoted to a live observation.

## Runs actually executed

Each row has its raw `.bin` and `-timing.json` under `.bboxy/runs/`. The Python wrapper exited 0 for these bounded captures; the **native child exit** is a separate column. No child success is inferred from the wrapper's exit status.

| Artifact stem | Wall duration (ms) | Probes / reports | Native child exit | Evidence scope |
|---|---:|---:|---:|---|
| `osc7501-claude-preflight` | 120213.914 | 1 / 7 | 143 | preflight: collector had UTF-8/C1 bug |
| `osc7501-claude` | 16050.134 | 1 / 12 | 143 | final evidence |
| `osc7501-codex-bboxy-baseline` | 30015.816 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-choice-unavailable` | 47805.174 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-explicit-untrusted` | 62884.927 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-hooks-disabled` | 35008.332 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-offset-diagnostic` | 8804.900 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-preflight` | 120009.952 | 0 / 0 | -15 | preflight: collector had UTF-8/C1 bug |
| `osc7501-codex-startup-diagnostic` | 12032.483 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex-startup-gated` | 45046.251 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |
| `osc7501-codex` | 10693.882 | 0 / 0 | -15 | final evidence |
| `osc7501-codex-trust-chooser` | 10324.857 | 0 / 0 | -15 | startup diagnostic; no positive protocol result |

The first collector treated a UTF-8 continuation byte `0x9d` as a C1 OSC introducer and could swallow later reports. The two `preflight` runs are preserved and must not support completeness/absence claims. The corrected Claude run is the positive evidence above. Later empty Codex runs are startup diagnostics; different terminal/runtime profiles do not isolate a cause for emission. An initial ad-hoc decoder also assumed ST and failed on BEL, exit 1; the final verifier chooses the actual terminator and passes.

## Verification and remaining work

- `python3 .bboxy/runs/osc7501-capture.py --self-test` — exit 0; byte-at-a-time parsing includes UTF-8 prompt text, ST/BEL frames and a cursor query; ordinary text is excluded.
- `python3 .bboxy/runs/osc7501-capture.py --verify` — exit 0; validates hashes, contiguous raw offsets, frame/code/terminators, timing order, base64 fields, task-ID grammar and counts. Claude: 13 frames / 12 reports / one task. Codex: empty capture explicitly unproven.
- `python3 .bboxy/runs/osc7501-capture.py --verify --require-both` — **exit 1, expected unmet acceptance**: Codex has no live status report.
- Report/link/source-line/whitespace checks — exit 0; verifies the literal raw probe and child frame against the stated offsets and confirms sidecars contain no startup token lists or environment dumps.
- `ls -l .bboxy/research/osc7501-capture.md .bboxy/runs/osc7501-{claude,codex}.bin` and `git diff --check` — exit 0. No runtime regression suite applies to a research-only capture.

**Not checked:** no-reply control; live iTerm2 response bytes/display nesting; Codex transitions; waiting/error/permission states; multiple simultaneous children; C1 OSC; arbitrary transcript replay; input readiness or message-consumption evidence. A displayed done/idle report does not authorize peer-input injection.

Only research/run artifacts were written. No application/spec/launcher files, user settings, trust hashes, credentials, provider configuration or task board were edited by this Executor. Native CLI session state remains outside Git; it was not exported. Other sessions committed some in-flight run files before this handoff; no commits were made by this Executor.

— Nyx Copperwing <EXECUTOR>
