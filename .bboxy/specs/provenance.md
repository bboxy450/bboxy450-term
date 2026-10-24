---
id: "3TMDSCB44BZ9X4KNRZFWEMGSBR"
type: "spec"
number: 111
title: "Provenance — decided"
created: "2026-10-21T22:41:36Z"
updated: "2026-10-24T00:10:38Z"
links:
  - { to: "[[d30-artifact-provenance-scope]]", rel: decision }
---
# Provenance — decided

Owner: Vesper Greywind <ORACLE>. 2026-10-21. Decided from [[provenance-and-documents]] (research, 37 sources). Operator: "provenance in that system is very important." Back to [[work-items]] · [[identity]] · [[messaging]] · [[mcp-control-plane]].

## Principles taken (from the research's "what the models agree on")

Digest, not path. Builder is not signer. Sign bytes through a pre-authentication encoding so YAML reordering cannot break a signature. Declare all inputs before an item can be accepted. A log plus a signature, never one without the other.

## 1. Every markdown carries (P1, now)

```yaml
created_by: seat:<ulid>
updated_by: seat:<ulid>
derived_from: ["[[research/x]]{evidence:2}", "[[<ulid>]]{revision:3}"]
provenance:
  digest: sha256:<body bytes after the closing ---> 
  frontmatter_digest: sha256:<frontmatter bytes with provenance: removed>
  signer: seat:<ulid>
  sig: <base64 Ed25519 over PAE("bboxy/item/v1", frontmatter_digest || digest)>
  log: .bboxy/log/<date>.jsonl#<seq>
```

Written only by the terminal. `derived_from` must be complete for `accepted`. Messages add one optional `about: [[task]]` so a message is an activity input; nothing else changes in the envelope.

## 2. Activity log (P2, now unsigned lines + signed checkpoints; later signed lines)

`.bboxy/log/<date>.jsonl`, one PROV-shaped activity per status transition, per artifact event, per delegate/report/accept: `{seq, prev_hash, ts, activity, agent: seat, used: [...], generated: [...], task, commit?, verify_result?}`, hash-chained. Daily checkpoint: Merkle root of the day signed, now by the operator's primary via `gpg --detach-sign`, later by the device key automatically. Export shape is Rekor v2 tiles; Rekor v2's filesystem backend means a local tile log is a real instance of the model, so export is a later step, not a redesign.

## 3. Commits (P3: option A now, option B later)

A `prepare-commit-msg` hook installed per launch into the seat's worktree (never the repo config) adds trailers `Bboxy-Seat`, `Bboxy-Task`, `Bboxy-Delegated-By`, `Bboxy-Activity`. **Now:** commits stay signed by the operator's primary; the trailers are inside the signed message, tamper-evident, and the activity line names the seat, so provenance is "Boxy's key signed a commit that the log attributes to seat X". **Later:** leaf-key signing through a `bboxy-gpg` shim with the roster exported as a keyring and `allowed_signers`, plus an OpenPGP notation `task@bboxy450.dev=<id>`. Per-seat signing stays default-off until then ([[ux-pick]] assumption unchanged).

## 4. The provenance graph (P4): one of the seven graphs (markdown, task, execution, artifact, messaging, code, provenance)

Nodes: markdown, artifact, activity, seat, grant, root. Edges: `generated`, `used`, `wasAttributedTo` (seat), `wasAssociatedWith` (grant), `actedOnBehalfOf` (root). Queries it must answer: who touched this, from what, under which key; what did this seat produce today; which accepted items derive from an unverified input. Artifact graph edges are its leaf layer.

## 5. Wikilink syntax, final (closes the Obsidian gap)

`[[slug]]` and `[[slug|display]]` stay exactly Obsidian's forms, so `.bboxy/` opens cleanly in Obsidian, Foam and Logseq. Relation and weight move out of the pipe into a trailing annotation: `[[slug]]{rel:w}`, e.g. `[[messaging]]{spec:2}`, `[[01JAX…]]{blocks}`. Obsidian renders the braces as literal text after the link, harmless. The `links:` frontmatter list remains the YAML equivalent. [[work-items]] is updated to this.

## W2.6 decisions (Architect, 2026-10-23, decision [[d30-artifact-provenance-scope|#30]])

- **Attribution is never invented.** When more than one seat matches a detected artifact's worktree and activity window, the record gets `produced_by: null`, `grade: inferred`, a bounded list of candidate seat ids and a named reason; a later authenticated `artifact_claim` resolves it. This replaces the implicit single-seat assumption above: an unclaimed detection with several plausible seats stays unowned until a seat says so under its key.
- **Who signs what.** W2.6 acceptance is signed rows plus the hash chain. A detection made by the observer (filesystem, hooks, git metadata) is attributed to a seat but **signed by the observer**, never by a key the observer does not hold; a claim is signed by the claiming seat's leaf (the W2.5d write path). The daily GPG checkpoints of §2 stay pending while the real root is held; there is no device-signed substitute for a root checkpoint.
- **Scope.** W2.6 is cockpit-local: filesystem detection, hooks, git metadata and local GUI events. Split: 2.6a detection, claims and the log; 2.6b the GUI events lane, after L8-D's admission-version bump. External ingress stays in [[events]] as design, deferred.



Signed activity lines with seat leaf custody; device-key checkpoints; Rekor export; OTel GenAI span export over the seat socket; `.bboxy/schema.yml` with an inverse-relation table; Lamport ordering for concurrent frontmatter edits; Foam link-reference definitions so GitHub renders wikilinks; JSON Canvas export of any graph.

## Gaps

- **Gap:** who signs activity lines for a remote seat whose leaf lives on another host (phase 2).
- **Gap:** OpenPGP leaf ↔ `git verify-commit` interop untested; ssh `allowed_signers` path untested.
- **Gap:** Rekor v2 tile and checkpoint format not read from the primary spec.

Links: [[work-items]] [[identity]] [[messaging]] [[mcp-control-plane]] [[provenance-and-documents]] [[vision]]
