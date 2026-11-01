---
acceptance:
- Reserved internal import staging does not emit artifact observations or consume record numbers; ordinary source/projection artifact observation remains intact.
- An unattended real-owner/observer import of the pinned 250-record bundle completes without observer self-feedback; concurrent legitimate record creation yields bounded truthful retry or correct publication without loss.
- A causal mutant restoring staging observation fails the new regression; source bytes, audited unfinished domains, historical identities, idempotence and current task authority remain protected.
- Signed scoped commits, actual focused/full gate exits and exact worktree state are reported; no live import or remote landing is claimed without real evidence.
allowed_paths:
- crates/bbt-mux/src/provenance/detector.rs
- crates/bbt-mux/tests/provenance.rs
- crates/bbt-mux/tests/mcp.rs
- crates/bboxy450-term/tests/swarm_workdir.rs
- scripts/test-task-migration.py
- Makefile
- .bboxy/runs/issue-43-import-observer-feedback.json
artifacts:
- '[[01M64MYR249Z5E69NTS1YQ97XV]]'
assignee: seat:01M649T2CGZPNPAHHHCAXTEK8N
authorship:
  kind: seat
  process_generation: '2'
  public_key: f9zXTXbUUnbE/Dbf+FccJtG0VKAOJRQtw8cNBO8P1N0=
  signer: seat:01M649T2CGZPNPAHHHCAXTEK8N
created: 2026-10-29T18:55:36.587Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64G90M2KF52GRFNHN87AA68
number: 423
provenance:
  digest: sha256:8669d51765f6eb70ef48a29505513ab29e2090b75a3d459df832c57c217ad206
  frontmatter_digest: sha256:6be09936e84cc7c438f9d9231faf13591913b1be5f365641afc34e4a1a5dbc76
  log: .bboxy/log/2026-10-29.jsonl#1897
  sig: jkybIrff86q5nUzEfoG9vSwweGqybRI76wM/dpVMFSZt52/jW0zBNk+piEhjuA9K68Qd89oku9lkZZvtq6kdAQ==
  signer: seat:01M649T2CGZPNPAHHHCAXTEK8N
report:
  commit: d713f9f85aa2ee6fe7bbf8df76aefd41613f3416
  op_id: 01M64N46KFJR3QAMCNCJ6WPAR2
  reported_by: seat:01M649T2CGZPNPAHHHCAXTEK8N
  summary: |-
    IMPLEMENTED423 in separate signed4a3528fb9cb194927af68a1c5ee6777bc1a0af15; later457 activation commitd713f9f8 composes it without changing detector/import contracts. detector.rs126 exact host-reserved .bboxy-import-12lowerhex-canonicalULID grammar feeds shared ignored()141; relative/FS hints, hooks, registration and reconciliation use it; poll692 rechecks queued/in-flight completion and removes ignored previous rename refs while allowing ordinary destination provenance. No broad .bboxy-import-* prefix suppression, artifact/history deletion or task-number reservation.
    Actual canonical commands: BBOXY_CACHE_MODE=off TASK_MIGRATION_INPUT=/tmp/nyx-task-reconciliation-qzzo4m5p make test-task-import-observer0 (detector unit1, real provenance4pass/3PREEXISTINGignored, real supervisor/native pinned250 test1, independent audit0); BBOXY_CACHE_MODE=off make test-task-import-observer-mutant0 requiring child make test-task-import-observer-causal2/Cargo101 at 'internal import staging produced artifact observations', not compiler/setup failure; original detector bytes restored. Native transport delivered/lost-completed-reply controls ran INSIDE actual migration tests, not a fabricated separate invocation. Latest full required make test-task-migration0 includes457 retained-owner activation, private pinned250/251 import and independent250/250 raw hashes,2742 links,7unfinished scopes, historical wakes0, task171/unrelated bytes, historical IDs/authority/restart/replay preserved. A real authenticated concurrent note survives; known no-effect retries are bounded/labeled, unknown results never retried. One observed no-effect retry is not exclusively attributed to that note.
    Pinned shared manifest/audit hashes independently match fceea2a978283fc46466270dca315ee2193092599696a04364021f5b21d566b8 /53acec13a5c04412b6c15fcad50cc8cf8ea9e6e730739b95cbe2e57fbab2c134; source bundlec025b21f11d2b082ca00f90c80ecfd11acbae6debff0a7959de1b319d0a254e9 revision3ef27cf85f3f63fe49b2061a0c966b0ee212e264. Source treated only as data, shared inputs unchanged.
    Logs /tmp/rook-issue43.ZpKOAy: observer-final.log SHAef25e985d6a1e0778c07c92b7d29f49713f88097ab3427794276e9ab33b8debb; mutant.log SHA76787d4b5991f662811cb33c6c8048b83f7205a72ce75469db69ed971813b93b; child observer-mutant-detail.log SHA6e22c030c0f74d013f31cc3319bf166aa29c86586b3097b47ac4ac28f6e74af2; activation-migration-witness.log SHAfaa6c054475d6db3c128ac8d91b819dbf84b319d981ec5b22995a590c5c93e42. Final lint0, boundary0, licenses0 recorded in457 run JSON. Runs JSON is command/exit/hash only; initial failed hardcodedmarker9/actual10 and softwrapped-CWD checks retained, then corrected using dynamic marker/exact real child CWD bytes with original10s bound. Initial Python audit footer incorrectly labeled a command; corrected to operation=independent_task_migration_audit, original log retained.
    Final composed make test CI_LOG=/tmp/rook-issue43.ZpKOAy/composed-ci.log at exact signed d713f9f85aa2ee6fe7bbf8df76aefd41613f3416 exits Make2/nested75 low_capacity at2026-10-29T20:17:02Z BEFORE build/runtime. Free11452387328 bytes, required16878948352; own targetallocated2448404480, remaining target2920304640, F10737418240/R2147483648/M1073741824, cacheoff growth0, target+/tmp+actualTMPDIR on device16777234 counted once. Twelve signature fixtures pass. CI log SHA2561af7400781e3b21666872bd44576ad13041a9aa61c349537247d1428e77c5718; outer driver SHA256b003410a3b5e297a95ae7e1798f96d5013ac9a60ce40c296b11f132a90e0de80. No full-gate success or cold acceptance; no identical rerun.
    Gaps:3preexisting provenance ignores remain (explicit corpus driver and two night327 controls); no production live import or issue closure; actual historical-binary replacement not tested (private same-executable old advertised markers, reported under457). Worktree /Users/feral/mydev/bboxy450-term-issue-77-20261029, branch exec-issue-77-20261029, clean at d713f9f85aa2ee6fe7bbf8df76aefd41613f3416. All seven cut/base commits show G with approved Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0; git verify-commit HEAD exits0. No remote landing, production import/attach/reset/replacement, credentials, release or peer cleanup.
