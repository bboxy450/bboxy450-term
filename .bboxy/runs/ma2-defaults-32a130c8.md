---
id: 01M5WS5X68YCQEA4CYDCJW22KC
type: evidence
number: 356
title: "MA-2 role precedence and direct wake regression evidence (Velvet)"
created: "2026-10-26T18:57:16Z"
updated: "2026-10-26T18:57:16Z"
---
# MA-2 attention defaults correction — 817e4b43

Source: `32a130c88f68ad7750fa7e2673175c992aa592aa` (G signature), on `18c91d48ee7cf6f6565f2d851d57acebe0ec54fc`. Signed spec ruling `817e4b43` is in that base. Tests ran in exec6 with the source bytes retained in `target/ma2-defaults-proof/source.json`; the tracked tree stayed clean after the source commit.

- `crates/bbt-mux/src/messaging/attention.rs:32` — exact Architect/Creativity names take precedence over Oracle priority; other Oracle-class seats retain Oracle defaults. Executor direct conversation defaults to Wake; channel defaults remain Digest, events Inbox.
- `crates/bbt-mux/tests/attention_policy.rs:31` — ten role/class combinations cover channel, direct conversation and event decisions, including Oracle-class Architect and Creativity.
- `crates/bbt-mux/tests/messaging_flow.rs:525` — ordinary Executor DM is receipted Now and immediately prepares native work. Six digest fixtures now explicitly select Digest, preserving window/cap/order/fence/restart assertions.
- `crates/bbt-mux/tests/mcp.rs:5354` and `:5467` — public attention/Orient views report direct Wake and the role-name precedence, retaining policy ownership and operation replay checks.

Result: **67 focused tests passed, 0 failed, 1 existing night-only test ignored**; mux/app strict all-target Clippy exit 0. Affected fast invariants: **19 checks, 0 failures**, 11 slow and 7 unaffected-crate checks skipped. The default-event invariant test also ran directly in the full messaging_flow target.

Red proof before the implementation: the policy regression failed with Digest versus Wake (7 passed/1 failed); the ordinary DM service regression failed with a Deferred receipt versus Now (0 passed/1 failed). These expected exits were 101. Every final check below exited 0.

Environment: sourced `scripts/cache-env.sh` (sccache port 4620), jobs 2, incremental 0, debug memory custody, DEV/TEST DEBUG=0, OPT_LEVEL=1, DEBUG_ASSERTIONS=true. Cargo target passed with `--target-dir`; invariants used `BBOXY_CARGO_TARGET_DIR` for that same absolute target. `SIGNED_RANGE=18c91d48..32a130c8` for the invariant check.

| Check / exact command | Exit | Seconds | Raw log SHA256 |
|---|---:|---:|---|
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test attention_policy` | 101 | 179.283 | `3fbfe275360f7fdd0cdbee264ba66b27a28c1ffddacbfb6734abaa1a3cea39dd` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test messaging_flow executor_direct_conversation_wakes_immediately_by_default -- --exact` | 101 | 12.982 | `bddcd585c9047d69fbb3f609db8e5a32583baf55dcacf0f5ff89decd72c4c9c1` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test attention_policy --test messaging_flow --test messaging --test identity` | 0 | 49.623 | `6e192880d31a8592f11fec68b645eec0f78f8cb5f2221a7ea9b4582dcf7f9b18` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp mcp_attention` | 0 | 12.31 | `ef46ae17a87aaf592472ea946e9c588315b8a436dd585ea292fd613869175b9d` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp mcp_orient_reports_role_defaults_overrides_and_always_wake_exceptions -- --exact` | 0 | 0.623 | `6c69b1072399bf5005c7c7b2479cda0404c5467c347618f0c27297b561ed054b` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --lib messaging::` | 0 | 45.273 | `30a4aebb498380cee37d7b8feb5c6df9afd226c52978acd350d1141d5842c1ba` |
| `cargo clippy -p bbt-mux -p bboxy450-term --all-targets --locked --target-dir target/m2-reply-box-dev -- -D warnings` | 0 | 45.484 | `3decf10adcf8da7ebd628015ab50ecd83187743914ba0e086f2ebf39d7e0c4ac` |
| `sh scripts/invariants.sh --affected 'bbt-mux bboxy450-term'` | 0 | 198.893 | `16b10b59e56b13c0f6349012b795a99a740a41f735dc4e5d2c98b05778bdd7dc` |

Raw logs and the verified command manifest remain in `target/ma2-defaults-proof/` (each check name plus `.log` / `.json`, and `verified-checks.json`).

Cleanup: owned `target/m2-reply-box-dev` retired after lsof exit 1 with empty stdout/stderr; 2,167,365,632 allocated bytes removed. `retirement.json` and `retirement-lsof.log` are retained. All proof/gallery directories and frozen MA-5 branches remain intact.

Limits: no full-workspace postsubmit, visual parity, or terminal night claimed for this backend defaults cut. Landing is handed to the Oracle; this source has not been published by this session.
