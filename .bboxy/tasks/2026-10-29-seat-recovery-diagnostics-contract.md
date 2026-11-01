---
acceptance:
- Original-operation recovery is specified under authenticated actor/root visibility, including restart/generation changes, terminal/unknown/unavailable grades and no effect replay.
- Seat error projection specifies grants, fields, redaction, pagination/cursor age, bounds, unavailable-store and operator read-mark independence.
- Existing operator-only ctl authority and no-gated-communications invariants remain intact; both sides and the reason for boundary changes are recorded.
- Executable verification requirements include lost-response exactly-one artifact, restart, unauthorized reads, unavailable ledger, pagination and secret sentinels.
- A signed scoped spec commit and actual verification output are reported; unresolved decisions are explicit and code/merge/lifecycle changes are not performed.
allowed_paths:
- .bboxy/specs/w2-5-mcp-and-seat-wiring.md
- .bboxy/specs/mcp-control-plane.md
- .bboxy/interfaces/interfaces.md
- .bboxy/invariants/ctl-never-prints-secrets.md
- .bboxy/runs/seat-recovery-diagnostics-contract-20261029.json
artifacts:
- '[[01M64F0MZ4292AAF60FPR5EPWH]]'
assignee: seat:01M649QP6AMVQWCG28X5MQP3JR
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:20:11.107Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64E84YWK22NGEM5NP2JMCJE
number: 381
provenance:
  digest: sha256:1f2f90f306c18e8d8a89adbea8afec780979d0904e9b8aa4cc0f2a5813e0af77
  frontmatter_digest: sha256:79d28f657147c3d40173b51b46943308d27c16ec071d51990bda5c1b86a2c09f
  log: .bboxy/log/2026-10-29.jsonl#1948
  sig: UooOV0adLpxwc/REakAA4KFyv8hgM/hoKXu+5ZhD2Nt88xufLyUJJGf0Di6O6YiVXxldQ/rT2ft7ZBTwO8J0Bw==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
rejection:
  by: seat:01M649MS05FAD5CY9CS0Q07H76
  reason: 'Draft reviewed against actual signed source. Two required contract corrections sent01M64F0Z5T1E449HZS2PTA9CEA were not incorporated: operation_get''s foreign versus absent/unprovable target must produce one indistinguishable response (the common forbidden rule and outcome.not_found table currently disagree), and diagnostic root/swarm authorization must require host-admitted producer provenance, not arbitrary plaintext context fields, with forged-context negatives and explicit legacy withholding. Amend only assigned paths, preserve IDs, keep REC/ERR/SEC marked future assertions. Broad gate2/101 capacity failure is truthful, no retry unchanged or threshold weakening. Before another broad run consume already-delegated urgent tasks388/394 and return their minimal signed ICD/budget drafts; their required focused verification and later exact-source gate remain unchanged. Do not let this draft serialise their independent law edits.'
report:
  commit: 9aec6da534424caa33180bd52759e345d4b31f8d
  op_id: 01M64J3VF76NT674HD8V29BN2Z
  reported_by: seat:01M649QP6AMVQWCG28X5MQP3JR
  summary: |-
    Revised both rejected #381 points in signed545f3c84/6b0d9652: operation_get returns one caller-namespace not_found/unknown/absent/no_record/null projection for absent, foreign or unprovable targets, including legacy, with no foreign-store probe or no-effect claim. Diagnostic visibility requires a mux-host-admitted association binding diagnostic ID/redacted payload digest to actual SeatActor/swarm/root/original operation; raw datagram/PID/op/context fields grant nothing. Legacy/unbound/forged/copied context withheld; unavailable provenance explicit; REC/ERR negatives specify both corrections. Existing own-seat/root read seam, current-grant checks and generation continuity retain mutation replay equality. These remain specs, not implemented capabilities or passing future assertions; consumer re-review of corrections/implementation remains open.
    Final signed HEAD9aec6da534424caa33180bd52759e345d4b31f8d clean; make test-task-contract0 (1 passed/0 failed), log SHA25641fedc64e8057490d0067e31ae300cda51a0c65efbbb17a53c5adc91a1998b84. Nine original Markdown frontmatters/IDs/numbers/check fields/links preserved except updated; added links resolve; git diff/signature checks0. Original7f6a7a7b broad make2/Cargo101 capacity failure retained. Later broad started4ddfa288 also make2/Cargo101, sole failed recovery guard free10539065344 <10737418240, and became superseded when law edits landed under Oracle sequencing authorization; target/issue73-ci-local.log SHA256416816a9fb7c560b336cf7a6209867be612c985f496bdcd4be6f3996c63b13df. No current-HEAD broad pass. Retired only inactive owned debug after lsof no-holder observation, preserving logs/app/focused/recovery binaries under target/retained-law-evidence. Current free15987892224 <cold18GiB allocation; no unchanged third full build. No product code, task-file lifecycle edit, runtime/credentials/peer cleanup/merge/push/recruitment or issue closure. Previous report op01M64J160HG5GHKB70E35SD820 was explicit no-effect workspace_pending refusal; this is a fresh report after task_status resolved.
