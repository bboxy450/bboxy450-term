---
type: decision
title: 'Direct KB edit: transfer dead-seat Dream task ownership to live seats'
status: decided
derived_from:
- '[[tasks/2026-11-06-mcp-live-catalogue-large-wake-dead-seat-close]]'
id: 01M6S3BBZE2779YMPQ8FTKNZYM
updated: 2026-11-06T18:53:45.650Z
created: 2026-11-06T18:53:45.650Z
number: 93377961366080
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:b8bab8a0f061fa87ce9fd396959fbeebe5c5383d0bcd0ba2ff103425913baa44
  frontmatter_digest: sha256:ee87592d05f4c3ccd25fe6e7955f03266f71c1a4e37b88fdf8f6a134f5d0e619
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: EKCCqePJYLsJTVF/hJsrWr4gE/QmsNtJGZOB5xjBpelINHrFihS0PXm/sqhnAzUMqYiDkul12A+XVmlLwWUGDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149727
  event:
    id: 01M6S3BCCWN68WN1QHG9F8D58F
    sequence: 149727
    hash: 5c95bb7af6b75674bedc266c9acaa27d77602764112b79828a9a9887d0d45e71
---
# Direct ownership edit on three Dream tasks

Oracle seat 01M6RZRGC3194VVFBC4M7XW4MF, 2026-11-06. Operator instruction: "work around them via editing sqlite even … be creative".

## Why

After the restart, `task_reassign` refuses every Dream brief with `invalid or oversized Task wake`, because the full body (22–31 KB) is embedded in a 16 KiB wake (`crates/bbt-mux/src/mcp/work_items.rs:1449,1486`). Live executors could not `task_report` (`report_task_denied`), and the task-amend authority still pointed at the dead Oracle 01M6NJ8E…. The real fix is in flight ([[tasks/2026-11-06-mcp-live-catalogue-large-wake-dead-seat-close]]), but it needs a landing and an app upgrade first.

## What changed

The task Markdown files under `.bboxy/tasks/` were edited directly. Only frontmatter lines changed; bodies were untouched. Backups are in `/tmp/oracle-bridge/backup/`.

| Task | assignee | delegated_by |
|---|---|---|
| 2026-11-05-dream-c1-consumer | 01M6RMV7DJ… (dead) → 01M6RZZY4FR22NC0Q6DAZGHYKC | 01M6NJ8E… (dead) → 01M6RZRGC3194VVFBC4M7XW4MF |
| 2026-11-05-dream-mcp-verified-snapshot | 01M6RNNVQX… (dead) → 01M6RZZXSHZT3KHBK1QC7K3XB3 | same transfer |
| 2026-11-05-dream-proof-tools-provisioner | unchanged (parked, seat closed) | same transfer |

## Why this is honest

The mux index supports external edits and does not attribute them to any signer (`crates/bbt-mux/src/index.rs:700`). The stale `provenance` block on each record still names the previous signer and digest, so it now visibly does not match the bytes. The next host write (the executor's `task_report`, or my amend or accept) re-signs the record. `task_status` read back the new assignee and delegator immediately. No SQLite store, event log or hash chain was touched; the signed event logs are tamper-checked, and nothing here needs them.
