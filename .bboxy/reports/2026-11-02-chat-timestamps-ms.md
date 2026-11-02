---
author: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
authorship:
  kind: seat
  process_generation: '1'
  public_key: /OIb7YSEjZ4vwrb0DGfWx/5YZnMzRnMYrcTl78DNuqs=
  signer: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
commits: []
created: 2026-11-02T04:01:03.571Z
created_by: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
id: 01M6D6NWJZDYGRASVTW4NFAZXK
number: 3153
provenance:
  digest: sha256:dc656f2f0c7e5f96f26df0f8f03b86d48dbdc5e634d6b2eaac622d67ab7a3d8c
  frontmatter_digest: sha256:e6a88c40c242f9a0775a800a0f2fc6d2cc0d5f2c8397a1abb02c09362b6ceea9
  log: .bboxy/log/2026-11-02.jsonl#13800
  sig: MGo60fh6/GM3hjXi0HLR/yIgVCoc6617GYE7NdvwUzJucxGYCiWnQLTxBS+TePmcK20E9pISFtWvFCv/s4ITBw==
  signer: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
task: '[[tasks/2026-11-01-chat-timestamps-seconds-as-ms]]'
title: 'Chat timestamps: explicit milliseconds with compatible seconds on the wire'
type: report
updated: 2026-11-02T04:01:03.571Z
updated_by: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
verify:
  command: cargo clippy --locked -p bboxy450-term-proto -p bboxy450-term-mux -p bboxy450-term --all-targets --target-dir target -- -D warnings
  exit_code: 0
---
# Chat timestamps: milliseconds in rows and displays

Author: Chat Timestamps, seat:01M6D4J7CTD3BX29WGVYXZ3TC9. Assignment: [[tasks/2026-11-01-chat-timestamps-seconds-as-ms]]. Worktree: `/Users/feral/mydev/bboxy450-term-chat-ts`; branch `chat-ts`, created from fetched `origin/main` at `86ce2653209b65c64e8c2bc81ce6aaa285bd447a`. No merge or push is authorized.

## Changes and acceptance evidence

1. **Explicit units and wire compatibility.** `crates/bbt-proto/src/messaging.rs:700` replaces Rust `MessageRow.ts` with `ts_ms`. `crates/bbt-mux/src/messaging/service.rs:1146` converts the message's own envelope/admission seconds to milliseconds once when projecting the row, using checked multiplication. `crates/bbt-proto/src/messaging.rs:709` preserves the existing W2.4 JSON key `ts` and its seconds value with a serde codec; old retained rows decode to Rust milliseconds, and new rows serialize to the identical old JSON shape. Overflow and subsecond values that cannot be represented by that legacy wire format are refused instead of wrapping or truncating. No protocol version or envelope semantics changed. The signed envelope and borrowed message view now document seconds at lines 269 and 559; signing code and timestamp bytes are unchanged. The boundary codec converts units during serialization/deserialization, without scaling retained Rust rows again.
2. **Every chat surface uses the sent/admitted instant.** `crates/bboxy450-term/src/messaging.rs:238` gives the stream widget both the millisecond instant and `local_time::hm(row.ts_ms)`. `message_entries_with_attachments` calls that same adapter; `messaging/history.rs:791` supplies the composer history, and `main.rs:3489` builds the conversation/sidebar from it. The regression at `tests/messaging_ui.rs:903` reads retained seconds JSON for two October 28 messages an hour apart, checks distinct and correct local labels, and checks direct entries and history with a deliberately unrelated projection time. It fails against original production source. The current design renders HH:MM, with no date separators or date label. `local_time.rs:18` passes the full historical instant into the retained timezone rules; its existing historical/DST/half-hour tests at line 40 retain the date context needed to choose the correct offset. No date UI was introduced.
3. **Notice ordering.** `crates/bboxy450-term/src/sidebar_chat.rs:26` compares message and notice epoch milliseconds using the existing insertion rule. Its regression at line 343 now constructs actual seconds-wire rows and calls the production message adapter, then inserts a notice thirty minutes between the two messages. It asserts all three millisecond instants in order. This replaces the former fabricated millisecond UI fixtures that masked the bug. The timestamp collection near the task's former line 369 is test evidence, not a second production sorting policy.
4. **Real service and external output.** `crates/bboxy450-term/tests/messaging_flow.rs:334` sends an actual signed decision through the private GUI socket and messaging service, obtains its envelope, and compares the rendered label and UI instant against that envelope's seconds times 1000. It checks GUI Recent serialization/readback and drives the normal composer-history readback commands. It also calls authenticated MCP `message_list` and compares the complete returned signed envelope with the original. MCP maps `message_list` to `Request::List` in `crates/bbt-mux/src/mcp/messaging.rs:365`; ctl inbox exposes the same retained `StoredMessage`/envelope in `crates/bboxy450-term/src/ctl/message.rs:160`. Those envelope timestamps remain seconds. Rust fixture constructors in history, live events, conversation cache and messaging UI were renamed with equivalent legacy wire values.
5. **Formal disposition.** No model changed: this is dimensional conversion, serialization compatibility and presentation, not a change to chronology policy, arbitration, concurrency, persistence, ownership, retries or lifecycle. The existing bus-sequence sort in `messaging/history.rs:837` is unchanged. The safety check here is `row.ts_ms = message.view().timestamp * 1000`, with display and notice comparisons in that same unit. The finite regression instance has two message instants and one intervening notice; projection is a pure atomic mapping and introduces no scheduler or liveness assumption. Existing formal verification remains required through the final `make test`.

