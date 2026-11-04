---
type: report
title: Actor capability admission for shared image attachments
task: '[[01M6KKWJVF71S4JT0NNF193EWJ]]'
commits:
- 4a940bab010a474278d2fb5fc228b68e3cfbfa5c
supersedes: '[[reports/2026-11-04-harness-attachments-delivery]]'
verify:
  command: make verify-harness-attachments
  exit_code: 0
author: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
created: 2026-11-04T17:08:16.684Z
id: 01M6KRGRH04X1MNQ36EMNN59MG
updated: 2026-11-04T17:08:16.684Z
number: 93377961361070
created_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
updated_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
authorship:
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  kind: seat
  public_key: S3I9J2cN2HkGGxi3t4PZvVBpIllzItN0PkRADkjVqvs=
  process_generation: '1'
provenance:
  digest: sha256:7379179ca4c8f9767f44bddba6b94f64bf865c20ae6a9ed1684beac2da7cd715
  frontmatter_digest: sha256:daf37444c12f543b1d997620d9ec4ec9eaeb20208f44891f7f6b34af23bdcf90
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  sig: AqQ7uSn1o/2uTV8AdppfJzItagllaK+8v0UsJjUaRQIqhS5XE5VLhQj+gpIca1BW5r/plMUitrCzMRuq2vjCCw==
  log: .bboxy/log/2026-11-04.jsonl#121647
---
The older-actor boundary is now gated before any local image UserTurn. This supersedes [[reports/2026-11-04-harness-attachments-delivery]] for the current source and adds the requested absent-capability proof. Producer source: `f2022dee24c344c5eee487a7a1dff28ae5ad7c42`; consumer source: `4a940bab010a474278d2fb5fc228b68e3cfbfa5c`. Both are signed Boxy commits using `9B07E849480C68328CA20684844ECC0587DE7AB0`.

- Producer `src/protocol/mod.rs:71`, `:184`, `src/engine/mod.rs:507` and `src/backend/acp_agent.rs:498` — an actor-level QueryInputCapabilities operation receives an explicit ephemeral InputCapabilities response naming its fresh challenge and image-custody protocol version. This does not create a model prompt, change launcher/MCP context or grant filesystem/approval authority.
- Producer `src/frontends/gui/bridge.rs:694`, `:716`, `:808` and `:970` — every live connection gets a fresh challenge bound to session and epoch. Image intake requires version 1, the matching live response and the same rendered/current proof. Acknowledgements, durable replay, wrong session/epoch and a prior reconnect challenge cannot grant support. The gate runs before registering pending intent or transferring custody. Non-image text delivery does not require this capability.
- Producer `src/frontends/gui/view.rs:2053` — absent/old actor capability produces an explicit “not advertised image intake support” notice and retains the exact image draft/custody. No unsupported UserTurn is sent. A missing connection keeps the existing unavailable-Bridge error behavior.
- Producer `tests/harness_attachments.rs:490` — private old-wire actor acknowledges the query but advertises no intake support. Captured UserTurn count stays zero; the original draft/attachment remains unchanged, no pending UserTurn is created, the direct Bridge path also refuses, and retained image bytes remain readable.
- Producer `src/frontends/gui/bridge.rs:2016` — production capability reducer rejects durable replay, stale epochs, unmatched challenges and old proof after a same-session reconnect.
- Producer `tests/gui_bridge.rs:487` — the existing withheld-user-ack/replacement-session fixture now acknowledges the harmless capability query independently. Its original stale-outcome/detach assertion still passes.
- Consumer `crates/bboxy450-term/src/rich_pane.rs` and its named native test expose/await the actual actor proof before positive image submission. The shared backend was never restarted or controlled.

