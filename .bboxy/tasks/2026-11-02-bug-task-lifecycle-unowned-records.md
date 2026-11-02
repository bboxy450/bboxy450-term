---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:19:49.664Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTBTGXT4X9R7R7Y9TZDXW
number: 5002208
provenance:
  digest: sha256:c00a94616bbbe5ea99d9b2bb2ec29fa95e286d4830fe88f3b972aaf349488a3c
  frontmatter_digest: sha256:ad127089661390c4b583a3d184efa342f8a938d702b1f5827352d27d2f80fff3
  log: .bboxy/log/2026-11-02.jsonl#402
  sig: 9sTOChNRDsZJwdapFU0ElQcpOfn0/lfB/X+XYSWuvPm5ir+5Vrovoebdk5cKKXyCqgjIFA/6Hp6pOSRO0IcQCQ==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: hand-written task records cannot be closed by any task_* tool (status active, no owner)'
type: task
updated: 2026-11-02T18:19:49.664Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify: cargo nextest run -p bboxy450-term-mux -E 'test(work_items)'
---
**Observed (2026-11-02 10:13 and 10:17).** `task_reconcile`, `task_status` and `task_delegate` refused real task records with three errors:
- `rejected: task swarm is ambiguous`: records with no `assignee` and no `delegated_by`, for example #5004, #5183, #505, #329 and #330.
- `rejected: invalid task status`: #5097 and #5003 carry `status: "active"`.
- `rejected: task seat unavailable`: records whose assignee seat belongs to another swarm, for example #5162 and #5119. An Oracle in the current swarm cannot reconcile landed work that a retired swarm left `claimed`.

**Mechanism.** `crates/bbt-mux/src/mcp/work_items.rs:1166`: `authorize` requires an assignee or delegator. Line 1172: the operator path also requires an assignee ("task swarm requires an assignee"). Line 1180: every referenced seat must be in the caller's swarm. Lines 1281-1289: `task_state` has no `active` state. Hand-written records, which have no owner and use `active`, are therefore unreachable. The only working route is a hand edit of the frontmatter, which leaves a stale `provenance` signature.

**Fix.**
- Let the account operator (`ctl task reconcile`) reconcile an owner-less record and a record whose seats have left their swarm.
- Map or refuse `active` at import time. Do not discover it at lifecycle time.
- Add a test for each of the three refusals.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
