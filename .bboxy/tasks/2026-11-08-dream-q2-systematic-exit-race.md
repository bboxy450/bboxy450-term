---
allowed_paths:
- tests/fixtures/dream-read/proof-helper-probe.c
- tests/fixtures/dream-read/private-engine-controls.c
- tests/fixtures/dream-read/check-private-engine-controls.py
- tests/fixtures/dream-read/prepare-c-probe.py
- tests/fixtures/dream-read/check-c-preparation.py
- src/dream/read.rs
- src/dream/resolve.rs
- src/dream/c1.rs
- tests/formal/dream_read.rs
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-08-dream-systematic-exit-race.md
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  kind: seat
  process_generation: '1'
  public_key: qxL/3a47UEqef+6VNGve7zFPxAJ4V2xtFwav0ldZ+l0=
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
closed:
  by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  op_id: 01M85CFWW6WFG0KX0VXZR1BVWK
created: 2026-11-09T07:58:49.961Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[tasks/2026-11-08-dream-q2-pause-stop-exit-race]]'
id: 01M6ZN2A59M7P10SC59GW0F6F6
number: 93377961366445
report:
  commit: 637cb234d93d08d0c8fcfb820b7e48f9538bc0e1
  op_id: 01M703GR7CAEH3719N86P89W33
  report: '[[reports/2026-11-08-dream-systematic-exit-race.md]]'
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  summary: Landed signed 637cb234d93d08d0c8fcfb820b7e48f9538bc0e1 with shared domain-specific observe_or_terminal handling, complete callsite inventory, 27 per-site supplied-error/real-original-settlement controls and bypass mutants, and a standalone formal companion. The exact required four-command chain and canonical make land both exited 0; all 446 native/pinned-Java graphs and named verdicts matched. ONE post-land INPUT69 was compiled/audited and sent (83682788…8511; runtime_approved=false, no builder run); owned build cleanup retained proofs/tools/inputs and left 30,160,269,312 bytes available. Full report committed as 626e0a130f60e77002136b41e0143cebae4c37cc.
  verify_revision: 0
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Q2: systematic exit-race handling for EVERY live helper observation syscall (shared observe_or_terminal + per-site fault injection), hand off INPUT69'
type: task
updated: 2026-11-23T23:40:06.583Z
updated_by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  evidence:
  - criterion: Every live observation callsite inventoried
    reference: reports/2026-11-08-dream-systematic-exit-race.md — inline 107 native/context,81 logical,67 route bindings and27 sites; inventory-v5 SHA7ba5e927d0d26bab3d4fd57428da71231eebf39191926f70f73ce9b3fdc4a4a1
  - criterion: Shared terminal-only handling preserves independent identity, raw domains, ownwait4 and full bill
    reference: 637cb234:tests/fixtures/dream-read/proof-helper-probe.c; SHA d1ec07415d4b8dcf953941f8d7f222677baf9a0d766c9fe33fb50869ea30f151
  - criterion: Per-site fault/bypass controls and unchanged witnesses
    reference: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical-C-input69-v1/run.json SHA2ab1bef2f6145e6f5ad3d068f41dd7f0af27c4daa9ceeb48b78b486f02f37921;288 actual commands/110compiles/108mutants/70baseline invocations
  - criterion: Exact new internal ledger names/types and no public terminal fields
    reference: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT69-source-freeze-tuple-v1.json — obs_terminal_site:string fixed27 enum; obs_terminal_errno:i32 raw API-domain code; no terminal/public field delta
  - criterion: Finite native/Java full graph and verdict parity
    reference: verification/evidence/1b03b642e291e62d7e776c6226d8f672ef6639cf7fc27e2338d1375f12bf6233 —446models/96contracts/1crate,3386547states/11375760labelled edges;4462retained rawfiles
  - criterion: Exact verification and canonical landing actual0 on unchanged signed HEAD
    reference: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/systematic-exit-exact-four-chain-final-v1.json logSHA907a5e6b43b5537e7e3ef28890aca2a7d20976513575e4ff17e44757f3af634a; canonical-land-input69-v1.json logSHA84745bdfdddc2fc1a3fe3092c8ebfc1b363e071a57bd842efe2c3a38f28e01e6
  - criterion: ONE post-land INPUT69 and owned cleanup
    reference: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT69-source-freeze-tuple-v1.json SHA8368278817ac5fc74da15e61d0f8ae069d8b9767849ee8ef4cac0cecd86f8511; native message/op01M702WV4KPHZ3DF0D0VCKWM2P; cleanup f8c4ee50a9106000785b477bbb0a20ddf716a679616e99873110920da99511af
  exit_code: 0
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
provenance:
  digest: sha256:876efbb90d7e060e5147bfd34b80df82afb23dc928d5e89e35fdecd9f0c7e52c
  frontmatter_digest: sha256:c150b260ae1866557504444e098617619f9017a878c5457a7f1a331368f678f6
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  sig: 2H71pQ3L6KjUTHyfYBa0SGVLwNETqiscYBxccmRtnp+ihc9XciAdlJ9mvZfhgyb9hsXsfMiNSfQqfXmXB0BcBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1438
  event:
    id: 01M85CFXNK2CZXKAEFZKJKKJ2W
    sequence: 1438
    hash: 005d84f1a8f4f32a60b63d1fba3020eb3d189f4acaefd1b35f0b5e92c5961f03
---
ROLE: EXECUTOR

