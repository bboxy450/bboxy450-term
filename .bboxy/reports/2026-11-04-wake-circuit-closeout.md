---
type: report
title: Wake circuit closeout — installed proof and later upstream formal gap
task: '[[bugs/2026-11-04-automatic-wake-storm]]'
supersedes: '[[reports/2026-11-04-wake-circuit-breaker]]'
commits:
- 5b6110d36a6c4a0c0315a70528b86c9d8bffe076
- 36aaf886d9c2a9a0c9e304ae2ffb8384e00e80a8
verify:
  command: make test-formal FORMAL_REUSE=1
  exit_code: 2
author: seat:01M6MADHFD19P4TKM7P7X6QESA
created: 2026-11-05T00:41:06.651Z
id: 01M6MJDXFQC3PJBYQ2KBRGPA0K
updated: 2026-11-05T00:41:06.651Z
number: 93377961363742
created_by: seat:01M6MADHFD19P4TKM7P7X6QESA
updated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
authorship:
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  kind: seat
  public_key: Rg4zwPSh/8Jytfz7nuhB5zfZyLtAZj9dFmnXz9d411U=
  process_generation: '1'
provenance:
  digest: sha256:66570e23f76ce3bcfc931852ed74b7f5b75a04deb6db6b9483907a61a3fe9369
  frontmatter_digest: sha256:395a8de7dfff1cd2fbe1c963d5b3a0c6313c70608232767d3fd911c0bea9a527
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  sig: oF3GjMpby+630GvWnq7QJyffksFOm/jvyy79XLNB7e4+YQmg/TiKhUiMvRT2ynkd+As7PTS1BHaHEt7AKvHrDQ==
  log: .bboxy/log/2026-11-05.jsonl#131563
---
The circuit implementation, installation and source-bound green evidence in [[reports/2026-11-04-wake-circuit-breaker]] remain valid for source `5b6110d36a6c4a0c0315a70528b86c9d8bffe076`. This correction discloses a later current-main verification gap; it does not claim a green formal gate for that later source.

While landing the report-only records, main advanced through `fa5280e8` (full-width harness panes and immediate ACP steering), pinning harness `0ab55023228a7f7f49305c9c08f5bdbcfb39c10f`. Report and native handoff records landed at `36aaf886d9c2a9a0c9e304ae2ffb8384e00e80a8`; both metadata landings passed the 15-test document corpus. The writing worktree was clean and the child synchronized.

`make test-formal FORMAL_REUSE=1` on that later source exited 2 in self-tests. The child `scripts/formal.py` raised `ValueError: missing mandatory checks: HarnessAcpSteering` from its contract inventory. Java parity for the later source was not reached. Exact log and hash are recorded in `.bboxy/runs/2026-11-04-wake-circuit-closeout.json`. The verified circuit source already has its complete 387-model archive under the common Git directory. I did not expand this messaging circuit assignment into repair of the newly landed steering contract, and did not bypass or claim that failed gate passed.

The native `task_report` was recorded once against the verified installed circuit source, status `reported`, operation `01M6MJ7H7EQ287NMX2EB9BFP3F`. Its single self-handoff message `0X8MKT9V2038V6H4GZ18C2W70N` was checked: recipient-stored, wake=none, attempt=null, native_id=null. A temporary receiver rule pinned that handoff to the inbox; effective direct-conversation wake behavior was then restored. No task_accept tool is exposed.

The signed installed bundle names source `5b6110d3`; its passing smoke observed zero idle frames/callbacks, all seven real circuit witnesses passed, and the causal removed-guard control failed as expected. Existing backend processes and sessions were preserved. Backend restart onto the new installed binary is still required for active protection. No raw transcript or log content is committed.