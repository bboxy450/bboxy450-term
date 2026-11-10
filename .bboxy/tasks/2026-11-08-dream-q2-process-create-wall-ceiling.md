---
type: task
title: Q2 implement process_create 250 ms wall ceiling (66331) and hand off INPUT60
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/proof-helper-probe.c
- tests/fixtures/dream-read/private-engine-controls.c
- tests/fixtures/dream-read/check-private-engine-controls.py
- tests/fixtures/dream-read/prepare-c-probe.py
- tests/fixtures/dream-read/check-c-preparation.py
- src/dream/c1.rs
- tests/dream_c1.rs
- tests/dream_read.rs
- tests/formal/dream_c1.rs
- tests/formal/dream_read.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-08-dream-process-create-wall-ceiling.md
- src/dream/read.rs
- src/dream/resolve.rs
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366331-dream-q2-process-create-wall-ceiling]]'
- '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
id: 01M6WVKFCVG0MB3RWSHH2TCR4S
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T07:45:51.097Z
created: 2026-11-08T05:55:20.404Z
number: 93377961366338
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Prepared process_create only250ms; CPU2ms/inclusive3ms and every other native wall25ms retained
    reference: Signed1602aff72e961e493cca14feb559d6a323733f67; proof-helper-probe.c:188; c1.rs:44; read.rs:1220; resolve.rs:1125. N/rust-integration-source-freeze-v2.json and current API tests0.
  - criterion: Exact u64 excess ns/count before admitted/refused verdict; full251ms226ms excess and no pre-effect false count
    reference: N/INPUT60-final-tuple-v1.json SHAb95da4686572b3176b3d7d95a09dd0e635e2f4fae72c3f4d0d439c1707677c62; independent5ledger-snapshot check SHA93d06090d35479132f38730de053e248bf58e9be550440860c91b65b960d187c; prepared4tests0 incl pre-effect/attempted-unknown CPU.
  - criterion: 34ms positive/251ms81/CPU>2ms81 and old25/ignoredCPU causal controls with actual owned settlement
    reference: N/process-create-causal-ybee1z4u/run.json SHA86687a2137b87eede9e979c9fa9ce1d88cd241647ac51ebda72fbe215691c99b; actual exits0,0,0,81,81,0,81,0,81. Supplied policy timing, actual OS cost/ownwait4/groupgone/full debt separate. Canonical current72/43/45/94 actual0.
  - criterion: Finite model/contracts and broken old25 control; current full native/Java states/labellededges/named verdict parity
    reference: verification/evidence/cf84c26b9bb0b194b46cb6cce1e0e1af1ecf251889e15478bc972dcc147e3fd1:433models/93contracts/3354453states/11303061edges. Raw4332file manifest SHA677eb843dcfb16843dd64f9ac420a93b93548596b55dd73b4ca07d17f388a2ca; clone validator0; cap100000 unchanged.
  - criterion: Exact assigned verify string once on final signed HEAD, actual integer exit0
    reference: N/process-create-exact-four-chain-v1.json/.log; actual0/sourceunchanged1602aff7; log SHA5cdff505744c5f8bc35239ea860f8e76bc96ee699f9df2b55d659f9eba8a4d7c.
  - criterion: Canonical make land and remote containment
    reference: N/process-create-canonical-land-v1.json/.log actual0, log SHAea17b9533526e81c415dc42de9ea9d7d31a88cf1ae860b903b4e2b4d4689afb7; harness-land-receipt SHA55c94c7bb219d8c27e170fee0ae9183916675a925d0bee70c20f2e34d04e1e78; postland fetch0 and ancestor0; clean origin/main1602aff7.
  - criterion: 'ONE signed immutable INPUT60 tuple: source/executable/compiler/10inputs/header closure/audit and exact field types'
    reference: Stored message1423/id01M6X1FFQ8G5HWGYREKBM6ACCH. Receipt SHA6d1b82a8f4fdb67da05b865a1ccddf8203764757d82dda57773d926fd8aa5584; executable67060d75347926eb9d4f8a64006d9444b0fa0f71fdeca010fef92f9c4e025e8d;997audit0 SHAaab25da473e3d1a4d1f90660f7fb5f559224bafa1f99ffa76ce05d1fec347890; source4domain8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8.
  - criterion: Source-derived full16 ordinary-floor estimate and honest qualification/performance boundary
    reference: Source estimate SHA3ccacb8048bc9f47a03fdddb607723787b9dd46f581c0ce68ccbf60e353fe9ae:33*(G+7)*9ms floor, G16=6.831s/G32=11.583s/G64=21.087s/G256=78.111s before excluded costs; actualG/dynamics unknown.Whole30s fit/realqualification/runtimeapproval UNPROVEN/false; no genuine builder run.
  - criterion: Historical failures/profiles/evidence preserved; no new ignore or old-task amendment
    reference: N/current-historical-deferred-tests-v1.json SHA8d21bc964e7f15c33d58f4547a81528e9e0f375db980621c6b903258e3e06607:26non-passes21QoS+5qualification unchanged. Actual profile/independent worker +same-inode observers preserved; all prior RED/tuples retained and storage metadata changes journaled.
  - criterion: Committed full report names THIS new task, authenticated seat and exact verify0
    reference: reports/2026-11-08-dream-process-create-wall-ceiling.md id01M6X1S1GDYQ60SYQ3EXQC2T3Y etag6c5d3bf52b9cf1a313a2f3bcd9f9527dd7c9358940ad4d80e9271c8d6a598ac6; full readback identical, committed KB4fabaedf04340041663ef2cab8d0e2893a765599 artifact01M6X1T59FYGPN03YY9D49MXFB.
