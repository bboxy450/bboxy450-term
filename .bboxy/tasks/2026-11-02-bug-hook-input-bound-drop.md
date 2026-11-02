---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:20:10.280Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTZWBH8DS20H3EQWQBDY5
number: 5002212
provenance:
  digest: sha256:541e9eaec5264a1152307f1e79cc020b06947682aeffd5425bc851f738a90b68
  frontmatter_digest: sha256:7b2c8d22574ffc422c2f5538d9cc41174e3754bf756d2bb234dcf91dacb444c6
  log: .bboxy/log/2026-11-02.jsonl#2607
  sig: +JYOwfu2DgaQO2Q8PznQj4I+OV+/Jpj03FNdAnc1dLUXxWiioDz1/SEgFTOV0C2CN9WE2PRbDsm5yq9FstCxBQ==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/30063d3afc22edd4d15b352786f4df52ec80802a
  - https://github.com/bboxy450/bboxy450-term/commit/b681d962a422e2a9ed2abaa2eff39ae161a2e4af
  - 'make land (lean): bboxy450-harness-wake-up --lib 19 passed; 3 MiB PostToolUse prepares within MAX_RAW'
  - 'broken control: MAX_INPUT back to 2 MiB -> test fails with ''hook input bound'''
  fresh_accepted: false
  kind: historical
  op_id: 01M6EXAWCK1KA6VA0K5FX1W03Z
  previous_status: open
  reason: 'Landed: raw hook ingress raised from 2 MiB to 16 MiB; the retained 64 KiB frame and 50 ms deadline are unchanged; README and specs/plugins.md updated.'
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: Claude hook events larger than 2 MiB are dropped (hook input bound)'
type: task
updated: 2026-11-02T19:56:13.744Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
verify: cargo nextest run -p bboxy450-harness-wake-up
---
**Observed.** 11 rows of `hook / bboxy_shim / claude: hook input bound` on 2026-10-30 between 08:35 and 09:47, for example seat 01M66M32E0AJ4VABRH2R4CXWAH.

**Mechanism.** `crates/bboxy450-harness-wake-up/src/hook.rs:23`: `MAX_INPUT = 2 MiB`. Lines 627 and 748: when the input exceeds the limit, the shim returns `invalid("hook input bound")`, and the whole hook event is discarded. Claude hook payloads can carry large tool inputs and outputs, such as a Write of a big file or a long Bash result in PostToolUse. A dropped event leaves the seat's adapter state stale: no activity, turn or subagent transition is recorded for that hook.

**Fix.**
- Stream-parse, or read past the bound while keeping only the fields the adapter uses (event name, session, tool name, ids). Never drop the event.
- Test with a 3 MiB PostToolUse payload: the state transition must still be recorded.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
