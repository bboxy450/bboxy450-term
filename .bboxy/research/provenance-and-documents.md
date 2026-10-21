# Provenance systems and plain-file document systems — research and proposal

2026-10-21 · research subagent for Vesper Greywind <ORACLE> · for [[work-items]] §Provenance. Facts carry `source:` or `file:line`; the Proposal section is proposal, not fact.

[[work-items]] · [[identity]] · [[messaging]] · [[vision]] · [[mcp-control-plane]]

## Scope read

- `specs/work-items.md:7` — one Markdown file + YAML frontmatter per item under `.bboxy/`; six types; "Git is the database". `:9-28` the frontmatter contract. `:46-56` wikilinks `[[slug|rel|w]]` as the only link primitive, default weights parent/child 3, depends/blocks 2, else 1. `:58-67` five graphs now (markdown, artifact, task, execution, messaging). `:80-95` artifacts: `artifact_claim`, detected artifact events with `grade: reported|inferred`, `history:` in frontmatter spilling to `.bboxy/log/`. `:97-99` the provenance rule: no anonymous write, `updated_by: seat:<id>`, `Bboxy-Seat`/`Bboxy-Task` trailers.
- `specs/identity.md:25-31` — pinned OpenPGP primary certifies device cert, device certifies seat cert; seat key signs messages; Git signing off by default. `:55` `SeatRoster{version, root_pubkey, device_grant_id, device_epoch, roster_sequence, previous_roster_id, issued_at, entries}`, append-only, device-signed. `:191-210` roster + detached OpenPGP manifest signature verifiable with `gpg --verify`; "a roster proves terminal-issued key attribution, not which model wrote a patch"; per-seat Git signing ASSUMED and default-off.
- `specs/messaging.md:63-84` — signed envelope: `account, device, seat, author (seat session pubkey), role, role_grant_id, sender_sequence, signature`; `:110` seat key signs a fixed-order tuple with a domain separator.
- `vision.md:56-58` — every seat is a unix socket; everything is an event stream. `:77-81` one concern at a time, checkpoints, no pre-emptive optimisation.

Lens: mechanism. What each system records, how it is signed, how it chains, what it costs us in Rust. Assumes the key chain in [[identity]] exists. Breaks when seats run on hosts whose clocks and keys we do not control.

---

## 1. Provenance models

