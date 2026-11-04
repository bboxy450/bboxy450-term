---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:20:15.814Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQV4G2SPVFSVXWSMTPC1K3
number: 5002213
provenance:
  digest: sha256:8c7e3e935429fe910bcd86e820a1255a09653840d905a9bb5a79f8bce4ed6b18
  frontmatter_digest: sha256:6da9c7a73dd1259488179b811435df69716402be6baaa00bc3deed7591e819e3
  log: .bboxy/log/2026-11-02.jsonl#2614
  sig: 1shBViKmkuE31EiuZOXPRNDqtBOeEYtPRYcqoqkJCgHizdwTZEqBmyPDPXMJ86BqMCxwfYFgnDMLCuJAMm2fDg==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/942780288ddb645614ab7813fb7f5e23d2a4e81f
  - 'mechanism: main.rs poll_ui_frame previously ran only from drain() (backend wakes); SidebarSection readiness waits for animation end (main.rs ui_effect_ready)'
  fresh_accepted: false
  kind: historical
  op_id: 01M6EXAZ6A1R65VTRK5BXCF90D
  previous_status: open
  reason: 'Landed: the UI frame waiter is re-polled after every produced frame, so an effect that settles during painting (sidebar animation end) without a backend wake still sets its frame target. Gap: no live-animation regression test; covered by clippy and the existing frozen-clock ctl tests only.'
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl ui click toggling the Tasks sidebar section times out (UI frame incomplete)'
type: task
updated: 2026-11-02T19:56:16.630Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
verify: cargo nextest run -p bboxy450-term --test ctl
---
**Observed (2026-11-01 16:12).** Two `ctl ui/click` failures:
- `unknown: timeout: UI frame incomplete; target=None, last=104, pending=0, effects=["SidebarSection { key: \"panels/Tasks\", open: false }=Ok(true)"]`
- the same, with `last=120` and `open: true`.

**Reading.** The click took effect (`Ok(true)`) and nothing was pending (`pending=0`), yet the acknowledgement frame never satisfied the wait. Because `target=None`, the click had no frame-sequence target to wait for. As a result, ctl reported `unknown`/timeout for a successful edit.

**Fix.** When an applied effect leaves no pending work, acknowledge on the next presented frame, or immediately when no redraw is needed. Do not wait for a target that was never set. Add a ctl test that toggles a sidebar section twice and expects two `ok` replies.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
