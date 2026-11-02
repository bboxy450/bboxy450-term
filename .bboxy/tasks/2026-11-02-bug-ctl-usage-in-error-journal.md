---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:19:54.451Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTGH0ZHAZ09Y5J4NY9MFY
number: 5002209
provenance:
  digest: sha256:620b09d4183fc359223a1120b44889279833a5f69e163e3fbcdc3f57d049cdb2
  frontmatter_digest: sha256:d71ab676a8a7c42b21c1d4e1848e18aef0cc6ef25dfe579a4fe39b6239e024ac
  log: .bboxy/log/2026-11-02.jsonl#410
  sig: RKOPrVzMOeK/NjeN8p/q+EpjGlqzGVBjR3VGApBlr5CcsaZCSflnrRpX7bewz+KhAKfLMEFrU7uWeVL5IZKeCA==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl usage errors and --help land in the error journal as failures'
type: task
updated: 2026-11-02T18:19:54.451Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify: cargo nextest run -p bboxy450-term --test ctl
---
**Observed.** The error journal holds 6 rows with `source: ctl, operation: command` whose message is ctl usage text. These come from 2026-11-01 16:13 through 2026-11-02 10:15:
- the multi-line help page, truncated mid-word (`"ct\napp endpoint the run directory…"`);
- `usage: seat verbs: ls, attention, …` (2 rows);
- `usage: invalid_ui_key: unsupported chrome key; use seat key for PTY input`.

The 10:15 row was caused by `ctl debug errors --help`. ctl has no per-verb `--help`, so the request fell through to the usage error.

**Impact.** Typing mistakes count as unread failures and mix with real faults. The help page is stored as truncated, multi-line diagnostic text.

**Fix.**
- Do not journal `kind: usage` (exit 2) refusals, or journal only a one-line class without the help body.
- Accept `--help` after any noun/verb and print that noun's lines with exit 0.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
