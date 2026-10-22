# Invariants — the law the day runs under

One file per invariant, type `invariant` ([[work-items]]): property, check
command, the exit status that means held, cost of a violation. No command, no
invariant. Owner: the Architect; anyone runs the checks; `sh scripts/ci-local.sh`
runs layering and ui-lint before every handoff and parity-geometry under `--parity`; `scripts/invariants.sh` (P1 follow-up) will run every `check:` line here, so each line is one command, side-effect free apart from cargo's target dir.

| Invariant | Check | Last result |
|---|---|---|
| [[layering]] every crate depends only on crates below it | `sh scripts/layering.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[ui-never-waits]] the UI thread never waits on IO | `sh scripts/ui-lint.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[modules-through-components]] L7 modules draw only through components | `cargo test -p bbt-ui --locked --test seat_rows module_sources_and_legacy_adapter_cannot_hand_draw_rows` | runner pass at 0bab92f, 2026-10-22 |
| [[parity-geometry]] web and native geometry agree for every parity case | `sh scripts/parity.sh` (slow; under `ci-local.sh --parity`) | P1 on main at 8e66b3a, 2026-10-22 |
| [[signed-main]] every commit on main carries a good signature | `git log --format='%G?' HEAD \| grep -vc '^G$'` is 0 and the log is non-empty, on the handed-off commit | runner pass at 0bab92f, 2026-10-22 |

A violation halts the work that broke it: the task goes `blocked` with
`[[invariant|violates]]`, and the halt names the invariant, the evidence and
what clears it ([[work-items]] Rules).
