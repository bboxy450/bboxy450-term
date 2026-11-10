---
type: decision
title: Classify exact managed public MCP environment references without secret registration
id: 01M6P2M8SH9865VT3J10F5HQFY
updated: 2026-11-05T14:43:25.463Z
created: 2026-11-05T14:43:25.463Z
number: 93377961364668
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:cf708cf5378df3fd32de265a8e5df8ee9a32f4bbd3053e4d39323c31ccaffddf
  frontmatter_digest: sha256:b2a888fe66ae138511e1e63b07d6ce5b30945566e7d13c13614879eb6d0be335
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: bxexxuEvlolmHkowCjMQPOqEQS271CIc3DwFLstUnpgt0PhKAttkw9Wlwee4ocmphU0c+GRgSwCcnp0uMKmyCA==
  log: .bboxy/log/2026-11-05.jsonl#136850
---
## Decision
Source review at producer src/mcp.rs:720-735 shows every resolved environment value is registered as a substring secret. src/redact.rs:99-143 then scrubs serialized keys/strings before deserializing typed IR. The managed launcher forwards BBOXY_GENERATION as public numeric metadata; registering "7" can corrupt a typed ULID and fault the Store actor. The producer reports a deterministic real UserTurn reproduction under /private/tmp/c1-redaction-probe-twxnm8sj; that report is not yet a verified passing fix.

Exempt ONLY host-origin Launcher entries whose key and exact reference match BBOXY_SEAT_ID=$BBOXY_SEAT_ID, BBOXY_GENERATION=$BBOXY_GENERATION or BBOXY_ROLE=$BBOXY_ROLE, and whose resolved value passes its existing public type/shape validation. These public routing labels grant no authority. No blanket BBOXY_* or Launcher exemption. Literal values, command-derived values, renamed aliases, profile/ACP inputs and all other environment values keep existing secret registration and capability guards. Capability contents/file handling remain unchanged. Do not unregister a value already known independently as a secret or weaken the redactor's actual credential behavior.

## Required proof
Add focused source/helper tests covering the exact public-reference allowlist, malformed/unknown/alias/literal/command/profile/ACP controls and actual secret redaction. Repeat the real managed launch through native MCP and typed UserTurn with generation7 and an op_id ending07; assert intact IDs, TurnQueued/TurnStarted and successful settlement rather than merely ACK. Retain existing secret-bearing stderr/spill tests and source mapping/formal coverage for affected existing launch boundary if changed. No forwarding omission, MCP bypass or general redaction redesign.

## Scope
Narrow producer task amendment for src/mcp.rs and tests/mcp_client.rs plus already allowed tests/seat_process.rs; inline source_tests are available. Existing private fixture decision [[decisions/d93377961364648-dream-c1-managed-fixture-inputs]] and all C1 limits/verification remain.