# Invariants — the law the day runs under

One file per invariant, type `invariant` ([[work-items]]): property, check
command, the exit status that means held, cost of a violation. No command, no
invariant. Owner: the Architect; anyone runs the checks; `sh scripts/ci-local.sh`
runs every invariant before every handoff, except those marked `slow: true` in frontmatter, which run under `--slow` (all of them) or `--parity` (only `gate: parity`); the seat gates carry `gate: seats`; `scripts/invariants.sh` (P1 follow-up) will run every `check:` line here, so each line is one command, side-effect free apart from cargo's target dir.

| Invariant | Check | Last result |
|---|---|---|
| [[layering]] every crate depends only on crates below it | `sh scripts/layering.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[ui-never-waits]] the UI thread never waits on IO | `sh scripts/ui-lint.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[modules-through-components]] L7 modules draw only through components | `cargo test -p bbt-ui --locked --test seat_rows module_sources_and_legacy_adapter_cannot_hand_draw_rows` | runner pass at 0bab92f, 2026-10-22 |
| [[parity-geometry]] web and native geometry agree for every parity case | `sh scripts/parity.sh` (slow; under `ci-local.sh --parity`) | P1 on main at 8e66b3a, 2026-10-22 |
| [[frontmatter-parses]] every .bboxy frontmatter parses | `cargo test -p bbt-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact` | held, 2026-10-22 (W3.5b parser) |
| [[seats-1000-bus]] an Oracle broadcast reaches 1000 seats within budget | `GATE_SEATS=1000 GATE_BUDGET=13299 sh scripts/gates.sh bus` (slow) | p99 median 10.6 s at 22f5956, 2026-10-22 |
| [[seats-400-memory-idle]] 400 idle seats fit in 420 MiB | `GATE_SEATS=400 GATE_RATE=idle GATE_SECONDS=20 GATE_BUDGET=420 sh scripts/gates.sh memory` (slow) | median 333.7 MiB, 2026-10-22 |
| [[signed-main]] every commit on main carries a good signature | `git log --format='%G?' HEAD \| grep -vc '^G$'` is 0 and the log is non-empty, on the handed-off commit | runner pass at 0bab92f, 2026-10-22 |

A violation halts the work that broke it: the task goes `blocked` with
`[[invariant|violates]]`, and the halt names the invariant, the evidence and
what clears it ([[work-items]] Rules).