role: architect
status: accepted
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: Baseline original-operation recovery and authorized seat diagnostics contracts (issues74/75)
type: task
updated: 2026-10-29T20:38:22.377Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
verify: make test-task-contract
verify_result:
  command: make test-task-contract
  evidence:
  - criterion: Original-operation recovery is specified under authenticated actor/root visibility, including restart/generation changes, terminal/unknown/unavailable grades and no effect replay.
    reference: .bboxy/specs/mcp-control-plane.md:349 and :367 — uniform caller-namespace not_found/null payload for absent, foreign and unprovable targets including legacy; no foreign probe/no-effect inference; generation continuity does not relax mutation replay.
  - criterion: Seat error projection specifies grants, fields, redaction, pagination/cursor age, bounds, unavailable-store and operator read-mark independence.
    reference: .bboxy/specs/mcp-control-plane.md:443 and :454 — scoped errors_read with mux-host-admitted ID/payload association, not raw context/PID/op fields; legacy/forged/copied context withholding, unavailable provenance, bounded cursor/read-mark rules.
  - criterion: Existing operator-only ctl authority and no-gated-communications invariants remain intact; both sides and the reason for boundary changes are recorded.
    reference: .bboxy/specs/w2-5-mcp-and-seat-wiring.md:160 and :192; interfaces/interfaces.md:31 — named existing boundary/owners/reason and corrections preserve operator-only ctl, no new authority and independent communication.
  - criterion: Executable verification requirements include lost-response exactly-one artifact, restart, unauthorized reads, unavailable ledger, pagination and secret sentinels.
    reference: .bboxy/specs/mcp-control-plane.md:557 — REC/ERR/SEC matrix specifies lost-response exactly-one artifact, restart, indistinguishable target lookup, forged host provenance, unavailable stores, pagination and secret sentinels; future assertions remain unproven.
  - criterion: A signed scoped spec commit and actual verification output are reported; unresolved decisions are explicit and code/merge/lifecycle changes are not performed.
    reference: .bboxy/runs/seat-recovery-diagnostics-contract-20261029.json:1 — actual source/check/hash/capacity evidence and review corrections. Final signed9aec6da5 clean with focused0; no current broad pass or implemented capability is claimed.
  exit_code: 0
---
# Original-operation recovery and seat diagnostics

ROLE: ARCHITECT — Nyx Copperwing. Delegator Vesper Greywind. Worktree /Users/feral/mydev/bboxy450-term-mcp-recovery-spec-20261029, branch architect-mcp-recovery-spec-20261029, base014e555c. The operator authorized the issue-fixing cohort and zero-open-issues goal. This is a native delegated specification task, not a DM work instruction.

Read issues74/75 and the actual current source, existing MCP/knowledge/ctl and interface records. Your earlier advice is now the starting question, not an already baselined ICD. Specify the smallest additive read-only interfaces on the existing seat MCP boundary; do not invent a parallel control plane. Existing tool catalogue has no original-operation reader or authorized error reader. Operator ctl remains operator-only. Establish both boundary owners, reasons, exact request/result shapes, visibility, timing/age/bounds, lifecycle/restart behavior, refusal/unknown grades, backward compatibility, and the executable evidence needed on both sides. The Oracle represents the seat-client consumer and will review your committed draft immediately; Rook will receive the implementation assignment after that review. Publish the draft location/hash promptly, then finish checks; no approval queue for ordinary communication.

Important new evidence: the shared existing-record index_error and task-write refusal were traced to ten unnumbered upstream Markdown files in ignored .bboxy/runs/clones. The raw clones are archived intact outside the indexed KB, with the original path preserved as a symlink; native #66 read and fresh task creation now work. This environmental repair does not provide operation recovery, prove original unknown outcomes or expose diagnostics. Original task01M649WAN2X9R965P8YSNJJQCX, artifact01M64B194P9N854VVHRRGXX65A and draft-task01M64DN5VHVZ3BNG1N34DKW7MN remain non-replayed recovery cases. The new reader must distinguish a missing artifact/path from authoritative no-effect evidence.

Allowed files are frontmatter. Amend the existing specs/interface matrix, preserving IDs/numbers and recording the current date/reason. No code, task lifecycle edits, runtime reset, authority expansion, installed-app changes, credentials, merge, push or new recruits. Keep docs in your assigned worktree. Keep raw logs outside Git. Record command/exit/hash summaries only under runs.

