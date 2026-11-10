---
type: task
title: 'Post-reap escape check: ESRCH means not escaped; other failures are diagnostics; only positive detection refuses (66459 A2), hand off INPUT71'
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/
- src/dream/read.rs
- src/dream/resolve.rs
- src/dream/c1.rs
- tests/formal/
- verification/
- reports/2026-11-09-dream-postreap-escape-check.md
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366459-dream-external-helper-shim-enforcement]]'
- '[[tasks/2026-11-09-dream-exec-shim-enforcement]]'
id: 01M70SMZY1ESE82R0XFF00H4Z7
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T19:32:30.582Z
created: 2026-11-09T18:38:10.747Z
number: 93377961366482
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M70SN0KF3TJEPW1CRZRB43GE
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Final signed HEAD, exact verification, landing and independent remote/clean-state match
    reference: target/dream-postreap-escape-witness/required-four.json; canonical-land.json; landed-receipt.json; landed-state.json; source-cut.json
  - criterion: ESRCH and other errno settle, diagnostics retain site/errno, positive live escape refuses, old-route mutant exits expected81
    reference: target/harness-landing/exec-shim-controls/run-wkj2r0iv/result.json; cleanup-esrch.ndjson; cleanup-eio.ndjson; census-esrch.ndjson; census-eio.ndjson; mutant.ndjson
  - criterion: Existing native controls and finite best-effort refinement
    reference: target/harness-landing/private-engine-controls-4i0fh7ov/run.json; verification/evidence/c630f883b9c8eecc8d623f35326ba6eb17910f4e5bba9519fa67a2958e81f008/evidence.json
  - criterion: ONE INPUT71, unchanged shim pins, exact field names/types and source/artifact binding
    reference: target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json; INPUT71-binding-audit.json; field-changes.json; preparation.log
  - criterion: Operational disk-floor breach retained, recovery/decoded raw evidence/owned cleanup recorded
    reference: target/dream-postreap-escape-witness/formal-storage-compaction.json; formal-dot-compression.json; post-compaction-formal-check.log; pre-land-debug-cleanup.json; cleanup-selective.json; preliminary-interrupted.json
report:
  summary: 'Landed signed head ad6c8b72fe0bb766164c879acd27c1ece2e565a7: shim retirement/post-reap escape ESRCH is non-gating, other unavailable reads are diagnostics, and positive same-incarnation live hits refuse. The exact four-command verification and make land exited 0; all 447 native/Java graphs match, with six escape controls and the expected-81 old-route mutant. ONE INPUT71 is /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json (SHA-256 f8d82d84b92557d346bc4942d398a21a841addd5769420d914d55c26519cccf1). Shim source/header/executable/allowlist/compiler pins are unchanged; runtime_approved=false; no builder/candidate run. Full report #93377961366487 is committed at checkpoint 62a25e5fadf63781c15d393996581db0fa093fe0. The report explicitly retains the earlier disk-floor breach (four-command minimum 5,194,911,744 bytes), subsequent lossless storage recovery, landing minimum 16,814,804,992 bytes, and cleanup of owned build outputs; worktree clean and final free space above 8 GiB. artifact_claim rejected the returned committed checkpoint artifact ID as artifact_not_committed; independent knowledge_fetch confirms publication_state=committed.'
  commit: ad6c8b72fe0bb766164c879acd27c1ece2e565a7
  reported_by: seat:01M70SN0KF3TJEPW1CRZRB43GE
  op_id: 01M70WQQ14GAGY0WGAQKF2E8BN
  verify_revision: 0
  report: '[[reports/2026-11-09-dream-postreap-escape-check.md]]'
provenance:
  digest: sha256:bce052ebf1a6c547dde98cab13fbe4611d0f1c113c7b4a32bae8ad3f293d5a51
  frontmatter_digest: sha256:fd9c5b0a2ae5e68f86973ed564825ed723c4735277c59072786fb726042e8d7f
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: HijRcLQnCJ/Q4PF9IkxDpR6v/Cr9KqvF8MtyVd/fjQ6ABjQnMIZHtUogfUKHcFXoYGqRMJM6ZS8W+OOziDHtCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151132
  event:
    id: 01M70WRFV6A126F63K28C0A1F4
    sequence: 151132
    hash: 24cc61b336c74e64ef14e03802c5b694209ca6c3ec2b44ef9c83c3492026b6bb
---
ROLE: EXECUTOR

