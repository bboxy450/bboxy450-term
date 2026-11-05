---
type: report
title: Rich seat sockets and the shared Unix/InProc native face
task: '[[tasks/2026-11-03-codex-rich-socket]]'
commits:
- bf8953a7c3982796e7875588d9f5f31c1f7004c1
- d5e7249a01bf4830c79979aa7a8e2f17753be83b
- 4495a98c3b21c0fe2457efdd74c4b183df585002
- b971c4426ec96d695ce1d8ab7b5a795360a32999
- 3d63a42b4d96de2589ce6331c4d859a7f06d22c2
- 46190c67add7ff82df7ae8a3aed5d0d41cb2e74f
verify:
  command: RUSTC_WRAPPER= make test
  exit_code: 0
author: seat:01M6HCVV32KS845WHVD5FCX7QE
created: 2026-11-03T20:36:06.286Z
id: 01M6HJ0JMXX8AGNEY1DCP369CY
updated: 2026-11-03T20:36:06.286Z
number: 17632
created_by: seat:01M6HCVV32KS845WHVD5FCX7QE
updated_by: seat:01M6HCVV32KS845WHVD5FCX7QE
authorship:
  signer: seat:01M6HCVV32KS845WHVD5FCX7QE
  kind: seat
  public_key: GKTJ8261cDV8D4owdFGG+rP8HGOy7l3XfykAhJLNtoE=
  process_generation: '1'
provenance:
  digest: sha256:4d4153c99261a5ef3e8a0659d012514c6257d2037e0a27afde219e06961e7afe
  frontmatter_digest: sha256:a4f3b04e89bb8135956d7fa4767c0a8d2e365cabfc41f953282ab6ceefda6ac6
  signer: seat:01M6HCVV32KS845WHVD5FCX7QE
  sig: 5t8F6VwW06Dp/ELdX/RICIMj+t14nc8OL1UDe9JlkpWvJE8qFkNyH8tF8dyO+XrlVI/KRuF3z/9XOdrscE8jAw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100080
---
Implemented task [[tasks/2026-11-03-codex-rich-socket]] (01M6HCYCQB66JTBKPHE1YE4QJB) in `/Users/feral/mydev/bboxy450-harness-pane-b12`, branch `pane/b1-b2`. Verified implementation source: `46190c67add7ff82df7ae8a3aed5d0d41cb2e74f`. The report commit adds only this indexed Markdown record. Its current-HEAD formal receipt is supplied in the task_report handoff.

Recovery and provenance: all 20 interrupted source hashes from the delegator's preservation-before.json match signed `433107e`, retained at `recovery/rich-socket-interrupted-2026-11-03`. The original `7da3091` tip remains at `recovery/rich-socket-original-2026-11-03`. Source was recovered before integration with origin/main `0c40316`; both sides' formal models, terminal rendering and GUI capture options were preserved during conflict resolution. Mainline source commits are `bf8953a`, `d5e7249`, `4495a98`, `b971c44`, `3d63a42` and `46190c6`. No reset, stash, broad staging, push, merge, publication or release installation was performed. Raw logs, metadata and preservation/signature audits remain outside Git at `/tmp/codex-rich-socket-recovery`.

Files and mechanisms:

- `src/frontends/headless.rs:429`, `src/frontends/seat.rs:23` and `:57` — bind the per-session socket before exposing the optional absolute, <=96-byte `--socket` alias, before prompting. UnixServer retains mode 0600 and peer-euid checks. Resume removes an owned stale session socket after writer admission; aliases refuse files, symlinks and live listeners. Host switches fork to fresh session IDs, retire old listeners after SessionClosed, and rebind the alias. Graceful exit unlinks both addresses. The superseded OSC 7502 discovery frame was removed.
- `src/gateway/unix.rs:250` — discovery request `{v, subscribe:{after,ephemeral}, approver}` learns identity from Hello at an arbitrary host-chosen path. An omitted subscription session selects the session already owned by this listener. Legacy explicit-session subscriptions remain supported. Every operation still requires its original session; old-session operations receive UnknownSession after rebinding.
- `src/gateway/unix.rs:228`, `src/gateway/mod.rs:423` — message_op_id hashes a fixed v1 namespace and message ID to a deterministic 128-bit operation key. Recovery reconstructs the existing 1024-operation admission window from durable causes. A retry retains its original session and ID, including after an unknown outcome. Profile requests use bounded admission/deduplication and are handed to the session host for the fork/rebind.
- `src/frontends/gui/bridge.rs:438`, `:500`, `:646` and `:695` — socket connect, decoding, sends, retries and preparation run on bbt_rt. Unix and InProc share consume, Mailbox, Latest, FrameState and preparation. Remote scheduling returns Pending; it cannot claim server admission. Correlated results are session/epoch fenced. Session changes reset projection/preparation and retire old intent ownership; stale outcomes cannot pollute the replacement face or hold detach alive. Reconnect attempts are at least two seconds apart. Reset rebuilds the trusted view. Detach releases approval ownership and sends no Close; idle detached workers exit before reconnect backoff.
- `src/frontends/gui/view.rs:1620`, `gui/options.rs`, `gui/window.rs`, `src/main.rs` — remote /profile uses the seat host, and standalone resume uses the same host/face. `src/gateway/mod.rs` adds a test-fixture publisher. `scripts/test.py` and `tests/gui_gate.py` isolate inherited seat identity. `ATTRIBUTION.md` records the project-owned socket and bridge work.
- `tests/formal/models.rs:2946` and `:3065`, `tests/formal_nonvacuity.rs:24`, `:816`, `:855`, and `verification/{contracts,corpus,coverage}.toml` / `verification/README.md` — two socket contracts, seven deliberately broken controls, nonvacuous lifecycle and source mappings. The complete corpus has 104 variants, 36 contracts and one inventoried crate.

