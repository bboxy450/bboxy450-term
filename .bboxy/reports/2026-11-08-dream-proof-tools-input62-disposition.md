---
type: report
title: Dream Proof Tools INPUT62 quarantine disposition and actual signal exits
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T11:47:05.530Z
commits:
- 0068adc9ed2597153f85ba1a8aa485c835f6054e
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-08-dream-proof-tools-input62.md]]'
id: 01M6XFQGP9K5Z9RSG7H6X7NF3G
updated: 2026-11-08T11:47:05.530Z
number: 93377961366365
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:8144911151e6c5fd2dcc4e379f8018c3b3e65d29aefcca653710151c77198146
  frontmatter_digest: sha256:80ad4a0d938ec4aa7a487c410575b9dddb0d7e996f16c75095bcfa7055b9a39f
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: oLEdb/apbrrONnHoCKq117QULbMTySLS/0iOXknN4po56fcfrMji3kqo3oWpfsTu0cN7Sk12soL/GFQP0RMIBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150674
  event:
    id: 01M6XFQJ98F7QCDXRHB0AVV16A
    sequence: 150674
    hash: de8fa94004c5174bfba1426445675d45414f5e70000fa11dc0dd50349f1a9fe2
---
# INPUT62 authorized quarantine teardown and actual exits

The authorized teardown is complete. Controller94751 was sent SIGTERM, remained alive after10.008731916s, then received SIGKILL. Its original parent47777 actually reaped it with wait_status9/exit_code−9, then returned normally through the recipe's refusal path (exit1 reported by Make). The original canonical command exited2. The run remains FAILED/quarantined. No retry or INPUT63 attempt occurred; parked pending its freeze. runtime_approved=false.

