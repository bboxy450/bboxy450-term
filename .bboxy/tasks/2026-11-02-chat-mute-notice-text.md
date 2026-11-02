---
acceptance:
- Mute and unmute chat rows show the affected seat name and action beside the timestamp.
- Live and retained/reopened history use the same readable projection without changing signed envelopes.
- Private native regression and required landing checks pass.
allowed_paths:
- crates/bbt-mux/src/messaging/service.rs
- crates/bboxy450-term/tests/swarm_workdir.rs
- Makefile
- .bboxy/specs/messaging.md
- .bboxy/reports/2026-11-02-chat-mute-notice-text.md
- .bboxy/tasks/2026-11-02-chat-mute-notice-text.md
assignee: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
authorship:
  kind: seat
  process_generation: '1'
  public_key: dz6NmhXKbHy9Oj4XPzJo0/rAkWqtxM4tdPbxlsgDDG4=
  signer: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
created: 2026-11-02T18:29:20.382Z
created_by: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
delegated_by: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
id: 01M6ERBS9H6X72BA83WYFA8SWS
number: 5002216
provenance:
  digest: sha256:13b0963755700b0c19c03ccf768bc63e24dbfc5fdf2f5b692622a330f4887286
  frontmatter_digest: sha256:e962ffbee954938062c9e20083284124436078a919cfc131ad3fe81b51b6953c
  log: .bboxy/log/2026-11-02.jsonl#536
  sig: yUUPxo/SFWdxwLt7kkpRPMTqedYOESqGIbk9NVWV1fpI5ZTFWu13TzFLRQOHCPorQ0ryin1O16UYt6+lQ+L1DQ==
  signer: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
role: solo
status: claimed
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: Show mute and unmute notice text in chat
type: task
updated: 2026-11-02T18:29:28.916Z
updated_by: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
verify: make test-chat-mute-notices
---
ROLE: SOLO

Direct operator brief: mute/unmute in chat currently produces timestamp-only rows; fix promptly and land alone.

Worktree: /Users/feral/mydev/bboxy450-term-chat-mute. Branch: fix/chat-mute-notices.

Project structured Presence bodies into readable MessageRow text for mute/unmute. Preserve signed source envelopes, IDs, ordering, timestamps and policy behavior. Add focused projection/history and private native chat regression evidence. Update the existing messaging spec, run the focused Make target, license and formal checks, then land through make land. No delegation, operator desktop access, or unrelated changes.