report:
  summary: 'Landed signed1602aff72e961e493cca14feb559d6a323733f67 on origin/main, clean: exact assigned four-command chain0 ONCE final head and literal canonical make land0. Only prepared process_create wall becomes250ms; full checked u64 excess/count retained before verdict in terminal/ledger, other gates unchanged. Current C72baseline/43mutants/45compiles/94commands0 and fresh433model/93contract native/Java full graph parity0. Audited immutable INPUT60 stored1423 with10inputs/compiler/325headers/997checks0.26 historical ignores remain non-passes; genuine qualification/runtime approval and whole16-form30s fit remain UNPROVEN, with source floor estimate supplied. Full report [[reports/2026-11-08-dream-process-create-wall-ceiling]], committed KB4fabaedf04340041663ef2cab8d0e2893a765599.'
  commit: 1602aff72e961e493cca14feb559d6a323733f67
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6X1WDF658C5QV6Z9G685K3V
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-process-create-wall-ceiling.md]]'
provenance:
  digest: sha256:2bd0e2e00bdd846c17be879f0746a40061eff2351b4e4ea5d22184b6b949ab1e
  frontmatter_digest: sha256:a7594863cc57d575d2ca061d660a340b1e4e00b711bea6ecf0245e31bf3982e8
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 5CZQ6+Kf+1rMGzUMLDXmRJpSm98yI+bHD3F+mvNiDQ24L4+sMK+N5hydE6loFz9utnYGqY0j1j5523CN+aw1DA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150587
  event:
    id: 01M6X1XV2S4XBKMW8DXNR9B85T
    sequence: 150587
    hash: e2d7a889091fb0c5b776843613fc7296c8455ea5da69aa85e7c334a8e23d7ca9
---
ROLE: EXECUTOR

## Goal
Implement [[decisions/d93377961366331-dream-q2-process-create-wall-ceiling]] in Q2's canonical Dream source, then hand off ONE frozen private C input, INPUT60, for the proof-tools builder.
Base: landed harness main af5de27c (strict ^{tree} peel). Worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2, the same Q2 tree.

