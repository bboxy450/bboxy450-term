---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:20:04.714Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTTN4H5KEF8CJEVET7A8A
number: 5002211
provenance:
  digest: sha256:b24eb16ff77e7beafd7d055e179ca8a48eec122aee625cdf4b04dea6102f957b
  frontmatter_digest: sha256:262afa86aa5e390d617080d978a2e53beccc09fdcf249a1fc837d6e413ec6ac7
  log: .bboxy/log/2026-11-02.jsonl#425
  sig: gQB/JG7iWJ+oQsWeTEAt68dDgjdvozx77B56zFARB0DTn+eoy9U2bbwsk3Im4+mL/xxW3y3jlpHq41tI4zo1Cg==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: knowledge plugin connection ends mid-response (plugin_eof, provider_closed)'
type: task
updated: 2026-11-02T18:20:04.714Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
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
