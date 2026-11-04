---
type: task
title: Unblock main with hash-pinned report corrections and startup-modal metadata repair
role: executor
verify: make test
allowed_paths:
- crates/bbt-doc/tests/corpus/**
- .bboxy/reports/CORRECTIONS.toml
- .bboxy/reports/2026-11-03-startup-modal.md
- .bboxy/reports/2026-11-03-report-correction-ledger.md
- .bboxy/tasks/2026-11-03-startup-modal-operator-brief.md
- .bboxy/tasks/2026-11-03-report-correction-ledger.md
- .bboxy/decisions/d16393-hash-pinned-report-corrections.md
- Makefile
- crates/bboxy450-term/src/main.rs
- crates/bboxy450-term/src/keyboard.rs
- crates/bboxy450-term/src/app_overlays.rs
- crates/bboxy450-term/src/pointer_cursor.rs
- crates/bboxy450-term/src/terminal_clipboard.rs
- crates/bboxy450-term/src/startup_modal.rs
- crates/bboxy450-term/src/cockpit/startup.rs
- verification/contracts.toml
- .bboxy/tasks/2026-11-03-startup-modal-input-regressions.md
- .bboxy/reports/2026-11-03-startup-modal-input-regressions.md
- .bboxy/tasks/2026-11-03-oracle-handoff-codex.md
id: 01M6HG4AQQE4N6AM4CKY7AHHS2
status: done
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T00:56:59.474Z
created: 2026-11-03T20:03:11.287Z
number: 16394
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
assignee: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: claimed
  disposition: implemented
  reason: 'Historically implemented through the separately accepted integration task #16404 and signed report #19654, rather than a fresh acceptance of this component task. Its original task_report intent 01M6HQMK5WECSTGFP2PZ8NB9QJ and artifact intent 01M6HQSS2BK0SJBWZKYR6JBGAW remain unresolved/unknown and were not replayed; this disposition records the independently landed code and does not claim those effects applied.'
  evidence:
  - 40f45c5838f521bd70e9b8ff907caca3f8784eb9 on origin/main; approved Boxy signatures verified
  - '[[reports/2026-11-03-logic-recovery-write-route-witness]]; signed report 184b770f9f43094c518570f143e5eeaeb7e999ef'
  - 'Canonical full gate commands.log SHA256 cc53be84a6ab628e9585267c9b584a6802c6a55ed4c10b525c5ceedbc3a802e7: 2623 passed; complete current-source formal receipt cd9d93570e189d605cea6a22e5bbd53815391cd4feb46d2d203dcb5f57b49fec: 290 models/34 crates/119 contracts; Oracle focused recovery/ownership rerun exit 0 (4/4)'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6J0Y9RG1WGX9GZZHHZ2RB8N
provenance:
  digest: sha256:55f19efb5380b96e768d86162d79469676d6cab95195f85b804988b63d19feac
  frontmatter_digest: sha256:7bb87f4498e3dcd225432fa19ff57c70a47d3dc6445fa43c1791cc9f1abd1edd
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: TBsYi3mn5JUiw+1AsT9KgGIRYlj3sEzsQbFTj9gBQCLi3zEWw+NS/apZ/qFphJ2LLFktfXvXHEaCX2M7tNx/CA==
  log: .bboxy/log/2026-11-04.jsonl#109854
---
# Repair the immutable-report dead end

ROLE: EXECUTOR. Worktree `/Users/feral/mydev/bboxy450-term-report-ledger`, branch `report-ledger`, created from `main-green` commit adf9796b. Work only there. Read AGENTS.md, ~/.agents/AGENTS.md, ATTRIBUTION.md and decision [[decisions/d16393-hash-pinned-report-corrections]] (in the shared checkout if absent from your branch). Oracle task [[tasks/2026-11-03-oracle-handoff-codex]].

Implement the decision's append-only CORRECTIONS.toml ledger in crates/bbt-doc/tests/corpus/**. Every allowed M must have a newly appended entry in that same commit binding path/from_blob/to_blob/reason/decision; reject D, rename, unlisted M, wrong blobs, prior entry mutation/deletion, reuse and malformed entries. Keep metadata/task/trailer/signature gates intact. Add positive and broken-control Git fixture tests. Validate all affected commits, not only today's working tree; before/after hashes must be the actual parent/commit blobs. Preserve project attribution headers.

First and only legacy repair is .bboxy/reports/2026-11-03-startup-modal.md. Keep its evidence/waiver body, allocate a unique report number (scan current records immediately before commit; don't copy 5002227), set a resolvable task locator, commits containing signed 1affb9865d75d775a9a5f306c043da8a05bf8f7c, and an honest correction author. The original seat is unproven; use your correcting seat as author and explicitly retain original Boxy machine-key attribution and distinguish correction from original authorship. Do not invent an original seat. Add one task record from the exact operator brief in `/Users/feral/mydev/bboxy450-term-startup-modal/.bboxy-brief-startup-modal.md`; retain implementation evidence and waived checks without claiming fresh verification. The Oracle created and historically reconciled that task as #16564 after verifying the original approved signature. Preserve its tool-created done/reconciliation fields unchanged; this is historical disposition, not new test acceptance. Copy this task and decision into your worktree unchanged so report/decision links resolve in your commit tree.

The main-green Executor (PID 90316) is already running make test on adf9796b and will stop on the malformed report. Do not write its worktree, wait for it, or repeat its work: this branch already contains the layering/coverage fixes. Reuse any current source-bound evidence the landing gate permits. Run the needed focused corpus checks; canonical final verification is make test (exit 0) and scripts/land.sh. No skip/quarantine or operator prompts. Use two Cargo jobs and worktree-local target. The three existing headless Executors remain running; do not launch duplicates. Full make test/test-formal currently use `/Users/feral/mydev/.bboxy-fulltest.lock`: inspect its pid file, only claim when free, and release your own claim reliably. Targeted checks need no such lock. Don't alter peers' caches, processes or targets.

Landing through scripts/land.sh is authorized. Full report at .bboxy/reports/2026-11-03-report-correction-ledger.md, signed commit and Report trailer; report real exit codes, fixture evidence, exact source and remaining gaps. Then task_report to this Oracle with the committed report locator. Scope gap: report exact required decision rather than weakening a gate.


## Integration amendment (2026-11-03 20:09 UTC)

Main-green's actual nextest run found four additional startup-modal input regressions beyond the malformed report. Task [[tasks/2026-11-03-startup-modal-input-regressions]] (#16417), assigned to seat 01M6HGCVKFA0E7CEGE8E7Q5D5P in its own worktree, repairs them and adds `make test-startup-input-regressions`. You are the sole designated integrator of those signed input fixes together with your ledger/main-green repair. The added UI/Makefile paths are authorized ONLY for cherry-picking that task's signed implementation and resolving necessary integration conflicts, not independent feature work. It will report focused verification and send its commit to the Oracle; do not wait for it to land, and it will not wait for you to land. Continue your own corpus implementation and focused fixtures independently meanwhile. Once both cuts are ready, combine them, run the missing full make test and current-source formal receipt once on the combined exact committed source, then scripts/land.sh using valid source-bound gate reuse. Keep all tests/guards. If a further failure appears, provide exact evidence immediately; do not repeat an unchanged red gate. Report all integrated commits and validation in your full report.


Copy `.bboxy/tasks/2026-11-03-oracle-handoff-codex.md` unchanged from the selected knowledge root into your branch; its path is now explicitly in scope so all mandatory task/decision links resolve. The historical startup task is deliberately already reconciled done; do not reopen it. Both updates remove the previously reported record blockers. No new implementation scope is added by this record-copy authorization.


## Upstream source is now repaired (2026-11-03 20:21 UTC)

Rebase your candidate onto current origin/main before integration checks. Main now includes 6f2eddce (the four startup input failures fixed) and 3eafd2be (HarnessTextZoom inventory). Do not keep duplicate coverage/fixture patches merely because the old adf9796b base failed. #16417 is amended to consume 6f2eddce and return only a demonstrated residual normalized-input/IME regression and focused Make target, if any. Once your record migration is ready, main can be verified using the upstream source repair; never wait for another branch's landing. If #16417 has an outstanding justified fix, integrate its signed cut when ready, then run the final combined gate once. The original main-green executor finished without landing; its source-only layering cut is already in your ancestry, and its blocked report need not be cherry-picked.


## Delivery/resource amendment (2026-11-03 22:07 UTC)

The operator is low on disk and inference and requests delivery sooner. Your two completed full green runs on 262ce5d7 and 633e9169 are valuable evidence; do not start a third baseline/full test or continue a separate moving-main landing loop. #16404 already consumes this component and is the single final incident-delivery integrator. Your independent landing obligation is removed; no gate waiver is granted for the final product.

Finish your committed full report now with the exact tested SHAs, commands/exits, both complete formal receipts/logs, preserved positive/negative corpus controls, accepted input integration and clean source state. State clearly that this component is verified on those sources and is not independently landed. Keep verify_result.command = make test, exit_code = 0 for the actual passing source; do not claim a full gate on a newer untested source. Commit only the report/current task copy needed for its record links; run the necessary focused records checks and existing records-only proof validation, without another broad source suite or rebasing onto moving main solely for this handoff. Then task_report to this Oracle. Send #16404 the final signed source/report and canonical gate-log/receipt locations. Preserve logs/receipts and source; no build-target cleanup is authorized until your report is consumed.

This supersedes earlier sole-lander and independent scripts/land.sh requirements for #16394 only. Do not modify any production or gate implementation. Stop after the factual handoff; #16404 owns current-main combination, ownership fix and final full/incremental landing gates.