## Evidence (builder run 01M70S5HNZYVKVYAE6SWSPJ1NE on frozen INPUT70 76045587; builder handoff sha256 59a6e21c...)
- The SHIM DESIGN WORKED: 16/16 shim receipts had all checks true at priority 4, including the verifier (pid 55601) with GS1/GS2 bound in-process. No shim refusal; settled 16/16; no quarantine. Debt 14.68 s, margin 12.8 s, ordinary_max 851 us.
- The positive still failed (81) on coverage_unknown "owned observation unavailable: escaped_cleanup_bsd".
- Path: engine_cleanup_escaped (proof-helper-probe.c ~4663-4668) -> engine_observe_bsd (~2849-2852) -> engine_observation_route (~3430-3450). After reap, a failed PROC_PIDTBSDINFO read (ESRCH) is routed to the same-incarnation terminal check, which cannot be satisfied post-reap, so it refuses.

## Ruling (decision 66459, Amendment 2: escape detection is best-effort; only a POSITIVE detection refuses)
At ENGINE_OBS_ESCAPED_CLEANUP_BSD (and any other POST-REAP escape/census site; inventory them):
- ESRCH / "no such process" = the incarnation is gone, so it did NOT escape. Treat it as cleanup-not-required (success), without the terminal-proof route.
- Any other read failure is recorded as a diagnostic (escape_check_unavailable with site and errno). It is NOT a refusal.
- REFUSE only on positive evidence: a live process with the SAME birth/start that is now outside the original pgid, or still alive after reap.
Pre-reap live sites keep their current observe_or_terminal behaviour.

## Controls
- Post-reap ESRCH at the cleanup site passes (settles).
- Another errno at the cleanup site passes, with the diagnostic recorded.
- A positive escaped live process (same birth, different pgid) refuses.
- A mutant that routes the cleanup ESRCH through the terminal-proof path reproduces this failure.
Update the finite model if it represents the cleanup check.

## Handoff
ONE INPUT71 tuple (same format as INPUT70, including the shim pins). The shim source should be unchanged; say so explicitly if it is. Report any field changes with exact names and types. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Run it once on the final head, then land with make land, report in reports/, and task_report once. One build tree; keep free disk above 8 GiB; clean your own build output after landing.


## Report from seat:01M70SN0KF3TJEPW1CRZRB43GE

Landed signed head ad6c8b72fe0bb766164c879acd27c1ece2e565a7: shim retirement/post-reap escape ESRCH is non-gating, other unavailable reads are diagnostics, and positive same-incarnation live hits refuse. The exact four-command verification and make land exited 0; all 447 native/Java graphs match, with six escape controls and the expected-81 old-route mutant. ONE INPUT71 is /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json (SHA-256 f8d82d84b92557d346bc4942d398a21a841addd5769420d914d55c26519cccf1). Shim source/header/executable/allowlist/compiler pins are unchanged; runtime_approved=false; no builder/candidate run. Full report #93377961366487 is committed at checkpoint 62a25e5fadf63781c15d393996581db0fa093fe0. The report explicitly retains the earlier disk-floor breach (four-command minimum 5,194,911,744 bytes), subsequent lossless storage recovery, landing minimum 16,814,804,992 bytes, and cleanup of owned build outputs; worktree clean and final free space above 8 GiB. artifact_claim rejected the returned committed checkpoint artifact ID as artifact_not_committed; independent knowledge_fetch confirms publication_state=committed.

```json
{
  "op_id": "01M70WQQ14GAGY0WGAQKF2E8BN",
  "commit": "ad6c8b72fe0bb766164c879acd27c1ece2e565a7",
  "report": "[[reports/2026-11-09-dream-postreap-escape-check.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Final signed HEAD, exact verification, landing and independent remote/clean-state match",
        "reference": "target/dream-postreap-escape-witness/required-four.json; canonical-land.json; landed-receipt.json; landed-state.json; source-cut.json"
      },
      {
        "criterion": "ESRCH and other errno settle, diagnostics retain site/errno, positive live escape refuses, old-route mutant exits expected81",
        "reference": "target/harness-landing/exec-shim-controls/run-wkj2r0iv/result.json; cleanup-esrch.ndjson; cleanup-eio.ndjson; census-esrch.ndjson; census-eio.ndjson; mutant.ndjson"
      },
      {
        "criterion": "Existing native controls and finite best-effort refinement",
        "reference": "target/harness-landing/private-engine-controls-4i0fh7ov/run.json; verification/evidence/c630f883b9c8eecc8d623f35326ba6eb17910f4e5bba9519fa67a2958e81f008/evidence.json"
      },
      {
        "criterion": "ONE INPUT71, unchanged shim pins, exact field names/types and source/artifact binding",
        "reference": "target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json; INPUT71-binding-audit.json; field-changes.json; preparation.log"
      },
      {
        "criterion": "Operational disk-floor breach retained, recovery/decoded raw evidence/owned cleanup recorded",
        "reference": "target/dream-postreap-escape-witness/formal-storage-compaction.json; formal-dot-compression.json; post-compaction-formal-check.log; pre-land-debug-cleanup.json; cleanup-selective.json; preliminary-interrupted.json"
      }
    ]
  }
}
```
