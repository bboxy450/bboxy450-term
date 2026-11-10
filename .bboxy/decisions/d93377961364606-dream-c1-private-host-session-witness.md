---
type: decision
title: Use the existing test-fixtures feature for a private retained-Host session transition witness
id: 01M6NYX1124Y1P839840F9KET2
updated: 2026-11-05T13:38:18.107Z
created: 2026-11-05T13:38:18.107Z
number: 93377961364606
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:09024302aee9b5ba5a390d6bc4a3bfbd968b14e528ff6e6b53a885b3e1f7ce1d
  frontmatter_digest: sha256:bd0f52032f2244b9b0f508c91cfc2be1731c9b3434602a8362535ebe56f30b0c
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: EQLotTWC0PNjNpVwjGCIUs09Qtvb34YRxerQp5ucQoUFK6bZ/KFBHWF6T6rtDYpYdKcUKMyMEy0TMPODg3cOBw==
  log: .bboxy/log/2026-11-05.jsonl#136072
---
# C1 same-generation pause witness

Existing ordinary headless/seat execution exits and joins its Host after durable SessionClosed (src/frontends/headless.rs around :596-617). Profile switching forks; C1 explicitly excludes fork sources. This does not currently expose a normal close-A/create-nonfork-B CLI route in one PID/seat generation. Existing Host::create/create_with_mcp supports creating a fresh non-forked session in the retained Host, and Cargo.toml already defines test-fixtures.

Choice: permit the proposed --dream-c1-fixture-next-session ULID only under the existing test-fixtures compile feature AND explicit seat + json + scripted provider + DreamOwner::TermC1 fixture mode. Exactly one fresh non-forked B may be created via the real retained Host after A's durable close is published, with B exposed on the actual owned socket. Reject the flag otherwise. Use private fixture directories/accounts only; no ambient profile/capability/credential fallback. Default builds and ordinary close behavior are unchanged. No new feature/dependency/IR/schema or product session-retention API is introduced.

The focused Make gate may enable the EXISTING test-fixtures feature for this private witness. It must label the result as a retained-Host API/transport same-generation pause witness, not claim ordinary production CLI supports that transition. Use actual Host/Gateway/Store/socket paths and the real C1 worker; no synthetic typed event substitution.

Keep the ordinary production close/process-exit and seat-restart/generation-cancellation witnesses separately. The special fixture must not replace proof that real generation changes cancel/fence older work. Rejected flag combinations and exactly-one-transition behavior are tested, and existing close/join/cleanup behavior stays intact.

Governs the witnesses in [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]], implemented within the existing headless/seat_process paths for [[tasks/2026-11-05-dream-c1-producer]] and joined by [[tasks/2026-11-05-dream-c1-consumer]].