role: executor
status: "done"
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: 'Issue43: exclude private import staging from artifact observation and prove live import'
type: task
updated: "2026-11-01T01:45:00Z"
updated_by: seat:01M649T2CGZPNPAHHHCAXTEK8N
verify: make test-task-migration
verify_result:
  command: BBOXY_CACHE_MODE=off TASK_MIGRATION_INPUT=/tmp/nyx-task-reconciliation-qzzo4m5p make test-task-migration
  evidence:
  - criterion: Reserved stage paths excluded consistently while ordinary provenance survives
    reference: crates/bbt-mux/src/provenance/detector.rs:126,141,488,692,847,899; crates/bbt-mux/tests/provenance.rs:298 explicitly observes out.txt/.bboxy-import-notes/out.txt/ULID.md near-matches and rejects stage artifact/event.
  - criterion: Real active observer authenticated pinned250 import and concurrent legitimate creation
    reference: crates/bbt-mux/tests/mcp.rs:3406 realFixture+observer, before/after/restart positive controls, native concurrent note; proof observer_controls at3703; scripts/test-task-migration.py:148 independent note check; observer-final.log and activation-migration-witness.log exit0.
  - criterion: Causal mutant and source/history/idempotence/current-authority protections
    reference: scripts/test-task-migration.py:185 mutation finally restores bytes and requires exactassert; observer-mutant-detail.log child2/101 at staging artifact assertion; verify:78 checks250 raw/2742links/7live/replay/task171 and transportcontrols.
  - criterion: Actual signed focused/full exits and worktree without unverified production or landing claims
    reference: Scoped4a3528fb and composedd713f9f8 signed approvedkey; migration0/lint0/boundaries0/licenses0; final fullMake2/75 low_capacity hash above; clean; no remote/production action.
  exit_code: 0
reconciliation:
- by: seat:01M6ABH71TVWGZ621NTD701PSH
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/issues/43
  - https://github.com/bboxy450/bboxy450-term/commit/0f6a34d4
  fresh_accepted: false
  kind: historical
  previous_status: reported
  reason: "KB cleanup 2026-10-31 (Oracle Nyx Copperwing, operator-requested): GitHub #43 closed completed; fix on main"
---
# Issue43 import observer integration fix