Verification: make test-task-contract plus the records checks needed to prove existing frontmatter and links are intact. Before final handoff make test must pass on exact signed source, using own target/jobs2 and no operator. If capacity prevents a broad gate, retain the real result and continue the design review/briefing work rather than parking communication. Send the scoped signed draft commit and file:line mechanisms early for Oracle review, then task_report with actual exits and evidence per acceptance. Do not claim implemented capabilities or close the GitHub issues.

## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Committed draft contracts and verification matrix in the assigned worktree/branch; final signed HEAD 6bf202a8ef8bb60a2e00ee6b5a0f0c43d595dc2b. Contract source is unchanged from the early signed draft 568fcc4e (primary file SHA256 42ed6c7f7a6e7582f3950cb848a5bcba7752c869eb202afe6db937c7c4ed1258). Only five allowed paths changed: MCP control spec, wiring spec, interface matrix, secret invariant extension and JSON command/exit/hash summary. Existing IDs/numbers/type/check/frontmatter links preserved, with updated date; existing wiki targets retained and newly added targets resolve. git diff --check and git verify-commit exit 0; clean worktree. make test-task-contract on final HEAD exits 0, exactly 1 passed/0 failed; target/task-contract-handoff.log SHA256 e37fa6be3270628ca73f04e79f95dcff65b76837d4d267c38c624e9359c54747.

Broad make test ran on exact signed 7f6a7a7bd9271ba02647d3918c351d780cf7eade and exited 2 (Cargo 101). Workspace build and strict Clippy passed, but checkpoint_bounded_recovery aborted at crates/bbt-mux/tests/recovery.rs:329: free disk 10676948992 below 10 GiB. This is the sole failed workspace test. Afterward free space remained 10428542976 bytes, below 10737418240 required. Post-test smoke/invariants were not reached. No retry under unchanged capacity, guard weakening or peer cleanup. The final commit only records that real failure in .bboxy/runs/seat-recovery-diagnostics-contract-20261029.json; no broad pass on final HEAD is claimed. Raw log target/ci-local.log SHA256 ba13fe8c1963601ff5deeaee53ca78d699493e413efb822bf239868691bac3db; make stdout SHA256 58a95438cbff545a99513092fadc7115af764f79c250fb05737bd336d909f4e4. Using the brief's explicit capacity exception, design review/briefing proceeds.

Draft was sent promptly and Oracle application-acknowledged it. Consumer review disposition/baselining remains open. The tools and future REC/ERR/SEC assertions are specified, not implemented or proven; original unknown outcomes remain unresolved and un-replayed. No code, task-file lifecycle edits, runtime reset, authority expansion, installed-app change, credentials, merge/push, new recruit or GitHub issue closure. Local artifact responsibility claim 01M64F0MZ4292AAF60FPR5EPWH links the primary contract and digest to this task; grade reported, not verified.

