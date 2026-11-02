---
author: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
authorship: {"kind":"seat","process_generation":"1","public_key":"6zlI3Xtz61NbNKB19xGmAaUggcsK+9h71GpWZr81k1I=","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
commits: []
created: "2026-11-02T05:19:26.106Z"
created_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
id: "01M6DB5BF91Q9NX4TRYM5TX377"
number: 5180
provenance: {"digest":"sha256:cc3a95d654d315ba00110033291c442803cc74689e159ffbcbc6d91afee6ffb2","frontmatter_digest":"sha256:53b13307d37701d4b35976660f06157b2fee46425f20b13157a2a0800490fcb0","log":".bboxy/log/2026-11-02.jsonl#34489","sig":"0pZRUx82EHVmtoe51pmtKcb9hX7wIKl+mZovNMhHL4JsIF+ATo2nf1Zx/jEPyIzy47KHNNQ4mQfmaf2ROrcKBw==","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
task: "[[tasks/2026-11-02-term-ring-batch-wallclock]]"
title: "Turn-bounded independent ring readers under CPU load"
type: "report"
updated: "2026-11-02T05:19:26.106Z"
updated_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
verify: {"command":"cargo test -p bboxy450-term-mux --lib logic::ring_batch_tests","exit_code":0}
---

ROLE: EXECUTOR. Implements #5162 in `/Users/feral/mydev/bboxy450-term-ringbatch`, branch `ring-batch-turn-bound`. The containing signed commit resolves this report through its Report trailer.

`crates/bbt-mux/src/logic/ring_batch_tests.rs:399` bounds each seat by `3 * (WATCHED_QUANTA + 1) * SEATS = 12 * SEATS` runnable owner opportunities. The immediate synthetic read needs three handoffs: ring/history-parser admission, history publication/live-parser admission, and live publication. The rotating 3:1 watched/background queues serve each ready handoff within four seat rotations. The 300-second deadline is only a hang guard.

`crates/bbt-mux/src/logic/ingestion.rs:41` adds a test-only, non-consuming parser completion query. A seat awaiting parser CPU does not spend its owner budget; a completed result waiting for publication does. This keeps CPU scheduling separate from owner starvation without introducing a shared completion barrier or changing production behavior.

`crates/bbt-mux/src/logic/ring_batch_tests.rs:466` retains the one-page/notification-per-turn assertion. Exact serial sequence, byte-end and screen comparisons, singleton-read prohibition, independent one-member requests, host boundaries and the SEATS RPC bound remain enforced. Both original 967-seat fixtures still run.

`crates/bbt-mux/src/logic/ring_batch_tests.rs:339` adds a healthy four-seat witness. At line 345 the broken reader withholds seat 1 indefinitely and must panic specifically at the runnable turn bound. All 20 runs rejected turn 49 against bound 48; none used the wall-clock backstop as its verdict.

`verification/contracts.toml:270` records the quantitative fixture bound and its readiness assumption in SeatFairness. The two-seat weak-fairness model and BrokenSeatFairness remain unchanged: no product scheduler changed, and the model does not claim a bounded OS/Tokio CPU schedule.

Verification completed before this report:
- `cargo test -p bboxy450-term-mux --lib logic::ring_batch_tests` — 20 consecutive exits 0, 11 tests each. Every run observed four or five other live `sh scripts/ci-local.sh` processes; one-minute load ranged from 24.37 to 42.51. Healthy 967-seat sweeps used 2,896–2,902 runnable opportunities against 11,604.
- `make test-licenses` — exit 0, all 32 workspace crates and source notices held.
- `rustfmt --edition 2024` on both changed Rust files and `git diff --check` — exit 0.

`.bboxy/runs/2026-11-02-term-ring-batch-wallclock.json` retains command exits, per-run load/process snapshots and source hashes. Raw targeted log SHA256: `2a1d6152dae90ad16a9da60c01b92cbe8204b596008d49decf022789866dfcba`. Raw evidence is outside Git and outside target at `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-ringbatch-5162-ianomcew/`.

The final-commit `make test` is pending the Oracle's combined-landing notification and an empty concurrent full-run inventory. Its actual result, signed commit, PR and target cleanup will be supplied by the one task_report, with retained `ci-local.log` and `full-gate.json` in that evidence directory. This report does not claim a full-gate result in advance. The task copy is committed alongside the implementation. No merge or land.sh is authorized.

