# GitKB data source

`bboxy450-term-data_source-gitkb` is a single Rust executable, built on
`bboxy450-term-plugin-sdk`, serving MCP over stdio in front of the installed
GitKB CLI. It implements the eight `data_source` 1.0 methods and
the optional explicit `commit` method. GitKB owns its storage, workspace tracking,
UUIDs, parsing, relationships, generation checks and native commits. The host
owns signing, role policy, delegation and task acceptance.

Build and check from the repository:

```sh
make build-gitkb-plugin
make check-gitkb-plugin
make probe-gitkb-harmony PLUGIN_WORKSPACE=/absolute/project
make install-gitkb-plugin-local PLUGIN_WORKSPACE=/absolute/project
```

`GITKB_BIN` selects the installed backend at build time (`build.rs` reads it as
`BBOXY_GITKB_BIN`; unset, `git-kb` on PATH); `TARGET_DIR` selects the
worktree-local output. The build pins the backend's canonical path, SHA-256 and
`--version`. Contract schemas come from the shared `bboxy450-plugin` crate, so
the executable needs no adjacent source, schema or skill files.
`plugins/gitkb/SKILL.md` is the canonical UTF-8 skill: its exact contents and
frontmatter metadata are embedded in `Shape.skills`. Rebuilding after a skill
edit changes the executable digest and requires retrust. The pinned GitKB
installation is the only runtime dependency.

`initialize` hashes the backend once and refuses `backend_changed` when it differs
from the pin. Every later call compares the binary's file identity (device,
inode, size, modification and change times) with the identity that was hashed;
only a changed identity triggers a new hash, and a different digest refuses
`backend_changed` before the binary runs, including through an already
initialized client, until the plugin is rebuilt and retrusted.
This provider requires the host's optional commit and signed-writer extensions.

Installation requires an explicit project, creates only its local plugin and
notice directories, and grants no host trust. This assignment installs only into
the operator-authorized Harmony workspace. Notices live outside executable
discovery, under `.bboxy/plugin-notices/gitkb`. Existing configuration is untouched.

In the app, discover the local candidate, trust its current digest, run its
conformance check, enable it and select GitKB for the intended swarm's knowledge
root. These are host operations. The plugin itself grants no trust or policy.
The Oracle's host integration fixture verifies these operations with a private
account before real activation. Ordinary `plugin check` uses its disposable
native GitKB fixture; it never points at a production knowledge base.

CLI plugin controls and `knowledge status/select/reset` accept `--swarm ID`.
Use the Harmony swarm ID on every activation step: `plugin list`, `plugin trust
<absolute-installed-path>`, `plugin check <candidate-id>`, `plugin enable
<candidate-id>`, then `knowledge select <root-id> <candidate-id>`. Obtain the
candidate and root IDs from the scoped list/status replies. Each command captures
the swarm's current workspace revision; a stale scope cannot select another root.
Other workspaces, including bboxy450-term itself, retain their native local KB.

Seats can call read-only `knowledge_status` to see the selected root/provider
digest, available choices, activation/check states and failures. An installed
or enabled provider is not selected automatically. Compare its selected digest
with the builder/installer output, then fetch a known native slug through the
seat to verify activation. `make test-harmony-host-gitkb` proves this complete
sequence against the current host in a private account, including restart.
The host keeps `.bboxy/plugins` and `.bboxy/plugin-notices` out of ordinary
knowledge inventory while preserving every license and notice on disk.

`make probe-gitkb-harmony` checks three fresh read-only sessions without retries,
including exact native raw/body/identity/generation parity, first/continuation
pages, links, changes and repeated resolution. Its JSON evidence records failed
operations too. It also requires `status:active` lists and `type:task
status:running` searches to return items, and every call to finish within two
seconds; the evidence records each call's elapsed time and item count.

`--shape`, a terminal run, or a run with closed stdin prints the descriptor. An
open stdin pipe serves newline-delimited MCP JSON-RPC, including the host's
no-argument launch; `--serve` explicitly selects that mode. Stdout contains only
the selected protocol. The first `initialize.root` binds one process to one
project, its `.kb`, or its `.bboxy` directory. The exact project must already
own a real `.kb`; the adapter never searches ancestor projects or initializes
production data. Normal backend processes keep the host's private plugin cwd.
Only the documented environment allowlist is inherited, with generated
`GITKB_ROOT` and `GITKB_WORKSPACE` selecting the bound source.

The initialized MCP server advertises `resources` and serves the same embedded
skill through `resources/list` and `resources/read` at the fixed URI
`gitkb://skills/gitkb/SKILL.md`. Unknown, escaped and filesystem URIs are refused;
resource reads never search cwd or adjacent files. This is a compatibility
resource surface, not an advertisement of the beta Skills-over-MCP extension.
Host delivery requires support for the additive `Shape.skills` manifest; older
hosts may ignore it. Supporting hosts relay only selected, trusted provider
skills through their admitted interface. Instructions grant no additional
tools, signing or task authority.

