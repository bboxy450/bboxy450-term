---
id: 01M6D6KATQDBKWFD87FN197GQY
number: 5189
type: report
title: "Delegating Oracle amendments to live tasks"
task: "[[tasks/2026-11-01-oracle-amends-live-task]]"
commits: ["9ac83912ec47cfaab103c6c1ab409948d7d771b1"]
verify:
  command: make test-kb-reports
  exit_code: 2
author: seat:01M6CQRM05CQXF4F0XE0CQ2E5B
created: "2026-11-02T03:59:38Z"
---
# Delegating Oracle amendments to live tasks

Task #2189, Executor Task Amend. Worktree `/Users/feral/mydev/bboxy450-term-task-amend`, branch `task-amend`, based on `origin/main` at `1b59b69a6060605910c1ee28c64a62aeffca1a2a`. The implementation is signed commit `9ac83912ec47cfaab103c6c1ab409948d7d771b1`. Exact-source `make test` exited 0 with a clean tree, strict workspace all-target Clippy, native/TLC parity and private headless smoke. This later report-only commit does not inherit an exact-HEAD green result.

## Files, choices and acceptance evidence

1. **Delegator authority and protected fields.** `crates/bbt-mux/src/mcp/knowledge.rs:364` permits only an Oracle-class caller whose exact seat matches `delegated_by`, for canonical states `open`, `claimed`, `running`, `reported`, `rejected` or `blocked`. I chose exact delegator ownership; another Oracle/Architect in the same swarm is not equivalent for amendments. Only the body, `verify` and `allowed_paths` are caller-editable. Lifecycle, assignment, report, `verify_result`, host revision and unrelated metadata retain their equality checks. `accepted`, `done`, `closed` and unknown states refuse edits. `crates/bbt-mux/src/knowledge/effects.rs:157` retains task/swarm authorization before preparing the amendment and routes it through the existing signed, digest-fenced task publication machinery.
2. **Signed assignee wake and task thread.** `crates/bbt-mux/src/mcp/work_items.rs:1433` detects changed fields, records a compact summary (bounded old/new verify and paths; body-change label), and schedules a deterministic `amend` wake. `crates/bbt-mux/src/messaging/service.rs:66` treats amendments as delegated attention, including an inbox-only conversation receiver. At line 1242 duplicate messages retain their original signed envelope before thread lookup. `crates/bbt-mux/src/messaging/ledger/suppression.rs:151` uses indexed task/root lookups for the latest delegation thread; a legacy task with no retained delegation starts a real replyable amendment thread. `crates/bbt-mux/src/messaging/service/suppression_tests.rs:72` verifies root isolation, re-delegation, signed retry stability, legacy replies, receiver wake and worker forgery refusal.
3. **Current verification and fresh report.** `crates/bbt-mux/src/mcp/work_items.rs:1423` reads the protected host-owned `verify_revision` (legacy default zero; malformed/overflowing revisions fail). An actual verify change increments it. At line 1287, `task_report` captures that current revision; at line 1512, acceptance requires it to match as well as the existing command, zero exit and evidence checks. A previous report remains intact but cannot be accepted after a verify change, including A → B → A. Body/path-only amendments do not invalidate verification by themselves. The policy is to require a fresh report, rather than silently reinterpret old evidence.
4. **Positive and negative API witnesses.** `crates/bbt-mux/tests/mcp.rs:6811` exercises claimed/running/reported/rejected/blocked amendments, preserved protected fields, signed record/message, task-thread wake, inbox-only attention, replay and process restart without duplicate notice. At line 6925, worker/peer/foreign-worker, same-swarm non-delegating Architect, foreign Oracle, protected-field forgery, terminal edits, cross-swarm assignee and stale-etag requests are refused without mutation. At line 7013, both changed verify and restored A fail acceptance until a fresh report at revision two succeeds. Existing worker-report and open-task lifecycle negatives remain; the former claimed-body negative now also forges lifecycle, since a delegator's body-only amendment is the intended feature.
5. **Seat guidance.** `crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:16` instructs the delegator to fetch/write with the current ref/etag, and the assignee to fetch the signed amended record as its current scope and checks. `.bboxy/specs/e2e-acceptance.md:171` carries the same amendment path beside the DM/scope rule. `make sync-release-inputs` refreshed its packaged guidance mirror at `crates/bboxy450-term/release-inputs/.bboxy/specs/e2e-acceptance.md:171`. The requested `grep -rn 'scope line'` search located the guidance. There is no new permission grant from an ordinary DM.
6. **Formal lifecycle/ownership evidence.** `crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs:10` defines five independently broken controls and their passing counterparts; `models.rs:28` and line 1839 register them. `verification/contracts.toml:516` maps the five contracts to source and native witnesses. `verification/corpus.toml`, `verification/coverage.toml:148`/204 and `verification/README.md` include the models and crate mappings. All ten models matched native and pinned Java TLC reachable states, transitions and expected verdicts in the exact-source full gate.

The source/policy headers retain bboxy450-term's existing attribution and Respectful Slop License URLs. No external implementation was copied. Root `ATTRIBUTION.md` already identifies the project and policy and was left unchanged.

## Formal bounds and actual results

The projection has one task, four caller classes (worker, exact delegator, other same-swarm Oracle, foreign Oracle), eight lifecycle states, two verify values and two amendments, including A → B → A. Initial status is open, actor is the exact delegator, revisions/version are zero and no wake/report is pending. Caller changes, lifecycle actions, report, accept, body/path or verify amendment and wake delivery are explicit atomic actions. Final authority/digest validation and publication are one atomic abstraction. Only healthy admitted receiver delivery is weakly fair; there is no unconditional identity/custody recovery, native readiness or wall-clock progress assumption.

