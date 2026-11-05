---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T19:00:10.151Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GHBRZEA5GDVD1C028HKTW
links:
- rel: constrains
  to: '[[mcp-control-plane]]'
- rel: constrains
  to: '[[w2-5-mcp-and-seat-wiring]]'
- rel: context
  to: '[[scale-extensions-release]]'
number: 443
provenance:
  digest: sha256:2221f3c108b39da7237930ebc5e226bd8cd7e6351e95bac53a7b084167ab2597
  frontmatter_digest: sha256:d925950d710c4f809a5d309c7772cbc545bf8f67923ddeda0a729da3aa355d60
  log: .bboxy/log/2026-10-29.jsonl#1976
  sig: gngvXqGTKEbKbUFPylcu3J0d4nHAQbi0U3IC+l7kqQ3OUPXrR3VyLrkDb+bB/s6Ph6zj+WNM7hN+xwDNAxjLDA==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
status: proposed
title: 'Issue63: one bootstrap skill source with negotiated MCP transport'
type: proposal
updated: 2026-10-29T20:39:54.138Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
# Current verified disposition — 2026-10-29

Issue63 is closed by the separately implemented compatibility cut at signed main6641b5b7a11833b8283b4f5a8fab476a1ad2e776. Oracle inspected the source, verified its signature, and independently read the exact-source normal Make log: result0,27 invariants/0 failures/11 slow skipped. Log /Users/feral/Library/Logs/bboxy450-term/issue63-plugin-skills-full-green-ci-20261029.log SHA2568d8dd362dd843d0f31913317d78bebe3a2187c26598c6d60d4050d9d6a44a6a3. GitHub closedAt2026-10-29T19:57:44Z; server state subsequently read CLOSED. This is a compatibility implementation, not claimed Beta-extension conformance.

The implementation reuses the canonical rendered bootstrap, built-in KB instructions and bounded currently trusted provider descriptors. It exposes authenticated root-scoped skill_list/skill_read tools and legacy resources/list/read using the same admission and exact root/origin/pin/digest/name identity. Reads recheck current trust/root, including cached content. No arbitrary path reader or permission activation is introduced. Source: crates/bbt-plugin/src/skills.rs and crates/bbt-mux/src/mcp/skills.rs plus the existing bridge. Existing2025/2024 negotiation remains; no2026 extension declaration, generic plug-in forwarding or supporting-file export is claimed. The bootstrap fallback remains for actual harnesses. Production retained-owner activation is distinct from a remote code commit.

The following dated proposal remains the rationale for a later negotiated Beta migration and real-harness conformance probe. Its former unimplemented/open status describes014e555c only.

# Combine the bootstrap skill with MCP

2026-10-29 consideration of [issue63](https://github.com/bboxy450/bboxy450-term/issues/63), inspected against main014e555c. This is proposed work, not an implemented extension or approval to load additional local skills.

The existing single source is crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md; adapters/launch.rs:175 renders it for the seat role. Claude gets a per-launch plugin, while Codex receives bootstrap instructions through the launch overlay. The stdio bridge (mcp/bridge.rs:21,236–344) negotiates only2025/2024 revisions and exposes tools; it does not currently implement Resources or server/discover. The old working-group link is context, not today's wire contract.

[SEP2640](https://modelcontextprotocol.io/seps/2640-skills-extension) is Final, but extension lifecycle and design acceptance differ. The [current ext-skills revision ff224337](https://github.com/modelcontextprotocol/ext-skills/commit/ff224337b01f0c9b18b81dcf99eefe66e9a52b5c) moved the unchanged released text into specification/beta/. The official overview still links stable/ and that link returned404 in this inspection. Pin the current document rather than treating a stale overview path as the normative source.

The [pinned Beta contract](https://github.com/modelcontextprotocol/ext-skills/blob/ff224337b01f0c9b18b81dcf99eefe66e9a52b5c/specification/beta/skills.mdx) uses base2026-07-28 request metadata and capability discovery. A declaring server supplies resources plus skills/list and skills/get; directoryRead is optional. Entries retain all frontmatter and complete per-file byte-size/SHA256 manifests. Identity is server plus URI. Reads transport bytes; activation remains a host decision. Retrieval stays lazy. Changed manifests invalidate content-bound consent; nested activation needs fresh consent. An older protocol client must not receive an unsupported extension declaration. This paragraph summarizes requirements, not implementation evidence.

The [implementation inventory](https://github.com/modelcontextprotocol/ext-skills/blob/ff224337b01f0c9b18b81dcf99eefe66e9a52b5c/docs/implementations.md) distinguishes released v1 servers, SDK work in progress and partial/prototype/planned hosts. This is insufficient evidence that our actual Claude/Codex versions negotiate and activate this extension. Their existing bootstrap path must remain until an unattended real-harness probe proves the replacement.

Proposed cuts, each requiring the relevant Architect ICD and a scoped Executor assignment:

1. Reuse the existing role-rendered bootstrap source for bounded authenticated resource bytes and manifests. Keep resource identity scoped to the admitted seat generation/role and exact rendered bytes; never publish environment/capabilities, arbitrary filesystem paths or a global skills directory.
2. Add the necessary base-protocol negotiation and extension methods without altering older tools-only clients. Treat partial resource exposure as partial support, not extension compliance. Preserve the existing bootstrap fallback.
3. Run the upstream pinned conformance cases plus private multi-seat denial, role change, digest-change, traversal/symlink/oversize and actual supported-harness probes. Show that file reads do not silently activate a skill or grant tools.
4. Remove only a fallback whose consumers have demonstrated equivalent unattended behavior; retain documented compatibility for older clients.

Initial product scope is the already-authorized bboxy450 bootstrap skill only. No automatic discovery/activation of optional persona skills, nested workflows, scripts or global harness config. Resolve the actual-harness support gap by a reproducible probe, not a version assumption. At this original draft, issue63 was open for an implemented and verified cut; see the superseding verified compatibility disposition above.
