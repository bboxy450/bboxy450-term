---
type: task
title: C1 per-unit CPU budget overruns persist at low load (1.05–1.44 ms vs 1 ms)
role: executor
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
verify: make test-dream-c1 && make test-formal-contracts
status: open
id: 01M6TGEVN80VQM9D6RAPSXZYKP
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T08:02:06.115Z
created: 2026-11-07T08:02:06.115Z
number: 93377961366244
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:d5090cec0d5bdf4dd0d10fa3d5656dd0019c9cbd92eb28f92b057295036d9666
  frontmatter_digest: sha256:0f09e162457070f0d30f6f9c54f46120687be4cbcc6272f2949b309641f8231a
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: hopKx5AOsdGbPbY0Ex0pa4Y+JWaFxKfVSANUXpPS96DKsmh4Axq2CI+VB6NLRoNKEBgG6Y41Iq0YhaMOz6uBDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150245
  event:
    id: 01M6TGEW91SYV7TGBM5W20Z84Z
    sequence: 150245
    hash: ddcaf414b1b2bc853e408bf32cbcc14e6fc1a13d6d23f1295f0e276a0bd32db3
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