This append-only report supersedes only the pending-exit status in [[reports/2026-11-08-dream-proof-tools-input62.md]] (#93377961366362). That report remains the source for the exact INPUT62 pin/preflight, original verifier/GS1/rollup evidence and unavailable final debt. The formerly unknown actual exit is now known to be forced signal9 termination; it is not a canonical native exit81 or successful settlement.

## Authority, ownership and unchanged source cut

Task: [[tasks/2026-11-05-dream-proof-tools-provisioner.md]]. Oracle disposition message01M6XFE9AJVMYG5SA98NSSD3CB explicitly authorized pre-signal ps/fd records, TERM only controller94751, after10s KILL that same PID if alive, normal parent reaping, no other signals, no retry and parking for INPUT63.

The task's owned-handle rule was preserved: the original wait parent retained its Popen/wait4 ownership until actual reap. No handle was dropped, no timeout exit was substituted and no peer process or target was controlled. This was cleanup of the exact controller started by this assignment, within its original run.

Signed implementation commit remains0068adc9ed2597153f85ba1a8aa485c835f6054e, Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. No code, tests, source pins, budgets, models, raw wire or terminal whitelist changed during disposition. git verify-commit HEAD exited0 with Good signature, git diff --check exited0 and git status --porcelain=v1 exited0/empty.

R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XEW5VT8F8HF56S96KB592B. C=R/components/positive. D=R/quarantine-disposition-20261108. E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input62-20261108. All new disposition records are private files under this original run; earlier reports/manifests remain untouched.

## Pre-signal process and FD evidence

D/ps-before.json records the actual `/bin/ps -axo pid=,ppid=,stat=,etime=` exit0, filtered to the requested controller, wait parent and every child/descendant of94751. The pre-signal snapshot was:

| PID | PPID | Stat | Etime | Numeric FDs from lsof |
| ---: | ---: | --- | --- | ---: |
|47777|47773|S|11:37|5|
|94751|47777|SNs|08:58|55|

There were no children or descendants of94751 at that snapshot, so there were no additional child FD-count commands. `/usr/sbin/lsof -nP -a -p <PID> -Ff` exited0 separately for47777 and94751; D/fd-counts-before.json preserves raw FD records, counts and timestamps. Numeric FD counts exclude cwd/txt/memory records; total lsof file-record counts were40 and58 respectively. File contents were not read.

D/controller-identity-before.json independently binds live PID94751 to parent47777 and the exact frozen INPUT62 probe path. C/pid.json identifies the original94751 controller; R/identity.json identifies build-recipe PID47777. C/wait4.json was absent before signaling.

## Exact signal actions and actual statuses

Each signal has an exclusive fsynced intent written before the syscall and an outcome record afterward. No signal was sent to another PID.

| Action/observation | Actual result | Evidence |
| --- | --- | --- |
| SIGTERM to94751 | kill syscall returned0, errno0 | D/sigterm-intent.json and sigterm-outcome.json |
| Wait and inspect | after10.008731916s,94751 still live, PPID47777, exact frozen probe command | D/ps-after-10s.json |
| SIGKILL to94751 | kill syscall returned0, errno0 | D/sigkill-intent.json and sigkill-outcome.json |
| Disposition script | exited0 | completed original exec session59939; D/actual-exits.json |
| Controller's original parent wait4 | actual_wait4true, wait_status9, exit_code−9, WIFSIGNALEDtrue, WTERMSIG9 | C/wait4.json |
| Recipe parent47777 | normal refusal exit1 reported by Make; never signaled | E/verify.log, Makefile:14–15 and recipe Refused handler |
| Canonical shell/make chain | actual exit2 | E/verify-exit.json and completed original exec session65180 |

TERM intent time_ns1794138246744805000; KILL intent time_ns1794138256754321000. A fresh identity/parent check preceded each signal. The post-disposition `ps -p 94751,47777 -o pid=,ppid=,stat=,etime=` exited1 with no rows, meaning neither requested PID remained. D/ps-after.json preserves that expected absence status; it is not an additional parent wait4 claim.

C/wait4.json is139B, SHA2569c152f4afdb1e95fb5581ed13e1982e2c03a69ab6c1ce1143e5aac8ae6197c8d. It records controller user_cpu_ns368011000, system_cpu_ns3051882000 and max_rss_bytes4243456. These include the retained quarantine lifetime and do not certify any original CPU budget or native proof. C/retained.json now records actual_exit−9 and binds the original raw artifacts and actual wait4 bytes.

## Completed canonical command, unchanged failed evidence

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

This is the same original invocation, not a rerun. It started_ns1794137525467985000 and ended_ns1794138256786006000, actual exit2. E/verify.log is9878B, SHA25620e526be2e01d1ef7dc51caf169ca6aa85930dff3ffa668b0bf32a67a569673b. It retains OWNED PENDING, then the post-reap REFUSED: canonical real proof/component verdict failed; no timing retry and Make build Error1. The recipe's Refused handler returns1; the enclosing Make/shell chain reports2. Parent47777 was not directly reaped by this seat; its normal recipe exit is reported by its owning Make process.

The raw positive refusal and ledger are unchanged in device/inode/length/hash from the pre-disposition manifest:

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C/stdout = C/output/result.json |2050each|b618e91dba27fdcd2fa9a62d0130a9ad44057bda5e8fb619a8091508683acec8|
| C/stderr, GS1 only |256|b68f7d75f78eb74346f8756051862b956744b255bed18774365c1fa63841ac37|
| C/output/ledger.ndjson |50123|75984cf519a44f9ebb858e6f8a4434af3668d362ed7b8bb9343345cb32ddd607|

Raw state remains quarantined, coverage_unknown retired child lacks qualified nonduplicated same-leader rollup, created16/reaped15, group_gonefalse, retired_coveragefalse, proofnotreached. Ledger91 still records signature leader sequence16/PID96569: fork_events1, exec_events0, child_discovered_viafork_event, missedfalse, registered_mask3, errno0, queue_fd_held1, close_attempted0. Ledger89 still has terminal timestamps0, rollup_mask27, rollup_proven0 and sticky child_cost_unproven1. Physical forced controller termination does not rewrite those snapshots or prove GS2/child CPU accounting.

No final private_candidate_terminal or final debt split was emitted. The last persisted raw result still has paid debt23.149177875s; derived booked helper debt subtotal1.278441s, ordinary aggregate remainder21.870736875s. Ordinary raw CPU versus floor split and final totals remain unavailable. Derived G43=26candidate+17fixture remains explicitly derived, not measured. The retained unique helper snapshots record process_create excess0ns/count0/all-known1. No repeated rows were summed.

## Verification and acceptance accounting

- Requested pre-signal ps/children/fd inventory: recorded before signaling, actual ps/lsof exits0; no controller children existed at that cutoff.
- Authorized bounded teardown: TERM0, more than10s wait, fresh same-identity check, KILL0 to94751only, actual original parent reap9/−9, parent normal recipe exit1, canonical exit2. Independent read-only audit confirmed action timing, status and raw preservation.
- Original source/pin implementation remains signed and clean. The earlier current-source66unit tests exited0. Seven fresh authenticators and all60 numbered build/configure/link commands exited0; no build or test was repeated for teardown.
- Native qualification remains failed/quarantined. Only positive exists; remaining components, final formal/license gates, command-tail tests, qualification, receipt, landing and runtime admission did not run. Candidate qualification.json and receipt.candidate.json are absent.
- D/preserved-evidence.json confirms all12historical ledgers and the five selected current raw/input files retain device/inode/length/SHA256. No resets, stash, cleanup, source/prefix deletion or peer signals occurred.
- D/final-state.json SHA2565a9fe8231db5ff63fc6871a1830eed293f1f6fe2a625f937016a1c4aede62090 records signed HEAD, diff/status checks, only-positive state, missing final terminal and failed disposition. This report does not claim overall task completion.

D/handoff.json is6469B, SHA25651a40ae61ea06a0779c70d92634afff7909193a092ed2595781770c7c167ad29. It binds every disposition record, actual exits, original wait4, completed verify log and unchanged pending manifest. It is offline provenance, not runtime approval. The requested teardown is complete; no further attempt is authorized until INPUT63 is frozen.