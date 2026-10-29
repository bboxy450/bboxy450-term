---
id: 01M5WG2WS221JWTJ6EQ04S78N0
type: interface
number: 350
title: "ICD: plugin ↔ host"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T18:43:59Z"
---
# ICD: plugin ↔ host

| | |
|---|---|
| Side A | the contracts and the host (`bbt-plugin`, the knowledge host in `bbt-mux`) |
| Side B | plugin providers, bundled and external |
| The agreement | [[plugins]]; [[knowledge-base]] |
| Amended by | #249 a plugin is an MCP server; #255 visualization is ours; #278 P-0 rulings |
| What crosses | JSON Schema contracts with wire names; data only, never pixels. At start the host sends `initialize.params.root`, one UTF-8 absolute path, and nothing else of identity: no root, author, signer, device or account field ever appears on a `data_source` method's arguments (Kestrel, 2026-10-26; both sides of the conformance fixture assert it) |
| Timing | hook ceilings as data per contract; the handshake and each call are capped at 2 s by default |
| States and lifecycle | trust is a content-addressed copy; nothing executes before trust; plugins start disabled; one provider instance per root, the root bound at start. **Swap guard in production** (Kestrel, 2026-10-26): a proposed provider is selected for a root when its handshake succeeds and a bounded read over that root (a `list` page and a `resolve` of a known record, within the call caps) answers without error; otherwise the selection is refused with the provider's reason and the current provider stays. A full comparison of path and digest maps is **not** a production guard, since digests are provider data and a whole-root scan per swap is a global scan; it is acceptance evidence only, for the same-representation stub swap of [[knowledge-base]] §6 |
| Failure | a refusal names its reason in the app; external writes reply `signed_writer_unavailable` until the signed-writer bridge; authorship is always the host's |
| Versioning | contracts are versioned in `bbt-plugin`; a provider declares the version it implements |
| Verified by | `plugin check` on its fixture store; contract conformance fixtures; promises G3, G4, D1 and up, N1 and up |
| Open | the signed-writer bridge for external providers |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.

## Integration-skill addendum — operator brief #63, 2026-10-29

This additive agreement joins the binary, MCP surface and workflow content described in [[plugins]] §3a. [[plugins-sota]] §B separates the current compatibility cut from the published Beta extension.

| Boundary | Agreement |
|---|---|
| Producer | A provider may add `Shape.skills: [{ name, description, content }]`; each `content` is its embedded canonical `SKILL.md`. Legacy shapes omit the field. |
| Bounds | At most 16 entries, 64 KiB UTF-8 content each, and 256 KiB serialized skill-array JSON; invalid metadata is refused before publication. |
| Consumer | The logic host resolves built-in and selected-provider skills once for both fixed `skill_list`/`skill_read` tools and legacy `resources/list`/`resources/read`. |
| Scope and lifetime | An external read retains its captured root and selected source. Every list/read checks the live source generation, enable state, trust and trusted binary digest. A stale selector is refused; focus never retargets it. Unrelated providers are not inspected. |
| Content and authority | The trusted binary contains the skill bytes. Returned content keeps its origin; it is instructional data and grants no role, signing, custody or execution permission. External instructions are never automatically injected at harness launch. |
| Failure | Missing legacy skill metadata produces no external skills. Malformed metadata is refused. An unavailable selected provider is omitted with a named `provider_error` while host guides remain available; a stale external URI is refused. Neither path substitutes another provider or the current focus. |
| Verification | Bounded descriptors; equal tool/resource catalogs and bytes; selected-root isolation; stale generation/disable/trust/digest fences; unchanged authority. |
| Next interface revision | Native Beta `server/discover` plus `skills/list`/`skills/get`, complete file manifests and supporting files require an explicit revision. Preserve origin and canonical bytes across that mapping; do not advertise this support before it exists. |

The KB plugin in PR #76 is the first concrete producer. Its builder derives embedded instructions and resource content from the same source. Generic tool forwarding and standalone bundled stdio export are separate work.
