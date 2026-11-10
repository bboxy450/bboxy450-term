---
type: note
title: Lossless archive of completed Dream producer formal evidence
id: 01M6RSXKCDZF76WP75HTCT9KD8
updated: 2026-11-06T16:08:57.335Z
created: 2026-11-06T16:08:57.335Z
number: 93377961366029
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:fb0ab09613c05e206b7f1e638f1565512d9a202baec3f8eb0b951a6dc464b94d
  frontmatter_digest: sha256:0526afb5f395f39f3887e85e1d49e2d41fe19293f44276467cb317211e7309d5
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: Zt4LTAUSIJDtF107+kgCOqfMUVXx26Ox/D78Shcxcq65ecF/MvstDAtrz4tnoJEWncNlr1HaudrCGng1Ha5rDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149546
  event:
    id: 01M6RSXKV5XQ6VJTK9RBQPFNRV
    sequence: 149546
    hash: 6ce37ef252c0fa316284934bcdfe3d4b6e565e765485f6d514d053aa6cb6df17
---
# Completed producer formal evidence retained losslessly

On2026-11-06 Root archived only the completed accepted producer formal directories, after verifying old47182320/d06 worktree heads and empty git cherry origin/main HEAD. Each exact formal directory had lsof exit1 with empty stdout/stderr before indexing AND before cleanup. The worktrees themselves remain; app-held root directories were not force-removed.

Archive: /Users/feral/mydev/bboxy450-term/target/dream-completed-formal-archive-u8u6q_po/completed-formal.tar.zst,446407760B, SHA256f6bf57db8c642ac1567637b772136d8394cec017b7c54809f1791516a263d844.
Manifest: same directory/manifest.json, SHA2560b41985f4b5657c4b5e1036cc57a93e2f4838f2a190d06d62147fb03a254b912. receipt.json and README.txt record paths, restoration and verification.

Original exact directories:
- /Users/feral/mydev/bboxy450-harness-dream-c1/target/formal
- /Users/feral/mydev/bboxy450-harness-dream-native-c1/target/formal
- /Users/feral/mydev/bboxy450-term/target/dream-c1-reviewed-d06-geht0ngf/formal

Native tar plus single-thread zstd ran niced10, both exit0. Streaming decompression/tar verification compared all8688 regular-file SHA256/lengths, all symbolic-link targets and10882 total entries with the pre-index manifest; original inode/size/mtime/ctime were rechecked unchanged. Only after successful verification and repeated no-open checks were those THREE original directories removed. Logical original bytes5134382788; source, ordinary/fixture workers, profiles, reports and gate logs outside those formal directories were preserved. No live source, active-target graph, peer/global archive, operator process or credential was removed.

The archive retains original logical names relative to /Users/feral/mydev. For inspection restore into a NEW EMPTY private directory and verify manifest; do not overlay a live worktree. Historical reports' old formal paths now resolve through this manifest/archive. Compressed storage is not a new verification verdict; existing source/model/tool/hash/parity limits remain.

Physical df did NOT show a matching recovery: shared available space was4.5GiB before and about4GiB afterward. Cause of that difference is not established; no physical5GiB reclamation claim. Consumer26e5's failed ENOSPC archival remains Failed2 and its own exact unpublished staging-u8jow54c/cache cleanup is assigned separately. [[decisions/d93377961366026-dream-c1-bounded-persistence-preparation]] separately governs the real private SQLite/authority CPU correction, with1ms law unchanged.
