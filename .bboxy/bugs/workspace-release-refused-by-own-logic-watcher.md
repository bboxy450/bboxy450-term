---
type: bug
title: workspace_release refuses finished checkouts because the host's own logic process holds them open
id: 01M73DJQG8DPA0QWKVT4H856F1
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T19:04:57.129Z
created: 2026-11-10T19:04:57.129Z
number: 1794292861627474
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:b93c10b07a112becadabffcf387b831b958d5ed65f712f128034724398ed9548
  frontmatter_digest: sha256:0b04f47cd1129ce3b11e2129a2ccf2b310d1b80b0fe63040f31471b1da0b376a
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: 4CG+1hJ2Xko7kptXxtmNQXeUxwA/5fn4UBMosrYoiYWv+118iaJzNTIFuSk7PVaaJCC9jxsvcgEWTCi2xtEjBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151578
  event:
    id: 01M73DJRCTKTKGC4R2QTFNWST8
    sequence: 151578
    hash: a126f09bfc1d3952e83fd9a658d1f299055b6057e0c545a2a4db96f4fd1369da
---
# workspace_release is refused by the host's own directory handles

## Expected

After a task is done and its writer seat is closed, `workspace_release` removes the
checkout ([[AGENTS]] close-out).

## Actual

`workspace_release` ends in `failed` with
`release: workspace_in_use: an external process still has the checkout open`.
The only process holding the checkout is the term's own `logic` process.

## Evidence (2026-11-10)

`lsof -p 36049` (`bboxy450-term logic`, up 12 h 48 min) holds a read directory
descriptor (`DIR`, `r`) on six checkouts under
`~/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/`,
done and live alike:

| Checkout | Task state |
|---|---|
| `01M71H0AXBF5TJ8MTFSQ5V7TQ0` | done, seat closed |
| `01M72702JBKZAS94A1EY0RJM15` | done, seat closed |
| `01M731RMKDTSJCR4MFJXVV37WA` | done |
| `01M729Q7H2C552842JRYFT241E`, `01M71S2YMCA8J1W10GTXQGPW2W`, `01M71TWAG4GTF1G271KJ4HBZS3` | other sessions |

`lsof +D` on the first two checkouts shows no other process.

## Reading

The host watches each workspace directory (likely the knowledge/source watcher) and never
drops the watch when the task finishes. The release check counts the host's own handle as
an external user.

## Fix

Either drop the watch when a workspace's task reaches done or its writer closes, before
release runs; or exclude the host's own pid from the "open by an external process" check.
The first is better: a done workspace has nothing to watch.

## Related

`seat_close` on an open, idle seat (`01M731RY7V5R0Y66H8G05M8EEJ`) is refused with
`ui_frame_refused: seat has no tab in this window`, so that seat's workspace
`01M731RMKDTSJCR4MFJXVV37WA` cannot pass the writer-closed step either.