| System | Records | Signed how | Chains how | Rust adoption cost |
|---|---|---|---|---|
| **W3C PROV-DM** | Entities, Activities, Agents; `wasGeneratedBy`, `used`, `wasInformedBy` (§5.1), `wasDerivedFrom` with Revision/Quotation/PrimarySource (§5.2), `wasAttributedTo`, `wasAssociatedWith`, `actedOnBehalfOf` (§5.3). source: https://www.w3.org/TR/prov-dm/ | None. PROV-DM has no signing or integrity layer in the read portion (§1-5.3). | Derivation edges entity→entity; delegation agent→agent (`actedOnBehalfOf`). | Vocabulary only. Zero deps: a `kind` enum and seven edge names in our frontmatter. Cheap now. |
| **in-toto attestation** | Statement `{_type: "https://in-toto.io/Statement/v1", subject:[{name, digest:{alg:hex}}], predicateType, predicate}`; subject `digest` required. source: https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md | **DSSE** envelope `{payload, payloadType, signatures:[{keyid, sig}]}`, signature over `PAE = "DSSEv1" SP LEN(type) SP type SP LEN(body) SP body`; signs exact bytes, no canonicalization. source: https://github.com/secure-systems-lab/dsse/blob/master/protocol.md | Subjects are content digests; chains by digest reference from one statement to the next. | DSSE is ~40 lines over `ed25519-dalek`. The PAE trick removes the YAML/JSON canonicalization problem for us. Cheap now. |
| **SLSA provenance v1** | `predicateType: "https://slsa.dev/provenance/v1"`; `buildDefinition{buildType, externalParameters (untrusted), internalParameters, resolvedDependencies}`, `runDetails{builder.id, metadata{invocationId, startedOn, finishedOn}, byproducts}`. Builder is distinct from signer: "Consumers MUST accept only specific signer-builder pairs." source: https://slsa.dev/spec/v1.0/provenance (page marked retired; 1.2 current) | Carried in an in-toto Statement. | `resolvedDependencies` digests link to upstream artifacts. | Maps directly: builder.id = seat, externalParameters = the task body, resolvedDependencies = `derived_from`. Use the shape, not the URI, until we publish. Cheap now. |
| **Sigstore Rekor** | Append-only log of signed metadata; entry has `logIndex`, `verification.inclusionProof{hashes, rootHash, logIndex, treeSize}`, `signedEntryTimestamp`; public instance rekor.sigstore.dev, self-hostable. source: https://docs.sigstore.dev/logging/overview/ , https://docs.sigstore.dev/logging/verify-release/ | Log operator signs tree heads/SETs; entries carry their own signatures. | Merkle tree (Trillian in v1). **Rekor v2 (rekor-tiles)** is "a modern, tile-backed transparency log" with a **POSIX backend: "atomic POSIX operations and a POSIX-compliant filesystem, with no cloud dependencies"**; public shard URL rotates ~6 months via TUF. source: https://github.com/sigstore/rekor-tiles/blob/main/README.md | Writing to public Rekor needs the sigstore client (Go/Rust `sigstore-rs`) and an identity; not v0.1. A local Merkle log in the tlog-tiles shape is ~200 lines and exports later. Later. |
| **Git commit signing** | `git commit -S` signs the serialized commit object: tree, parents, author/committer, headers, **message including trailers**; stored in a `gpgsig` header. `gpg.format = openpgp|x509|ssh`; ssh needs `gpg.ssh.allowedSignersFile` (principal + pubkey per line). `git verify-commit [--raw] <commit>` exits non-zero on failure. source: https://git-scm.com/docs/git-verify-commit | OpenPGP via `gpg.program`, already on this machine: `~/.config/git/local.conf` sets `commit.gpgsign=true`, `signingkey=9B07E849…7AB0`. | Parent hashes: a Merkle DAG. | Already paid. Trailers inside the signed message are the cheapest signed provenance we can get. Cheap now. |
| **Git trailers** | `Key: value` block at message end, preceded by a blank line; block is all trailers or ≥25% trailers with one known key; folding per RFC 822; `trailer.<alias>.key`, `trailer.ifExists` (`addIfDifferentNeighbor` default, `replace`, …), `trailer.where`. Read back with `git log --format='%(trailers:key=X,valueonly)'`. source: https://git-scm.com/docs/git-interpret-trailers | Covered by the commit signature. | None by themselves. | `git interpret-trailers` in a `prepare-commit-msg` hook. Cheap now. |
| **git notes** | Blobs attached to any object by hash in a notes tree; default `refs/notes/commits`, namespaces via `--ref`; every change is a commit on the notes ref; `notes.rewriteRef` to carry across rebase; merge strategies `manual|ours|theirs|union|cat_sort_uniq`. **Not pushed by default; not signed.** source: https://git-scm.com/docs/git-notes | Unsigned. Notes commits can be `-S` signed, but the page does not define it. | Notes ref history. | Good for after-the-fact attribution (agentblame uses it). Weak: unsigned, unfetched by default. Use only as an index. |
| **OpenPGP (RFC 9580)** | Certifications 0x10-0x13 over UIDs; 0x18 subkey binding; 0x1F direct-key. **Trust Signature** (§5.2.3.21): level 1 = trusted introducer, amount 120 = complete. **Notation Data** (§5.2.3.24): `name@domain` user namespace, flag 0x80000000 = UTF-8 value; "Names in a user namespace are controlled by the owners of that domain." Key Flags §5.2.3.29: 0x01 certify, 0x02 sign. Issuer Fingerprint §5.2.3.35 "SHOULD be included in all signatures". Signature Target §5.2.3.33 names another signature. source: https://www.rfc-editor.org/rfc/rfc9580.html | Itself. | Certification chain root→device→seat (= `identity.md:25-31`). | `pgp 0.21.0` (rPGP) already gated in `identity.md:94-105`. Notations give us `seat@bboxy450.dev` / `task@bboxy450.dev` annotations **inside** certifications and commit signatures for free. Cheap once identity lands. |
| **Content addressing** | Git: `sha(type SP size \0 content)`; blob→tree→commit. source: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects . CID: `<multibase><cidv1><multicodec><multihash>`, self-describing; CIDv0 = bare SHA-256 multihash `Qm…`. source: https://github.com/multiformats/cid (archived; spec moved to specs.ipfs.tech/cid). IPLD data model = JSON + bytes + links, codecs dag-cbor/dag-json. source: https://ipld.io/docs/data-model/ | None; integrity by hash. | Hash in parent = Merkle DAG. | We already have git's. For our own records: `sha256:` prefixed hex, no CID machinery. Cheap now. |
| **Nix derivations** | `.drv` fields: name, inputs ("all inputs be explicitly collected in the inputs field"), outputs, system, builder, args, env; derivation itself content-addressed ("Text" method, ATerm encoding); output deriving path = drv path + output name; store path = 20-byte digest, base-32, + name. source: https://nix.dev/manual/nix/2.28/store/derivation/ , https://nix.dev/manual/nix/2.28/store/store-path , https://nix.dev/manual/nix/2.28/protocols/derivation-aterm | None. | Every output traces to its .drv and transitively to inputs. | Lesson, not dependency: **declare all inputs explicitly** (= `derived_from` must be complete for a record to be `verified`). Our monorepo is Nix/weavie (`vision.md:103+`), so `nix path-info --json` already yields input closures for builds. Later. |
| **DVC** | `.dvc` YAML: `outs:[{path, md5|etag|checksum, hash, size, nfiles, cache}]`, `deps`, `wdir`; imports carry `deps.repo{url, rev, rev_lock}`. Small file in git, data in a hash-keyed cache. source: https://doc.dvc.org/user-guide/project-structure/dvc-files | Git commit signature only. | `rev_lock` pins the source commit. | Pattern for binary artifacts: a frontmatter record with `digest` and the content outside git. Already the shape of `.bboxy/artifacts/<id>.md` (`work-items.md:84`). Cheap now. |
| **lakeFS** | Git-like commits over an object store. **Gap:** docs.lakefs.io redirected three times (`/understand/model`, `/concepts/internals/`, `/concepts/architecture/`) and the targets 404'd under `/latest/`; no primary-source facts recorded. | — | — | Not relevant at our scale; skip. |
| **C2PA 2.1** | Manifest = assertions + one claim + claim signature (§2.3.4); claim `c2pa.claim.v2` with `created_assertions`, `gathered_assertions`, `redacted_assertions`, `claim_generator_info` (§10.2); exactly one **hard binding** (`c2pa.hash.data` byte ranges, §9.1-9.2); `c2pa.actions` mandatory, never redactable (§5.2.1, §6.8); ingredients' manifests copied into the store, same-id conflicts suffixed `:2_1` (§10.3.2.2). source: https://spec.c2pa.org/specifications/specifications/2.1/specs/C2PA_Specification.html | COSE_Sign1 over the CBOR claim (Core Deterministic Encoding); X.509 only (§5.2.2); RFC 3161 timestamp in `sigTst2`. | Ingredient manifests nest; update manifests record non-content changes (§6.8). | X.509 + COSE + CBOR + JUMBF: heavy, wrong trust root (CA list, not our GPG). Take three ideas: one hard binding per record, actions are never redacted, an update record for metadata-only changes. Do not adopt. |

