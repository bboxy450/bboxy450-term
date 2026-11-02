---
id: "01M6D1ZREJPSFM7CAPMGFTR773"
type: "task"
number: 5004
title: "gitkb plugin: fix the eight findings of the git-kb CLI side-by-side review"
status: "open"
role: "executor"
verify: "cargo test -p bboxy450-term-data_source-gitkb && make land"
created: "2026-11-02T02:40:00Z"
updated: "2026-11-02T02:40:00Z"
---
# gitkb plugin review fixes

A side-by-side review compared the GitKB data_source plugin (`plugins/gitkb`) with the
`git-kb` CLI on Harmony KB doc `tasks/harmony-2119`. It reported eight findings, each
with a decided fix. Each finding is reproduced first (red test or recorded command),
or gets a non-reproduction note with evidence.

Worktree `~/mydev/bboxy450-term-gitkb-review`, branch `gitkb-review-fixes`. Harmony KB
is read-only (`show`, `list`, `sync status`, `push --dry-run`); write tests use temp KBs.

## Acceptance

- [ ] 1. The signed record leaves the document: no `bboxy_signed_record` frontmatter. The
  proof is kept in the plugin data dir, keyed by document identity and version digest.
  The seat-attribution fields stay. Host write verification keeps working.
- [ ] 2. "Create next in family": family plus title runs
  `git-kb create task --next <family> --title <t> --json` and returns `slug`,
  `authority` and `sync_required` unchanged. Plain `knowledge_write` is unchanged.
- [ ] 3. One status vocabulary: only GitKB's native `status` is stored, never
  `bboxy_status`. One documented table maps host↔native both ways; unmappable statuses
  are refused with a typed error; docs that carry `bboxy_status` read through `status`.
- [ ] 4. `id` is the bboxy ULID, `external_id` the GitKB UUID, `slug` the path, in every
  result. Every tool schema says which identifiers it accepts; slug and ULID both address a doc.
- [ ] 5. Fetch pages by default (about 16 KB), with `truncated` and a next cursor; a
  50 KB body comes back in pages.
- [ ] 6. `workspace_pending` waits within the call's deadline, then fails with `retry_after_ms`.
- [ ] 7. Fetch carries `references`, `depends_on`, `blocks` and `parent` equal to
  `git-kb show <slug> --json`; the cause of the `tasks/harmony-2119` parent disagreement
  is recorded as file:line plus mechanism.
- [ ] 8. A read-only sync check for a pathspec wraps `git-kb sync status <remote> '<pathspec>' --json`,
  reads the remote from the KB config, never invents `origin`, never pushes, pulls or repairs.
- [ ] Unchanged: scoped commits, human commit authorship, search parity with `knowledge_search`.
- [ ] Temp-KB e2e with the real `~/.cargo/bin/git-kb`; `make land`.

## Evidence

Red runs used the unchanged plugin binary (built from `86ce2653`) and a scratch
worktree at `86ce2653` holding only the new host tests. Green runs use this branch.
Plugin checks: `plugins/gitkb/tests/conformance.rs` `review_checks`, run alone as
`cargo test -p bboxy450-term-data_source-gitkb --test conformance -- --ignored --exact review`
and inside `make check-gitkb-plugin`. Host e2e: `make test-harmony-host-gitkb`.

1. **Signed record in frontmatter.** Red: native frontmatter held
   `"bboxy_signed_record":"review proof one"`, and after another editor changed only the
   body the fetch still returned that proof (stale). Cause: baseline
   `plugins/gitkb/src/methods.rs:787` inserted the host proof (`plugins/signed.rs` payload:
   the whole request, body and authorship) into the YAML. Fix: `store.rs` `store_signed`/
   `load_signed` keep it in `<plugin data dir>/signed/<sha256(native id, version digest)>`
   (0600 files, 0700 dir); `methods.rs` `write` stores it after the readback, and
   `read_source_state` returns it only for that exact digest. Host
   `plugins/source.rs` no longer accepts a proof from `frontmatter.bboxy_signed_record`.
   Green: no proof in the raw document, `authorship` kept, fetch returns the proof, a body
   edit by another editor returns none. Host e2e asserts the raw GitKB document of a real
   signed write has no `bboxy_signed_record` and keeps `authorship`/`updated_by`.
2. **No numbered slug allocation.** Red: `tool_create_next` → `unknown_method`. Fix:
   declared seat tool `create_next` (Oracle) runs `git-kb create task --next=<family>
   --title=<title> --json` and returns `slug`, `authority`, `sync_required` unchanged
   plus `ref`. Green: `tasks/demo-1` with `authority: local`, `sync_required: null` (the CLI
   omits it for local families); an offline remote-reserved family gives `tasks/prov-1`,
   `authority: local_provisional`, `sync_required: true`. Host e2e: seat tool
   `plugin_tool_gitkb_create_next` → `tasks/review-1`, `authority: local`.
3. **Two status vocabularies.** Red: writes stored `bboxy_status` plus native
   `status: draft|active|blocked|completed`, and a legacy `bboxy_status: claimed` read as
   `claimed`. Cause: baseline `methods.rs:784` and `native.rs:33`. Fix (revised after
   review: GitKB `status` is free text, `gitkb-types/src/document.rs:738`, so nothing is
   unmappable): `native.rs` `TASK_STATUS` maps every host state to one native status,
   GitKB's word where one exists (open→backlog, running→active, blocked→blocked,
   done→completed) and the host state verbatim otherwise (claimed, reported, accepted,
   rejected). `accepted` is not folded into `completed`: the host tells them apart
   (`mcp/work_items.rs` reconcile refuses over `accepted` but rewrites `done` to `done`).
   Reads invert the table exactly; `draft` and unknown statuses (`investigating`) read as
   `open`, the document keeps the raw value, also through an update that keeps the task
   open, and an `open` filter uses no native prefilter so those documents list. Only a
   status outside the host enum is refused with `unmappable_status`. `project` drops
   legacy `bboxy_status`/`bboxy_signed_record` unread. Documented in
   `plugins/gitkb/README.md`. Green: all eight host states round-trip; host e2e runs
   delegate → report → accept and checks native `backlog`, `claimed`, `reported`,
   `accepted` with no `bboxy_status` at each step.