## Commands and observed results before signing

Commands ran in this worktree with the repository cache environment, two Cargo jobs, worktree-local `target`, Make-equivalent opt-level 1/debug 0 profiles and `RUST_TEST_THREADS=2`. No operator interaction, credentials, desktop input or screen capture was used; the real-path fixture owns its temporary account, memory keys, supervisor and zsh process.

| Command | Actual result |
| --- | --- |
| `make init-submodules` | exit 0; pinned harness `47b140fa9bd2bec99054394c206deb1b36d7b3b0` initialized |
| `cargo test --locked -p bboxy450-term --target-dir target --test messaging_ui chat_timestamps_an_hour_apart_keep_their_sent_times_in_views_and_history -- --exact --nocapture` against original production source | exit 101; both labels were `10:06` |
| `cargo test --locked -p bboxy450-term-proto --target-dir target --test messaging message_rows_preserve_legacy_json_seconds_with_explicit_rust_milliseconds -- --exact --nocapture` | exit 0; one compatibility/bounds regression passed |
| Same hourly messaging UI command after the fix | exit 0; one regression passed |
| `cargo test --locked -p bboxy450-term --target-dir target --bin bboxy450-term close_notice_joins_chat_chronology_between_real_messages -- --nocapture` | exit 0; one ordering regression passed |
| `cargo test --locked -p bboxy450-term --target-dir target --test messaging_flow chat_timestamp_from_real_service_matches_envelope_across_gui_and_mcp -- --exact --nocapture` | exit 0 after correcting the fixture; one real-path regression passed |
| `cargo clippy --locked -p bboxy450-term-proto -p bboxy450-term-mux -p bboxy450-term --all-targets --target-dir target -- -D warnings` | exit 0 |
| `git diff --check` | exit 0 |
| `cmp /Users/feral/mydev/bboxy450-term/.bboxy/tasks/2026-11-01-chat-timestamps-seconds-as-ms.md .bboxy/tasks/2026-11-01-chat-timestamps-seconds-as-ms.md` | exit 0; exact assigned record copied unchanged |

The first real-path test attempt exited 101 because it injected a canonical row through the live-event entry point before the service had published its optional bus position. The corrected test uses the actual History Recent-response path, which explicitly supports that state; production ordering/admission logic was not changed. Setup initially tried nonexistent `make harness-init` (exit 2), then used the existing `make init-submodules` target above.

Raw logs remain outside Git under this worktree's `target/`: `chat-ts-baseline.log` (SHA256 `d516509cfa2bc80fb12f5f87217daac743401bf0a64cc426b94aea063c8120ed`), `chat-ts-targeted.log` (`ddfc1cfc6a2000c28d7818f03b78a84875854b8beb3c47cdeb2b232a4b394780`, first three post-fix regressions passed, initial fixture failure retained), and `chat-ts-real-path.log` (`6bf54d26bc39a42a72492d1e19d470bc56ec8cf7fa8f1d3c47e296591341e7a0`, corrected service-path exit 0). Strict lint output is `target/chat-ts-clippy.log`.

## Final commit gate and limits

This append-only report is committed with the implementation before the single full run required by the CPU budget. The required next command is `make test` on the containing signed commit, after checking `pgrep -fl "ci-local|make test"` and waiting for any other full run to finish. Its actual exit, exact SHA result line and log/evidence hashes will be supplied in `task_report`; the canonical full log is this worktree's `target/ci-local.log`. **No full-suite pass is claimed before that command runs.** The containing commit's `Report:` trailer supplies provenance without an impossible self-referential SHA in `commits`.

No additional boundary suite or visual parity run was requested: no process-boundary implementation, geometry, reference design or interaction changed. The real messaging-service/GUI/MCP witness above was run. No standalone formal rerun was added during iteration; the final canonical suite includes formal verification and ordinary unattended smoke. No changes to signed envelope generation, timestamps on stored envelopes, delivery priority, bus cursors, receive times, time-zone policy or date layout were made. The Oracle must independently verify the tree and command logs.
