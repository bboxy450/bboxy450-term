---
acceptance:
- Real GitKB MCP binary fulfills data_source
- Harmony-only installation works
- Private-fixture checks pass
allowed_paths:
- plugins
- scripts
- Makefile
artifacts:
- '[[01M6324Z691DJMRAC82F04YH7H]]'
assignee: seat:01M62ZC87EV8MWGA4FS7Y7VFYA
authorship:
  kind: seat
  process_generation: '1'
  public_key: vFR8BYdX3V6luC6QRGyjYg7oXoyfuycnEwl9ve6Q6aI=
  signer: seat:01M62ZC87EV8MWGA4FS7Y7VFYA
created: 2026-10-29T04:46:04.663Z
created_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
delegated_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
id: 01M62ZNFB6N9TT3BGQNE9H5J8Z
number: 369
role: executor
status: done
title: GitKB binary provider and Harmony-only installation
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: make test
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/a80e7b86", "https://github.com/bboxy450/bboxy450-term/commit/c53978bc"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "GitKB provider landed and was later ported to Rust on the plugin SDK. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR

Worktree /Users/feral/mydev/bboxy450-term-harmony-adapter. Allowed new plugin implementation under plugins/ (or propose self-contained crate before edits), scripts/, Makefile and adapter docs. Vesper owns host crates. No recruiting, merge or push.

Read project/shared instructions, vision, plugin/knowledge specs, ATTRIBUTION, Harmony AGENTS/.kb/AGENTS. Study /Users/feral/work/gitkb/harmony/open-source/gitkb and installed CLI. Implement real executable bboxy450-term-data_source-gitkb: bare/--shape JSON with human name/description/optional icon/identity, stdio MCP server, initialize binds root. Fulfill data_source 1.0 resolve/fetch/search/list/links/graph/write/changes_since using real GitKB semantics/backend, not a fake Markdown KB. Preserve numbers, slugs/IDs, types, frontmatter/body, links, canonical GitKB source-aware create/update. Host owns signing, delegation policy and task state transitions; coordinate signed writer wire with Oracle/Vesper. Task/document writes must hit same source. Honest named refusals, paging, stale references, confinement and scrubbed env/cwd.

Add repeatable canonical Make build/check/install-local targets plus unattended private-fixture checks for real create/update and task lifecycle. Install artifact ONLY /Users/feral/work/gitkb/harmony/.bboxy/plugins (operator authorizes this); no global install or edits/test data in real Harmony KB. Read-only Harmony resolution evidence required. Use truthful GitKB attribution. Verification make test on final signed committed source plus appropriate boundary/conformance checks. Report actual exits, source pointers, installed hash, activation evidence/instructions and all gaps. Stop before merge/publish; preserve peers and existing config.