Acceptance evidence:

- `tests/support/seat_socket.rs:370` — real private PTY seat: both sockets, 0600, one turn after duplicate sends, socket SetProfile and terminal switching, old subscriber SessionClosed, cleanup of old addresses, reconnect at the alias within two seconds, old-session refusal, distinct A/B/A births, SIGKILL stale-address recovery at a new epoch, graceful cleanup, and check_transcript. The GUI host witness uses private scripted pipes and never opens a native window.
- `tests/support/seat_socket.rs:428` — a persisted message is retried after crash/reopen with the same deterministic ID; Ack is duplicate and the log contains exactly one TurnQueued for that cause.
- `tests/gui_bridge.rs:148` — all 11 locked IR v1 JSONL fixtures traverse both transports; complete Transcript and rendered layout_text match, with duplicate_durable == 0.
- `tests/gui_bridge.rs:389` — an arbitrary seat address reconnects across three births; a controlled unbound window refuses connection; old intent is refused rather than retargeted. Other bridge cases cover unsafe aliases, unavailable cancellation, off-UI sends, rendered approvals and detach without stopping its turn.
- `tests/gui_bridge.rs:487` — SessionClosed is ordered before a withheld reply, then the address is replaced. Stale results neither alter the new face nor leave detach waiting for an old intent. Removing the two intent-retirement paths makes this real API witness fail with cargo 101 at its detach deadline; restored source passes. A first unordered disconnect control settled normally and was replaced by this deterministic control.

Commands and actual results:

- `RUSTC_WRAPPER= make test` — exit 0 on 46190c6; complete output `make-test-verified.log`, machine-readable `canonical-result.json` and copied final gate receipts under `/tmp/codex-rich-socket-recovery`. All canonical unit, integration, process, formal, GUI smoke and private packaging checks run unattended.
- `RUSTC_WRAPPER= make test-formal-parity` on `46190c6` — exit 0. All 104 complete reachable-state and transition sets, and registered safety/deadlock/liveness verdicts, matched pinned Java TLC. Receipt: `verification/evidence/8c3c67aa98e0639217e2644cd99b048c0dbd035358a27b7c0d8f6f9b41010c72/evidence.json`; dirty=false.
- `RUSTC_WRAPPER= cargo test --offline --locked --all-features --test gui_bridge` — exit 0, 10 tests (`gui-retirement-final.log`). Earlier standalone/test-fixtures bridge + CLI checks also returned 0, including the 11 layout comparisons and three CLI cases.
- Private-HOME `cargo test --offline --locked --test seat_process -- --skip non_pty_spawn_does_not_keep_a_sibling_master_alive --skip seat_launch::` — exit 0, 11 cases. The same command with `--features acp,test-fixtures` — exit 0, 19 cases. The omitted launcher/CLOEXEC cases are separate canonical steps, not claimed by these targeted runs (`seat-pipe-results.json`).
- `RUSTC_WRAPPER= cargo test --offline --locked --test formal_nonvacuity socket` — exit 0, three nonvacuity cases (`socket-model-retirement.log`). The deterministic source-removal control above returned cargo 101 as required; its supervising assertion returned 0 (`stale-dispatch-broken-deterministic.json`).
- Preservation hash audit — exit 0, all 20 hashes equal signed recovery commit 433107e. Signed source commits use approved key `9B07E849480C68328CA20684844ECC0587DE7AB0`. Final signature, KB-record, secret and current-HEAD formal checks are recorded by the task_report handoff.

Corrected failures: the first full gate on d5e7249 returned make 2 / cargo 101 because a fixture asserted healthy connection state before Lagged repair completed; 4495a98 waits for repair and isolates direct fixture environments. The next gate on 4495a98 returned make 2 / cargo 101: the alias could answer before the per-session address existed, and a GUI PTY fixture timed out. b971c44 fixes bind order; 3d63a42 replaces the unnecessary GUI PTY with isolated scripted pipes and descriptor cleanup. A superseded launch was stopped with exit 143 upon observing an earlier machine-wide canonical gate; only its recorded owned descendant PIDs were stopped. The successful gate self-serves capacity without operator approval. 46190c6 fixes stale-intent retirement and the idle detach backoff discovered by the additional ordered API witness.

Bounds and remaining coverage: delivery models one message, one retry, two operation keys and three distinct A/B/A births. Lifecycle models two epochs per birth, one job, <=2 completions and Boolean pending/retired ownership. Weak fairness applies to enabled acknowledgement/refusal, attached live Connect, and eligible detached-worker retirement actions. No arbitrary OS fairness or Rust refinement is claimed. Hello/ack deadlines (5 s), reconnect spacing (>=2 s), server drain (1 s), socket/PTY/JSON bytes, filesystem permissions and process scheduling remain implementation witnesses. Idempotence retains its existing 1024-operation horizon. Side A delivery receipts, its 20-second readiness policy, the in-tree toolkit-copy check, aggregate pane budgets and term's offscreen visual comparison remain consumer-owned work. Tests use private scripted identities; no live-provider credential, operator click, Keychain prompt or desktop input was required. Generated formal archives remain untracked for review; generated tracked run receipts are preserved outside Git and restored to their committed versions.
