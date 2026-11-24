---
allowed_paths:
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/tests/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- Makefile
- scripts/test-dream-c1.py
- verification/
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-07T08:02:06.115Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6TGEVN80VQM9D6RAPSXZYKP
number: 93377961366244
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - commit ff8109e68 — crates/bbt-mux/src/logic/dream_c1.rs:54 50 ms OVERRUN
  - 'successor: tasks/2026-11-24-term-stale-known-defect-ignores.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K7TPEN2G6ASCTRAPC98R
  previous_status: open
  reason: Budget fixed; remaining 11 ignores carried forward.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: C1 per-unit CPU budget overruns persist at low load (1.05–1.44 ms vs 1 ms)
type: task
updated: 2026-11-24T17:10:30.854Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-c1 && make test-formal-contracts
provenance:
  digest: sha256:d5090cec0d5bdf4dd0d10fa3d5656dd0019c9cbd92eb28f92b057295036d9666
  frontmatter_digest: sha256:06d5625fe6be8c7f7beda97389ea886e36e2ae0cabb8cdbe7e5a1e00ec3897f3
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: gkXU7gVcWcKkfzVbFoxT4bRugdryjgtml5GMtFvsB2V52foyqzuk/nsxXJSxOFJ9vRsHchMLg9hWdokbPmsIDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1867
  event:
    id: 01M878K8V41NMA8PPW9P53FAAS
    sequence: 1867
    hash: 3c135326d9ac4462e58ebf19f290673e30c837a61dccdb43a9e6ef52c70c3c4f
---
# C1 units overrun the 1 ms CPU budget even at low load

Evidence (QoS executor 01M6S48V…, report `reports/2026-11-06-qos-builds-native-unit-budget-blocked`, KB a7dcbc99), load ~4 with 66% idle, after QoS landed:
- `final_actual_authority_rechecks_row_head_grant_inode_aba_after_debt`: CPU 1.127 / 1.051 ms.
- `source_failure_is_bounded_immutable_and_fenced_before_any_replay`: CPU 1.076 / 1.442 ms.
Each also failed its one warm isolated retry (exit 100). Related earlier overruns under load: journal setup/replay (1.045, 1.360, 2.150 ms), and Q2's ordinary debug admission unit (1.150 ms).

## Required

1. **Profile first.** Measure these units under the exact profile the C1 brief mandates (optimized fixture: `make test-dream-c1`'s CARGO_PROFILE settings) AND under the plain nextest/land profile, and report both. If the overruns occur ONLY in a less-optimized test profile, the defect is that the CPU-budget assertions ran under the wrong profile: route budget-asserting witnesses through the mandated optimized profile (Make, not a threshold change), keeping correctness assertions everywhere.
2. **If they overrun in the optimized profile:** split the real code path into ≤1 ms units with 9× rest via the existing cooperative hooks (decisions 65086/66026). No threshold raise, no new native tag.
3. Re-enable both tests (they currently carry KNOWN-DEFECT `#[ignore]`s pointing here) and show them passing, plus a broken control (the unsplit unit must fail the budget).

Lean landing; write the report `reports/2026-11-07-dream-c1-unit-budget.md` and call `task_report` once.
