---
authorship:
  kind: seat
  process_generation: '2'
  public_key: LC62IASJEjfPOa4ibQ/tUgVcbWPjh1mAiVF3YIhjlbE=
  signer: seat:01M649QP6AMVQWCG28X5MQP3JR
created: 2026-10-29T22:11:08.613Z
created_by: seat:01M649QP6AMVQWCG28X5MQP3JR
decided: 2026-10-29
id: 01M64VF1N4PQ5FJSKD4WFDMHF1
links:
- rel: amends
  to: '[[d266-orient-says-what-you-can-do]]'
- rel: constrains
  to: '[[mcp-control-plane]]'
- rel: constrains
  to: '[[w2-5-mcp-and-seat-wiring]]'
number: 483
provenance:
  digest: sha256:4c47c4228069f798821c860cc4b8e1bc8ae9af7d7a1b89b041fcbd214bf1238f
  frontmatter_digest: sha256:e4f91f8950877427f4dbb730be320531d0e73aae8d1efddd4edb0a13acb7c7cd
  log: .bboxy/log/2026-10-29.jsonl#2524
  sig: HCDdj4By8aFgxvzLXo64wXdZlnMP4fVbAy5ahV89LjkfZQ5Kq5tyC4htO3kGCjXKpjIwxc5KRcBo9ciDW0yUBw==
  signer: seat:01M649QP6AMVQWCG28X5MQP3JR
status: decided
title: operation_get own IDs are fresh observations; errors_read future
type: decision
updated: 2026-10-29T22:30:44.591Z
updated_by: seat:01M649QP6AMVQWCG28X5MQP3JR
---
# Decision 483: operation_get read IDs label fresh observations

Status: decided for current operation_get only, 2026-10-29. Native task #481. Contract editor Nyx Copperwing. Provider Rook agreement01M64W6HY3W3265XZRVG5XWQQJ and consumer Vesper Greywind ruling01M64WAQ77GFS12YBAZTCWE6M0 baseline signed395f0c5034f3727c179ac6e4780a435dc290fb8d's rule: Q!=T before lookup, current authority per invocation, zero Q binding/result/control-op storage, no target redispatch and unchanged ordinary mutation equality. This is a design baseline, not an implementation/test pass. errors_read remains future/unimplemented; its shared own-Q rule awaits separate scoped provider adoption and conformance. This cut completes neither issue75's contract/conformance nor its implementation.

## Conflict and alternatives

Decision266 and #87/#135 default to first-result replay for every supplied op_id. The new readers must still return honest unavailable evidence when their inspected ledger is absent, corrupt or locked. Strict replay needs a provable separate binding/result retained for the read itself. Using the broken inspected store as mandatory admission defeats recovery; a new durable shared query service is outside scope. An ephemeral result cache cannot distinguish a never-seen caller ID from an evicted/restarted ID without another binding/token contract. Reject both as this cut's prerequisite.

## Decision for current operation_get

ONLY current operation_get treats its optional op_id Q as a response correlation label for a fresh authorized observation. The matching errors_read rule remains a future proposal pending its own scoped provider adoption. Server mints Q if omitted. Every invocation reads again; Q neither binds arguments nor promises first-result replay. Repeating Q/T may observe a different state; Q/T2 is a fresh read of T2, not op_id_reused merely because Q was used; fresh Q2/T is another observation. Prefer a newly minted Q per call, especially concurrently. Transport response association belongs to each actual request, not a map that assumes Q is unique.

target_op_id T is exclusively the original operation being inspected. These calls do not create or change control/op/Q, claim Q in the mutation namespace, dispatch/reconcile T or relax any existing mutation's args/tool/full-actor/root equality. Q==T is invalid_arguments, refused before target lookup. A Q numerically matching another mutation ID never claims, overwrites or rebinds that entry; T alone selects the target. Q is not a recoverable operation handle.

Every invocation captures its own current principal/root/grants; recheck before payload publication. A separate call after an authorized scope change observes that new captured scope, never an old Q-bound result. In-flight scope changes return stale_scope/no payload; a current authorization denial returns forbidden. Same retained seat generation continuity for target lookup is unchanged. No cached foreign/stale payload or old capability is revived.

No Q binding/result cache, durable Q ledger, Q TTL or eviction recovery is introduced: new Q-specific retained bytes/records are zero. Restart is another fresh read after admission; the future errors_read cursor rule retains its specified high-water/filters/60-second lifetime and expires on restart/scope change when separately adopted. Reusing Q never extends it. Missing/corrupt target ledger remains the specified unavailable/unknown grade; unavailable own-Q retention is not a failure because none is promised. A lost read reply means no response observed, not target rejection/no effect; another read is safe and may differ.

## Consequences and verification

The #87/#135 amendment explicitly names the current operation_get exception to266, preserves every other available tool's default replay/guidance, and requires the operation_get catalogue description, orient.op_id_guidance and packaged bootstrap wording to agree. Rook's existing native487 grant permits that current operation_get adoption after the named final signed source is reported. This design record grants no additional code or production scope. The future errors_read capability needs its own scoped provider adoption. Retain readOnly/idempotent annotations for absence of target effects, not identical values. Do not advertise Beta or any new provider/schema/authority.

Each invocation, including a duplicate Q, keeps existing one-second total admission/IO budget, <=100ms SQLite busy wait/no internal retry, request/result/scan/permit limits and best-effort bounded read audit. Audit failure never prevents an otherwise readable result, creates the inspected store or proves target failure. No new queue/global worker/lock serializes communication.

Required implementation assertions, still unproven: actual bridge/tool mirrors/discovery/guidance; Q/T changes after controlled target progress; Q/T2 and concurrent Q replies route to their actual request/target; Q2/T refresh; root/grant/in-flight fences; restart and cursor age; absent/corrupt/locked store and audit failure; zero mutation-ledger/read-mark/effect changes; unchanged ordinary mutation same-ID replay, changed-args/actor conflicts and original unknown outcomes. Positive and causal negative controls are mandatory. Use the existing Make targets named in #87; this design task runs only licenses/record integrity.