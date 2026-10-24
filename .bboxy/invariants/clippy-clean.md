---
id: 01M5K9FA801MWBMNY19TCPRW6E
number: 147
type: invariant
title: the workspace compiles with zero clippy warnings on every target
status: active
slow: true
gate: ci
check: cargo clippy --workspace --all-targets --locked -- -D warnings
cost: a warning tolerated once becomes a hundred nobody reads, and the one that mattered hides among them
links:
  - { to: "[[runtime]]", rel: constrains }
  - { to: "[[open-source]]", rel: constrains }
created: 2026-10-23T02:29:37Z
updated: 2026-10-23T02:29:37Z
---
# Zero clippy warnings, every target

**Property.** `cargo clippy --workspace --all-targets --locked -- -D warnings`
exits 0: no warning in any crate, test, example or bench (P5, Velvet, on main
at 3adccc6, 2026-10-22). [[ui-never-waits]] is the stricter subset for the UI
crates; this one keeps the rest of the tree at the same bar the public crates
will need ([[open-source]] §2).

**Check.** The command above, run directly by `sh scripts/ci-local.sh` on every
handoff (`scripts/ci-local.sh:12`). Marked `slow` with `gate: ci` so the
invariants runner does not run clippy a second time in the same gate; `--slow`
runs it again, harmlessly.

**Cost of a violation.** A warning tolerated once becomes a hundred nobody
reads, and the one that mattered hides among them.

**Evidence.** Exact clean CI at 3adccc6 (Oracle acceptance, 2026-10-22).

Owner: Corvin Duskmantle <ARCHITECT>.
