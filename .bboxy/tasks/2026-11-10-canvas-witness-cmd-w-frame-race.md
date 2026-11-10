---
type: task
title: Canvas native witness races its cmd-w key against a newer UI frame
role: executor
allowed_paths:
- crates/bboxy450-term/tests/headless.rs
- crates/bboxy450-term/tests/headless/canvas.rs
- .bboxy/quarantine.md
- .bboxy/reports/
verify: sh scripts/quarantine-test.sh --locked -p bboxy450-term --test it -E 'test(/headless::canvas::/)'
id: 01M73QS69BWZ551RBBZYAPRZ3G
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T22:03:14.480Z
created: 2026-11-10T22:03:14.480Z
number: 1794292861627480
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:67faca15d592ebc9b051eb73b29a541efc17379d698ceaf3972493a91fb7ba3f
  frontmatter_digest: sha256:e23b4cd9e1b89fa48b7501dd32231745822fbf5e4e7efa36b24863406781ea2b
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: +zT/nTB6Vkm7YS3IO2abQY4PWlG5zH19EbxKSoOxiEHPDWNx4xVKw1G4QZp71DP3YIou2+Mh01K9l2k8uAN3AQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151594
  event:
    id: 01M73QS72JP8JYMR2N9TG000BX
    sequence: 151594
    hash: e226c2936b4b90ab3cd57b1af7baf04de38bff3eecb852c9a76b460873bd0fdd
---
# Canvas native witness races its cmd-w key against a newer UI frame

## Evidence (2026-11-10, main + 82001ca49)

In `make land`, `headless::canvas::canvas_native_mcp_three_batches_restart_sidebar_and_headless_capture`
failed once at `crates/bboxy450-term/tests/headless.rs:123`: the scripted
`["ui", "key", "cmd-w"]` returned
`ui_frame_refused: a newer UI state superseded this operation before its frame`.
The previous landing run passed it, and alone it passed three of three (5.4 to 5.7 s).

The witness sends a UI key while the canvas tab is still producing frames; the host
correctly refuses a key aimed at a superseded frame. The test treats that refusal as a
failure instead of waiting for a settled frame and retrying the same operation.

## Disposition

Quarantined for seven days through `scripts/quarantine.py`; it still runs everywhere and
fails the gate if it also fails alone.

## Done when

The witness waits for the frame it targets (or retries the identical key on
`ui_frame_refused` with the documented superseded-frame reason only), the quarantine row is
removed, and the test passes in five consecutive `make land` runs.

Quarantine evidence (2026-11-10): `headless::canvas::canvas_native_mcp_three_batches_restart_sidebar_and_headless_capture` at `82001ca49e08c8bdfc01c8913773943cb14cf9dd` passed an isolated exact retry; raw `/Users/feral/.local/state/bboxy450-term/landing/quarantine/f9943dc84f9c4ae79ef84eeca2b88f78.log`. Expires 2026-11-17.
