---
type: decision
title: Preserve validated managed launch inputs in the feature-gated C1 session fixture
id: 01M6P0PXTHKJNRT4F5CX13V52X
updated: 2026-11-05T14:09:55.600Z
created: 2026-11-05T14:09:55.600Z
number: 93377961364648
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:ccbbd3a9f981af1eb9bf0242f1a463b8f0a8da7702ae39b4f2ec21b35c609a72
  frontmatter_digest: sha256:f22b82c82a71759d0f1fa9ef6b428a8c0afa0cd3b28d4590aae75a9168159e58
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: PtanRbviQzJwzvLtj6NvInEIFtYRLVtAAGTr7RrUOeAKNbn1y682Q5D4wZFNlbam7rf8pqGemoYLyGiM1U3nCw==
  log: .bboxy/log/2026-11-05.jsonl#136503
---
# Managed private inputs for the C1 retained-Host fixture

Clarifies [[decisions/d93377961364606-dream-c1-private-host-session-witness]]. Keep the existing test-fixtures compile feature, explicit seat/json/scripted provider/TermC1 mode, exactly one fresh nonfork next session, private accounts/paths and ordinary-close isolation. Do not silently strip or transform the managed launch into a different scenario.

The private seat fixture may preserve the managed launcher's explicit --profile, --mcp-config and --append-system-prompt. Resolve only the supplied profile inside the private HOME/.bboxy450-harness/profiles tree; require an owned regular nofollow single-link 0600 bounded profile/config file. Keep the selected name/metadata, auto_compact:false and normal read_launch MCP behavior. ScriptedProvider stays forced; no provider credentials, network/discovery, onboarding fallback, ACP, resume or replay path is enabled. Invalid private profile/MCP input fails the fixture, rather than replacing it with an unrelated scripted metadata profile.

Accept the supplied private layout: fresh owner-0700 fixture root (for example /private/tmp/dc1-*), HOME at root/home, cwd at sibling root/repository, owned runtime/socket/overlay directories under that SAME root. Existing HOME/.bboxy450-harness/{profiles,sessions,run} is allowed. Do not require the entire sessions directory or harness home to be absent/empty; require the actual new A/B log names and socket publications not to clobber existing owners. Validate the common root and explicit paths/components with no symlinks/traversal, correct uid and bounded sizes; do not inspect an ambient home or crawl the tree. Standalone private layouts wholly within HOME remain valid. Data-file 0600 and private-root ownership requirements remain; source fixtures may explicitly chmod their generated files.

Managed seat environment allowlist: BBOXY_SEAT_ID, BBOXY_GENERATION, BBOXY_ROLE, BBOXY_SOCK, BBOXY_CAPABILITY_FILE; TERM_PROGRAM, TERM_PROGRAM_VERSION; HOME, ZDOTDIR, CODEX_HOME, CLAUDE_CONFIG_DIR, PATH, TERM, LANG, LC_ALL, TMPDIR, TZ. Validate canonical seat ID/nonzero generation and bounded role/metadata; all config/home/runtime/overlay paths belong to that same private root. The socket and capability path must match the private admitted overlay (capability file sibling of its mcp.json), regular/owned/nofollow metadata as appropriate. The fixture validator/report never opens or logs capability contents. The ordinary existing private MCP bridge may consume its capability through its normal admitted protocol. Environment labels/role strings never grant authority. No raw BBOXY_CAPABILITY, arbitrary BBOXY wildcard, API credentials or ambient bootstrap/custody fallback. Reject unsupported inputs explicitly, never silently remove them.

This applies ONLY to the feature-gated PRIVATE SEAT fixture. The separate real dream inspect-c1 worker still receives the ICD's cleared minimal environment and no HOME/profile/MCP/seat capability. Do not relax its startup/custody boundary.

Witness the actual managed profile/auto_compact metadata, generated MCP wiring and admitted seat IDs/generation survive the joined test, with zero live credential fallback and unchanged normal close/default builds. Cover invalid mixed roots, symlink/mode/ownership, unknown/credential environment and missing feature/mode guards. Existing ordinary restart/cancel, >=240 same-app frames, real worker/foreground and all resource/latency gates remain.

Both [[tasks/2026-11-05-dream-c1-producer]] and [[tasks/2026-11-05-dream-c1-consumer]] may implement this within their existing headless/seat_process/private-fixture source paths. No new product lifecycle, dependency, service or authorization surface is introduced.