## Required change
- process_create (the 65086 prepared posix_spawn tag) ONLY: CPU 2 ms and inclusive 3 ms stay hard; the native wall ceiling becomes 250 ms (hard).
- A wall between 25 and 250 ms is admitted, and its exact excess is recorded: process_create_wall_excess_ns (u64) and process_create_wall_excess_count (u64), in the terminal line and the ledger. Never clamp or hide it.
- Above 250 ms, refuse as now: the effect is retained, killed, reaped and settled.
- Every other native class keeps 2 ms CPU, 25 ms wall and 3 ms inclusive. The 1 ms ordinary quantum, 9x debt, the 250 ms per-command and 3 s per-job CPU caps, 30 s, 16 MiB, reap/settlement and the strict ^{tree} peel are all unchanged.
- Apply the same bound wherever Rust c1.rs enforces process_create, so C and Rust agree.

## Controls (causal)
- A 34 ms process_create wall is admitted and its excess recorded.
- A 251 ms wall is refused (81).
- A CPU above 2 ms is refused.
- A mutant using the old 25 ms bound must fail the 34 ms positive.
- Update the finite model and contracts for the changed bound, with a broken control.

## Handoff
Send ONE INPUT60 tuple to the Oracle, in the same format as INPUT58/59: signed commit, receipt, probe hash, the ten inputs, compiler and audit. Include:
- the exact new terminal field names and types;
- a source-derived estimate of the ordinary-floor total for a full 16-form positive against the 30 s limit.
runtime_approved stays false. No builder run, real qualification or approval.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Report that exact string as verify_result.command. Run it once on the final head. Land with make land, report in reports/, and task_report once.


## Oracle amendment (2026-11-08): two more prepared-spawn gates
Added src/dream/read.rs and src/dream/resolve.rs, ONLY for the prepared-spawn wall gates:
- read.rs ProofClock::native_created (~1214, span.wall_ns > 25_000_000);
- resolve.rs HelperProfile::record Spawn (~1106, wall > 25_000_000).
Use the shared 250 ms policy at those two gates only, record the full checked u64 excess before the verdict, emit the exact fields, and update only the affected source-unit vectors and targeted controls.
Unchanged: c1's generic thread/publish 25 ms gate, every other native class, and all CPU, inclusive, debt, lifecycle, schema and performance law.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed1602aff72e961e493cca14feb559d6a323733f67 on origin/main, clean: exact assigned four-command chain0 ONCE final head and literal canonical make land0. Only prepared process_create wall becomes250ms; full checked u64 excess/count retained before verdict in terminal/ledger, other gates unchanged. Current C72baseline/43mutants/45compiles/94commands0 and fresh433model/93contract native/Java full graph parity0. Audited immutable INPUT60 stored1423 with10inputs/compiler/325headers/997checks0.26 historical ignores remain non-passes; genuine qualification/runtime approval and whole16-form30s fit remain UNPROVEN, with source floor estimate supplied. Full report [[reports/2026-11-08-dream-process-create-wall-ceiling]], committed KB4fabaedf04340041663ef2cab8d0e2893a765599.

