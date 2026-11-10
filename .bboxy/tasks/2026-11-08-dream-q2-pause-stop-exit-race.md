---
type: task
title: 'Q2: foreground pause STOP vs helper-exit race (ESRCH -> terminal-before-stop settled by own wait4), hand off INPUT68'
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/proof-helper-probe.c
- tests/fixtures/dream-read/private-engine-controls.c
- tests/fixtures/dream-read/check-private-engine-controls.py
- tests/fixtures/dream-read/prepare-c-probe.py
- tests/fixtures/dream-read/check-c-preparation.py
- src/dream/read.rs
- src/dream/resolve.rs
- src/dream/c1.rs
- tests/formal/dream_read.rs
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-08-dream-pause-stop-exit-race.md
- scripts/formal.py
- scripts/land.py
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[tasks/2026-11-08-dream-q2-unit-split-and-margin]]'
id: 01M6Z3HK8A4YRY15RGGT5APE0C
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T08:04:26.306Z
created: 2026-11-09T02:52:36.415Z
number: 93377961366427
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Raw STOP receipt and original-terminal-only settlement, with missing-reap coverage and preserved first error
    reference: reports/2026-11-08-dream-pause-stop-exit-race.md; proof-helper-probe.c:3545/3573/3608/4077; 15 typed existing-ledger fields; no public/terminal changes
  - criterion: Supplied failed-STOP API positives/negatives, real preobserved exit and both causal mutants
    reference: E/canonical-C-input68-v5/run.json SHA4f0ab0f848a2298ef34ed09015ef7d73cce8b350ffdaceeca00509e6739aa0b0;114cases/74mutants/76strictcompiles/179commands; physical PID87982 own-reap/groupgone/full24.804ms helperdebt, no actual STOP claim
  - criterion: Complete unchanged-pin formal parity and scoped clone workflow landed first
    reference: Archive80a07c0dcac05acad1cfbea929fb6be9c76512413282f5c5b94b31e79fcee1aa;444models/95contracts/1crate;3378237states/11367153labellededges/allgraph_equal; workflow d49 landed before final chain; clone/fallback/mode/mismatch controls0
  - criterion: Exact final verify and canonical landing0 on same signed head
    reference: Exact log7b0cd627a51060fdbb110787ca311c2865ffd18773a44ea032e159a7124afa28; Land log93fa3c0f61e1b1cfbc1acfb21812b68129dc90ad674d1b6fe4b7c64cf43c9ee2; origin/main contains a68d200268ca816a6741e00c6cd334b28628f805; source before/after equal
  - criterion: ONE post-land INPUT68 plus independent audit and preserved cleanup evidence
    reference: Tuplee0f3ba9d8120af99f8e7dfdd8470ccff4414732f855a5ddfb7ef21353bdd3008; receipt7f736d1abc0b493aa0ec12adc443594e5bd9e1ecb65df306101b3dfa1321eb05; audit0/10current signed inputs/325headers/997checks; stored1572; cleanupfbf6a53c40c9cb4b4c46a43c3191285f272c1cc633b28700b159e6a4e70f7bf9; runtime/candidate false
report:
  summary: Landed signed a68d200268ca816a6741e00c6cd334b28628f805 with raw STOP errno/order telemetry and pending terminal resolution gated by original own-wait4, group absence and full known paid bill; no ACK/CONT inference. The exact four-command verification and canonical make land both exited0 on that same head. Current C114/74/76/179 and full444-model/95-contract native/Java graph/verdict parity pass; ONE post-land INPUT68 was stored as1572/01M6ZMA612BNCJEHZE714TXE1A and owned cleanup completed with retained full evidence. All historical failures and29 existing ignored attributes remain nonpasses; actual kernel ESRCH cause and installed-tool qualification remain unproven. Full report committed at04100aacacbc17a2b4beccd4e0e3aac63cf5a008.
  commit: a68d200268ca816a6741e00c6cd334b28628f805
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6ZNC7Q94HQAZZ9AWSQDSWS7
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-pause-stop-exit-race.md]]'
provenance:
  digest: sha256:92ed4079274f3cd8c7842b4cc908cfd375226e993569371e4508e6fd199ddbae
  frontmatter_digest: sha256:ae7395fbb97f26bb3ef0525a8ed592a9411f0cda8d691c73adc34be95c65bcd8
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: BSFtA5jDB99D+pcMXEmD+VoYVeIPdORo/i6XAcso/wAeF5hkPbV2RMko/98DDUSnvgYkbsH5sGVGjPaW7xiUDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150979
  event:
    id: 01M6ZNCK5M6X5YTT89QJA6ZSTR
    sequence: 150979
    hash: a1af682f27adfd8f44d6ae9e3a6d33ba0e3f9e8f27afe4bff024f06097a48fcb
---
ROLE: EXECUTOR

## Evidence (builder run 01M6Z2NJ9Q3GETXWTJ43M9FZGC on frozen INPUT67 1f1b30dd; builder handoff sha256 fc1117a4...)
- positive and corrupt_proof PASSED again: all 28 terms true, margins about 15.5 s.
- pause_resume: actual PID 91279 exit 81, coverage_unknown "foreground STOP pending independent terminal settlement"; settled 8/8, proof not reached.
- Owners 1-2 were ACKed and resumed: stopped 178.573 ms, running max 13.108 ms, SYSTEM 0. The stopped-interval exclusion works.
- Last seq 8 / PID 92030: no ACK. kill(-pgid, SIGSTOP) returned nonzero (frozen C ~3585-3586). The errno is stored but NOT serialized. All 8 were then own-wait4 reaped with their groups gone.
- Most likely cause: the helper exited between the pause decision and the group SIGSTOP (ESRCH). Not proven.