```json
{
  "commit": "6bf202a8ef8bb60a2e00ee6b5a0f0c43d595dc2b",
  "op_id": "01M64GKQ0BQJR3BJKYQ98Z6VMY",
  "verify_result": {
    "command": "make test-task-contract",
    "evidence": [
      {
        "criterion": "Original-operation recovery is specified under authenticated actor/root visibility, including restart/generation changes, terminal/unknown/unavailable grades and no effect replay.",
        "reference": ".bboxy/specs/mcp-control-plane.md:364 — operation_get(target_op_id,op_id?), own retained actor/root scope, same-seat read continuity without mutation replay, outcome/effect/basis table and authoritative absence limits."
      },
      {
        "criterion": "Seat error projection specifies grants, fields, redaction, pagination/cursor age, bounds, unavailable-store and operator read-mark independence.",
        "reference": ".bboxy/specs/mcp-control-plane.md:434 — errors_read closed version-1 fields, trusted root/swarm/seat provenance, 60-second bound cursors, retention/oversize/unavailable grades and acknowledged:false; :498 shared byte/scan/time allocations."
      },
      {
        "criterion": "Existing operator-only ctl authority and no-gated-communications invariants remain intact; both sides and the reason for boundary changes are recorded.",
        "reference": ".bboxy/specs/mcp-control-plane.md:310 — provider Rook/contract Nyx and consumer Vesper, reason and review state; .bboxy/specs/w2-5-mcp-and-seat-wiring.md:160 and .bboxy/interfaces/interfaces.md:31 — existing boundary only, no ctl.command expansion, messaging continues independently."
      },
      {
        "criterion": "Executable verification requirements include lost-response exactly-one artifact, restart, unauthorized reads, unavailable ledger, pagination and secret sentinels.",
        "reference": ".bboxy/specs/mcp-control-plane.md:525 — REC-1..3, ERR-1..2, SEC-1 map both client bridge and provider assertions to existing Make targets; .bboxy/invariants/ctl-never-prints-secrets.md:46 explicitly distinguishes current operator checks from pending seat assertions."
      },
      {
        "criterion": "A signed scoped spec commit and actual verification output are reported; unresolved decisions are explicit and code/merge/lifecycle changes are not performed.",
        "reference": ".bboxy/runs/seat-recovery-diagnostics-contract-20261029.json:1 — signed source, preserved-record hashes, actual command exits and broad disk-floor failure; final 6bf202a8 signature verified, clean tree, final make test-task-contract exit 0. Broad make test exit 2 retained under explicit capacity exception; consumer baseline/implementation remain open."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Revised both rejected #381 points in signed545f3c84/6b0d9652: operation_get returns one caller-namespace not_found/unknown/absent/no_record/null projection for absent, foreign or unprovable targets, including legacy, with no foreign-store probe or no-effect claim. Diagnostic visibility requires a mux-host-admitted association binding diagnostic ID/redacted payload digest to actual SeatActor/swarm/root/original operation; raw datagram/PID/op/context fields grant nothing. Legacy/unbound/forged/copied context withheld; unavailable provenance explicit; REC/ERR negatives specify both corrections. Existing own-seat/root read seam, current-grant checks and generation continuity retain mutation replay equality. These remain specs, not implemented capabilities or passing future assertions; consumer re-review of corrections/implementation remains open.
Final signed HEAD9aec6da534424caa33180bd52759e345d4b31f8d clean; make test-task-contract0 (1 passed/0 failed), log SHA25641fedc64e8057490d0067e31ae300cda51a0c65efbbb17a53c5adc91a1998b84. Nine original Markdown frontmatters/IDs/numbers/check fields/links preserved except updated; added links resolve; git diff/signature checks0. Original7f6a7a7b broad make2/Cargo101 capacity failure retained. Later broad started4ddfa288 also make2/Cargo101, sole failed recovery guard free10539065344 <10737418240, and became superseded when law edits landed under Oracle sequencing authorization; target/issue73-ci-local.log SHA256416816a9fb7c560b336cf7a6209867be612c985f496bdcd4be6f3996c63b13df. No current-HEAD broad pass. Retired only inactive owned debug after lsof no-holder observation, preserving logs/app/focused/recovery binaries under target/retained-law-evidence. Current free15987892224 <cold18GiB allocation; no unchanged third full build. No product code, task-file lifecycle edit, runtime/credentials/peer cleanup/merge/push/recruitment or issue closure. Previous report op01M64J160HG5GHKB70E35SD820 was explicit no-effect workspace_pending refusal; this is a fresh report after task_status resolved.

```json
{
  "commit": "9aec6da534424caa33180bd52759e345d4b31f8d",
  "op_id": "01M64J3VF76NT674HD8V29BN2Z",
  "verify_result": {
    "command": "make test-task-contract",
    "evidence": [
      {
        "criterion": "Original-operation recovery is specified under authenticated actor/root visibility, including restart/generation changes, terminal/unknown/unavailable grades and no effect replay.",
        "reference": ".bboxy/specs/mcp-control-plane.md:349 and :367 — uniform caller-namespace not_found/null payload for absent, foreign and unprovable targets including legacy; no foreign probe/no-effect inference; generation continuity does not relax mutation replay."
      },
      {
        "criterion": "Seat error projection specifies grants, fields, redaction, pagination/cursor age, bounds, unavailable-store and operator read-mark independence.",
        "reference": ".bboxy/specs/mcp-control-plane.md:443 and :454 — scoped errors_read with mux-host-admitted ID/payload association, not raw context/PID/op fields; legacy/forged/copied context withholding, unavailable provenance, bounded cursor/read-mark rules."
      },
      {
        "criterion": "Existing operator-only ctl authority and no-gated-communications invariants remain intact; both sides and the reason for boundary changes are recorded.",
        "reference": ".bboxy/specs/w2-5-mcp-and-seat-wiring.md:160 and :192; interfaces/interfaces.md:31 — named existing boundary/owners/reason and corrections preserve operator-only ctl, no new authority and independent communication."
      },
      {
        "criterion": "Executable verification requirements include lost-response exactly-one artifact, restart, unauthorized reads, unavailable ledger, pagination and secret sentinels.",
        "reference": ".bboxy/specs/mcp-control-plane.md:557 — REC/ERR/SEC matrix specifies lost-response exactly-one artifact, restart, indistinguishable target lookup, forged host provenance, unavailable stores, pagination and secret sentinels; future assertions remain unproven."
      },
      {
        "criterion": "A signed scoped spec commit and actual verification output are reported; unresolved decisions are explicit and code/merge/lifecycle changes are not performed.",
        "reference": ".bboxy/runs/seat-recovery-diagnostics-contract-20261029.json:1 — actual source/check/hash/capacity evidence and review corrections. Final signed9aec6da5 clean with focused0; no current broad pass or implemented capability is claimed."
      }
    ],
    "exit_code": 0
  }
}
```
