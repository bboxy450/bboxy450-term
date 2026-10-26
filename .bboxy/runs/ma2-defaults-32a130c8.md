---
id: 01M5WS5X68YCQEA4CYDCJW22KC
type: evidence
number: 356
title: "MA-2 role defaults and named conversation regression evidence (Velvet)"
created: "2026-10-26T18:57:16Z"
updated: "2026-10-26T19:22:24Z"
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

## Named channel conversation — 50cf3f65

Signed source `f35f3c0aa601d87c16787870772290c4710b0a67` on `b1d71e7925f8c7ec3ca6bd43f98171a0d7978b5d`, branch `velvet-ma2-named-wake`. One production line changes the Executor named-conversation default from Digest to Wake. Ordinary channel conversation remains Digest; direct conversation Wake; events Inbox.
The role/class matrix exercises named Text and named events for all ten combinations; existing receiver overrides still win. MCP Orient reports named Wake. **10 tests passed / 0 failed; mux/app strict all-target Clippy EXIT 0.** Before the fix, the named Executor regression failed Digest versus Wake (EXIT 101).

| Command | Exit | Seconds | Raw log SHA256 |
|---|---:|---:|---|
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test attention_policy role_defaults_never_wake_an_event_and_unknown_names_use_executor_defaults -- --exact` | 101 | 86.6 | `b1bfe672e860354a0345d14190ff2f6cd41e307581a37f33ea8d63c23b285260` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test attention_policy` | 0 | 47.157 | `f7f3801979d2bb17b0ee65c12502bbf8f3c7ee31d003ae90cf828ddd46a0bb90` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp mcp_orient_reports_role_defaults_overrides_and_always_wake_exceptions -- --exact` | 0 | 14.232 | `8069305e36be6a9afc9aa00affd7cb5ca21e63165262f7a120c4643e50de5a4b` |
| `cargo clippy -p bbt-mux -p bboxy450-term --all-targets --locked --target-dir target/m2-reply-box-dev -- -D warnings` | 0 | 60.671 | `1668710084821d9645a4fb1bbc9eb9f3d3639bc92bcdd78f3fa7e8beccce654e` |

Raw logs/manifests remain in `target/ma2-named-proof/`; environment matches the earlier MA-2 checks (cache helper, jobs 2, own target supplied by --target-dir, debug memory custody, dev/test opt-level 1, debug 0, debug assertions true).

Scope/limit: `messaging/service.rs:406` still supplies `addressed_by_name: false`, the only production writer of that fact. This proves the default for an already-resolved named-address fact and its public view, not live name detection. The producer/ICD gap was reported to Corvin in ICQ `f1da0d7a-2a1c-4310-8031-054906a5c562`; it is outside this one-line correction. No new routing, wire field, renderer or publication.

Record validation: `cargo test -p bbt-doc --locked --target-dir target/m2-reply-box-dev --test corpus` — EXIT 0, 6 passed, 0 failed, 2.84 s; SHA256 `0a3b782bc18fced02d48ce3f16287768e9661ca124315c4da25847c5f945b5dc`. The initially inherited MA-5 notes lacked frontmatter (5 passed/1 failed); their existing metadata-only repair is included as `770ba1b6`, with no runtime change after source `f35f3c0a`. Raw failure retained as `target/ma2-named-proof/corpus-before-metadata.log`. Owned idle target retired=True, lsof EXIT 1, allocated bytes removed=1,212,833,792; proof/gallery directories retained.
