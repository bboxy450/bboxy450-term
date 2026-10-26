---
id: 01M5WT42WYS0ZE470GYTF9VPCC
type: evidence
number: 357
title: "MA-5 visible reply validation and typed refusal evidence"
created: "2026-10-26T05:29:49Z"
updated: "2026-10-26T19:13:45Z"
---
# MA-5: visible reply validation and typed refusals

Source: `5140262246d30ce50b8d25cc1b7c4c2bd569a407` (signed), branch `velvet-ma5-thread-validation`.
Accepted contract: `709f77df`; this cut changes no protocol or golden bytes.

Unknown, invisible and mismatched reply chains return `unknown_thread` before a message or signing-counter effect.
The bounded indexed hint contains up to eight newest visible actual-chain tips; acknowledged retained threads remain usable.
Private operator-inbox threads are excluded even for Oracle priority seats; missing and invisible ids produce identical refusals.
Valid replies inherit the retained root label and reply to the actual parent sender; legacy sender-only caches migrate on reopen.
MCP preserves kind, message, hint and operation replay. GUI/ctl retains the same typed refusal through its journal and ctl exits 1.
The composer keeps its refused draft and recovery hint. Existing normal routing, receipts, identity and digest regressions remain green.
Verification: **82 passed, 0 failed, 1 existing night-boundary test ignored**; strict proto/mux/app all-target Clippy exit 0.
This is the refusal runtime cut only. Superseded/done/redelivery suppression is the next separately assigned cut (Oracle `07f99532`).
No full workspace gate or publication was run here; Oracle landing/presubmit remains required.
The one-line cache 20G commit `9aa147dc` copies separately handed-off `0afc784b`; skip it if already accepted.

## Mechanism

- `crates/bbt-mux/src/messaging/service.rs:236` — resolve a visible parent before admission; typed refusal and real-chain inheritance.
- `crates/bbt-mux/src/messaging/ledger.rs:133` — additive root-id cache migration; indexed inbox/outbox hint scan at line 291.
- `crates/bbt-mux/src/messaging/ledger.rs:1191` — resolve actual reply chains, independent of caller thread labels.
- `crates/bbt-mux/src/logic.rs:360` and `src/mcp/messaging.rs:467` — typed rejection survives journal replay and both transports.
- `crates/bboxy450-term/src/ctl/message.rs:241` and `src/ctl.rs:1443` — shared error printer retains the server DTO and exit 1.
- `crates/bboxy450-term/src/messaging.rs:828` — retained draft reports the refusal and recovery hint.

## Actual commands and results

Environment: cache helper, jobs 2, incremental 0, debug-memory test custody, dev/test opt-level 1, debug 0, debug assertions true; target is an own stable path.

- `cargo test -p bboxy450-term --locked --target-dir target/m2-reply-box-dev --test ctl ctl_unknown_thread_keeps_the_typed_hint_and_exits_one -- --exact` — EXIT 0, 265.752s. Raw log `target/ma5-refusal-proof/verified-ctl.log`; SHA256 `fa957c91d4cc560ff24226bf9272a200bbe7eb1c7cee60b6c69c88abcb03b6e6`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test messaging_threads --test messaging --test messaging_flow --test identity` — EXIT 0, 167.764s. Raw log `target/ma5-refusal-proof/verified-core.log`; SHA256 `dfb0df3e84fb23e6787aa06f59a681aab296c83e29c7ba5e9618099efba5c647`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --lib messaging::service::` — EXIT 0, 87.085s. Raw log `target/ma5-refusal-proof/verified-service.log`; SHA256 `4335855f3d8e1f39e1d60108ef03a1d73e99d3db7c4ca42c863cb9dab3963578`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp unknown_thread_mcp_refusal_keeps_the_visible_hint_and_replays_without_a_send -- --exact` — EXIT 0, 23.002s. Raw log `target/ma5-refusal-proof/verified-mcp.log`; SHA256 `8a252a011957b352ce984fb8157c8bc7da64825799113f8cad58b53fa99d3db2`.
- `cargo test -p bboxy450-term --locked --target-dir target/m2-reply-box-dev --test messaging_ui` — EXIT 0, 3.674s. Raw log `target/ma5-refusal-proof/verified-ui.log`; SHA256 `d5999d0ad4e63349065f81957c061c8f887518a726c84352d1c880903de13625`.
- `cargo clippy -p bbt-proto -p bbt-mux -p bboxy450-term --all-targets --locked --target-dir target/m2-reply-box-dev -- -D warnings` — EXIT 0, 54.962s. Raw log `target/ma5-refusal-proof/verified-strict.log`; SHA256 `2db37aa7755b0cb54c47e57036fdd026697e5c2c22b4dc69e1053a1e92bba5ca`.

Earlier red regression on `8200c0cb`: 3 reply-validation tests failed on the prior implementation (missing rows/plain mismatch); raw `target/ma5-refusal-proof/threads-before.log`, SHA256 `d829526c8f2d0df8ec35081af4272e645aa28998d968a4168c9ee563947f6ef5`.
