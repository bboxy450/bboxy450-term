---
type: task
title: Bring cold C1 MCP Status/page latency inside the 250ms/1ms budget
role: executor
allowed_paths:
- crates/bbt-mux/src/dream/
- crates/bbt-mux/src/registry/
- crates/bbt-mux/src/mcp/
- crates/bbt-mux/tests/
- scripts/test-dream-c1.py
- verification/
- .bboxy/reports/
verify: make test-dream-c1 && make test-formal-contracts
status: open
id: 01M6S38C4A93FP9MA7A4FQWHQW
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T19:19:05.056Z
created: 2026-11-06T18:52:07.658Z
number: 93377961366079
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:8dec101233637d78e34e75a9cc869595f0289ab41891533b9cfd418022bc5437
  frontmatter_digest: sha256:65ee1f2708e6c3fda9c1bfc472ce70eba92c8e61a9177697771abfd275ed4bc4
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: pD6rVq0K5x3tNfSZKaeTt+Lu2MiVi/fFlTA1GAhLjwN1WPx2Xo5dsEgPKTT6D6BqJJhRiujANbNNshRHck9EBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149766
  event:
    id: 01M6S4SR6QM6K5SK5N29W6VVWK
    sequence: 149766
    hash: e4adbb03ca643c4001249bcf451b31a7ce9f484d4716b6e744d58535acf63244
---
# Cold C1 MCP Status/page latency

The C1 landing (Oracle override, 2026-11-06) recorded cold healthy MCP Status and page traces of **1.44 s and 2.93 s** (seat 01M6RZZY4FR22NC0Q6DAZGHYKC) as a known limit instead of a blocker. Its report gives the cold and warm numbers.

1. Measure cold versus warm per phase. Decision [[decisions/d93377961366076-2026-11-06-dream-c1-first-launch-admission]] showed that the first exec of a new binary costs about 425 ms on this machine. If part of the excess is first-launch admission, it moves into provisioning.
2. Split every remaining slow phase into ≤1 ms units with 9× rest, using the existing cooperative hooks. Do not raise thresholds or warm up a fixture.
3. Add a test that fails when a cold Status or page call exceeds the budget, plus a broken control.

Lean landing: run only the affected tests, then `make land`. Write the report `reports/2026-11-06-dream-c1-cold-mcp-latency.md` and call `task_report` once.

## Measured cause (Oracle amend, 2026-11-06, executor trace `/tmp/dream-c1-admission-cold-warm.log`)

It is **not** first-launch admission. Warm is nearly as slow as cold:

| Call | Cold | Warm |
|---|---|---|
| Status | 1.753 s | 1.459 / 1.491 s |
| Pages | 3.153 s | 2.800 / 3.027 s |

Per phase (warm2): public preparation took 384.8 ms wall for 0.68 ms CPU; retirement 188.3 ms for 0.14 ms; the page journal 432.9 ms for 0.63 ms; domain plus retirement 1744 ms for 5.99 ms cumulative CPU. **Wall time is roughly 300–600× CPU time.** These read calls pay the paced 9× rest debt, plus waits, on the same lane as background Dream work.

**Changed direction for item 2:** splitting phases further adds more rest; it does not fix this. Serve Status and page **reads** from a prepared, bounded, revision-checked snapshot that the paced lane publishes, so a read is one ≤1 ms control-lane unit with no per-call public-authority preparation or retirement. Mutating and preparation work stays paced exactly as decisions 65086 and 66026 require. Target: warm Status ≤ 50 ms and page ≤ 100 ms wall; cold has the same targets plus a single bounded snapshot build. Keep revision, ABA and tamper negatives: a stale snapshot is labeled stale, never served as current.

**Oracle amend (re-enable duty transferred from QoS, 2026-11-07):** a test now carries a KNOWN-DEFECT #[ignore] pointing here (see the QoS report). Re-enabling it and showing it pass is part of THIS task's acceptance.