4. **`id` means two things.** Not reproduced in results: every plugin result (write,
   fetch, resolve, list, search refs, graph nodes, changes) already had `id` = ULID,
   `external_id` = UUID, `slug` = path, and resolve accepted slug, path, ULID and UUID
   (GREEN at baseline, 12 results checked). Reproduced in the schemas: red
   `every_knowledge_tool_names_the_identifiers_it_accepts` ("knowledge_fetch
   description lacks ULID"). Fix: `mcp/knowledge.rs` `IDENTIFIERS`/`LOCATOR` in every
   knowledge tool description; `plugins/host.rs` data_source seat tool descriptions; the
   plugin's own tool descriptions.
5. **Fetch does not page by default.** Red:
   `fetch_pages_bodies_by_default_with_an_explicit_truncated_flag` "page of 51205 bytes".
   Cause: baseline `mcp/read_pages.rs` `body_page` returned the whole body whenever it fit
   the MCP result bound. Fix: `BODY_PAGE` = 16 KiB, UTF-8 safe cut, `next` cursor and
   `truncated` (more follows) beside `body_truncated`. Host e2e: a 52,700-byte GitKB body
   comes back in at least four pages of at most 16 KiB and reassembles exactly.
6. **workspace_pending gives no retry time.** Red: the held-scan test's refused write
   had `retry_after_ms: null` and returned at once (the existing 500 ms assertion). Fix:
   `logic.rs` `control_call_within` resends a `workspace_pending` refusal every 100 ms
   (every such refusal precedes admission) until the wait ends, then adds
   `retry_after_ms: 100`; the seat socket waits `SEAT_PENDING_WAIT` = 4 s. Green: a
   300 ms wait still answers pending with the hint and admits nothing; a call waiting when
   the scan is released applies once.
7. **Relationships missing from fetch.** Red: fetch had no relationships. Fix:
   `methods.rs` `relationships` copies `references`, `depends_on`, `blocks`, `parent` from
   `git-kb show --json -- <slug>`; fetch and the write readback carry them; contract
   `Record.relationships` (optional, additive). Green: equal to `git-kb show` for a child
   with parent and depends_on, an orphan, a parent and a plain task; host e2e equal for a
   written note. Harmony read-only probe (this branch's plugin): `tasks/harmony-2119`
   relationships equal `git-kb show` (`parent: null`, two references).
   **2119 parent cause:** baseline `plugins/gitkb/src/store.rs:534` reads the workspace
   source first (`git-kb show --source workspace --raw-content --json`), whose
   `document.frontmatter` keeps the raw YAML `parent: tasks/harmony-2092`, and
   `store.rs:242` (`project`) strips only `references`/`blocks`/`children`, so bboxy showed
   that value. `git-kb show tasks/harmony-2119 --json` reads committed storage
   (`source: sqlite`), where the parent edge does not resolve because
   `tasks/harmony-2092` does not exist (`git-kb show tasks/harmony-2092` → "Document
   'tasks/harmony-2092' not found"), so it reports `parent: null`. bboxy now agrees through
   `relationships.parent: null`; `frontmatter.parent` still mirrors the source YAML.
8. **No sync visibility.** Red: `tool_sync_status` → `unknown_method`. Fix: declared seat
   tool `sync_status` (Worker) reads remotes with `git-kb remote list --json`, uses the
   only one or the named one, never assumes `origin`, and runs only `git-kb sync status
   <remote> <pathspec> --json` (`backend.rs` `cli_report` keeps its JSON from stdout or
   the trailing stderr object). Green: `remote_unconfigured` with no remote,
   `remote_required`/`unknown_remote` with two, the CLI JSON with one. Harmony read-only:
   `remote_required` lists `origin, staging`; `remote: origin` returns the CLI's
   `authentication error: unauthorized` (expired login on this machine; a successful
   status is unproven here).

Doctor follow-up: **unstable `gitkb.created_at`.** Red (`review_checks` "doctor"):
two identical fetches of a workspace-only note, 1.1 s apart, returned different
`created_at` and `modified_at` (`04:20:00.718826Z` then `04:20:01.918839Z`), and a
committed document's clean checkout returned read time instead of its committed
`created_at`. Cause: `git-kb show --source workspace --raw-content --json` (0.2.19)
reports both timestamps as read time, and `store.rs` `read_source_state` copied
`document.created_at`/`modified_at`. Fix: `read_source_state` takes both from the
workspace source's `availableDocumentSources[].modifiedAt` (the file timestamp,
stable between reads); `methods.rs` `stable_created` sets `created_at` from
`show --source committed` when the document has a committed revision, on fetch and
the write readback only, so list and search pay no extra read. Green: equal values
across identical fetches for both documents, and the checkout's `created_at` equals
the committed one.

Unchanged and rechecked: scoped commits (`commit` still refuses another document;
conformance and host e2e commit exactly the named slugs), human commit authorship
(native config user), search parity (conformance compares index and backend search).