```json
{
  "op_id": "01M6X1WDF658C5QV6Z9G685K3V",
  "commit": "1602aff72e961e493cca14feb559d6a323733f67",
  "report": "[[reports/2026-11-08-dream-process-create-wall-ceiling.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Prepared process_create only250ms; CPU2ms/inclusive3ms and every other native wall25ms retained",
        "reference": "Signed1602aff72e961e493cca14feb559d6a323733f67; proof-helper-probe.c:188; c1.rs:44; read.rs:1220; resolve.rs:1125. N/rust-integration-source-freeze-v2.json and current API tests0."
      },
      {
        "criterion": "Exact u64 excess ns/count before admitted/refused verdict; full251ms226ms excess and no pre-effect false count",
        "reference": "N/INPUT60-final-tuple-v1.json SHAb95da4686572b3176b3d7d95a09dd0e635e2f4fae72c3f4d0d439c1707677c62; independent5ledger-snapshot check SHA93d06090d35479132f38730de053e248bf58e9be550440860c91b65b960d187c; prepared4tests0 incl pre-effect/attempted-unknown CPU."
      },
      {
        "criterion": "34ms positive/251ms81/CPU>2ms81 and old25/ignoredCPU causal controls with actual owned settlement",
        "reference": "N/process-create-causal-ybee1z4u/run.json SHA86687a2137b87eede9e979c9fa9ce1d88cd241647ac51ebda72fbe215691c99b; actual exits0,0,0,81,81,0,81,0,81. Supplied policy timing, actual OS cost/ownwait4/groupgone/full debt separate. Canonical current72/43/45/94 actual0."
      },
      {
        "criterion": "Finite model/contracts and broken old25 control; current full native/Java states/labellededges/named verdict parity",
        "reference": "verification/evidence/cf84c26b9bb0b194b46cb6cce1e0e1af1ecf251889e15478bc972dcc147e3fd1:433models/93contracts/3354453states/11303061edges. Raw4332file manifest SHA677eb843dcfb16843dd64f9ac420a93b93548596b55dd73b4ca07d17f388a2ca; clone validator0; cap100000 unchanged."
      },
      {
        "criterion": "Exact assigned verify string once on final signed HEAD, actual integer exit0",
        "reference": "N/process-create-exact-four-chain-v1.json/.log; actual0/sourceunchanged1602aff7; log SHA5cdff505744c5f8bc35239ea860f8e76bc96ee699f9df2b55d659f9eba8a4d7c."
      },
      {
        "criterion": "Canonical make land and remote containment",
        "reference": "N/process-create-canonical-land-v1.json/.log actual0, log SHAea17b9533526e81c415dc42de9ea9d7d31a88cf1ae860b903b4e2b4d4689afb7; harness-land-receipt SHA55c94c7bb219d8c27e170fee0ae9183916675a925d0bee70c20f2e34d04e1e78; postland fetch0 and ancestor0; clean origin/main1602aff7."
      },
      {
        "criterion": "ONE signed immutable INPUT60 tuple: source/executable/compiler/10inputs/header closure/audit and exact field types",
        "reference": "Stored message1423/id01M6X1FFQ8G5HWGYREKBM6ACCH. Receipt SHA6d1b82a8f4fdb67da05b865a1ccddf8203764757d82dda57773d926fd8aa5584; executable67060d75347926eb9d4f8a64006d9444b0fa0f71fdeca010fef92f9c4e025e8d;997audit0 SHAaab25da473e3d1a4d1f90660f7fb5f559224bafa1f99ffa76ce05d1fec347890; source4domain8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8."
      },
      {
        "criterion": "Source-derived full16 ordinary-floor estimate and honest qualification/performance boundary",
        "reference": "Source estimate SHA3ccacb8048bc9f47a03fdddb607723787b9dd46f581c0ce68ccbf60e353fe9ae:33*(G+7)*9ms floor, G16=6.831s/G32=11.583s/G64=21.087s/G256=78.111s before excluded costs; actualG/dynamics unknown.Whole30s fit/realqualification/runtimeapproval UNPROVEN/false; no genuine builder run."
      },
      {
        "criterion": "Historical failures/profiles/evidence preserved; no new ignore or old-task amendment",
        "reference": "N/current-historical-deferred-tests-v1.json SHA8d21bc964e7f15c33d58f4547a81528e9e0f375db980621c6b903258e3e06607:26non-passes21QoS+5qualification unchanged. Actual profile/independent worker +same-inode observers preserved; all prior RED/tuples retained and storage metadata changes journaled."
      },
      {
        "criterion": "Committed full report names THIS new task, authenticated seat and exact verify0",
        "reference": "reports/2026-11-08-dream-process-create-wall-ceiling.md id01M6X1S1GDYQ60SYQ3EXQC2T3Y etag6c5d3bf52b9cf1a313a2f3bcd9f9527dd7c9358940ad4d80e9271c8d6a598ac6; full readback identical, committed KB4fabaedf04340041663ef2cab8d0e2893a765599 artifact01M6X1T59FYGPN03YY9D49MXFB."
      }
    ]
  }
}
```
