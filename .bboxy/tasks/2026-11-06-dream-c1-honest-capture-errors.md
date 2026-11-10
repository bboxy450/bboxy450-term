---
type: task
title: C1 capture() masks inner budget/IO errors as stale_context
role: executor
allowed_paths:
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/tests/dream_c1.rs
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- verification/
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/dream_c1/)' && make test-formal-contracts
status: open
id: 01M6SK7HKWY9ECRG3DVSRK8M1A
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T23:31:17.712Z
created: 2026-11-06T23:31:17.712Z
number: 93377961366165
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:7c8b84544f9f65c7833182634832f5ad841f7b2ae1f151d2a3b7d9948afb8bd9
  frontmatter_digest: sha256:b919439f80f91bbdc7c5778c0a7d656cf04ea5449abd50fb109de6bee54147c3
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: zmSSt9+XT0+pcv6+SAJ3xt8M5zec4cXIkZIgWrOoJAOFKKlnxxRinjkMEPILG3YOxBBJ9YburgA/Sk/i1LriAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150002
  event:
    id: 01M6SK7JAG94EFE2YMMVCRF7C6
    sequence: 150002
    hash: 021ce65b9a8b0222ea0769bfd028f367bf76e436a0952ce107c616dbb346be5c
---
# capture() reports the wrong error

`crates/bbt-mux/src/registry/dream_c1.rs` `capture()` (around lines 1218–1256, on the C1 landing branch) does `.map_err(|_| Code::StaleContext)` on `prepare_session_cooperative` and `current_prepared_cooperative`. Every inner error therefore becomes `stale_context`, including a CPU-budget `ResourceLimit` under load. That's how `dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs` (tests/dream_c1.rs:272) got `stale_context` instead of the expected refusal at load 18; it is `#[ignore]`d under the operator's "cut corners" order.

**Required:** keep the honest code. A budget overrun stays `ResourceLimit`, a genuine revision or metadata change stays `StaleContext`, and other errors map to their real codes. Re-enable that test (and confirm it passes at background-QoS build load once [[tasks/2026-11-06-qos-builds-never-starve-mcp]] lands). Add a unit test per mapped error kind, plus a broken control: the blanket mapping must fail the new test. Lean landing; write the report `reports/2026-11-06-dream-c1-honest-capture-errors.md` and call `task_report` once.

**Oracle amend (re-enable duty transferred from QoS, 2026-11-07):** a test now carries a KNOWN-DEFECT #[ignore] pointing here (see the QoS report). Re-enabling it and showing it pass is part of THIS task's acceptance.
