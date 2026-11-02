---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:19:54.451Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQTGH0ZHAZ09Y5J4NY9MFY
number: 5002209
provenance:
  digest: sha256:620b09d4183fc359223a1120b44889279833a5f69e163e3fbcdc3f57d049cdb2
  frontmatter_digest: sha256:0fbcbec0fe1424dc4f67f201db677c9d35c9a2f698712a563969d733e46ee8e1
  log: .bboxy/log/2026-11-02.jsonl#2595
  sig: 2yGOn3oc0+7vGri074tMhE0fbkBtjcnkwKL1NI/AD6Ndx3Wn0KsauQ9F0Vj8gPdMWFlBos1gKbE2LrJz/g59BA==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/844b822cc7ea85d8092288b9f3402ffa77db0ea2
  - 'make land (lean) LANDED b681d962: ctl::tests::journal_keeps_one_usage_line_and_whole_other_reasons in bboxy450-term --lib, 432 passed'
  fresh_accepted: false
  kind: historical
  op_id: 01M6EXASH2PY54VN84V77S2KRD
  previous_status: open
  reason: 'Landed: a usage refusal is journaled as its first line only (timeouts and other reasons stay whole), and `ctl help` prints help with exit 0. Usage refusals stay journaled by the existing contract (diagnostics.rs malformed_cli test). Correction: the 10:15 help-page row came from `ctl help`, not `ctl debug errors --help`, which already printed help.'
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl usage errors and --help land in the error journal as failures'
type: task
updated: 2026-11-02T19:56:10.688Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
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