### What the models agree on

1. A record is **about a content digest**, never about a path (in-toto subject, C2PA hard binding, DVC `md5`, Nix store path).
2. **Builder ≠ signer** (SLSA). The seat did the work; the terminal/device signs the record. Both appear.
3. **Sign bytes, not structures** (DSSE PAE). Store the exact signed bytes; never re-serialize to verify.
4. **Inputs are declared completely or the record is incomplete** (Nix).
5. A **log** proves order and non-deletion; a **signature** proves authorship. You need both (Rekor + Sigstore signing).

---

## 2. Document and knowledge systems on plain files

| System | Schema | Link model | History and authorship | Maps to our six types / graphs |
|---|---|---|---|---|
| **Obsidian** | YAML properties: text, list, number, checkbox, date, datetime; defaults `tags`, `aliases`, `cssclasses`; links in properties quoted `link: "[[Episode IV]]"`. source: https://obsidian.md/help/properties | `[[Note]]`, `[[Note#Heading]]`, `[[Note#^blockid]]`, `[[Note\|Custom name]]`, `![[Note]]` embed; resolves by filename, folder path from vault root. source: https://obsidian.md/help/links | None in the format; sync/git plugins outside. | Our `[[slug\|rel\|w]]` is a superset: Obsidian puts display text after `\|`, we put relation and weight. **Decide:** keep `\|rel\|w` and accept Obsidian renders `rel` as display text, or move rel/weight to a trailing `{rel=… w=…}`. Quoted `"[[…]]"` in YAML is the rule we must follow for `links:`. Canvas: JSON Canvas 1.0 `nodes[{id,type:text\|file\|link\|group,x,y,width,height,color}]`, `edges[{id,fromNode,fromSide,fromEnd,toNode,toSide,toEnd,color,label}]` (source: https://jsoncanvas.org/spec/1.0/) is a ready export format for any of our graphs. |
| **Logseq** | Outliner; `pages/<title>.md`, `journals/YYYY_MM_DD.md`, config in `logseq/`; block gets `id:: <uuid>` property when referenced. source (community, not official): https://discuss.logseq.com/t/block-reference-to-allow-page-name-block-ref-name-not-just-uuid/2746 | `[[page]]`, `((uuid))` block ref, embed. UUIDs regenerate on reindex unless `id::` is written. | None in format. | Block refs = our need to cite a line of a task body. We do not need block ids yet; `[[slug#heading]]` covers it. **Gap:** official Logseq docs site is a 10 MB JS shell; facts above are forum-sourced. |
| **Dendron** | Required frontmatter: `id` (23-char lowercase alnum), `title`, `desc`, `updated`, `created` (unix ts); reserved `parent`, `children`, `tags`, `nav_order`, `published`. Hierarchy by dotted filename `project.designs.promotion.md`; optional `*.schema.yml` types. source: https://wiki.dendron.so/notes/ffec2853-c0e0-4165-a368-339db12c8e4b/ , https://wiki.dendron.so/notes/c6fd6bc4-7f75-4cbb-8f34-f7b99bfe2d50/ | `[[foo.bar]]`, note refs. | None in format. | Closest to ours: id-in-frontmatter + parent/children + schemas. Their `schema.yml` is what our six-type contract would look like as a file; worth copying as `.bboxy/schema.yml` so other editors validate. |
| **Foam** | Plain Markdown in VS Code. | `[[note]]`, `[[note#Section]]`, `[[note#^blockid]]`; ambiguous names resolve alphabetically with a warning; auto-generated link-reference definitions at file bottom so GitHub renders wikilinks. source: https://github.com/foambubble/foam/blob/master/docs/user/features/wikilinks.md | None in format. | The link-reference-definition trick makes `[[slug]]` render on GitHub. Cheap, worth doing in the writer. |
| **Zettelkasten** | One idea per note ("Principle of Atomicity"); time-based IDs `202006110955` that "never change" so titles may change. source: https://zettelkasten.de/introduction/ | Links **with stated reason** ("always state explicitly why you made it"); structure notes = notes about notes. | Author is the single owner. | Our ULID = their timestamp id. Their link-context rule = our typed `rel`. Our `invariant` type = a structure note that binds. |
| **Roam** | Every block has a uid. source (third-party cheatsheet): https://unpkg.com/roam-research-mcp@5.0.0/build/Roam_Markdown_Cheatsheet.md | `((uid))`, `{{[[embed]]: ((uid))}}`, `[[Page]]`. | Proprietary DB. | Same lesson as Logseq; **Gap:** official help page is a JS shell. |
| **org-mode** | `:PROPERTIES: … :END:` drawer under a headline, `:KEY: value`, `:KEY+:` accumulates, `#+PROPERTY:` file-level, `KEY_ALL` restricts values. source: https://orgmode.org/manual/Property-Syntax.html | `[[id:…]]`, `[[file:…]]`. | None in format. | `KEY_ALL` = enum constraint per property, same job as Dendron schema. `+` accumulation is a nice model for append-only `history:`. |
| **Jekyll** | YAML "must be the first thing in the file"; `layout`, `permalink`, `published`, `date`, `category/categories`, `tags`; custom keys free. source: https://jekyllrb.com/docs/front-matter/ | Markdown links. | git. | Confirms: frontmatter first, custom keys allowed. |
| **Hugo** | YAML `---`, TOML `+++`, JSON; reserved `title, date, lastmod, publishDate, expiryDate, draft, weight, slug, url, aliases`; "Create custom fields under the `params` key"; taxonomies configured. source: https://gohugo.io/content-management/front-matter/ | Markdown links. | git. | Reserve our keys, namespace extensions. Our `weight` must not collide with Hugo's if anyone ever renders `.bboxy/` with Hugo. |
| **Notion** | Database → data sources → pages; 24 property types incl. `relation` (`single_property`/`dual_property` with `synced_property_name`), `rollup`, `created_by`, `last_edited_by`, `created_time`, `last_edited_time`, `unique_id`. source: https://developers.notion.com/reference/property-object , https://developers.notion.com/reference/database | Relations are bidirectional by `dual_property`. | `created_by`/`last_edited_by` built in; no history export. | `dual_property` = our `children` kept in sync with `parent` (`work-items.md:16`). `created_by`/`last_edited_by` = `created_by`/`updated_by` seat fields. |
| **Linear** | Parent/sub-issues; sub-issues inherit team, priority, project; "Convert to project" dissolves the hierarchy. source: https://linear.app/docs/parent-and-sub-issues | Parent/child; **Gap:** blocking/related/duplicate docs pages 404'd. | Server-side history. | Inheritance down the parent edge is worth copying: a task inherits `allowed_paths`/`verify` defaults from its spec/epic unless overridden. |
| **Jira** | Link types have Name, **Outward** ("affects") and **Inward** ("is affected by") descriptions; `Blocks` default; clone link types auto-link clones. source: https://support.atlassian.com/jira-cloud-administration/docs/configure-issue-linking/ | Directional typed links. | Changelog per issue. | Our `rel` needs an inverse table: `blocks`↔`blocked-by`, `parent`↔`child`, `evidence`↔`evidenced-by`, `violates`↔`violated-by`. |
| **Beads (bd)** | "Distributed graph issue tracker for AI agents, powered by Dolt"; `.beads/embeddeddolt/` is the DB, `.beads/issues.jsonl` "an export… not the source of truth"; hash ids `bd-a1b2`, hierarchical `bd-a3f8.1`; deps `blocks, related, parent-child`; links `relates-to, duplicates, supersedes, replies-to`; `bd ready` = no open blockers; `--claim` atomic assignee+in-progress; `closed_by_session` from `CLAUDE_SESSION_ID`; sync via `refs/dolt/data` on a git remote. source: https://github.com/steveyegge/beads/blob/main/README.md | Typed edges. | Dolt commits; `closed_by_session`. | Nearest neighbour. Differences: they went DB-first, we are file-first by decision (`work-items.md:103` "No database, no sidecar JSON"). Take: hash ids immune to merge collision (our ULID does this), `ready` query, `supersedes`/`replies-to` relations, session id on close. |
| **git-bug** | Entities are operation logs stored as git objects: pack = blob with a JSON array of operations, tree `ops`, commit per edit session, chain under `refs/<namespace>/<id>`; state = fold of operations; **Lamport clocks** in tree entry names (`edit-clock-154`); concurrent edits make a merge commit, ordered by clock then pack id; entity id = hash of the first operation; packs have an `author` = another entity (identity). Signed commits "limit how this data model can be abused". source: https://github.com/git-bug/git-bug/blob/trunk/doc/design/data-model.md | Author refs to identity entities. | Full, in git, no working-tree files. | The purest "git is the database". Too invisible for us (operator wants readable Markdown), but its **operation log + fold** is exactly our status-transition `activity` log; its Lamport ordering answers `work-items.md:113` (two seats edit one frontmatter). |
| **Fossil** | Everything is a content-addressed artifact (SHA1/SHA3-256); structural artifacts are **cards**: manifest `C D F P U Z`, ticket change `J K D U`, wiki `L W`, tags `T`; `Z` checksum required. source: https://fossil-scm.org/home/doc/trunk/www/fileformat.wiki , https://fossil-scm.org/home/doc/trunk/www/concepts.wiki | `P` parent hash, `K` ticket id. | `U` user card + `D` date on every artifact. | Ticket change artifact `{K ticket, J field=value…, D, U}` is the minimal activity record. Ours adds a signature. |
| **Dolt** | SQL DB with git semantics; commit = root hash of the whole DB in a parent graph, SHA-256-based; author/email/date/message. source: https://www.dolthub.com/docs/concepts/dolt/git/commits | Row-level history via system tables (not read here). | Per-commit author. | Beads' engine. Not for us (binary store, no Markdown). |

