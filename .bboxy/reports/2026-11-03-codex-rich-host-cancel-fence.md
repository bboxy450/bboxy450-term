---
type: report
title: 'Rich host Cancel correction: complete captured identity and causal process witness'
task: '[[tasks/2026-11-03-codex-rich-host]]'
supersedes: '[[reports/2026-11-03-codex-rich-host]]'
commits:
- d41992424bf73853ef906c9612d2e12167082a1b
author: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
created: 2026-11-03T21:23:48.637Z
verify:
  command: BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity
  exit_code: 2
id: 01M6HMQY9Z4SRXG7MRZM2C66TQ
updated: 2026-11-03T21:23:48.637Z
number: 18711
created_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
updated_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
authorship:
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  kind: seat
  public_key: YKv54MXoFa4rl83lF7g/hJHRLfs8fsSGu2+nb6F1XqY=
  process_generation: '1'
provenance:
  digest: sha256:4d86e1a15603ff17dfad4d7c651728d5e5b4607412ba8e267317d85b4a76421a
  frontmatter_digest: sha256:a84b11ca195e9b20949ad0150f93ab23f9da993e96eabd57ec2d4ee7d3ef9639
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  sig: bPABMkN9ZjG5i4N7mcm0YEuunYccxsJBm/+m8ApCO0V++uvoXjcuUCuXGkoRfAPhoqydgWwgLsoS7VosIDNBAw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#103166
---
# Rich Cancel: captured identity and current private verification

