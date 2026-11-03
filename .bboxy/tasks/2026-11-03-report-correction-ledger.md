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
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:15:36.304Z
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
provenance:
  digest: sha256:ce228459597361697cd924b973eccf92df5ea77f24812535dbb9a5787059b515
  frontmatter_digest: sha256:226c340cdf045394961a44f4825fea2e227a1311d0f3fe085821551cded80735
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: dRosZDyntsN4PDvdrPo5ZzgTkzIXoVyY+UIbUmdfAW/sWw3BNjIUgxepc9gAHN+1+yjgpjCaJUCEio+Xmz50Cw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#96794
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
