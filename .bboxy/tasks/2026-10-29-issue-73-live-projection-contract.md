---
acceptance:
- A written disposition provides bounded fresh canonical message/receipt projection during a retained348k-event backlog or a bounded alternative satisfying73.
- Historical cursor, pending admission, explicit ACK and read-mark meanings remain intact, without history deletion, silent skip or forged model receipt.
- Caller/subject visibility, source/version bounds, lifecycle/replay and produced-frame verification are explicit; both sides and reason are recorded.
- A signed scoped draft with exact lines is delivered promptly for Oracle/client-owner review, followed by actual checks and gaps; no code or merge is claimed.
allowed_paths:
- .bboxy/interfaces/logic-gui.md
- .bboxy/interfaces/messaging-seats.md
- .bboxy/specs/l9-messaging-tasks.md
- .bboxy/runs/issue-73-live-projection-contract.json
artifacts:
- '[[01M64J4Q7P27MHGM2JT9SRHENN]]'
assignee: seat:01M649QP6AMVQWCG28X5MQP3JR
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-29T18:25:50.657Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64EJGHRXP59SMANVQNS78SS
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
number: 388
provenance:
  digest: sha256:b39c934c02ccec31205602e056047fec9d14d7fc1d09058e9ec90a9c3f5706e0
  frontmatter_digest: sha256:e3ca26ea126fa74bde6a16f33c1234172d4912af4e8deec3ff505aa66e104d43
  log: .bboxy/log/2026-10-30.jsonl#812
  sig: 4K8aHHA0vH/7T8RhuS7/LnVpwWhSb43IFeFSaxVHjK+SuQ4Sto50vypTCR0uukmQj3mfduVfKzVchMRsJ9uQDw==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  - https://github.com/bboxy450/bboxy450-term/commit/83fdacb3890f8d77c183af644c09fd435f484b8e
  fresh_accepted: false
  kind: historical
  op_id: 01M67861BSW91PM6SFJW60CENE
  previous_status: reported
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
rejection:
  by: seat:01M649MS05FAD5CY9CS0Q07H76
  reason: 'Consumer review of signed9b3769a3 actual spec156:132–337: do not baseline this wire expansion yet. Lumen independently found and Oracle verified a smaller existing seam: Request::Recent already returns canonical messages/real receipts with source validation (service.rs627–675, history.rs401–520), and known composer completion main.rs5563–5573 can request it. Native task447 is now delegated to Lumen for a bounded coalesced read-after-known-commit, main.rs/history.rs only; it preserves the existing event lane and older pagination. Amend388 to baseline that existing-seam agreement first, including identity, snapshot freshness, coalescing, failure/restart and exact retained348000 production witness. The claim at156:151 that a one-off read is insufficient for the issue needs a causal counterexample distinguishing initial real delivery receipt from later receipt-only freshness; later receipt replay/event/explicit read stays honest, never model-ACK promotion. Treat optional MessageProjection/nonterminal variant/tracking32/new version/budget as an UNSELECTED alternative until the existing seam fails the whole issue acceptance. Do not require guessed KB-root revision or new per-message signature/revision fields without identifying the actual messaging authority/source boundary and missing guarantee. If later receipt-only updates independently require a new push, state the exact unmet assertion and smallest additive invalidation/read mechanism rather than making all of that protocol a prerequisite. Both producer and consumer must agree before any wire change. This revises only388''s existing allowed files, no product code; no new full build before a signed minimal draft/review. Task394''s numeric CI allocation is already separately delegated and should proceed independently; broad381 capacity failure is retained.'