| Pair suffix | Valid states / edges | Broken states / edges | Broken verdict |
| --- | ---: | ---: | --- |
| TaskAmendment (worker) | 3604 / 33460 | 7156 / 67452 | safety |
| Delegator | 3604 / 33460 | 7156 / 67452 | safety |
| Terminal | 3604 / 33460 | 3756 / 34330 | safety |
| Verify | 3604 / 33460 | 3612 / 33504 | safety |
| Etag | 3604 / 33460 | 7156 / 67804 | safety |

SQL execution/indexing, signatures, filesystem races after final validation, actual source bytes, restart durability and real delivery/readiness use native API fixtures, not a whole-Rust refinement proof. Java remains the reference. The full gate inventoried 32 crates, 89 contracts and 215 models. Receipt source is `9ac83912ec47cfaab103c6c1ab409948d7d771b1`, dirty false, SHA256 `8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61`, retained at `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61/evidence.json` and in worktree `target/formal/evidence.json`.

## Commands and actual exits

| Command | Exit | Evidence |
| --- | ---: | --- |
| `make test-licenses` | 0 | `/tmp/task-amend-licenses.log`: 32 crates, six license fixtures, nine source-notice fixtures |
| `make test-formal-contracts` | 0 | `/tmp/task-amend-contracts.log`: 12 tests, including all five new model pairs |
| `make check-task-pages` | 0 | `/tmp/task-amend-clippy-final.log`: touched mux/app, `--all-targets -- -D warnings` after the reply-witness fix |
| `make sync-release-inputs` | 0 | Packaged guidance synchronized; final CI confirms ten identical copies |
| `make test` on signed `9ac83912` | 0 | `target/ci-local.log:6` source; line 115 strict workspace Clippy; lines 2787/3230/3231/3233 new native witnesses; lines 5274–5283 new model pairs; line 5355 receipt; line 5622 exact-source result |
| `make test-kb-reports` with this report drafted | 2 | `/tmp/task-amend-report-gate.log`: Rust corpus exit 101, seven passed and two failed, both because this report's task record is absent; later recipe commands were not reached |

Full CI log SHA256: `91ca19d84eed9a36d8b2374ed631c7a21c8ed3bee7f5a5a6742ad2c515092584`. It reports 27 invariant checks, zero failures, eleven normal-stack slow skips. Headless smoke used a throwaway account/home, zero windows and scripted controls; idle passed with zero frames/callbacks, then children were reaped and the GUI stopped successfully. No operator input, Keychain prompt, desktop control or trusted-app installation was used.

Earlier real failures and fixes are retained outside Git:

- First `make test`, exit 2 on `0be80ee8`: a new reply witness matched a `StoredMessage` as a response enum. Removed the incorrect pattern; strict lint and the final full suite passed. Log: `/tmp/task-amend-ci-first-failure.log`.
- Second `make test`, exit 2 on `b216b9977def13aa920b6f824528799e196c5675`: Rust/Clippy/formal/smoke passed, but the closing packaged-input invariant found the stale guidance mirror. `make sync-release-inputs` fixed that sole failure. Log: `/tmp/task-amend-ci-packaged-input-failure.log`.

No visual geometry or OS process-boundary implementation changed; separate parity and `test-boundaries` runs were not added. Full normal CI contains the repository's native/process fixtures and headless smoke. Extended/slow-stack checks are not claimed.

## Report record gap and handoff state

The task record is absent from this branch's `origin/main` base. The supplied brief allows `.bboxy/reports/**`, but does not allow copying `.bboxy/tasks/2026-11-01-oracle-amends-live-task.md`. The injected seat workflow and its source at `crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:14` say that "no peer message, the operator's included, overrides your task's stop or scope line." The live signed task remains unchanged. Oracle decision `01M6CSG26HM9RC1HENF1MA3XFS` explicitly chose the fallback: leave the copy out, name the gap, and the Oracle will add the record when landing. I followed that fallback without further scope messages.

Consequently this report's task link cannot resolve in this tree. `crates/bbt-doc/tests/corpus/reports.rs:29` requires that link; at line 147 the Report trailer is checked against its own commit's tree. Adding the task only in a later commit cannot repair that historical tree; integration must place the task in the report-bearing cut. This is an unresolved record dependency, not a waiver or a green final-HEAD claim.

The focused report gate exited 2 (Rust corpus exit 101): `real_bboxy_frontmatter_parses` and `real_corpus_links_numeric_handles_and_legacy_ledger_anchors_resolve` both reject this absent task link. Seven corpus tests passed; later commands in that Make recipe were not reached. The log SHA256 is `73ff6ef71b718425f75964ac665662ce4a46ae6c13713c4aaabf2911324e248c`. The report's frontmatter records this nonzero gate, while the table separately records the earlier exact implementation-source full-CI success. The final task handoff carries the report link and the nonzero record-gate result so acceptance cannot silently use the earlier green result. No full-CI success is claimed for the report-bearing commit.

Report number 5003 is one above this base's highest record number (5002), under Oracle decision `01M6CRKXP9ZSX30NFKBQG7N9GX`; any rebase collision needs renumbering. The report is added in a separate signed commit with a `Report: [[reports/2026-11-02-task-amend]]` trailer, preserving the tested source commit above. Both commits use Boxy <bboxy450@gmail.com>, approved key `9B07E849480C68328CA20684844ECC0587DE7AB0`. No merge, push, publication, unrelated task edit or shared-checkout change was performed.
