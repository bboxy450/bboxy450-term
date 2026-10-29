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
| Failure | a refusal names its reason in the app; providers without the signed writer capability reply `signed_writer_unavailable`; authorship is always the host's |
| Versioning | contracts are versioned in `bbt-plugin`; a provider declares the version it implements |
| Verified by | `plugin check` on its fixture store; contract conformance fixtures; promises G3, G4, D1 and up, N1 and up |
| Open | native GitKB commit has no generation-CAS argument; preflight references are checked, and the native commit transaction owns the final snapshot |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.

## Writable GitKB bridge (operator scope, 2026-10-28)

The host admits and signs the document write intent. The additive
`signed_record` string crosses `data_source_write` and is retained and echoed
byte for byte by a writable provider. No private key or caller-supplied author
crosses this boundary. GitKB can canonicalize its native UUID, timestamps and
status representation; an opaque host-signed intent is distinct from the
provider's canonical Markdown. A refusal or stale generation is explicit.
The descriptor advertises `signed_writer: true` for this optional bridge;
ordinary presentation identity fields never change execution behavior.
The GitKB adapter retains the opaque intent in `bboxy_signed_record`, maps the
same 128 bits between native UUID and portable ULID, and returns the native
UUID as `external_id`. Native YAML keeps its UUID, slug and native status;
`bboxy_status` carries the host's finer task lifecycle. Only the host's
evidence gate can produce `accepted`; native completion alone cannot.

Foreign providers own their numbering semantics; no `next_number` extension or
second global numbering ledger is required. GitKB's full slugs are first-class
document and task locators, within the selected root. Native identifiers remain
available for identity continuity across slug changes. Numeric family handles
must never resolve an ambiguity by ordering.

Each swarm captures its own cwd/root, plugin discovery, provider binding,
document/task view and message context. A completion admitted for one swarm
cannot populate another swarm's view after selection changes. Acceptance uses
two roots with the same slug, distinct providers, tasks and messages, including
switches with reads in flight and persisted selections after restart.
Slow provider writes and Git commits run as bounded IO work with their captured
root and actor; they cannot hold the logic owner and stall another swarm's
input, resize or status requests. Journaled unknown outcomes never redispatch
automatically.

`data_source_commit` is an optional additive operation: explicit document
locators and a commit message produce the provider's commit identifier and
committed document references. The host resolves/admit-checks them in the
captured swarm/root. Missing support is an explicit command refusal, not a
fallback to bundled Git. Existing eight-method providers remain conformant.
The commit identifier is a bounded opaque string: Git returns its actual hash,
GitKB its actual native commit UUID. Committed references identify the admitted
scope; clients resolve again for the post-commit generation.
The optional commit operation has a 15-second host ceiling, including its
reference preflight and native transaction. The original eight data operations
and production initialization keep their 2-second ceilings. Commits run off the
logic owner and never retry an unknown native outcome automatically.

Conformance setup remains disposable. A provider that requests fixture
bootstrap receives an explicit fixture initialization only from the checker's
owned temporary account. GitKB imports the canonical seeds into a real private
GitKB workspace before contract checks. Production initialization never carries
this marker and the checker never redirects mutations to Harmony.
Only that disposable bootstrap initialization may use a 15-second cap: real
GitKB initialization measured 6.077 seconds, including its SQL checkpoint.
Production initialization and ordinary contract calls retain the 2-second cap.
