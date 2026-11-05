---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:56:39.938Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GAYFKFT6TKS8DE7Z8YXYR
links:
- rel: context
  to: '[[vision]]'
number: 433
provenance:
  digest: sha256:5974ddf1065975151f5a7ff1b4e0ece74e019d9ced00df7160241a9b29b27d52
  frontmatter_digest: sha256:7815bc75b2ce596b7a4294bf8f411a18ce22554eead5f776b666f12a8caa01c2
  log: .bboxy/log/2026-10-29.jsonl#1356
  sig: rqcIBHvs52KxG/Om09hx8rhUp3Bn+qgfVZcexzBEyI7A6xoGjf/eGc469CqnTGxrLCQuKoDhOTTpj+vnSCo7CQ==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: 'Journey: addressable records and trusted work'
type: note
updated: 2026-10-29T18:56:39.938Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
<!-- bboxy450-term: https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Addressable records and trusted work

Back to [[implementation-journey]]. Reviewed against main `014e555c` on 2026-10-29. These GitHub items remain open; a script that does not exist or a partial pass is an explicit gap.

| Issue / next task | What is already present | Remaining requirement and verification |
|---|---|---|
| [#1 · N1](https://github.com/bboxy450/bboxy450-term/issues/1) | Global numbers, allocator/refusal corpus checks; [[d48-every-record-has-a-number]] | Add canonical N1 night: app creates next free number; duplicate/invalid records are refused and visibly broken. N1.sh is absent. Existing corpus/unit checks alone do not close it. |
| [#2 · N2](https://github.com/bboxy450/bboxy450-term/issues/2) | Individual decision nodes and typed links | Add N2 night proving actual Graph decision node and all five edge types. N2.sh is absent. Boundary: [[logic-gui]]. |
| [#3 · N3](https://github.com/bboxy450/bboxy450-term/issues/3) | Digest-addressed records and comment/RFC model | Add N3 night proving version-pinned comment placement and proposal → comment → decision → spec in the app. N3.sh is absent. |
| [#4 · N4](https://github.com/bboxy450/bboxy450-term/issues/4) | Execution/task/Markdown graph projections | Add N4 night proving actual agent role/now/blocked, work tree, provenance and subscriptions. N4.sh is absent. Historical graph.md is not a live witness. |
| [#5 · I1](https://github.com/bboxy450/bboxy450-term/issues/5) | Identity/custody facade and unattended automation policy | Add I1 using an isolated native-custody test backend: one enrollment/prompt request, none on restart. I1.sh is absent. Never require the operator Keychain. |
| [#6 · I2](https://github.com/bboxy450/bboxy450-term/issues/6) | Generation keys and signed envelope validation | Add I2 night: device certification, envelope verification and nil-seat/non-operator refusal. I2.sh is absent. |
| [#7 · I3](https://github.com/bboxy450/bboxy450-term/issues/7) | Local-dev issuer and receiver policy | Add I3 night: offline operation is labelled in every visible projection; pinned receiver refuses local-dev. I3.sh is absent. |
| [#8 · I4](https://github.com/bboxy450/bboxy450-term/issues/8) | Signed record writer and signed Git workflow; existing make test-signatures | Add I4 night joining app-written device signer to record evidence and good main signatures. I4.sh is absent; testing Git signatures alone is a subset. |
| [#43 · historical task rollout](https://github.com/bboxy450/bboxy450-term/issues/43) | Authenticated reconcile/import implementation in `018808d3`/`b9a51664`; private 250-record proof | Apply the audited dispositions through authorized task tools and retain the source archive/mapping. Current old task files #277/#337/#338/#339/#342 still say open; task_status on all five returned task swarm is ambiguous. Do not hand-edit lifecycle fields or reseed tested promises. make test-task-migration verifies the private mechanism, not live rollout. Keep #40 separate. |
| [#74 · inspect Knowledge operations](https://github.com/bboxy450/bboxy450-term/issues/74) | Search works; write and read boundaries preserve explicit error grades | Classify existing-record index_error and unresolved write `01M649WAN2X9R965P8YSNJJQCX`; baseline actor/root-scoped operation recovery and prove exactly one effect after lost response/restart. Extend make test-workspace-boundary, make test-task-contract and make test-scoped-workspaces. |

Source/evidence: [work-items](../specs/work-items.md), [identity boundary](../interfaces/identity-app.md), [task migration proof](../runs/night-task-migration.json), [record corpus gate](../../crates/bbt-doc/tests/corpus.rs), [diagnostic triage](diagnostic-triage.md).

The private migration proof independently audits 250 source records, 41 seeds and 251 published projections. Its own record explicitly says live application was not attempted. Historical accounting completion does not prove the #40 process-start cause. The source task statuses and authenticated scope refusal are retained as separate facts.

Next assignment shape: ROLE: EXECUTOR, one issue/acceptance lane per private worktree, exact files from the relevant ICD, canonical Make workflow and no operator input. Missing N/I wrapper work must finish the full promise through the existing night runner; do not create a competing acceptance system.



Admission note, 2026-10-29: this dated triage was first prepared in signed private commits de793c11b7dd3bfd17ac453d8c47c98dc91adbae and 0efe0d917de225fa702667dc646b7981ae7edecc. The authenticated host now assigns this local copy a fresh identity and global number; the private draft numbers are not imported as current authority. Follow [[2026-10-29-zero-issues-cohort]] for subsequent assignments and verified dispositions. Unresolved promises remain unresolved.
