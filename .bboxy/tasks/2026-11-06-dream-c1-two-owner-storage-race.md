---
type: task
title: 'HIGH: C1 two-owner start: dispatched child''s own record write refused (StorageUnavailable, busy0 race)'
role: executor
allowed_paths:
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- crates/bbt-mux/tests/dream_c1.rs
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- verification/
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/dream_c1_two_real_sqlite/)' && make test-formal-contracts
status: open
id: 01M6SKXFMZRDGTFB11S87YHGQZ
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T23:43:16.609Z
created: 2026-11-06T23:43:16.609Z
number: 93377961366168
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:f1190b4f8977f585c75ce88e1065ee8b41098ca21b9751acf63022e74138d484
  frontmatter_digest: sha256:b05def2335e355e0b3efa41006d9e1f2e1ec4c56933b205a1f1d15b87dcdaaa4
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: Psd6fFTe/AspEOiS0aWFBbfSwU/01L/HmZ9Wk0vICIADnkOZqbVCEjY+5TlfN/lGYGYxrR+uGR6arKMNFfkiBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150010
  event:
    id: 01M6SKXGB0NMR1TQMRM048866F
    sequence: 150010
    hash: f8121b8c3621169bb7cd0e0780cabc97c18e7c96e61c7382a4c14a51c3e8293b
---
# Two SQLite owners: the winner's own write is refused after dispatch

Evidence: C1 `make land` on signed 618f4b7f (log `/tmp/dream-c1-load-dispositions-make-land.log`). `dream_c1_two_real_sqlite_start_owners_dispatch_exactly_one_private_child` failed at `registry/dream_c1.rs:4058` with **StorageUnavailable** after the private child had been **dispatched, exited 0 and been reaped** (child 38076). Then the thread join panicked.

**Probable mechanism (Oracle, from code and decision 66026):** C1's private catalogue runs busy_timeout 0 with PERSIST journaling. When the losing owner's refusal path holds a lock at the instant the winner writes its post-dispatch record, the winner's write fails at once. Load widens the overlap. The real effect has happened, but its durable record is refused. That is a custody and honesty defect ("effect happened, record unknown"), not only flakiness.

**Required:** the winner's post-dispatch record must never be lost to the loser's transient lock. Options: (a) a bounded, paced retry of ONLY the winner's own record commit (each attempt ≤1 ms with 9× rest, cap N, then honest `unknown` with the effect retained), or (b) ordering, so the loser's refusal never holds a write-conflicting lock. The loser still refuses promptly. Model the two-owner interleaving (dispatch/record/refuse, A→B→A) with a broken control in which the winner's record is refused after dispatch. Re-enable the test, which C1 marked `#[ignore]` under the operator's landing order. Lean landing; write the report `reports/2026-11-06-dream-c1-two-owner-storage.md` and call `task_report` once.

**Oracle amend (re-enable duty transferred from QoS, 2026-11-07):** a test now carries a KNOWN-DEFECT #[ignore] pointing here (see the QoS report). Re-enabling it and showing it pass is part of THIS task's acceptance.