`make verify-harness-attachments` exited **0** on the final stable source. All 53 source hashes were compared independently with both working files and the signed source-commit bytes. The committed receipt `.bboxy/runs/codex-harness-attachments.json` contains every command, actual exit, log hash, source hash, tool pin and graph verdict. Producer manifest SHA256: `4df0d02e40ac46f21a0aed2de112b987928adacbd33ba39e78c471faf9848ba6`; consumer manifest SHA256: `618b67952413c151e1a7da048d9700fd1fa516a568fc638fce681bdca9e8e400`. Manifests hash sorted compact JSON of each repository-relative path → SHA256 map.

| Focused check | Actual result |
|---|---|
| Private composer/real ACP/old-actor cases | 7 passed; exit 0 |
| Causal pending, unknown custody and stale capability proof | 3 passed; exit 0 |
| Existing stale-dispatch/reconnect/detach case | 1 passed; exit 0 |
| Named finite-model export/control test | 1 passed; exit 0 |
| Producer strict Clippy and embedded-only compilation | both exit 0 |
| Consumer native clipboard/drop/worker/Unix Bridge | 1 passed; exit 0 |
| Private native PNG/TIFF/text pasteboard | 1 passed; exit 0 |
| Consumer strict library/app/named-test Clippy | exit 0 |
| Source stability, signed-commit byte comparison, whitespace checks | exit 0 |

| Model | States / transitions | Native and pinned Java TLC |
|---|---|---|
| HarnessAttachmentIntake | 36 / 66 | pass; TLC exit 0; full graphs equal |
| BrokenHarnessAttachmentIntake | 65 / 116 | expected safety violation; TLC exit 12; full graphs equal |
| HarnessAttachmentCapability | 16 / 47 | pass; TLC exit 0; full graphs equal |
| BrokenHarnessAttachmentCapability | 32 / 114 | expected safety violation; TLC exit 12; full graphs equal |

The new finite capability contract abstracts three distinct connection births, including A/B/A and same-session reconnects. An old actor may acknowledge queries forever without advertising; no response fairness or eventual support is assumed. Reconnect revokes the grant, only a matching live response restores it, and text sending remains enabled. The broken control removes the image-send guard. Protocol framing, actual nonces/session/epoch values, ephemeral admission and retained bytes are implementation witnesses; no arbitrary Rust refinement claim is made. Existing pending/context models remain unchanged.

| Acceptance criterion (verbatim) | Current evidence |
|---|---|
| Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once | All original real InProc/Unix ACP byte, one-prompt, preview/remove and durable-blob cases rerun green with explicit positive actor proof; the old-wire actor receives zero unsupported UserTurns. |
| Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference | The named Unicode/space directory keyboard and file mouse selection case reruns green; actual file content remains in the same private ACP prompt. |
| Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context | Shared composer/real ACP/native worker cases, concurrent paste/later-message cases, once-only bootstrap/content-as-data case, causal pending check and existing stale reconnect/detach check all pass. |
| Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership | New old-actor absent-capability fixture, stale/replayed proof regression, capability model/control, plus existing A/B/A, upstream unsupported-image, missing-file, closed-Bridge and unknown-custody witnesses pass. |
| Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart | Make exit 0; all named checks and strict compilation exit 0; 53 source hashes match signed commits. No operator clipboard read, OS click, paid request or shared-backend action. |

The first new test iteration exposed fixture teardown keeping its own client writer alive and an unavailable-connection assertion taking the new missing-capability branch; both were corrected. A unit fixture initially used an optional test-fixtures-only constructor; it now uses the existing crate-local gateway constructor. The final gate above is green and binds the committed source.

Limits from the original report remain: local same-uid custody, PNG/TIFF clipboard support, bounded PNG/text-file input and admitted-workspace lookup. The old actor is a private wire-compatible absent-capability fixture, not a claimed live old-binary/provider probe. Missing support never auto-sends or drops the image draft; this work does not upgrade or restart an older running actor.

Skipped as instructed: full test/parity/formal/runtime corpora, paid/live providers, OS automation, shared-backend restart, other worktrees, dependency pins, merge, push and install. No raw logs/images or credentials are committed. The producer is clean; the consumer retains only the intentionally unstaged nested gitlink after the report/receipt checkpoint. The earlier report stays append-only.
