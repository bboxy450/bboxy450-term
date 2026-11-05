---
type: note
title: Automatic wake circuit breaker — direct operator brief
id: 01M6MC7ZHSEDY0AF13FAVJP7GS
updated: 2026-11-04T23:58:16.238Z
created: 2026-11-04T22:52:59.651Z
number: 93377961363619
created_by: seat:01M6MADHFD19P4TKM7P7X6QESA
updated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
authorship:
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  kind: seat
  public_key: Rg4zwPSh/8Jytfz7nuhB5zfZyLtAZj9dFmnXz9d411U=
  process_generation: '1'
provenance:
  digest: sha256:9fc453994f6d79dc2e85dd39ac110a0ddb1bf622095ab315a84a7a03f29df872
  frontmatter_digest: sha256:b21fa09c6d550b769d0113c598429fd8d0ee2eeae5100d5193eee42740109153
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  sig: GkQfji2hSYQ+DBecXZ6hGBqLz2fIoZuP8jOifeet6EMoFUzb6d886//0R47hgDqwIDQZg7KZzzyZ82Y9aKKhBg==
  log: .bboxy/log/2026-11-04.jsonl#130946
---
ROLE: SOLO — Sable Quill. Direct operator request, 2026-11-04: implement an automatic circuit breaker after the Vesper self-notification incident. Worktree: /Users/feral/mydev/bboxy450-term-wake-circuit-breaker; branch fix/wake-circuit-breaker-20261104. No delegation.

Scope: messaging admission and durable scheduling in crates/bbt-mux/src/messaging/, its focused tests, receiver diagnostics, a bounded formal model and negative controls, Make target, attribution and this committed evidence. Keep signed messages and inbox readback available. No credential changes, message deletion, global queue/lock, or unrelated repairs.

Contract: self-authored messages never automatically wake their own seat, including queued legacy rows. Reserve at most eight automatic native wake attempts per recipient within a 60-second fixed window. Before a ninth, latch the recipient circuit and retain subsequent automatic messages in its inbox. The latch survives clock advancement/restart; only a newly admitted direct operator conversation resets it. Operator messages remain deliverable; other recipients remain independent. Reserve and journal atomically before native I/O; unknown outcomes consume the reservation, proven pre-effect failures refund only their own epoch. Agent ACKs, own policy edits, retries and replay cannot reset the circuit. Expose the latch/count/limits in attention_get. This bounds automatic admissions, not token usage or model-internal loops without messages.

Verification: make test-wake-circuit-breaker; make land (includes native/Java formal parity), make test-licenses. Test the 24-self-notification incident, peer bursts/queued backlog, reopen, clock rollback/rollover, operator reset/replay, receipt retention, unknown/pre-effect outcomes, and recipient isolation. Keep a causal removed-guard control and explicit finite-model limits. Direct operator implementation is authorized independently of task tool availability; the current tool catalogue exposes no task_delegate or ctl.

Verification disposition: canonical gates exposed existing formal inventory omissions, unresolved canonical KB references, strict Clippy/source adapter defects and one idle-smoke callback. Restore exact missing canonical records and required inventory without weakening checks. Diagnostic smoke proved the callback was a retired TaskRetry timer, so retain/cancel that timer with its generation owner, preserve the existing admission guard, add a paused-runtime callback witness and finite lifetime model/broken control. Keep the idle assertion at zero frames and zero callbacks. These are concrete prerequisites to the requested signed landing, not permission for unrelated cleanup.