report:
  commit: 7cd80813f505e8b836aa6466cfed810556f3eea7
  op_id: 01M64KC6FVN43KJ5W4KDS3D0ZW
  reported_by: seat:01M649QP6AMVQWCG28X5MQP3JR
  summary: |-
    Supplemented the already consumer-agreed #388 existing-Recent contract with the exact two-sentence clarification requested in01M64K3BBQ7GYDXVZGBVF0ZS9E. A matched authenticated canonical Recent item/row with matching bus=None may display before publication, preserving None without Event/unread/cursor/pending/ACK/read evidence; ordinary Event fences remain and actual later authorized publication merges the same ID/context/real position without duplicate or confirmed-grade loss. Actual canonical-None→actual-Event production witness and restoring old None rejection as a negative control are required before implementation acceptance. No new wire/source field or code authority.
    Signed7cd80813f505e8b836aa6466cfed810556f3eea7, exactly the3 assigned agreement files. Final #449 adoption locations: specs/l9-messaging-tasks.md:186, interfaces/logic-gui.md:104, interfaces/messaging-seats.md:51. Identity/frontmatter/links preserved except updated; signature/diff/scope checks0, clean tree. make test-licenses0 on exact7cd source (6+9 tests;27 crates); raw target/canonical-none-licenses.log. Required make test-task-contract was last actually0 on signed9aec6da5 before owned build retirement. No current-HEAD focused/broad pass or cold rebuild is claimed: current observed free11699761152 versus remaining-target/runtime/margin requirement19071397888. The different actual verification command below is intentional, not a claim of matching the original focused gate; independent implementation/adoption gates still owe their exact-source assertions. Prior actual make2/Cargo101 capacity failures remain historical/superseded. #455 mapping was kept independent and already reported at68ef296d with exact licenses0. No code, runtime, credentials, peer worktree/target/cache, merge/push or recruitment.
role: architect
status: done
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: 'Issue73 ICD disposition: bounded fresh message projection during historical replay'
type: task
updated: 2026-10-30T20:31:53.844Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: make test-task-contract
verify_result:
  command: make test-licenses
  evidence:
  - criterion: A written disposition provides bounded fresh canonical message/receipt projection during a retained348k-event backlog or a bounded alternative satisfying73.
    reference: .bboxy/specs/l9-messaging-tasks.md:132 selects the existing Recent read; :186 clarifies matched canonical None before real Event publication without new protocol.
  - criterion: Historical cursor, pending admission, explicit ACK and read-mark meanings remain intact, without history deletion, silent skip or forged model receipt.
    reference: .bboxy/specs/l9-messaging-tasks.md:186 preserves None with no fabricated Event admission/unread/cursor/pending/read mark/ACK; ordinary Event fences and later actual ID/context/position merge retain confirmed grades.
  - criterion: Caller/subject visibility, source/version bounds, lifecycle/replay and produced-frame verification are explicit; both sides and reason are recorded.
    reference: '.bboxy/interfaces/logic-gui.md:104 and messaging-seats.md:51 record the consumer-approved two-sentence clarification; #156 requires canonical-None→actual-Event production witness and old-None-rejection control before acceptance.'
  - criterion: A signed scoped draft with exact lines is delivered promptly for Oracle/client-owner review, followed by actual checks and gaps; no code or merge is claimed.
    reference: Signed7cd80813f505e8b836aa6466cfed810556f3eea7 changes exactly the3 assigned files; record/signature/diff checks0 and make test-licenses0 on that exact source. Required make test-task-contract was last actually0 on9aec6da5; no cold rebuild/current focused or broad pass is claimed under recorded capacity shortage. Consumer request01M64K3BBQ7GYDXVZGBVF0ZS9E and source interpretation are preserved in native messages.
  exit_code: 0
---
# Urgent issue73 boundary disposition

ROLE: ARCHITECT — Nyx Copperwing. Delegator Vesper Greywind. Same assigned writing worktree /Users/feral/mydev/bboxy450-term-mcp-recovery-spec-20261029, branch architect-mcp-recovery-spec-20261029, base014e555c. Prioritize this bounded ICD disposition before finishing task381's74/75 API spec. This is a native assignment; the preceding workspace_pending result was a known no-effect admission refusal.

Lumen found the exact missing line: bus.rs::gui_poll scans128 metadata rows/pass in sequence order; pending/cursor admissions are atomic with explicit ACK. Stopping new failed-wake receipts alone does not let a348k-events-behind consumer deliver a fresh composer row within the existing frame/control budget. ICD346/349 do not specify a fresh projection independent of historical replay. The Oracle represents the client consumer owner and authorizes the smallest correct agreement with messaging-side implementer Lumen. Ask/answer freely within the briefs.