The disposable checker alone may send
`fixture: {version: 1, kind: "data_source_conformance"}`. A guarded
`checker-knowledge/.bboxy/p0-fixture.md` under the system temp directory is
required. Its native `.kb` is initialized through the real CLI and its seed is
imported through `workspace write`. This setup has an explicit 15-second budget:
a measured native init took 6.077 seconds, including a SQLite checkpoint.
Production initialization and the eight base methods retain the two-second cap.
Only this disposable fixture permits unsigned conformance writes.

GitKB and portable document semantics:

| GitKB | Portable contract |
|---|---|
| Canonical UUID | Same 128 bits encoded as ULID in `id`; UUID retained as `external_id` |
| Full slug | First-class locator and relative `slug.md` path; updates retain native identity |
| Native frontmatter number | Preserved; numbered family suffix is a fallback handle; collisions return candidates |
| No numeric handle | `number: null`; no separate numbering ledger |
| Explicit workspace or committed source | Source-aware digest and GitKB generation `etag`; no fallback from a deleted workspace |
| Frontmatter and Markdown | Native parsed frontmatter/body preserved, with canonical metadata under `gitkb` |
| Typed native graph relationships | Portable edges with native relationship names; unweighted native edges have weight 1 |
| Native task status | Draft/backlog → open, active → running, completed → done |
| Host lifecycle status | Native extension `bboxy_status`; outward portable `status` |
| Host-signed write intent | Exact opaque `bboxy_signed_record` string; dedicated `signed_record` result field |

The proof and native lifecycle extension are removed from ordinary outward
frontmatter after projection, so repeated task transitions do not recursively
nest proofs. The host must verify its proof and acceptance evidence. A native
completed task does not become host-accepted merely because it completed.

Writes use `git-kb workspace write --stdin --if-generation`; creation uses its
atomic `--create-if-missing` path. Updates to committed-only records first use
the absent workspace's generation together with `--create-if-missing`, so native
CAS checks the committed revision and materializes the workspace in one write.
Graph-derived native `references`, `blocks` and `children` stay in the graph;
they are excluded from editable source frontmatter. All document
and task fields go to that same workspace source. There is no automatic commit,
push, alternate Markdown database or separate delegation engine.

Explicit commit takes 1–32 exact record references and a supplied message. It
preflights current source references and invokes `git-kb commit` with only their
slugs. The result retains the native commit UUID, plus the admitted input refs.
This explicit durable operation has a separate 15-second cap, matching bounded
local Git commit; it runs in host IO side work.
Resolve again to obtain canonical post-commit references. Unrelated workspace
documents remain pending. Native commit has no generation-CAS option: an
external edit between preflight and commit can change the committed bytes.
The adapter promises the native transaction, not a stronger atomic guard.

Pages fill up to the requested `limit` (default 32, at most 128) while the call
budget lasts: a page stops with a `next` cursor when about 400 ms of the
two-second budget remain (graph pages keep twice that for their edge read), or
before its serialized items would approach the 256 KiB result cap. Each call
examines at least one record, and keeps scanning past filtered-out records, so
empty pages with a `next` cursor occur only when the budget ran out. Follow
`next` through them as well.

Committed-only records (no workspace file) are read in one batched
`git-kb show --raw-content --json -- <slug>...` call; their raw bytes equal the
committed source, so their references and digests equal an exact read's.
Records with a workspace file use the exact source-aware read. A per-process
cache keyed by slug and the record's full catalog row (including the committed
`modified_at` or the exact workspace file stat) reuses earlier summaries and
exact graph records; `fetch`, `write`, `commit` and reference checks always read
exactly. Known gap: a document deleted in the workspace but not yet committed has
no workspace file, so a listing shows its committed reference and an exact fetch
then answers `not_found`; detecting it needs `git-kb status`, which costs about a
second on a large KB.