## Evidence (builder run 01M6ZMJP97QRM0YS85A806NJZW on frozen INPUT68 a68d2002; handoff sha256 76f0be1d...)
- REGRESSION: the positive passed on INPUT66 and INPUT67 and now fails. The controller exited 81 (status 20736): coverage_unknown "current helper audit identity unavailable" at frozen C ~3076 (known_control_token refresh) for seq 10 / PID 6458. Separately, the live BSD identity predicate (~3180) failed before group_kill.
- seq 5 / PID 6305: a TASKALLINFO ESRCH was correctly recovered (own wait4, group gone), through the INPUT61 fix.
- Final terminal: 10/10 settled; cost and persistence passed. Debt 10.13 s, margin 18.4 s. No live quarantine.
- Pattern across INPUT61/62/63/67/68: every new run hits an exit race at the NEXT live-observation syscall (TASKALLINFO, then child census, then RI exit, then the group SIGSTOP, now the audit token and BSD identity). Fixing one site per cycle does not converge.

## Required: ONE systematic cut, not another single-site fix
1. Inventory EVERY syscall that observes an owned helper member while it may be alive:
   - proc_pidinfo (TASKALLINFO, TBSDINFO);
   - proc_pid_rusage;
   - the audit token (task_name_for_pid / proc_pidinfo audit token / known_control_token);
   - census, kill(-pgid, sig), kqueue registration;
   - any others in proof-helper-probe.c.
   List each call site with its file:line in the report.
2. Route EVERY such failure through ONE shared helper, `observe_or_terminal()`:
   - on ESRCH (or the call's equivalent "process gone" error, for example a TASK_NAME failure for an exited task), when the member is owned, unreaped and identity-complete, run the existing same-incarnation terminal check: RI exit, knote NOTE_EXIT, or SZOMB with the same birth/start/token;
   - if that check passes, take the terminal path (settled by own wait4 later, full bill);
   - otherwise, and for any other errno, keep coverage_unknown with a DISTINCT detail naming the site.
   Never infer a stop or an exit from a sent signal; never signal after own reap.
3. Fault injection: a source-known control that forces the "process exited just before this call" outcome at EACH inventoried site in turn, and asserts each one settles through the terminal path, with full cost and own wait4. Add a matrix entry per site, and a mutant per site that bypasses the shared helper and must refuse.
4. Serialize the observation site name and errno for any terminal-path takeover (ledger field obs_terminal_site, a string from a fixed enum, plus obs_terminal_errno i32). Report the exact names.
5. Unchanged: caps, 9x, 50 ms, the stopped rules, 66331, 30 s and every existing witness.

## Handoff
ONE INPUT69 tuple (same format). runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Run it once on the final head, then land with make land, report in reports/, and task_report once. Disk: one build tree; keep free disk above 8 GiB.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed 637cb234d93d08d0c8fcfb820b7e48f9538bc0e1 with shared domain-specific observe_or_terminal handling, complete callsite inventory, 27 per-site supplied-error/real-original-settlement controls and bypass mutants, and a standalone formal companion. The exact required four-command chain and canonical make land both exited 0; all 446 native/pinned-Java graphs and named verdicts matched. ONE post-land INPUT69 was compiled/audited and sent (83682788…8511; runtime_approved=false, no builder run); owned build cleanup retained proofs/tools/inputs and left 30,160,269,312 bytes available. Full report committed as 626e0a130f60e77002136b41e0143cebae4c37cc.

```json
{
  "op_id": "01M703GR7CAEH3719N86P89W33",
  "commit": "637cb234d93d08d0c8fcfb820b7e48f9538bc0e1",
  "report": "[[reports/2026-11-08-dream-systematic-exit-race.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Every live observation callsite inventoried",
        "reference": "reports/2026-11-08-dream-systematic-exit-race.md — inline 107 native/context,81 logical,67 route bindings and27 sites; inventory-v5 SHA7ba5e927d0d26bab3d4fd57428da71231eebf39191926f70f73ce9b3fdc4a4a1"
      },
      {
        "criterion": "Shared terminal-only handling preserves independent identity, raw domains, ownwait4 and full bill",
        "reference": "637cb234:tests/fixtures/dream-read/proof-helper-probe.c; SHA d1ec07415d4b8dcf953941f8d7f222677baf9a0d766c9fe33fb50869ea30f151"
      },
      {
        "criterion": "Per-site fault/bypass controls and unchanged witnesses",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical-C-input69-v1/run.json SHA2ab1bef2f6145e6f5ad3d068f41dd7f0af27c4daa9ceeb48b78b486f02f37921;288 actual commands/110compiles/108mutants/70baseline invocations"
      },
      {
        "criterion": "Exact new internal ledger names/types and no public terminal fields",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT69-source-freeze-tuple-v1.json — obs_terminal_site:string fixed27 enum; obs_terminal_errno:i32 raw API-domain code; no terminal/public field delta"
      },
      {
        "criterion": "Finite native/Java full graph and verdict parity",
        "reference": "verification/evidence/1b03b642e291e62d7e776c6226d8f672ef6639cf7fc27e2338d1375f12bf6233 —446models/96contracts/1crate,3386547states/11375760labelled edges;4462retained rawfiles"
      },
      {
        "criterion": "Exact verification and canonical landing actual0 on unchanged signed HEAD",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/systematic-exit-exact-four-chain-final-v1.json logSHA907a5e6b43b5537e7e3ef28890aca2a7d20976513575e4ff17e44757f3af634a; canonical-land-input69-v1.json logSHA84745bdfdddc2fc1a3fe3092c8ebfc1b363e071a57bd842efe2c3a38f28e01e6"
      },
      {
        "criterion": "ONE post-land INPUT69 and owned cleanup",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT69-source-freeze-tuple-v1.json SHA8368278817ac5fc74da15e61d0f8ae069d8b9767849ee8ef4cac0cecd86f8511; native message/op01M702WV4KPHZ3DF0D0VCKWM2P; cleanup f8c4ee50a9106000785b477bbb0a20ddf716a679616e99873110920da99511af"
      }
    ]
  }
}
```
