---
allowed_paths:
- crates/bbt-mux
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:32.097Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HEAA842TDT2KA4QVYTV5
number: 1794292861627914
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: artifact_claim refuses a report claimed by path/id after knowledge_commit
type: task
updated: 2026-11-24T17:09:32.097Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: cargo nextest run -p bboxy450-term-mux --test mcp -E 'test(artifact_claim)' --retries 0
provenance:
  digest: sha256:6a6144a18439680d60ce496608fde8e62d2cfcfc0cc5c04f44a77f48ef8c8507
  frontmatter_digest: sha256:c2740f98dc9611b74625eb7d2d9f1c6baf47354ac7e6144a814284097493ca63
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: HRn3fgwj6INrdMavR4PCZOJla0ZzXz4bfvkLs4hludj6CFDleGE84DCSbEmI6EXmX21VugO54dKq8gH7UKnIDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1847
  event:
    id: 01M878HFK32ETW8X494ZSSPCRR
    sequence: 1847
    hash: 1a7ddd9c5df2eebbbb04d64da12680f9e7f2032bc64f3e456dea853823fb0cbf
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-06-artifact-claim-after-commit.md` (field evidence there).

## Evidence
Refusal at `crates/bbt-mux/src/logic.rs:4883-4897`; `provenance/engine.rs:527` `claim_ready` looks entries up by ref and is unchanged since `342faefd5`. The existing fixture `tests/mcp.rs:4305` only claims the `knowledge:commit/<sha>` ref.

## Do
Reproduce the field case (claim a report by path or artifact id after `knowledge_commit`) as a failing test first; make `claim_ready` resolve the committed record by its receipt; keep a control where an uncommitted draft is still refused. `make land`.
