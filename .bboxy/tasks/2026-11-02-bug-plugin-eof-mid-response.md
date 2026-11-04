---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:20:04.714Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTTN4H5KEF8CJEVET7A8A
number: 5002211
provenance:
  digest: sha256:b24eb16ff77e7beafd7d055e179ca8a48eec122aee625cdf4b04dea6102f957b
  frontmatter_digest: sha256:cddcb303c6717f0aa84844a02cfde16921683d07aa185afca6f440f2c6e7a814
  log: .bboxy/log/2026-11-02.jsonl#8325
  sig: bmV+yqiaOIuDWzva1E+ILWePLTrthsA0X2i0N143Yh8DNajeXgp/OdoI0RhI+4ula+3yLONqq7VhdZajM7JwAA==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/863a66cf
  - 'cargo nextest run -p bboxy450-term-mux --lib a_plugin_that_exits_mid_call_reports_its_exit_status: passed (exit status: 3 fixture)'
  - 'cargo clippy -p bboxy450-term-mux --all-targets -D warnings: passed'
  fresh_accepted: false
  kind: historical
  op_id: 01M6F1D2SYQTQKB1F8ABZQX0FW
  previous_status: open
  reason: Client::shutdown journals plugin_exit with the child's own failing exit status when it ended before the host stopped it, so the next plugin_eof names its cause. The underlying crash cause of the 11-01/11-02 rows is unknown until it recurs with this diagnostic.
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: knowledge plugin connection ends mid-response (plugin_eof, provider_closed)'
type: task
updated: 2026-11-02T21:07:20.087Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
verify: cargo nextest run -p bboxy450-plugin
---
**Observed.** Three paired failures from the plugin host, at 2026-11-01 15:29 (pid 80568), later on 11-01, and 2026-11-02 07:03. Each pair is:
- `plugin_eof: stdout ended before a complete response`;
- `provider_closed: this plugin connection is closed`, with the same timestamp.

Separately, 9 `deadline_exceeded: tools/call exceeded its fixed deadline` rows appeared on 2026-11-01 between 15:25 and 17:04. These are probably the cold GitKB index builds addressed by `90e8dfc5`, `fa63023d` and `33f12e7a`. None has appeared since 11-01 17:04.

**Unknown.** Whether the plugin process crashed, was killed by the host after a deadline, or exited cleanly mid-call. stderr was empty in every row.

**Fix.**
- Record the plugin child's exit status or signal and the in-flight tool name on `plugin_eof`, so the next occurrence names its cause.
- Then fix the cause. A plugin that dies mid-call should be respawned once for idempotent reads, with an explicit refusal returned for writes.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