Catalogs, filtered or not, come from an in-memory index once it is ready,
instead of one `git-kb list` per call. `initialize` starts a background thread that
runs `git-kb export --format jsonl` (all committed documents; 4.0 s and 61 MB
for Harmony's 3691) and keeps only each document's native id and its
`assignee`, `swarm` and `bboxy_status` values. Output is capped at 1 GiB,
lines at 64 MiB and documents at one million; malformed and duplicate lines
are counted under `health.index.skipped` and those documents stay
candidates. One unfiltered native `git-kb list` supplies the exact catalog
rows (title order, type, status, ids), so index-built catalogs have the same
rows and snapshots as the backend path. The index reproduces native 0.2.19
prefilter semantics measured on a private fixture: `--type` and `--status`
lowercase their input and compare the row exactly; `--where` compares the
committed frontmatter of committed documents, even when their workspace file
differs, and the workspace source of workspace-only documents. A document
whose values are unknown stays a candidate. Items, summaries and `RecordRef`
digests still come from the batched or exact reads above, which apply the
authoritative filter, so a committed value is never served for a
workspace-backed record.

Freshness: the committed layer is valid for one store signal, the (device,
inode, size, modification and change times) of `.kb/store`, its
`manifest.json`, `documents`, `commits`, `refs` and `refs/document-tips.json`.
A native commit renames a new manifest and tips into place; workspace writes,
`rm` of a committed document and every read leave them unchanged (measured).
GitKB's `.kb/.cache` database is excluded because reads rewrite it. The row
layer is also keyed by the exact workspace file stats. A changed store signal
starts one background rebuild. While a build runs (including a cold process's
first reads), a call that needs `--where` fields waits for it, at most 700 ms
and never past its own deadline less 800 ms; it takes the backend path only
when the build fails, finishes for another store signal, has not finished by
then or the call is cancelled, and whenever the store or workspace moves
while rows refresh. The host's per-call deadline is the declared 2 s,
shortened to whatever its operation (an 8 s task list) has left, and the
plugin cannot see that remainder; short waits keep each cold call near one
second. A cold Harmony process answered the host's 126-call task list in
4.0 s from spawn with no call over the host's deadline (initialize 0.74 s,
build ready 2.97 s, slowest call 0.72 s), and a respawn that loaded the
snapshot in 1.15 s (slowest call 0.11 s, no fallback). Each finished build is
saved to the plugin's private data directory (its cwd) as
`gitkb-index-<root digest>.json`: a private file written and renamed into
place, holding the format version, root, store signal and per-document
fields. `initialize` loads it synchronously only for the same root, version
and current store signal (Harmony: under a millisecond), so a respawned
process, for example after the host closed its client on a refusal, serves
filtered calls at once; anything else is ignored and rebuilt. Filters
the index cannot prove equal (`since`, other fields, values with spaces or
quotes, numbers and `true`/`false`/`null`) always take the backend path.
`health.index` reports readiness, build and ready times, served and fallback
counts and both signals. `BBOXY_GITKB_INDEX=off` in the plugin's own
environment (not forwarded by the host) disables the index, for comparison.
`make check-gitkb-plugin` compares index and backend results on a fixture,
and `make probe-gitkb-harmony` replays the host task list's 126 filtered
calls against both.

Status filters match the portable status (`running`) or the native one
(`active`), so `status:active` and `type:task status:running` both select active
tasks. Graph edges to non-document targets, such as GitKB code-symbol
references, are omitted. Search pages use GitKB's strict `--all-terms` matching and native FTS
ordering. The paged workspace supplement also requires every term, because
native FTS excludes uncommitted edits. Cursor
tokens are root/query/snapshot-specific, expire when source metadata changes,
and are retained only within this process (128 tokens). Unknown or restarted
cursors return `stale_cursor`; start changes polling with `cursor: "0"` for a new
baseline. Deletions return a stable tombstone digest and their former identity.
For workspace-backed rows, GitKB 0.2.19's read-time `modified_at` is excluded
from cursor/change fingerprints; exact workspace file stats fence edits instead.
Committed-only rows retain their native modification timestamp.
Native filters are prefilters: fetched frontmatter is checked again for type,
status, assignee and swarm because 0.2.19 can include nonmatching workspace
overlays. Swarm graph pages apply the same source check.

Image links are copied from regular, confined GitKB files to the plugin's private
resource directory and linked from there. Symlinks, hard links and oversized
resources are refused. A serialized portable result above 256 KiB returns
`response_too_large`; there is no silent truncation. Cancellation or timeout of
a mutation can leave a completed native write: inspect its source before retrying.

`check-gitkb-plugin` runs the ignored Rust test `tests/conformance.rs` against
the built executable, with fresh private homes, projects and native GitKB
stores. It covers create/update, native source CAS, lifecycle projection and proof
storage, graph and resources, paging, bootstrap, colliding slugs in two roots,
and selected-only native commit. This suite checks storage; cryptographic
admission and real host task transitions are verified by `test-harmony-host`.
The read-only probe (`tests/harmony_probe.rs`) records method durations, references and hashes without
recording document bodies. Its project-brief locator is an explicit Harmony
probe convention, not adapter initialization behavior.

`install-gitkb-plugin` (an example in this crate, so it is never installed as a binary; decision 362) performs the installation with
private temporary files renamed into place: the plugin at mode 0755, and
LICENSE, NOTICE and ATTRIBUTION.md at 0644. It refuses a missing `.kb`, the home
directory, and symlinked `.bboxy`, plugin, notice or destination paths.

See [NOTICE](NOTICE) for GitKB attribution. The plugin invokes the backend;
it does not copy or distribute GitKB source.
