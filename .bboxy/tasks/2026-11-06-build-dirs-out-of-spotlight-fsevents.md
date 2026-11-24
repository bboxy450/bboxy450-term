---
allowed_paths:
- scripts/cache-env.sh
- Makefile
- scripts/test-landing.py
- LAND.md
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-06T19:13:58.770Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6S4G9CY5VEA8JYZ1H4HM8DK
number: 93377961366092
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-build-dirs-never-index.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K38JJYJFDZA6BN3GE9XE
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Keep build/evidence trees out of Spotlight indexing (fseventsd at 102% during builds)
type: task
updated: 2026-11-24T17:10:26.172Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: python3 -I scripts/test-landing.py
provenance:
  digest: sha256:248e52f663a01a9fb94e4d8e5a4298786cc7d95968b97e9c5d7d8b86e0bedb36
  frontmatter_digest: sha256:d3629c265a37b0a829ef192b97d67ec613fdd898242551eb00ddfc026763867d
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: 39bB4GQ89YQpfuvaTae4hOWsSNBlPoa6r+DtSmz8AQDlslNqYfP5bFYCfzuBJsvGriazmTA7aah3SyaddWAnBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1859
  event:
    id: 01M878K49DRQ1KHHXFN5P7HAZQ
    sequence: 1859
    hash: cb53ec4983b8072e72641f2e9ba1b62e8660d6830d389d7614bd13fd9044fb7b
---
# Builds feed Spotlight and fseventsd

On 2026-11-06 at 11:13, `fseventsd` was at **102% CPU** while worktree builds and archive writes ran. Every `target/` tree (multi-GiB, thousands of files per build) is also eligible for Spotlight indexing (mds and mdworker), on top of the compile itself.

1. **Opt out of Spotlight.** `scripts/cache-env.sh` (sourced by every Make entry point) creates `target/.metadata_never_index` (and the same for formal-evidence staging directories and the sccache directory), so Spotlight skips them. It's idempotent and cheap: an existence check, then touch.
2. **Measure.** Compare mds/mdworker and fseventsd CPU during a cold `cargo build -p bboxy450-term-mux`, with and without the marker. Use a private target dir and run at background QoS. Report the numbers.
3. **Test.** The landing test asserts the marker exists after any Make build entry point.

Lean landing; write the report `reports/2026-11-06-build-dirs-spotlight.md` and call `task_report` once.