This corrects and supersedes [[reports/2026-11-03-codex-rich-host]] (#17675) for input identity claims and the most recent private-candidate verification. The complete delivery/view implementation and outstanding combined-pin integration described there remain in scope. Task [[tasks/2026-11-03-codex-rich-host]], Executor seat 01M6HCYVGE7CJ8KK6PB20SPF3D, worktree `/Users/feral/mydev/bboxy450-term-codex-rich`.

The Oracle's finding 01M6HHPQXD7ESPFC155DA1QNCP was valid: Escape called `HarnessView::cancel_prompt` after draining the current bridge state, without checking the queued request's session. A delayed A Escape could deny B's newly pending approval. The previous session-only Enter/Click guard did not establish epoch, generation or Cancel safety.

## Implementation and witnesses

- `crates/bboxy450-term/src/rich_pane.rs:61` defines the complete captured identity: session, epoch and displayed snapshot's generation. `:71` puts **Cancel, Enter and Click** behind the same ready/nonretired/full-identity admission. `:135` applies it after draining the bridge and before calling any of these operations. An old snapshot cannot acquire the current generation by being routed through a new Pane.
- `rich_pane.rs:95` retires the generation token before aborting the actor. Blocking preparation can finish after a future is aborted; that token prevents subsequent remote admission by a retained preparation job. The atomic boundary is admission: already-admitted socket work may still complete after retirement. This is not a claim that an in-flight kernel write can be recalled.
- Worker completion count and notice in Snapshot give the process test an explicit rejection receipt. The session process and PTY remain independently owned; this adds no UI/logic socket IO and no renderer implementation.
- `crates/bboxy450-term/tests/rich_client.rs:345` runs the production actor against a separate scripted Unix process, each child under a private HOME. The session-switch case captures A's real production snapshot, waits for B's projected actionable approval, delivers Escape stamped with A, observes its rejection and **zero ResolveApproval ops**, then sends current B Escape and observes exactly one Deny for B's actual approval id. The epoch and generation cases independently isolate those stamps and calibrate current-identity Deny after rejection. The three cases change **only session**, **only epoch**, and **only local generation** respectively. The epoch fixture honors Reset/refetch. The generation case creates generation 2 and retains generation 1's displayed snapshot; it does not infer generation from the session id.
- `rich_pane.rs:376` separately exercises the actual predicate with unready, retired, absent and each mismatched identity field. It is a local API check, not a real-process proof of all retirement interleavings.
- `scripts/rich-cancel-control.py` removes exactly `Input::Cancel` from the production remote-admission classifier, runs the same delayed-A/B-approval process test, and requires **exit 101 at `stale cancel crossed identity fence: session`**. The finally block checks that nobody changed the mutant and restores the exact original bytes. The source SHA256 and restoration result are recorded in `target/rich-cancel-control/report.json`. `make test-harness-native` runs this isolated control before rerunning the positive production tests. No product environment bypass was added.

The original actor fixture selected the first item snapshot (a tool) rather than the approval snapshot, and its single-connection server could let a retained detached subscriber block a new view's Hello. This correction selects the actual approval body/id, emits a matching completion, and serves subscribers independently. The new identity witness similarly distinguishes the session-started metadata, approval item and Reset semantics. Failures from these fixture errors were corrected without extending any deadline. Raw failed fixture roots/logs are private under `/tmp`; only command/exit/hash summaries are committed.

## Formal contract

`tests/formal_contracts/rich_host.rs:104`, `verification/contracts.toml:807`, the independent corpus and term coverage add RichCancelFence/BrokenRichCancelFence. State variables start with one queued Escape captured on A/e1/g1, current session 1 of three distinct A/B/A births, epoch 1 of two, generation 1 of two, ready=true, retired=false and stale_admission=false. Captured identity is immutable. Session owner changes birth/epoch, logic changes generation, UI retires the view, and worker dequeues/admission is atomic. Switch, Resume, Restart, Ready, Disconnect, Retire and DeliverCancel can interleave. NoStaleCancelAdmission requires all three identity fields, readiness and nonretirement. WF{DeliverCancel} guarantees the queued key retires without network fairness. The broken control bypasses Cancel admission and must fail safety.

The full canonical target exited 0: **286 models, 34 crates, 117 contracts**. RichCancelFence passes with **96 states/352 edges**, TLC exit 0; BrokenRichCancelFence fails safety with **144 states/504 edges**, TLC exit 12. Both complete reachable state and transition sets match native Rust. Current-source evidence SHA256 `cde003aa2deaaed3aa86f950298876183567eeedbb783878639b4a7d413a6cde`, source `d41992424bf73853ef906c9612d2e12167082a1b`. Receipt `dirty=true` names the intentional private submodule difference; it is not a clean acceptance receipt.

Finite graph/verdict parity does not establish arbitrary Rust refinement, OS timing, complete TLC replacement or recall of previously admitted IO. The wire/process and source-mutant witnesses supply separate implementation evidence.

## Commands and source binding

Signed source commit: `d41992424bf73853ef906c9612d2e12167082a1b`, parent `678eb9fc14ff4937af81363f3269d830967c9c52`. All private builds use signed harness `d5e7249a01bf4830c79979aa7a8e2f17753be83b`. The committed gitlink remains `0c403165e815c2f7bf77ccac293784b021f42f96`; it is deliberately unstaged, not an unpublished committed dependency. Raw logs stay outside Git. The appended cancellation section in `.bboxy/runs/codex-rich-host.json` records current command exits, hashes, the mutant's causal assertion and exact restoration, and formal receipt/model metrics.

| Command | Exit | Actual log |
| --- | --- | --- |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-harness-native` | 0 | `/tmp/codex-rich-cancel-native.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-formal` | 0 | `/tmp/codex-rich-cancel-formal.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-boundaries` | 0 | `/tmp/codex-rich-cancel-boundaries.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make check-rich-host` | 0 | `/tmp/codex-rich-cancel-lint.log` |
| `make test-licenses` | 0 | `/tmp/codex-rich-cancel-licenses.log` |
| `BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity` | 2 | `/tmp/codex-rich-cancel-parity.log` |
| `make check-formal-evidence` | 2 | `/tmp/codex-rich-cancel-receipt.log` |

Offscreen row structure mismatch is 0.0 (required ≤0.01); the same fixture through standalone's native renderer and the term pane remains the comparison. Four-copy composition p99 is 0.033958 ms. Aggregate allocations remain 0.3 ms drain, 0.6 ms composition, 32 MiB per view. Live multi-stream load, hard projection maximum and heap usage remain **UNMEASURED**, not PASS.

## Remaining integration boundary

This fixes the reported Cancel hole against the private B1/B2 candidate. **The overall task remains blocked on the published combined standalone+B1/B2 pin and a signed task amendment allowing its gitlink.** Published standalone ea753072/af65c6c was inspected read-only; it has IR 1.2 and changes the capture_frame API to Options, but does not yet provide Bridge::connect. The two term capture call sites need that Options/no_chrome migration when the combined pin is provided. Final combined-pin real App detach/reattach during a rich harness turn, transport heartbeat, additive migrations and live four-view budget checks remain outstanding as named in #17675. No source pin was invented and no gate was relaxed.

`make test-parity` exits 2 before full CI/visual parity: **dirty source tree; no exact-SHA proof**. `make check-formal-evidence` exits 2: **formal evidence requires clean source**. The observed machine CI process count was 2 (within the ≤3 start cap). Every workflow in this correction ran sequentially, and the source-restoring mutant had exclusive build access. These red gates remain mandatory.

The report/summary commit adds evidence only; task_report supplies its containing signed SHA. Final worktree status should contain only the unstaged private harness checkout. No push, merge, publication, trusted install, global setting/credential change, recruitment or operator desktop input/capture occurred.