### What maps

- **Six types** ← Dendron schema (`type` constrained), org `KEY_ALL`, Hugo reserved keys. Write `.bboxy/schema.yml` once; everything else validates against it.
- **Markdown graph** ← Obsidian/Foam wikilinks; export as JSON Canvas.
- **Task graph** ← Jira inward/outward pairs, Beads `ready`, Linear inheritance.
- **Execution graph** ← Beads `--claim`/`closed_by_session`, Notion `created_by`.
- **Artifact graph** ← DVC digest records, C2PA ingredient copying.
- **Provenance graph (proposal)** ← git-bug operation log, Fossil ticket-change cards, PROV `actedOnBehalfOf`.

---

## 3. Agent-era provenance

### Local transcript layouts (keys only, no content)

| Harness | Layout | Record shape observed |
|---|---|---|
| Claude Code | `~/.claude/projects/<cwd-slug>/<sessionId>.jsonl`; `<session>/subagents/`, `<session>/tool-results/`; swept after `cleanupPeriodDays` (default 30). source: https://code.claude.com/docs/en/claude-directory | `python3 -I` over `/Users/feral/.claude/projects/-Users-feral/d1e88d4d-….jsonl` (first 400 lines) — record types `user, assistant, system, attachment, file-history-snapshot, queue-operation, cost-state, …`; every `user`/`assistant` carries `uuid, parentUuid, sessionId, cwd, gitBranch, timestamp, version, isSidechain`; `assistant.message.content[]` has `tool_use{id, name, input, caller}` and `tool_result{tool_use_id, content, is_error}`; `thinking{signature}`. So: a **parent-linked DAG of turns**, tool calls keyed by `tool_use.id`, git branch per record. No signature. |
| Codex | `~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl`; `~/.codex/session_index.jsonl {id, thread_name, updated_at}`; `history.jsonl` via `history.persistence`; `log_dir` default `$CODEX_HOME/log`; SQLite state (`state_5.sqlite`, `thread_history_1.sqlite`, …). source: local `ls ~/.codex`; https://learn.chatgpt.com/docs/config-file/config-reference | `python3 -I` over `rollout-2026-10-13T15-57-20-….jsonl` — record `{type, ordinal, timestamp, payload}`; `session_meta.payload{id, cwd, git, cli_version, originator, parent_thread_id, creator_account_id, …}`; `turn_context.payload{turn_id, model, approval_policy, sandbox_policy, cwd, comp_hash}`; `response_item.payload.type ∈ {message, reasoning(encrypted_content), function_call{call_id,name,arguments}, function_call_output, custom_tool_call, agent_message{author, recipient}}`; `event_msg.payload.type ∈ {task_started{turn_attribution}, item_completed, task_complete, token_count}`. So: **ordinal-sequenced, turn-scoped**, tool calls keyed by `call_id`, git state in `session_meta.git`. No signature. |

