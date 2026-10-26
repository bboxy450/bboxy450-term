---
id: "01M5VV6Y755T0T4BDGYQ7ZXHZ7"
type: "task"
number: 342
title: "MA-7: private wake ledger and nightly input-token attribution"
status: "open"
role: "executor"
owner: "Sable Embertail"
verify: "python3 -I -B scripts/test-resource-ledger.py"
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[resource-ledger]]", rel: spec }
  - { to: "[[notices-never-wake]]", rel: invariant }
created: "2026-10-26T10:13:33Z"
updated: "2026-10-26T10:13:33Z"
---
# MA-7 wake accounting

Assigned by Vesper in `cfcf252c-92aa-48d8-9b0f-5c1ad7573cb4`, sequenced after the launcher promises by `6fa7b0f1-90de-4888-ade7-c83dc5ef1548`. MA-3's rebased implementation `ffd8c568` is an ancestor of main; #333's nightly inference reader already landed at `5d1b4c76`.

## Scope

- `crates/bbt-mux/src/messaging/` and the existing native-delivery boundary in `logic.rs`: metadata-only wake records for the admitted attempt, destination/class/mode/message IDs and native binding/outcome.
- `scripts/inference-ledger.py`, existing resource-ledger helpers/tests and the nightly invocation only as required for per-turn input-token fill.
- Private ledger only; no message/tool/thinking content, credentials, capability, provider call, new product wire or board renderer.

## Acceptance

1. One stable wake identity per native attempt, including a batched/digest delivery; record seat, destination, class, policy mode and message IDs. Inbox-only traffic has no wake cost line.
2. Native-written/queued/unknown remain their actual evidence, never a claim that a model accepted a turn. Accounting errors never stop or retry delivery.
3. Nightly token attribution uses actual, unambiguous session/turn or request identities. Missing or conflicting metadata remains unknown, never guessed from nearby timestamps or charged twice.
4. A ledger fixture proves batching, mode/class attribution, deduplication/restart, exact token linkage, unknowns and body exclusion; preserve the notices-never-wake and messaging-never-held checks.

No central collector or new correctness gate. The resource-ledger 90% machine-CPU target is outside this cut and remains unproven.

Quarantine evidence (2026-10-26): `adapters_real_zsh_overlay_reports_prompt_133_b_and_command_boundary` at `4810620ad93529df2b7d13476656c6f7ca3d48ff` passed an isolated exact retry; raw `/Users/feral/.local/state/bboxy450-term/landing/quarantine/14aa42640a684d0db36f0a540e80272b.log`. Expires 2026-11-02.

## Author-check finding, not an accounting defect

The first author run on `4810620a` failed the unchanged real-zsh boundary test at `crates/bbt-mux/tests/adapters.rs:396`: `native::process_start(info.pid)` returned `InvalidData: harness process unavailable` immediately after spawn. This fixture constructs Supervisor directly and does not invoke Messaging/wake accounting; its adapter/kit/supervisor source is unchanged. Whether the child exited or the process lookup failed is unproven; the old fixture retained neither exit nor PTY diagnostics.

First author EXIT 77/check 101, Rust 687 passed / 1 failed / 80 ignored; raw `presubmits/4810620ad93529df2b7d13476656c6f7ca3d48ff-33fa0023/commands.log` SHA256 `7f2fc600c6a27ab845d186e1684e253855a431bb075658adccb1f809a58f67cf`. One exact isolated control passed 1/1 (0.06 s), raw `ma7-adapter-isolated.log` SHA256 `66452f3235d2e4ce62865d5ecdb067552da4205b1439be7e28494c438f4465f2`. Standard #324 admission independently reran the exact test successfully; no assertions, deadlines, native code or tier changed. Sable owns this verification finding here until repaired or reassigned; quarantine expires 2026-11-02 and an isolated failure still blocks. The MA-7 acceptance above is unchanged; no fix of the process-start failure is claimed.
