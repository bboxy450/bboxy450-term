---
type: task
title: Keep build/evidence trees out of Spotlight indexing (fseventsd at 102% during builds)
role: executor
allowed_paths:
- scripts/cache-env.sh
- Makefile
- scripts/test-landing.py
- LAND.md
- .bboxy/reports/
verify: python3 -I scripts/test-landing.py
status: open
id: 01M6S4G9CY5VEA8JYZ1H4HM8DK
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T19:13:58.770Z
created: 2026-11-06T19:13:58.770Z
number: 93377961366092
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:248e52f663a01a9fb94e4d8e5a4298786cc7d95968b97e9c5d7d8b86e0bedb36
  frontmatter_digest: sha256:7e85569fdcce2752d93986a88dbeaa6a9925e767182f2efaf33e06b0cd908fd9
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: lEadMRg6KqoGUHtLN7WMyIgwmye6xer0Hd0VaODSatDjmzj4EiU/qZ/vJ8NU/lBh7Dz15yV9p1icAaU9mZt6CA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149761
  event:
    id: 01M6S4GD3PGC7TZM60ZM1JGR91
    sequence: 149761
    hash: 584a945ca19cb970bd957db56527bcfeb0d81244f154f3a447690ba020852b6b
---
# Builds feed Spotlight and fseventsd

On 2026-11-06 at 11:13, `fseventsd` was at **102% CPU** while worktree builds and archive writes ran. Every `target/` tree (multi-GiB, thousands of files per build) is also eligible for Spotlight indexing (mds and mdworker), on top of the compile itself.

1. **Opt out of Spotlight.** `scripts/cache-env.sh` (sourced by every Make entry point) creates `target/.metadata_never_index` (and the same for formal-evidence staging directories and the sccache directory), so Spotlight skips them. It's idempotent and cheap: an existence check, then touch.
2. **Measure.** Compare mds/mdworker and fseventsd CPU during a cold `cargo build -p bboxy450-term-mux`, with and without the marker. Use a private target dir and run at background QoS. Report the numbers.
3. **Test.** The landing test asserts the marker exists after any Make build entry point.

Lean landing; write the report `reports/2026-11-06-build-dirs-spotlight.md` and call `task_report` once.