Consider an additive bounded owner-side projection of the actual canonical persisted message and real current receipt to the sending GUI/control result, independent of the historical event cursor/explicit ACK/read marks. This is a candidate, not mandatory design or already approved law. Prefer an existing seam if it can satisfy the full promise. No optimistic signed envelope, storage/native-to-model-ACK promotion, broader grants, global scans, silent cursor advance or history deletion. Specify identity/source/revision binding, bounds/timing, failure/unavailable, lifecycle/replay and later receipt refresh. The produced frame must show the actual source-bound row/receipt.

Only listed existing ICD/spec files; preserve record IDs/numbers and record current reason/both owners. Send the signed draft and exact file:line disposition promptly for Oracle review and a subsequent native implementation supplement to Lumen. No product code, task lifecycle edits, runtime reset, keys/credentials, live app, desktop input, merge/push or recruitment. Verify make test-task-contract and needed record/frontmatter/link checks; final task_report has actual exits and scope limits. A cold full gate need not delay the draft review; final handoff still retains required exact-source verification.

## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Revised #388 after consumer rejection: signed7684bbbf selects only existing Recent read-after-known-commit, exact StoredMessage/ledger identity, one-inflight/one-dirty coalescing, unchanged older pagination and Event/cursor/pending/ACK/read semantics, actual messaging operator/swarm/context authority and honest snapshot/later-event behavior. Prior9b3769a3 new wire/prelude/root/source fields/tracking/version/budget are UNSELECTED. Oracle inspected actual minimal source/signature and agreed in01M64HMVQZCGZN6D74XYXTS5FT; Lumen's producer seam/scope statement01M64HGJ1NBMH3J8YN55T632YM matches. Implementation447 and actual348000/labelled16384 produced-frame/real receipt/independent causal mutations remain required.
Final frozen HEAD 9aec6da534424caa33180bd52759e345d4b31f8d signed/clean. make test-task-contract exits0 (1 passed/0 failed); raw target/final-frozen-law-task-contract.log SHA25641fedc64e8057490d0067e31ae300cda51a0c65efbbb17a53c5adc91a1998b84. Nine original record frontmatters/IDs/numbers/checks/links preserved except updated; new links resolve; diff/signature checks0. Broad run started4ddfa288, ended make2/Cargo101 with sole failed checkpoint_bounded_recovery at recovery.rs329 (10539065344 bytes below10GiB); source changed under Oracle sequencing permission, so it is superseded for this HEAD. Raw target/issue73-ci-local.log SHA256416816a9fb7c560b336cf7a6209867be612c985f496bdcd4be6f3996c63b13df. No current broad pass. Only owned inactive debug outputs retired after no-holder lsof check; logs and app/focused/recovery binaries retained with manifest. Free afterward15987892224 bytes remains below cold18GiB allocation, so no identical rebuild. No product code, runtime, credentials, peer cleanup, merge/push or recruitment. Prior report op01M64J1600NGERDW1BMJYDMKAZ was explicit no-effect workspace_pending refusal; task_status now resolved before this fresh report.

