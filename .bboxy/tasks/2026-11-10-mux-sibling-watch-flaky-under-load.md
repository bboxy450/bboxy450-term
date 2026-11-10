---
type: task
title: 'mux index watcher test times out under full-suite load: sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation'
role: executor
allowed_paths:
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/logic/workspace_routing_tests.rs
- crates/bbt-mux/src/logic.rs
- .bboxy/quarantine.md
- .bboxy/reports/
verify: cargo nextest run --locked -p bboxy450-term-mux --lib -E 'test(sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation) | test(deferred_scan_replaces_failed_watcher_and_observes_later_edits)'
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
id: 01M73PJDFM5X80SEQ7KND5R5SE
number: 1794292861627479
created: 2026-11-10T21:42:04.277Z
updated: 2026-11-10T22:13:24.521Z
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
verify_revision: 1
provenance:
  digest: sha256:7022cb1f16bfa22617bd558d4940f0ee1b9ab3b30d9e20865d0e80ab0cabd744
  frontmatter_digest: sha256:d4e977cdd69a24db9b9e0a45063bb0fd9f694ad632da47a9f0eb7fa49c8713ef
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: VwTsN9xTPTW2dVy53bsCvNCAKOoMxk/C12UHJjvsE6DIAegoqct+4FGz+u+un5kCWI7p6mslmYi8GdQBoMPKDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151596
  event:
    id: 01M73RBTGZ4469RPKZ3CSWW2B9
    sequence: 151596
    hash: 1483b503ec921a5530bfce8a780c46ed8af68a7941214f9adb809018cce5bc06
---
# mux file-watcher tests time out under machine-wide build load

Two `bbt-mux` library tests wait on macOS FSEvents with a 30 s deadline and fail when the
machine is building heavily:

| Test | In `make land` | Alone |
|---|---|---|
| `index::tests::sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation` | failed at 30.1, 30.1, 35.1 s (three runs, 2026-11-10) | 3/3 passed, 2.7–6.8 s |
| `logic::workspace_routing_tests::deferred_scan_replaces_failed_watcher_and_observes_later_edits` | failed at 30.2 and 30.4 s (2026-11-10) | 3/3 passed, 0.6–2.7 s |

nextest already runs two tests at a time, so the delay is not these tests competing with
each other: FSEvents is machine-wide, and every build on the machine floods it
(`fseventsd` has been seen at 100% CPU during gate runs).

## Disposition

Both quarantined for seven days through `scripts/quarantine.py` (still run everywhere; fail
the gate only if they also fail alone).

## Done when

Neither test depends on event latency under load: for example they assert on the watcher's
own processed-event state instead of a wall-clock wait, or the deadline follows the
watcher's measured latency. Both quarantine rows are removed in the same change.
