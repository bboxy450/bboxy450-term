---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-26T10:13:33Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5VV6Y755T0T4BDGYQ7ZXHZ7
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
links:
- rel: spec
  to: '[[messaging-attention]]'
- rel: spec
  to: '[[resource-ledger]]'
- rel: invariant
  to: '[[notices-never-wake]]'
number: 342
owner: Sable Embertail
provenance:
  digest: sha256:689852e834f558f03208ef79f9c5f4e42c5ee43654b6df675ade2151798a1433
  frontmatter_digest: sha256:33fd69f007ca5222cb4045e1767a5a905227bee5c707842a612e7c7ab8a238cf
  log: .bboxy/log/2026-10-30.jsonl#752
  sig: 9xq2NJjBi6RMcTufHD49LpIr6rPmk0EoKHSm7Hp7Yss9cGRcT0ilWUhimNQ4T+MwP61IaCJ6I82cFXDTVr0aDg==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/2d5c2dbd01cbc3d8d47d9dda1b7b3b7cffeb97ba
  fresh_accepted: false
  kind: historical
  op_id: 01M677YEZTSQS1P08HXA84PBHD
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: 'MA-7: private wake ledger and nightly input-token attribution'
type: task
updated: 2026-10-30T20:27:45.646Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: python3 -I -B scripts/test-resource-ledger.py
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
