---
type: report
title: 'Q2 external helper budget model: decision 66280'
task: '[[tasks/2026-11-07-q2-external-helper-budget-model]]'
author: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
created: 2026-11-07T15:54:27.526Z
commits: []
verify:
  command: test -n "$(ls .bboxy/decisions/*helper*budget* .bboxy/decisions/*external-helper* 2>/dev/null)"
  exit_code: 0
id: 01M6VBFRN1NT5E4706WEY0PGFR
updated: 2026-11-07T15:54:27.526Z
number: 93377961366281
created_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
updated_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
authorship:
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  kind: seat
  public_key: gX8ozXnq5IspMTpqtE9rHRFSbgoDxsJh5dGOJgtj2o8=
  process_generation: '1'
provenance:
  digest: sha256:d8efad5cd1594c24bf5d6997ecb2ee78077ef6906bfd351f28f43618feda6c28
  frontmatter_digest: sha256:b4bfca45face819a6f5b80ec36c2112d66a340844e4a6a57dae59395763aa0e5
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  sig: bif4XrPJYsfPqt/rWplkF0jERmTx5rWJg9bh1kZDUBj4z98JvaR9lf/nUeuxAEvLdqaPw0/YZ7GCJqI/L6AnCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150383
  event:
    id: 01M6VBFSD4RZ9BA68YCF9SH5EP
    sequence: 150383
    hash: fadbed733d518419340c1e0c3e4bb11243d028123c55f5697f7ece3220b94d17
---
# Q2 external helper budget model

## Ruling

Option **(b)**, recorded in [[decisions/d93377961366280-dream-q2-external-helper-budget]]. External Git/GPG helper processes are no longer sliced into 1 ms grants. Instead:
- the helper is spawned at background QoS in its own process group, and its observed priority (Darwin 4) is a hard gate;
- CPU caps: ≤250 ms per command and ≤3 s per job, enforced live through RI reads with SIGKILL, with the leader's own wait4 as the terminal verdict;
- after the reap, the job pays 9× rest debt on the wait4 CPU;
- wall time is bounded by the existing 30 s deadline;
- memory: the existing shares, plus a terminal ru_maxrss ≤16 MiB per process;
- foreground is a single SIGSTOP pause inside the 100 ms gate.

The 65086 1 ms law is unchanged for all work on our own threads.

The **settlement_unknown** was an expected exit race, misrecorded by a probe bug. `kill(-pgid)` returns EPERM on a zombie-only group; I observed this on this machine. The probe discards the errno and never retracts the unknown after its own reap and group-gone. The fix is to apply 66050's pending-terminal state to the final kill. Run 4 stays FAILED.

## Changes (knowledge tools only, drafts, not committed)

| Record | Mechanism |
|---|---|
| `decisions/d93377961366280-dream-q2-external-helper-budget.md` | New decision: evidence, the rejection of (a), a 10-item law, a clause-disposition table for 65086/65126/65489/66236/65654/66050/66074, the settlement ruling, model Q2HelperTotalBudget with 5 invariants, 8 broken controls and their real witnesses |
| `specs/dreaming-mcp.md` | Q2 helper paragraph rewritten to the new law; "Q2 helper window/debt coverage" verification row replaced by "Q2 helper total budget/debt coverage" |
| `interfaces/dreaming-mcp.md` | Four lifetime paragraphs (old 199–205) replaced by two; model reference, runtime-check list and private-candidate rule wording updated |

Each body was diffed against a python-generated expected body (the old file with exactly these replacements): both are `equal`.

## Evidence used

- Run 01M6VAGHD4Y6MXHSASDMN15XE9 `components/positive/output/result.json`: C_ns 3,104,291 > A_ns 2,360,628; W_ns 3,849,000; settlement {reaped 1, group_gone true}; `stderr` line 3 is the settlement_unknown.
- `proof-helper-probe.c` in bboxy450-harness-dream-read-q2. Its SHA256 e23f68d3… equals the run's probe.source_sha256. :4383-4389 is the final kill; the errno is kept only at :2474/:4387.
- Local C witness, Darwin 27.0.0: zombie-only group `kill` → EPERM; after the reap → ESRCH.
- `taskpolicy -c background` → PRI 4 for the process and its child; the default is PRI 31. The machine has 4 P-cores and 6 E-cores.

## Gaps and things not done

- No implementation, build or test; this is design only. The Q2 author needs an Oracle-amended brief: c1.rs HelperGroup, read.rs reap/usage, resolve.rs spawn attributes, probe.c, tests/dream_read.rs, and the new formal model with its broken controls.
- The 250 ms / 3 s / 50 ms allocations are not measured distributions. They must be revisited after qualification through a decision.
- `specs/dream-term-c1.md` was not changed: it references only 65086, which is unchanged.
- **Host defect found:** on update, `knowledge_write` drops `created` and rewrites `created_by` unless the caller resupplies them. My first spec write lost them. I restored them by resupplying them on a second write (body digest unchanged, 5e259c93…). The ICD write resupplied them from the start.
- Nothing committed with knowledge_commit; the records are drafts for Oracle review.