Both are plaintext, unsigned, local, and swept. They are evidence for a seat's claim, not proof; the terminal should **reference** them (`transcript: {harness, session_id, ordinal|uuid}`) from an activity record and never copy them into `.bboxy/` (`README.md` "No secrets, no raw transcripts").

### Hooks as the attribution channel

`~/.codex/hooks.json` — `SessionStart` runs `icq … session-hook --target codex` (matcher `startup|resume|clear|compact`). Claude Code hook input carries `session_id`, `cwd`, `scratchpad_dir` (source: https://code.claude.com/docs/en/claude-directory §Session scratchpad). `specs/subagent-detection.md` and `vision.md:62` already route `PostToolUse` through `bboxy-hook` with `BBOXY_SEAT_ID`. That is the `reported` grade source for "which seat wrote this file" (`work-items.md:90`).

### OpenTelemetry GenAI semantic conventions (status: Development)

`gen_ai.operation.name` ∈ `chat, create_agent, invoke_agent, invoke_workflow, plan, execute_tool, embeddings, retrieval, fetch_response, text_completion, generate_content, *_memory*`. Execute-tool span: name `execute_tool {gen_ai.tool.name}`, kind INTERNAL, `gen_ai.tool.name` (Required), `gen_ai.tool.call.id` (Cond. Req.), `gen_ai.tool.type ∈ function|extension|datastore`, `gen_ai.tool.call.arguments`/`.result` (Opt-In), `gen_ai.agent.name`, `gen_ai.conversation.id`. Agent spans: `create_agent {gen_ai.agent.name}`, `gen_ai.agent.id` ("provider-assigned stable identifier", not transient), `gen_ai.agent.version`; `gen_ai.provider.name`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`. source: https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-spans.md , …/gen-ai-agent-spans.md . **Use the names** for our activity records (`gen_ai.agent.id` = seat id, `gen_ai.tool.call.id` = harness `tool_use.id`/`call_id`) so an OTLP export is a rename, not a redesign.

### "Which agent wrote this line" in git, as practised

| Practice | Mechanism | Limit |
|---|---|---|
| `Co-authored-by: name <email>` trailer | GitHub attributes the commit to each trailer whose email is on an account. source: https://docs.github.com/en/pull-requests/committing-changes-to-your-project/creating-and-editing-commits/creating-a-commit-with-multiple-authors | Commit-level, unsigned claim inside a (possibly) signed message; "records no prompt, reviewer, or model version". source: https://dev.to/pullflow/the-new-git-blame-whos-responsible-when-ai-writes-the-code-285j |
| Claude Code `attribution.commit` / `attribution.pr` / `attribution.sessionUrl` (replaces deprecated `includeCoAuthoredBy`). source: https://code.claude.com/docs/en/settings-reference | This session's own harness reminder instructs `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` — model name in the trailer, observed 2026-10-21. | Prompt-injected into the model, not enforced by git. |
| Codex `commit_attribution` in `config.toml`: absent → `Co-authored-by: Codex <noreply@openai.com>`, custom string, or `""` for none; applied via prompt, not a hook. source: https://codex.danielvaughan.com/2026/03/28/codex-cli-commit-attribution/ | Same. **Gap:** not found in the official config reference slice read; third-party source. |
| `Assisted-by: AGENT:MODEL` trailer (kernel-style) plugin. source: https://skillselion.com/marketplace/bcmyguest/assisted-by | Convention only. |
| **Mesa Agent Blame**: hooks (Cursor, Claude Code, OpenCode) capture added lines → exact + whitespace-stripped hash per line → SQLite until commit → post-commit hook writes attribution to **git notes**; squash/rebase re-matched by content (1.0 exact, 0.95 normalized); records `{file, line range, provider, model, confidence}`. source: https://www.mesa.dev/blog/agentblame-deep-dive | Unsigned notes; "prioritizes precision over recall". |
| Beads `closed_by_session` from `CLAUDE_SESSION_ID`. source: Beads README above | Session id, not a key. |
| `~/.agents` repo | Commits signed `G` by `Boxy <bboxy450@gmail.com>` (`git log --format='%G?'` → `G` on the last 4); `.skill-lock.json{version, skills{<name>{source, sourceType, sourceUrl, skillPath, skillFolderHash, installedAt, updatedAt}}}` is the vendored-skill provenance record (`~/.agents/README.md` "`.skill-lock.json` is the provenance record"). | Hash + source URL, no signature on the lock itself; the commit signature covers it. |

Nobody in this list signs the attribution with the agent's own key. The field is open: a seat leaf key under a certified chain is the differentiator, and [[identity]] already specifies it.

---

## Proposal — `.bboxy/specs/provenance.md` (for the Oracle to decide)

Lens: DSSE + PROV vocabulary + git. Assumes [[identity]] seat leaf keys and the device-signed `SeatRoster` exist. Breaks when a seat's host is not ours (remote/cloud seats: the remote supervisor must hold the leaf key; `work-items.md:114` gap).

### P1. What every work item carries

Two new frontmatter keys, both written only by the terminal:

```yaml
created_by: seat:01JAX…            # first writer
updated_by: seat:01JAX…            # last writer (already decided, work-items.md:99)
derived_from:                      # PROV wasDerivedFrom; wikilinks with weight
  - "[[research/provenance-and-documents|evidence|2]]"
  - "[[01JAX…|revision|3]]"        # previous item this supersedes
provenance:
  digest: sha256:<hex of body bytes after the closing --- >
  frontmatter_digest: sha256:<hex of frontmatter bytes with `provenance:` removed>
  signer: seat:01JAX…              # leaf key fingerprint resolvable via roster
  sig: <base64 Ed25519 over PAE("bboxy/item/v1", frontmatter_digest || digest)>
  log: .bboxy/log/2026-10-21.jsonl#<seq>   # where the activity landed
```

Rules: `derived_from` must be complete for the item to reach `accepted` (Nix rule); the record is about digests, not the path (in-toto rule); the signed bytes are the two digests, so YAML re-ordering by an editor does not break the signature (DSSE rule). Verification = recompute two digests, look up `signer` in the current `SeatRoster`, check the certification chain root→device→seat, verify. Obsidian-compatible: `"[[…]]"` quoted.

**Messages** ([[messaging]] envelope) add nothing: they already carry `author`, `role_grant_id`, `sender_sequence`, `signature` (`messaging.md:63-84`). Add one optional field `about: [[task-id]]` so a message is an activity input.

### P2. Activity records (PROV Activity, one per status transition and per artifact event)

Append-only JSONL in `.bboxy/log/<date>.jsonl`, one line each, DSSE-signed by the seat that caused it (the mux signs on the seat's behalf with the seat leaf key, as it already does for messages):

```json
{"v":1,"seq":1842,"prev":"sha256:…","ts":"2026-10-21T22:40:11Z",
 "activity":"status","item":"01JAX…","from":"claimed","to":"running",
 "agent":"seat:01JAX…","on_behalf_of":"seat:01JAX…oracle",
 "used":["sha256:… (item digest before)","[[messaging-envelope-id]]"],
 "generated":"sha256:… (item digest after)",
 "commit":"<git sha or null>","verify":{"cmd":"cargo test -p bbt-core --locked","exit":0,"log":".bboxy/runs/…"},
 "transcript":{"harness":"codex","session":"01a1377c-…","ordinal":312},
 "grade":"reported","signer":"seat:01JAX…","sig":"<b64 over PAE(\"bboxy/activity/v1\", line-without-sig)>"}
```

`activity ∈ {create, status, assign, release, edit, artifact.created|modified|deleted|renamed|committed|published, verify, accept, reject}`. `agent`/`on_behalf_of` are PROV `wasAssociatedWith`/`actedOnBehalfOf`; `used`/`generated` are PROV `used`/`wasGeneratedBy`; `derived_from` on the item is `wasDerivedFrom`. OTel names map 1:1 for export (`gen_ai.agent.id` = `agent`, `gen_ai.tool.call.id` = `transcript.ordinal`/`tool_use.id`).

`prev` is the SHA-256 of the previous line's bytes: a hash chain. Every 256 lines the mux writes a `checkpoint` line `{"activity":"checkpoint","tree_size":N,"root":"sha256:<Merkle root over line hashes>","sig":…}` signed by the **device** key (the log operator, SLSA "signer ≠ builder"). That is the tlog-tiles shape in one file; exporting to Rekor v2 later is: for each line, submit the DSSE envelope as a `dsse` entry and keep its `logIndex` next to `seq`. The POSIX backend of rekor-tiles proves a filesystem log is a legitimate instance of the model, not a toy.

Frontmatter `history:` (`work-items.md:95`) stays as the bounded human view: newest N activity `seq` numbers and one-line summaries; the log is the record.

### P3. How the terminal attaches provenance to commits it causes

When a seat's harness commits inside a bboxy worktree, a `prepare-commit-msg` hook installed by the terminal into the worktree (not the repo config; per-launch like the settings overlay in `vision.md:62`) runs `git interpret-trailers --if-exists replace` with:

```
Bboxy-Seat: seat:01JAX… <fpr:…leaf fingerprint…>
Bboxy-Task: 01JAX…
Bboxy-Delegated-By: seat:01JAX…oracle
Bboxy-Activity: 2026-10-21.jsonl#1842
Co-authored-by: <harness default, left as is>
```

Signing: two options, decide one.

- **A (cheap now):** commits stay signed by the operator's primary (`~/.config/git/local.conf`, `commit.gpgsign=true`). Trailers are inside the signed message, so they are tamper-evident but **the seat did not sign them**; the activity line in `.bboxy/log/` (seat-signed) names the commit sha after the fact and closes the loop. `identity.md:208` keeps per-seat Git signing default-off; this option respects it.
- **B (later):** `git -c user.signingkey=<seat leaf fpr> -c gpg.program=bboxy-gpg commit`, where `bboxy-gpg` is a shim that signs with the leaf through the mux. Then `git verify-commit` needs the leaf certificate in a keyring: export the public roster bundle (`identity.md:197`) to `.bboxy/keys/roster.pgp` and have the hook run `gpg --no-default-keyring --keyring .bboxy/keys/roster.pgp --verify`. Or ssh format: write `.bboxy/keys/allowed_signers` from the roster (`seat:<id> ssh-ed25519 <key>` per line) and set `gpg.ssh.allowedSignersFile` for the worktree — the raw Ed25519 keys are the same bytes (`identity.md:40`). ssh is simpler; OpenPGP keeps one chain. **Gap:** `identity.md:210` — leaf/Git interop untested.

Add an OpenPGP **notation** `task@bboxy450.dev=<task id>` to the seat certification or commit signature when B lands; it travels inside the signature packet, invisible to tools that ignore it.

### P4. The provenance graph

Nodes: item, artifact, activity, seat, device grant, root. Edges (weights in the markdown-graph convention): item ←`generated`(3)— activity —`agent`(3)→ seat —`grant`(2)→ device —`certified-by`(2)→ root; activity —`used`(1)→ inputs; item —`derived_from`(w)→ item; activity —`on_behalf_of`(2)→ oracle seat. Built from `.bboxy/log/` plus the roster; no hand maintenance. Queries it answers that the other graphs cannot: "every write to this item, by which key, under which grant, verified how"; "all items a revoked seat touched after epoch N" (walk seat→activities→items); "which accepted items have no verified `derived_from` closure". The brief called it the fifth graph; the spec table now has five (`work-items.md:62-67`), so it lands as the sixth, and the artifact graph becomes its leaf layer as `work-items.md:65` already says.

### Cheap now vs later

| Now (v0.1, no new crypto deps beyond identity's) | Later |
|---|---|
| `created_by`/`updated_by`, `derived_from` with weights, `provenance.digest` over bytes | P2 signatures on every line (needs seat leaf custody from [[identity]]) |
| `.bboxy/log/<date>.jsonl` hash-chained, unsigned lines + operator-key signed checkpoint via `gpg --detach-sign` | Device-key checkpoints, Merkle root per checkpoint, Rekor v2 export |
| Trailers via `prepare-commit-msg` (option A), operator primary signs commits | Option B leaf signing + `allowed_signers`/roster keyring for `git verify-commit` |
| Transcript references `{harness, session, ordinal\|uuid}` from hooks and rollout files | OTel GenAI span export over the seat socket (`vision.md:56`) |
| `.bboxy/schema.yml` (Dendron-style) + inverse `rel` table (Jira-style) | Lamport/sequence ordering for concurrent frontmatter edits (git-bug) — answers `work-items.md:113` |
| Foam link-reference definitions so GitHub renders `[[…]]` | JSON Canvas export of any graph |

### Gaps

- **Gap:** Obsidian renders `[[slug|rel|w]]` with `rel` as display text. Keep or move rel/weight out of the pipe? — operator call.
- **Gap:** who signs activity lines for a remote seat whose leaf lives on another host; `work-items.md:114` phase 2.
- **Gap:** OpenPGP leaf ↔ `git verify-commit` interop untested (`identity.md:210`); ssh `allowed_signers` path also untested here.
- **Gap:** lakeFS, Linear relations, Logseq and Roam official docs unreachable; facts for those are secondary or absent.
- **Gap:** Codex `commit_attribution` default trailer confirmed only from a third-party post; official config reference slice read did not contain it.
- **Gap:** Rekor v2 entry types and checkpoint format not read from primary spec (C2SP tlog-tiles); export path is a shape claim, not a tested one.

Links: [[work-items]] [[identity]] [[messaging]] [[vision]] [[mcp-control-plane]] [[subagent-detection]] [[harness-adapters]]