## Required
1. Serialize the foreground STOP errno (pause_stop_errno, already a field: make sure it is written in this path) and the order of STOP-attempt vs terminal observation.
2. Settle the race under the existing law (decision 66050: EPERM on a zombie-only group is pending-terminal):
   - if the group SIGSTOP fails with ESRCH, or with EPERM on a zombie-only group, the pause request becomes "terminal before stop";
   - it is resolved ONLY when the original leader's OWN wait4 subsequently reaps it, with the group gone and the full CPU bill from wait4 (the existing verdict);
   - no ACK is required, because no further CPU is possible after own reap;
   - any other errno, or no own reap, keeps coverage_unknown.
   Never infer the stop from the sent signal, and never signal after own reap.
3. Controls:
   - an API case where SIGSTOP returns ESRCH followed by own reap settles (pass);
   - ESRCH with no own reap refuses;
   - EINVAL or another errno refuses;
   - a real-process witness of a helper that exits just before a foreground pause, if it can be produced deterministically (otherwise an API control, labelled);
   - a mutant that ignores the terminal-before-stop path reproduces this refusal.
4. Unchanged: caps, 9x, 50 ms, stopped-interval rules, 66331 and the 30 s limit.

## Handoff
ONE INPUT68 tuple (same format), with the exact new ledger field names and types. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Run it once on the final head, then land with make land, report in reports/, and task_report once. Disk: one build tree; keep free disk above 8 GiB.


## Oracle amendment (2026-11-08): clone-based formal archive copies (capacity disposition)
The formal archive step copies about 7.4 GB of formal data with shutil.copyfile (scripts/formal.py:350-353), so archive plus canonical needs about 14.9 GB plus the 8 GiB reserve. On this shared APFS volume that duplication is the capacity problem.
Authorized narrow workflow change in scripts/formal.py (and scripts/land.py only if it copies the same archive):
- copy archive files by APFS clone (`cp -c` / clonefile) with the same paths, names, bytes and modes;
- fall back to a plain copy only when cloning fails (for example across volumes), and log which method was used;
- after staging, verify EVERY file's sha256 against its source before publishing; any mismatch fails the gate;
- the archive format, contents, evidence hashes and receipts are byte-for-byte unchanged; only the storage method changes;
- add a self-test or control: clone path, fallback path and the hash-mismatch refusal.
This preserves the canonical exporter and the 8 GiB reserve. Land it within this task, as its own signed commit, before the INPUT68 final chain.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed a68d200268ca816a6741e00c6cd334b28628f805 with raw STOP errno/order telemetry and pending terminal resolution gated by original own-wait4, group absence and full known paid bill; no ACK/CONT inference. The exact four-command verification and canonical make land both exited0 on that same head. Current C114/74/76/179 and full444-model/95-contract native/Java graph/verdict parity pass; ONE post-land INPUT68 was stored as1572/01M6ZMA612BNCJEHZE714TXE1A and owned cleanup completed with retained full evidence. All historical failures and29 existing ignored attributes remain nonpasses; actual kernel ESRCH cause and installed-tool qualification remain unproven. Full report committed at04100aacacbc17a2b4beccd4e0e3aac63cf5a008.

```json
{
  "op_id": "01M6ZNC7Q94HQAZZ9AWSQDSWS7",
  "commit": "a68d200268ca816a6741e00c6cd334b28628f805",
  "report": "[[reports/2026-11-08-dream-pause-stop-exit-race.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Raw STOP receipt and original-terminal-only settlement, with missing-reap coverage and preserved first error",
        "reference": "reports/2026-11-08-dream-pause-stop-exit-race.md; proof-helper-probe.c:3545/3573/3608/4077; 15 typed existing-ledger fields; no public/terminal changes"
      },
      {
        "criterion": "Supplied failed-STOP API positives/negatives, real preobserved exit and both causal mutants",
        "reference": "E/canonical-C-input68-v5/run.json SHA4f0ab0f848a2298ef34ed09015ef7d73cce8b350ffdaceeca00509e6739aa0b0;114cases/74mutants/76strictcompiles/179commands; physical PID87982 own-reap/groupgone/full24.804ms helperdebt, no actual STOP claim"
      },
      {
        "criterion": "Complete unchanged-pin formal parity and scoped clone workflow landed first",
        "reference": "Archive80a07c0dcac05acad1cfbea929fb6be9c76512413282f5c5b94b31e79fcee1aa;444models/95contracts/1crate;3378237states/11367153labellededges/allgraph_equal; workflow d49 landed before final chain; clone/fallback/mode/mismatch controls0"
      },
      {
        "criterion": "Exact final verify and canonical landing0 on same signed head",
        "reference": "Exact log7b0cd627a51060fdbb110787ca311c2865ffd18773a44ea032e159a7124afa28; Land log93fa3c0f61e1b1cfbc1acfb21812b68129dc90ad674d1b6fe4b7c64cf43c9ee2; origin/main contains a68d200268ca816a6741e00c6cd334b28628f805; source before/after equal"
      },
      {
        "criterion": "ONE post-land INPUT68 plus independent audit and preserved cleanup evidence",
        "reference": "Tuplee0f3ba9d8120af99f8e7dfdd8470ccff4414732f855a5ddfb7ef21353bdd3008; receipt7f736d1abc0b493aa0ec12adc443594e5bd9e1ecb65df306101b3dfa1321eb05; audit0/10current signed inputs/325headers/997checks; stored1572; cleanupfbf6a53c40c9cb4b4c46a43c3191285f272c1cc633b28700b159e6a4e70f7bf9; runtime/candidate false"
      }
    ]
  }
}
```
