---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:20:10.280Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTZWBH8DS20H3EQWQBDY5
number: 5002212
provenance:
  digest: sha256:541e9eaec5264a1152307f1e79cc020b06947682aeffd5425bc851f738a90b68
  frontmatter_digest: sha256:227d2a74936463d3e192672555d19d67a2a7471d767e963bc5ce22bc340ce274
  log: .bboxy/log/2026-11-02.jsonl#433
  sig: mbR45MSBzTMRP67qHSnIvuLMKQQvnUD75Jf20YOih6XxyVUsn1YLzmU7BMBDMScqGXHTpPvFH54foLDMaQwaDQ==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: Claude hook events larger than 2 MiB are dropped (hook input bound)'
type: task
updated: 2026-11-02T18:20:10.280Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify: cargo nextest run -p bboxy450-harness-wake-up
---
**Observed.** 11 rows of `hook / bboxy_shim / claude: hook input bound` on 2026-10-30 between 08:35 and 09:47, for example seat 01M66M32E0AJ4VABRH2R4CXWAH.

**Mechanism.** `crates/bboxy450-harness-wake-up/src/hook.rs:23`: `MAX_INPUT = 2 MiB`. Lines 627 and 748: when the input exceeds the limit, the shim returns `invalid("hook input bound")`, and the whole hook event is discarded. Claude hook payloads can carry large tool inputs and outputs, such as a Write of a big file or a long Bash result in PostToolUse. A dropped event leaves the seat's adapter state stale: no activity, turn or subagent transition is recorded for that hook.

**Fix.**
- Stream-parse, or read past the bound while keeping only the fields the adapter uses (event name, session, tool name, ids). Never drop the event.
- Test with a 3 MiB PostToolUse payload: the state transition must still be recorded.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
