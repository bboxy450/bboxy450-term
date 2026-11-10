---
type: task
title: 'mux index watcher test times out under full-suite load: sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation'
role: executor
allowed_paths:
- crates/bbt-mux/src/index.rs
- .bboxy/quarantine.md
- .bboxy/reports/
verify: cargo nextest run --locked -p bboxy450-term-mux --lib -E 'test(sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation)'
id: 01M73PJDFM5X80SEQ7KND5R5SE
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T21:42:04.277Z
created: 2026-11-10T21:42:04.277Z
number: 1794292861627479
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:83c3fc5e55afa7b365bf912c01aec39ef9c60d052bbde916e9cc40209645f899
  frontmatter_digest: sha256:58eff4703305b7fbf373dab6d7cbfe9caa453bb2df1b3d567c0b07587794a6d3
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: 1BXiefZal1COpWAlAyAP56NalJLF5CeiDraakiYWgqaXLPQXTBoKVuy4AbfIjAcZ8Itu+qxMCv4iqlS29BHdBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151592
  event:
    id: 01M73PJEYQTZEJ9MFNMQ77KN54
    sequence: 151592
    hash: 8edf31c14401a6d99d8792fd14f850148692503526b720a9ec90708ac878f99d
---
# mux index watcher test times out under full-suite load

## Evidence (2026-11-10, main + ccc453264)

- In `make land`, `bboxy450-term-mux --lib` (401 tests, debug half of the Dream C1 split):
  `index::tests::sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation`
  failed at 30.156 s; 400 others passed. Another session's `make land-full` was running;
  load average 18 to 24.
- Alone, same worktree: passed three of three in 2.7 s, 6.8 s and 4.6 s (load 14 to 18).

The test waits for macOS FSEvents (`wait_for_change`, `settle`) with a 30 s deadline.
Under heavy build churn `fseventsd` has been seen at 100% CPU, so events arrive late.

## Disposition

Quarantined for seven days through `scripts/quarantine.py` (still runs everywhere; fails
the gate only if it also fails alone).

## Done when

The test no longer depends on event latency under load: for example it asserts on the
watcher's own processed-event state instead of a wall-clock wait, or the deadline is tied
to the watcher's measured latency. The quarantine entry is removed in the same change.

Quarantine evidence (2026-11-10): `index::tests::sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation` at `f23a4df791a569a2e9f276b03e0c3f8afea328dc` passed an isolated exact retry; raw `/Users/feral/.local/state/bboxy450-term/landing/quarantine/2d508055a9944360875f8e92ab69b412.log`. Expires 2026-11-17.
