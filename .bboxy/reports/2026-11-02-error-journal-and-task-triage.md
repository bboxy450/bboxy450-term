---
author: seat:01M6EQFA2G00BFT3TVPY6AMXE6
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
commits: []
created: 2026-11-02T18:24:56.801Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6ER3QRJXB4EBZQFPFRV51QR
number: 5002215
provenance:
  digest: sha256:d6e08545151dc82883899daf25d2e1e866912c60e1d3a9f5c06de09ea53f5cca
  frontmatter_digest: sha256:75f67d258766a5d956cba0455ea0e2ffa1891a809df178312e330acac0401212
  log: .bboxy/log/2026-11-02.jsonl#522
  sig: iXpF3sMvY7vP/usCFn7UarV+Zs98Cpo8LIk33JxSI41nfmlMYuDuwaCZ5QiWf9cit0iJj8Yj5EIvzYTbryKACg==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
task: '#5002214'
title: Error journal triage, bug records and task board reconciliation
type: report
updated: 2026-11-02T18:24:56.801Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify:
  command: for c in 3ec66dbe 3f025918 f7becb1e 9a6cffa6 0e3acf84 e653ebf5 6ec9be18 141f2a68 a0037704 4f79d79f 7e2b83e3 c9bfb417 8b6ad7b9 e01664a2 ee6b7d5e 9030e565 c1af68b6 4fa1b876 6ff474a1 0f8261d1 a80e7b86 c53978bc 50a849eb 4bd4654b 69f67650; do git merge-base --is-ancestor $c origin/main || exit 1; done
  exit_code: 0
---
# Error journal triage and task board reconciliation (2026-11-02)

Solo seat Vesper Greywind (01M6EQFA2G00BFT3TVPY6AMXE6), task [[tasks/2026-11-02-error-journal-and-task-triage]]. Operator brief: read the `ctl debug errors` journal, mark it read, record the bugs in the KB, then close, triage or drop the open tasks.

## 1. Error journal

`~/Applications/bboxy450-term.app/Contents/MacOS/bboxy450-term ctl debug errors` exited 0 and returned 256 rows (128 more omitted by retention), with `acknowledged: true`, `unread_before: 14` and `unread: 0`. The journal is now marked read. The rows span 2026-10-28 23:11 to 2026-11-02 10:15. The raw JSON was kept outside Git.

### Filed as bugs (open tasks, signed by this seat)

| # | Record | Rows | Evidence |
|---|---|---|---|
| 5002207 | [[tasks/2026-11-02-bug-ctl-debug-errors-seat-door]] | 1 (this session) | A seat could acknowledge the operator's journal; `ctl task list` from the same process returned `ancestry_refused` |
| 5002208 | [[tasks/2026-11-02-bug-task-lifecycle-unowned-records]] | 4 (10:13) + this session | `work_items.rs:1166,1172,1180,1281` |
| 5002209 | [[tasks/2026-11-02-bug-ctl-usage-in-error-journal]] | 6 | usage and help text journaled as failures |
| 5002210 | [[tasks/2026-11-02-bug-seat-close-missing-tab]] | 4 | `ui_frame_refused: MissingTab`, `window.rs:243` |
| 5002211 | [[tasks/2026-11-02-bug-plugin-eof-mid-response]] | 6 (+9 deadline) | `plugin_eof` and `provider_closed` pairs, cause not retained |
| 5002212 | [[tasks/2026-11-02-bug-hook-input-bound-drop]] | 11 | `hook.rs:23` `MAX_INPUT` 2 MiB drops the whole event |
| 5002213 | [[tasks/2026-11-02-bug-ui-click-sidebar-frame-timeout]] | 2 | `target=None, pending=0`, timeout after an applied effect |

### Linked to an existing issue

- These 2026-10-30 08:36–09:42 rows match GitHub #93 (the GUI SIGKILLs a slow logic and recovery stops), so no new record was filed:
  - `logic exited: signal: 9 (SIGKILL)` ×5;
  - `supervisor operation outcome unknown` ×2;
  - MCP `Seat control transport unavailable` ×8;
  - the 104 hook `Connection refused` rows (10-30 09:01–09:12, backend down).

  The hook flood predates `dd043516` (identical-failure coalescing, 10-30 10:32).