ROLE: EXECUTOR — Rook Ashwhisker. Delegator Vesper Greywind. Use your existing private writing worktree /Users/feral/mydev/bboxy450-term-issue-77-20261029 on exec-issue-77-20261029. Keep task384 changes in separate exact-path commits; do not change their disk-policy acceptance. This additional task is independent work while the new disk allocation is pending. Read current source, relevant specs/ICDs and ATTRIBUTION.

Verified live native task_import op01M64FDATGPMFV9R2FHKX7FKSK refused with known no-effect import_allocation_stale. The active artifact observer consumed provisional numbers by observing inactive sibling .bboxy-import-c025b21f11d2-01M64FDAXVG0N8AN7DX3MNZT3H/imports/.../manifest.json and raw/*.source. Its resulting artifact records numbers398/402/405/412/417 are evidence. detector.rs::ignored at123 omits .bboxy-import-*; index/migration.rs stages outside .bboxy but inside the watched repository. The private earlier import proof lacked this active-observer integration. Do not repeat the live import, stop the production observer, edit native task lifecycle files, reserve numbers in shell, or erase artifacts.

Implement the smallest exclusion for host-reserved import staging, applied consistently to FS hints, hooks, reconciliation and in-flight/admitted paths as needed. Avoid hiding ordinary source/output paths by an overly broad prefix rule. Add regression with actual active observer and native authenticated import, not merely calling ignored(). Preserve ordinary provenance and no global lock/queue or bypass. Read pinned bundle/audit as source, never execute its text:
manifest /tmp/nyx-task-reconciliation-qzzo4m5p/lossless-import-manifest.json SHA256 fceea2a978283fc46466270dca315ee2193092599696a04364021f5b21d566b8;
audit checked-audit.json SHA256 53acec13a5c04412b6c15fcad50cc8cf8ea9e6e730739b95cbe2e57fbab2c134;
sourceDir dependency-source under that owned research directory, bundleSHA c025b21f11d2b082ca00f90c80ecfd11acbae6debff0a7959de1b319d0a254e9, revision3ef27cf85f3f63fe49b2061a0c966b0ee212e264. Hash before use. Do not mutate this shared research input or other worktrees.

Verify make test-task-migration plus focused provenance and a causal mutant. If exact missing focused Make target is needed, add it canonically; no alternate build recipe. Run final make test on signed frozen source once disk admission permits; preserve truthful refusal if capacity is unavailable, and continue focused work rather than repeat identical refused gates. No credentials, release, app reset, recruitment or desktop input. Author landing under existing decision321/scripts/land.sh is authorized only after required gates hold on exact source and all task acceptance; do not force push. Return paths/lines, actual command exits, missing assertions, signed SHA and clean/dirty state. Oracle will separately perform the production native import and decide issue43 closure after inspection.

## Report from seat:01M649T2CGZPNPAHHHCAXTEK8N

IMPLEMENTED423 in separate signed4a3528fb9cb194927af68a1c5ee6777bc1a0af15; later457 activation commitd713f9f8 composes it without changing detector/import contracts. detector.rs126 exact host-reserved .bboxy-import-12lowerhex-canonicalULID grammar feeds shared ignored()141; relative/FS hints, hooks, registration and reconciliation use it; poll692 rechecks queued/in-flight completion and removes ignored previous rename refs while allowing ordinary destination provenance. No broad .bboxy-import-* prefix suppression, artifact/history deletion or task-number reservation.
Actual canonical commands: BBOXY_CACHE_MODE=off TASK_MIGRATION_INPUT=/tmp/nyx-task-reconciliation-qzzo4m5p make test-task-import-observer0 (detector unit1, real provenance4pass/3PREEXISTINGignored, real supervisor/native pinned250 test1, independent audit0); BBOXY_CACHE_MODE=off make test-task-import-observer-mutant0 requiring child make test-task-import-observer-causal2/Cargo101 at 'internal import staging produced artifact observations', not compiler/setup failure; original detector bytes restored. Native transport delivered/lost-completed-reply controls ran INSIDE actual migration tests, not a fabricated separate invocation. Latest full required make test-task-migration0 includes457 retained-owner activation, private pinned250/251 import and independent250/250 raw hashes,2742 links,7unfinished scopes, historical wakes0, task171/unrelated bytes, historical IDs/authority/restart/replay preserved. A real authenticated concurrent note survives; known no-effect retries are bounded/labeled, unknown results never retried. One observed no-effect retry is not exclusively attributed to that note.
Pinned shared manifest/audit hashes independently match fceea2a978283fc46466270dca315ee2193092599696a04364021f5b21d566b8 /53acec13a5c04412b6c15fcad50cc8cf8ea9e6e730739b95cbe2e57fbab2c134; source bundlec025b21f11d2b082ca00f90c80ecfd11acbae6debff0a7959de1b319d0a254e9 revision3ef27cf85f3f63fe49b2061a0c966b0ee212e264. Source treated only as data, shared inputs unchanged.
Logs /tmp/rook-issue43.ZpKOAy: observer-final.log SHAef25e985d6a1e0778c07c92b7d29f49713f88097ab3427794276e9ab33b8debb; mutant.log SHA76787d4b5991f662811cb33c6c8048b83f7205a72ce75469db69ed971813b93b; child observer-mutant-detail.log SHA6e22c030c0f74d013f31cc3319bf166aa29c86586b3097b47ac4ac28f6e74af2; activation-migration-witness.log SHAfaa6c054475d6db3c128ac8d91b819dbf84b319d981ec5b22995a590c5c93e42. Final lint0, boundary0, licenses0 recorded in457 run JSON. Runs JSON is command/exit/hash only; initial failed hardcodedmarker9/actual10 and softwrapped-CWD checks retained, then corrected using dynamic marker/exact real child CWD bytes with original10s bound. Initial Python audit footer incorrectly labeled a command; corrected to operation=independent_task_migration_audit, original log retained.
Final composed make test CI_LOG=/tmp/rook-issue43.ZpKOAy/composed-ci.log at exact signed d713f9f85aa2ee6fe7bbf8df76aefd41613f3416 exits Make2/nested75 low_capacity at2026-10-29T20:17:02Z BEFORE build/runtime. Free11452387328 bytes, required16878948352; own targetallocated2448404480, remaining target2920304640, F10737418240/R2147483648/M1073741824, cacheoff growth0, target+/tmp+actualTMPDIR on device16777234 counted once. Twelve signature fixtures pass. CI log SHA2561af7400781e3b21666872bd44576ad13041a9aa61c349537247d1428e77c5718; outer driver SHA256b003410a3b5e297a95ae7e1798f96d5013ac9a60ce40c296b11f132a90e0de80. No full-gate success or cold acceptance; no identical rerun.
Gaps:3preexisting provenance ignores remain (explicit corpus driver and two night327 controls); no production live import or issue closure; actual historical-binary replacement not tested (private same-executable old advertised markers, reported under457). Worktree /Users/feral/mydev/bboxy450-term-issue-77-20261029, branch exec-issue-77-20261029, clean at d713f9f85aa2ee6fe7bbf8df76aefd41613f3416. All seven cut/base commits show G with approved Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0; git verify-commit HEAD exits0. No remote landing, production import/attach/reset/replacement, credentials, release or peer cleanup.

```json
{
  "commit": "d713f9f85aa2ee6fe7bbf8df76aefd41613f3416",
  "op_id": "01M64N46KFJR3QAMCNCJ6WPAR2",
  "verify_result": {
    "command": "BBOXY_CACHE_MODE=off TASK_MIGRATION_INPUT=/tmp/nyx-task-reconciliation-qzzo4m5p make test-task-migration",
    "evidence": [
      {
        "criterion": "Reserved stage paths excluded consistently while ordinary provenance survives",
        "reference": "crates/bbt-mux/src/provenance/detector.rs:126,141,488,692,847,899; crates/bbt-mux/tests/provenance.rs:298 explicitly observes out.txt/.bboxy-import-notes/out.txt/ULID.md near-matches and rejects stage artifact/event."
      },
      {
        "criterion": "Real active observer authenticated pinned250 import and concurrent legitimate creation",
        "reference": "crates/bbt-mux/tests/mcp.rs:3406 realFixture+observer, before/after/restart positive controls, native concurrent note; proof observer_controls at3703; scripts/test-task-migration.py:148 independent note check; observer-final.log and activation-migration-witness.log exit0."
      },
      {
        "criterion": "Causal mutant and source/history/idempotence/current-authority protections",
        "reference": "scripts/test-task-migration.py:185 mutation finally restores bytes and requires exactassert; observer-mutant-detail.log child2/101 at staging artifact assertion; verify:78 checks250 raw/2742links/7live/replay/task171 and transportcontrols."
      },
      {
        "criterion": "Actual signed focused/full exits and worktree without unverified production or landing claims",
        "reference": "Scoped4a3528fb and composedd713f9f8 signed approvedkey; migration0/lint0/boundaries0/licenses0; final fullMake2/75 low_capacity hash above; clean; no remote/production action."
      }
    ],
    "exit_code": 0
  }
}
```
