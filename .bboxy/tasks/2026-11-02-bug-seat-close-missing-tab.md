---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:19:59.714Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTNFB67X1KM79QVDFZ50R
number: 5002210
provenance:
  digest: sha256:52b7c3aa5782f57bbe24eef44bd1eebdace216e37b6a34f57e4c1fb7d71152de
  frontmatter_digest: sha256:1da79603c297e9d3f3be011c0a2e6ffdcfd288dcd395ee446e7d663ea4426745
  log: .bboxy/log/2026-11-02.jsonl#417
  sig: m/gaCfgMPgKYDT5kr13ixE9+BGPQ2SzVzy+zSwCVUcR9a83Y8Gdr6rL/KKCSsP2X+FHYnz+1iZBczCNdhk0jAA==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl seat close fails with ui_frame_refused: MissingTab'
type: task
updated: 2026-11-02T18:19:59.714Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify: cargo nextest run -p bbt-ui-model --test input
---
**Observed (2026-11-02).** The `ctl seat/close` operation failed 4 times:
- two rows with `ui_frame_refused: MissingTab` at 08:59;
- two redacted rows at 07:30 and 07:34.

**Mechanism.** `crates/bbt-ui-model/src/model/window.rs:243,370,375`: `tab_of(seat)` returns `EditError::MissingTab` when the window model has no tab for the seat. Examples are a seat that is live in the registry but has no pane in the current window layout (detached or headless), or one whose tab was already removed. The close is then refused, and the seat cannot be closed through ctl.

**Expected.** Closing a live seat with no tab in the window should still close it through the backend, as the MCP `seat_close` in `3e79499f` (landed 09:45, after these failures) is meant to. Otherwise the refusal should name the reason and the fallback.

**Check first.** Whether `3e79499f` already covers the tab-less case. If so, add the regression test and close this record.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
