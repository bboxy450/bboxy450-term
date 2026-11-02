---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:19:59.714Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTNFB67X1KM79QVDFZ50R
number: 5002210
provenance:
  digest: sha256:52b7c3aa5782f57bbe24eef44bd1eebdace216e37b6a34f57e4c1fb7d71152de
  frontmatter_digest: sha256:f211a87121027b2fe3560267e7716acd30026510838a5d600825b559dc77a65a
  log: .bboxy/log/2026-11-02.jsonl#8328
  sig: aDU4pLiB1GnupfagSW7A0vXm/KxokjPNrtYiTAaxI6QHYgoBS1blR2wW0puCVr39R4Ca7Jk9wG1qQg9YDCDPCw==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/bc7e9c84
  - 'bboxy450-term --lib --bins: 426/427 passed (close::tests pending asserts included; the one failure was the unbuilt app binary for a live real-process test)'
  - 'cargo clippy -p bboxy450-term --all-targets -D warnings: passed'
  fresh_accepted: false
  kind: historical
  op_id: 01M6F1D5FABJKPMB4ZY9PEGFT6
  previous_status: open
  reason: A repeated close of a seat already closing now refuses with 'a close is already pending or in grace; seat undo-close restores it'; a seat with no tab says 'seat has no tab in this window' instead of MissingTab. Closing a live tab-less seat through the backend is not added (no repro of such a seat).
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl seat close fails with ui_frame_refused: MissingTab'
type: task
updated: 2026-11-02T21:07:22.794Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
verify: cargo nextest run -p bbt-ui-model --test input
---
**Observed (2026-11-02).** The `ctl seat/close` operation failed 4 times:
- two rows with `ui_frame_refused: MissingTab` at 08:59;
- two redacted rows at 07:30 and 07:34.

**Mechanism.** `crates/bbt-ui-model/src/model/window.rs:243,370,375`: `tab_of(seat)` returns `EditError::MissingTab` when the window model has no tab for the seat. Examples are a seat that is live in the registry but has no pane in the current window layout (detached or headless), or one whose tab was already removed. The close is then refused, and the seat cannot be closed through ctl.

**Expected.** Closing a live seat with no tab in the window should still close it through the backend, as the MCP `seat_close` in `3e79499f` (landed 09:45, after these failures) is meant to. Otherwise the refusal should name the reason and the fallback.

**Check first.** Whether `3e79499f` already covers the tab-less case. If so, add the regression test and close this record.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
