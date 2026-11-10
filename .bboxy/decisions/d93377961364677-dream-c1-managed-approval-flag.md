---
type: decision
title: Preserve the managed launcher's existing approval flag in the private C1 witness
id: 01M6P3CBHF3BP0A40ZJ2V1Y38R
updated: 2026-11-05T14:56:34.680Z
created: 2026-11-05T14:56:34.680Z
number: 93377961364677
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:db9d87469934ad4f25fd401edf13642cd8a93f3f6e377477e39b587315aa630c
  frontmatter_digest: sha256:61f9b584dd0e6f4cf759970a738d594ba9d7d0b9dda229c30299e08152d302d7
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: mGn3Kt6LOZlmmvJ05DrXyi3I1O27S5UU+FwNkrUIUIh35aCxJ0EADkes2sZOBxCe2sV/EDxOHqVywWLaMKsyBg==
  log: .bboxy/log/2026-11-05.jsonl#137017
---
## Choice and source
Supplement [[decisions/d93377961364648-dream-c1-managed-fixture-inputs]]: preserve the managed launcher's existing explicit --yolo flag in the retained-Host PRIVATE fixture. Term crates/bboxy450-harness-wake-up/src/launch.rs:231-233 automatically appends it for bboxy450-harness. Producer src/frontends/headless.rs:97-113 currently rejects it in the fixture-only argv allowlist, although the ordinary parser:170 and policy/launch propagation:480-481,533 already implement it. Removing it would test a different managed launch.

Allow that exact existing option through the private argv gate and apply its NORMAL policy and LaunchOptions.yolo semantics for both retained sessions. This is explicit launcher approval intent, never identity/seat authority. Do not infer it from environment, force it when absent, introduce another bypass/alias, broaden arbitrary options, rewrite policy or alter normal product behavior. Existing test-fixtures compile feature + explicit seat/json/scripted/TermC1 + private validated paths/profile/MCP/environment + exactly one fresh nonfork next session remain required. ScriptedProvider stays forced; no live provider, credentials, network, ACP/resume/replay or ambient fallback. The separate inspect-c1 worker's minimal argv/environment remain unchanged.

## Scope and proof
Both current C1 tasks may preserve this exact managed option within their existing allowed headless.rs/seat_process.rs and term private join-fixture paths. Add --yolo to the actual managed joined positive and verify the ordinary approval-intent/launch behavior survives A→B; keep the absent-flag behavior and invalid feature/mode/path/unknown-option negatives. A malformed or unguarded private fixture must still refuse even when --yolo is supplied. Run only the affected focused witness before the existing final C1 gate; no limit/schema/source eligibility change. Retain all CPU/duty/heap/control/foreground/settlement gates. This authorizes fixture compatibility only and does not attest a passing final joined test.