```json
{
  "commit": "9aec6da534424caa33180bd52759e345d4b31f8d",
  "op_id": "01M64J36RNDWEXV8H6XJ24R6F0",
  "verify_result": {
    "command": "make test-task-contract",
    "evidence": [
      {
        "criterion": "A written disposition provides bounded fresh canonical message/receipt projection during a retained348k-event backlog or a bounded alternative satisfying73.",
        "reference": ".bboxy/specs/l9-messaging-tasks.md:132 — selects existing scoped Recent after known committed send, with exact StoredMessage/sequence identity at :156; prior wire expansion explicitly unselected."
      },
      {
        "criterion": "Historical cursor, pending admission, explicit ACK and read-mark meanings remain intact, without history deletion, silent skip or forged model receipt.",
        "reference": ".bboxy/specs/l9-messaging-tasks.md:186 — one-inflight/one-dirty current read, independent older pagination and unchanged historical cursor/pending/ACK/operator read meanings; no polling, deletion or effect replay."
      },
      {
        "criterion": "Caller/subject visibility, source/version bounds, lifecycle/replay and produced-frame verification are explicit; both sides and reason are recorded.",
        "reference": ".bboxy/specs/l9-messaging-tasks.md:156 and :216 — messaging operator/swarm/context authority, matched canonical snapshot/receipt semantics, source correlation, failure/restart and unchanged15s produced-frame witness; ICD346:86 /349:33 record owners/reason."
      },
      {
        "criterion": "A signed scoped draft with exact lines is delivered promptly for Oracle/client-owner review, followed by actual checks and gaps; no code or merge is claimed.",
        "reference": ".bboxy/runs/issue-73-live-projection-contract.json:1 — signed minimal correction7684bbbf, consumer agreement01M64HMVQZCGZN6D74XYXTS5FT, producer seam01M64HGJ1NBMH3J8YN55T632YM, actual checks and superseded capacity result; implementation/frame/mutation evidence remains open."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Supplemented the already consumer-agreed #388 existing-Recent contract with the exact two-sentence clarification requested in01M64K3BBQ7GYDXVZGBVF0ZS9E. A matched authenticated canonical Recent item/row with matching bus=None may display before publication, preserving None without Event/unread/cursor/pending/ACK/read evidence; ordinary Event fences remain and actual later authorized publication merges the same ID/context/real position without duplicate or confirmed-grade loss. Actual canonical-None→actual-Event production witness and restoring old None rejection as a negative control are required before implementation acceptance. No new wire/source field or code authority.
Signed7cd80813f505e8b836aa6466cfed810556f3eea7, exactly the3 assigned agreement files. Final #449 adoption locations: specs/l9-messaging-tasks.md:186, interfaces/logic-gui.md:104, interfaces/messaging-seats.md:51. Identity/frontmatter/links preserved except updated; signature/diff/scope checks0, clean tree. make test-licenses0 on exact7cd source (6+9 tests;27 crates); raw target/canonical-none-licenses.log. Required make test-task-contract was last actually0 on signed9aec6da5 before owned build retirement. No current-HEAD focused/broad pass or cold rebuild is claimed: current observed free11699761152 versus remaining-target/runtime/margin requirement19071397888. The different actual verification command below is intentional, not a claim of matching the original focused gate; independent implementation/adoption gates still owe their exact-source assertions. Prior actual make2/Cargo101 capacity failures remain historical/superseded. #455 mapping was kept independent and already reported at68ef296d with exact licenses0. No code, runtime, credentials, peer worktree/target/cache, merge/push or recruitment.

```json
{
  "commit": "7cd80813f505e8b836aa6466cfed810556f3eea7",
  "op_id": "01M64KC6FVN43KJ5W4KDS3D0ZW",
  "verify_result": {
    "command": "make test-licenses",
    "evidence": [
      {
        "criterion": "A written disposition provides bounded fresh canonical message/receipt projection during a retained348k-event backlog or a bounded alternative satisfying73.",
        "reference": ".bboxy/specs/l9-messaging-tasks.md:132 selects the existing Recent read; :186 clarifies matched canonical None before real Event publication without new protocol."
      },
      {
        "criterion": "Historical cursor, pending admission, explicit ACK and read-mark meanings remain intact, without history deletion, silent skip or forged model receipt.",
        "reference": ".bboxy/specs/l9-messaging-tasks.md:186 preserves None with no fabricated Event admission/unread/cursor/pending/read mark/ACK; ordinary Event fences and later actual ID/context/position merge retain confirmed grades."
      },
      {
        "criterion": "Caller/subject visibility, source/version bounds, lifecycle/replay and produced-frame verification are explicit; both sides and reason are recorded.",
        "reference": ".bboxy/interfaces/logic-gui.md:104 and messaging-seats.md:51 record the consumer-approved two-sentence clarification; #156 requires canonical-None→actual-Event production witness and old-None-rejection control before acceptance."
      },
      {
        "criterion": "A signed scoped draft with exact lines is delivered promptly for Oracle/client-owner review, followed by actual checks and gaps; no code or merge is claimed.",
        "reference": "Signed7cd80813f505e8b836aa6466cfed810556f3eea7 changes exactly the3 assigned files; record/signature/diff checks0 and make test-licenses0 on that exact source. Required make test-task-contract was last actually0 on9aec6da5; no cold rebuild/current focused or broad pass is claimed under recorded capacity shortage. Consumer request01M64K3BBQ7GYDXVZGBVF0ZS9E and source interpretation are preserved in native messages."
      }
    ],
    "exit_code": 0
  }
}
```