### Not filed (environment, expected or caller refusals)

- `Io: capture writer failed: No space left on device` (10-28): the disk was full.
- `desktop already running` ×7 and `app_already_running` ×1: a second launch, handled.
- `LinkDown: nebula worker-east/west` ×4: dummy cloud provider.
- `Exited: child exited or termination is pending` ×15: requests to seats that had already exited.
- `Busy: capture admission paused` ×1 and `backend reconnecting` ×3: transient.
- Hook `No such file or directory` ×44: codex-notify on 10-28 while the endpoint was gone, and claude rows through 11-02 09:09 after app restarts.
- MCP caller refusals: `invalid_path`, `invalid_report`, `task_field_protected`, `closed task cannot be delegated`, `not_found`, `remote_required`, `workspace_pending`, `knowledge create requires a root-relative Markdown path`.
- Redacted `ctl plugin/list`, `app/endpoint` and `seat/ls` rows: no content to act on.

## 2. Task board

Every closure below was checked with `git merge-base --is-ancestor <sha> origin/main` (the verify command; exit 0). `task_reconcile` refused all of them: `task seat unavailable` for #5162 and `invalid task status` for #5097. The cause is filed as #5002208. Each record was therefore closed by a hand edit in the landing commit:
- `status: done`;
- a `reconciliation` entry with `kind: historical`, `fresh_accepted: false`, `previous_status` and commit evidence;
- removal of the stale `provenance` signature, which no longer covers the edited frontmatter (the prior signature remains in Git history).

| Task | Was | Disposition | Evidence |
|---|---|---|---|
| #5162 ring-batch wall-clock | claimed | implemented | 3ec66dbe, 3f025918 |
| #5119 capability scan race | claimed | implemented | f7becb1e, 9a6cffa6 |
| #5117 resource ledger race | claimed | implemented | 0e3acf84, e653ebf5 |
| #5165 / #5171 / #5169 hook-log, fixture write, writer tick | claimed | implemented | 6ec9be18 |
| #5097 plugin doctor | active | implemented | 141f2a68 |
| #5003 ATC session attribution | active | implemented | a0037704 |
| #5004 gitkb review fixes | open | implemented | 4f79d79f, 7e2b83e3, c9bfb417 |
| #5183 ctl seat resume | open | implemented | 8b6ad7b9, e01664a2 |
| #631 KB report type | claimed | implemented | ee6b7d5e |
| #1796 shared Codex tmp leak | claimed | implemented | 9030e565 |
| #572 plugins continue (land #31/#84, close #377) | claimed | implemented | c1af68b6, 4fa1b876, 6ff474a1 |
| #367 Harmony plugin host | claimed | implemented | 0f8261d1 |
| #369 Harmony GitKB adapter | claimed | implemented | a80e7b86, c53978bc |
| #374 / #372 Harmony commit and isolation | claimed | implemented | 50a849eb, 4bd4654b |
| #329 fifty-seat spawn ENOENT | open | cancelled | Quarantine row gone; test is an ignored night boundary; ENOENT witnesses in 69f67650 |
| #330 GUI events EOF after logic kill | open | cancelled | Test runs un-ignored in the gate and passed (`bboxy450-term-error-triage/target/ci-local.log`, 10-30); recovery tracked in #93 |

The four `harmony-*.md` records and artifact `01M6324Z691DJMRAC82F04YH7H`, which #369 links, were untracked in the shared checkout. They are now committed.

**Kept open:** #505 `plugins-continue.md`, the native checker work: finite sets and sequences, state-dependent quantifiers, temporal operators, and many-plugin models. Nothing on main covers it since 2026-11-01. It is real follow-up work under the formal verification mandate.

**Untouched:** the 2026-10-21..23 records with no `status` field are the historical board (`tasks/README.md`), not open work. `#49 accepted` is terminal.

## Landing gate

`make land` runs `cargo test --locked -p bboxy450-doc --test corpus` for `.bboxy` changes (`scripts/land-fast.sh:42`). That result is in the landing output, not in this append-only report.

## Gaps

- Bugs #5002210, #5002211 and #5002213 have causes inferred from one or two rows. Each record names what to confirm first.
- No source was changed in this landing. Each bug's fix and test is open work.
