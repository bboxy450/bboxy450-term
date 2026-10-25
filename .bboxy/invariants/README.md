---
id: "3T6C0NK1JHAAQ8EKP0ZVD4PEKC"
type: "note"
number: 138
title: "Invariants — the law the day runs under"
created: "2026-10-22T21:14:24Z"
updated: "2026-10-25T09:54:49Z"
---
# Invariants — the law the day runs under

One file per invariant, type `invariant` ([[work-items]]): property, check
command, the exit status that means held, cost of a violation. No command, no
invariant. Owner: the Architect; anyone runs the checks; `sh scripts/ci-local.sh`
runs every invariant before every handoff, except those marked `slow: true` in frontmatter, which run under `--slow` (all of them) or `--parity` (only `gate: parity`); the seat gates carry `gate: seats`, `gate: ci` marks a check ci-local already runs directly, and `gate: pending-<task>` marks one whose test the named task has yet to create (skipped by default, fails by design under `--slow`, joins the default set when the task lands); `scripts/invariants.sh` (P1 follow-up) will run every `check:` line here, so each line is one command, side-effect free apart from cargo's target dir.

| Invariant | Check | Last result |
|---|---|---|
| [[layering]] every crate depends only on crates below it | `sh scripts/layering.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[cross-os-facade]] raw OS bindings live only in per-OS crates behind a facade | `sh scripts/cross-os.sh` | held at the adding commit (three dated exceptions) |
| [[ui-never-waits]] the UI thread never waits on IO | `sh scripts/ui-lint.sh` | runner pass at 0bab92f, 2026-10-22 |
| [[modules-through-components]] L7 modules draw only through components | `cargo test -p bbt-ui --locked --test seat_rows module_sources_and_legacy_adapter_cannot_hand_draw_rows` | runner pass at 0bab92f, 2026-10-22 |
| [[parity-geometry]] web and native geometry agree for every parity case | `sh scripts/parity.sh` (slow; under `ci-local.sh --parity`) | P3 at dd47f4e: 1704 cases, 0 unproven, 0 mismatches, 2026-10-23 |
| [[frontmatter-parses]] every .bboxy frontmatter parses | `cargo test -p bbt-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact` | held, 2026-10-22 (W3.5b parser) |
| [[seats-1000-bus]] an Oracle broadcast reaches 1000 seats within budget | `GATE_SEATS=1000 GATE_BUDGET=13299 sh scripts/gates.sh bus` (slow) | p99 median 10.6 s at 22f5956, 2026-10-22 |
| [[seats-400-memory-idle]] 400 idle seats fit in 420 MiB | `GATE_SEATS=400 GATE_RATE=idle GATE_SECONDS=20 GATE_BUDGET=420 sh scripts/gates.sh memory` (slow) | median 333.7 MiB, 2026-10-22 |
| [[clippy-clean]] zero clippy warnings on every target | `cargo clippy --workspace --all-targets --locked -- -D warnings` (run by ci-local itself; `gate: ci`) | clean at 3adccc6, 2026-10-22 |
| [[mermaid-fixtures]] every .bboxy Mermaid block has a fixture the engine lays out | `cargo test -p bbt-mermaid --locked every_bboxy_mermaid_block_has_a_fixture -- --exact` | held, 2026-10-22 |
| [[kitty-keyboard]] every key encoded once, kitty protocol against the seat's live mode | `cargo test -p bbt-core --locked --lib kitty_keyboard_invariant -- --exact \| grep -q 'ok. 1 passed; 0 failed'` | held at the K1 merge 71d7204, 2026-10-23; in the default set |
| [[rim-never-covers-text]] the glow rim never covers a text cell | `cargo test -p bbt-ui --locked --lib rim_never_covers_text -- --exact \| grep -q 'ok. 1 passed; 0 failed'` (pending G1) | none yet |
| [[rim-size-is-layout]] rim size moves only the gutter and the stage inset | `cargo test -p bbt-ui --locked --lib rim_size_moves_only_gutter_and_inset -- --exact \| grep -q 'ok. 1 passed; 0 failed'` (pending G1) | none yet |
| [[glow-never-moves-layout]] glow settings never move layout | `cargo test -p bbt-ui --locked --lib glow_never_moves_layout -- --exact \| grep -q 'ok. 1 passed; 0 failed'` (pending G1) | none yet |
| [[signed-main]] every commit on main carries a good signature | `git log --format='%G?' HEAD \| grep -vc '^G$'` is 0 and the log is non-empty, on the handed-off commit | runner pass at 0bab92f, 2026-10-22 |
| [[bboxy450-prefix]] everything we publish is branded bboxy450- | `sh scripts/brand.sh` | held at the adding commit |
| [[images-resolve]] the face is the identity file and every image wikilink resolves | `sh scripts/images.sh` | held at the adding commit |
| [[restart-keeps-state]] a restart of the app loses no working state | `cargo test -p bboxy450-term --locked --test recovery restart_keeps_working_state -- --exact` | pending RS1 (slow, gated) |
| [[local-first]] the mux depends on no remote transport until the cloud phase opens | `sh scripts/local-first.sh` | held at the adding commit |
| [[no-copyleft-deps]] every dependency offers a permissive licence | `sh scripts/deps-licences.sh` | held at the adding commit, 483 deps |
| [[seats-967-frame]] the sidebar and 50 watched panes hold frame time at 967 seats | `GATE_SEATS=967 GATE_WATCHED=50 GATE_SECONDS=10 GATE_BUDGET=8.33 sh scripts/gates.sh frame` | pending a settled run (slow, gated) |
| [[checkpoint-bounded]] crash recovery work per seat stays under the checkpoint bound | `cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact` | pending R5g (slow, gated) |
| [[ctl-opt-in]] no gui.endpoint exists unless ctl is enabled, and the app's own seats are refused at the door | `cargo test -p bbt-mux --locked --test gui_endpoint ctl_endpoint_opt_in_and_ancestry_refusal -- --exact` | pending A-1 (slow, gated) |
| [[idle-swarms-cost-nothing]] the message pump's cost is independent of the number of idle swarms | `cargo test -p bbt-mux --locked --test logic pump_scans_only_dirty_swarms -- --exact` | pending R5i (slow, gated) |
| [[ctl-never-prints-secrets]] ctl never prints a capability or a Keychain secret | `cargo test -p bboxy450-term --locked --test ctl ctl_never_prints_secrets -- --exact` | pending C-0 (slow, gated) |
| [[watched-replay-priority]] watched seats get at least half of the replay pages when both lanes are runnable | `cargo test -p bbt-mux --locked --test logic watched_lane_gets_at_least_half_of_replay_pages -- --exact` | pending R5h (slow, gated) |
| [[no-seat-starves]] no seat starves under a full retained-notification queue; every pending seat admitted within one rotation of the retry sweep | `cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::no_seat_starves_under_full_queue -- --exact` | active; retained retry turn and deterministic credit-count regression, 95ba7ba2 |
| [[e2e-runs-unattended]] the e2e night runs unattended: no operator input, no login-Keychain access, no window, no build or signature | `sh scripts/e2e-unattended.sh` | pending U-3 (slow, gated) |
| [[never-block-swarm]] never block the swarm: no open task is parked on a person, and no test needs the operator | `sh scripts/never-block.sh` | held at the adding commit |
| [[messaging-never-held]] a custody hold never holds local delivery | `cargo test -p bbt-mux --locked --test messaging_flow custody_hold_never_holds_local_delivery -- --exact` | pending NG1 (slow, gated) |
| [[ring-reads-batched]] a catch-up pass over N seats costs a bounded number of supervisor round trips, never one RPC per seat | `cargo test -p bbt-mux --locked --test logic ring_reads_are_batched -- --exact` | pending Sable's batched ReadRing (slow, gated) |
| [[no-chokepoints]] no bottlenecks and no chokepoints, in the swarm and in the product | `sh scripts/never-block.sh && sh scripts/cross-os.sh` | held at the adding commit; product instances gated by #268 and #276 |

A violation halts the work that broke it: the task goes `blocked` with
`[[invariant|violates]]`, and the halt names the invariant, the evidence and
what clears it ([[work-items]] Rules).
