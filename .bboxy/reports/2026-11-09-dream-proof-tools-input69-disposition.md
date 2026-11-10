---
type: report
title: INPUT69 quarantine teardown completed; input loop paused
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T12:20:09.363Z
commits:
- eb42d39aafb86d9de7ded7ec30d9bc00a13297cf
supersedes: '[[reports/2026-11-09-dream-proof-tools-input69.md]]'
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M7040S4S991R6SB2W8EWPGRV
updated: 2026-11-09T12:20:09.363Z
number: 93377961366457
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:a72880f3a1412b5ac062b40a80455fd06bdffc655ed1c679a326e32963e216a3
  frontmatter_digest: sha256:294ba8cfafaacd26b890ddec09bd793008ec6ff14da044023b8566c1d966517a
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: njLqhVDOtupfgIP46ZLv5JrrlfkUahQD42sQ/gq9ImKRXTzcuUD1IekeX/EgMtDqs4CknvcRWjPfxJ9+DT0/BA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151011
  event:
    id: 01M7040TKGEW7AXFC6WCBPFSFN
    sequence: 151011
    hash: d18f1af64c04c15c2ed092ae351c8d78ceebd44d9314a60680073fa70b7934d1
---
INPUT69 teardown is complete under Oracle disposition01M703R0TFV6W7FRQR6KKQEX6G. Original controller90389 was signalled only as authorized and reaped by its original parent43277. Actual controller wait4: exit_code=-9, wait_status=9 (SIGKILL). The original canonical invocation then completed with actual exit2. Positive remains PASSED; corrupt_proof and the run remain FAILED/quarantined. The INPUT loop is PAUSED for Architect review. runtime_approved=false.

This append-only disposition report supersedes the pending custody/unknown-exit state in [[reports/2026-11-09-dream-proof-tools-input69.md]] (#93377961366454, committed knowledgef1c677083b500be0ce7c0a1119e3dfb9efc1d0db). That report's original snapshots, source audit and limitations remain preserved. Teardown supplies no missing child proof, successful component verdict or native-lifetime qualification.

## Authority, original identity and actions

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], own worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools. Oracle seat01M6RZRGC3194VVFBC4M7XW4MF authorized inventory, SIGTERM exactly90389, then SIGKILL to the same PID if still alive after10s; let43277 reap, signal nothing else, no retry, then park. Message01M703R0TFV6W7FRQR6KKQEX6G in thread5AAVRMHC56ASJ57EZ3ZP96EACH is retained by ID in the authorization record.

R=target/dream-proof-tools/runs/01M7035811Q2HTZ37K75WQ5K3C.
D=R/quarantine-disposition-20261109.
E=target/dream-proof-tools/input69-20261109.
All are under the own worktree above; new evidence directories0700/data0600.

D/inventory-before.json records controller90389/PPID43277/PGID90389, exact frozen INPUT69 probe path, original corrupt pretest path/hashcd1525c5450f567f8ac6f85a20c078baea6d4cee265e77106ccea1b2782fab5a and lstart Mon Nov9 04:07:45 2026. This ties the signal target to the original effect and guards against PID reuse. ps showed parent43277 S, controller90389 SNs, no children of90389. lsof exited0 for both; numeric FD counts were55 for controller,5 for parent. Full ps/pgrep/lsof command outputs and actual statuses are retained.

Actions:
1. SIGTERM exactly90389: os.kill returned success0/errno0, wall1794226631506749000ns. Durable intent/result written separately. Signal success was not treated as an exit.
2. After at least10s, the same controller was still SNs. Recheck elapsed31057555875ns (31.057555875s). PID/PPID/PGID/lstart/executable/input hash were checked again.
3. SIGKILL exactly90389: success0/errno0, wall1794226662568016000ns. No other PID received a signal.
4. Original wait parent43277 wrote the actual wait4 record for90389. Parent was never signalled. Final ps -p90389,43277 exited1 with empty output: neither remained.

D/{term-intent,term-result,after-term-check,kill-intent,kill-result}.json retain intent, actual syscall status, elapsed interval and identity recheck. No signal operation was repeated. This was teardown of the original failed effect, not a qualification retry.

## Actual exit and verification evidence

R/components/corrupt_proof/wait4.json:
- actual_wait4=true, pid90389, exit_code=-9, wait_status9.
- USER340240000ns, SYSTEM3000086000ns, maxRSS4308992bytes.
-139bytes, SHA25660cfcc80718002765d1758f852cc10fdf99411d9d5f9ee68dd7bab7812bb896c.

The wait record was produced by the original build/wait parent43277. Canonical wrapper PID43082 is a different process. Make observed its build recipe return Error1, then the wrapper's actual canonical exit was2. No separate raw wait4 record for43277 is available; its absence from final ps and the recipe Error1 are the retained parent-completion evidence.

Exact original command:
    make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses

E/verify-exit.json now records actual exit2, started1794225902884912000ns, ended1794226662596178000ns. E/verify.log9947bytes, SHA256caff595b806792a04da28e9f99e6c806ca634bb3836543ac6818dafc85afa6f4. Original exec session92254 returned exit2. The log retains the positive0/check0, original OWNED PENDING line, canonical refusal and make Error1. No new invocation was launched.

The run did not reach pause_resume, cancel, stale_incarnation, clock_failure, debt_overlap or reap_delay. Final test-dream-proof-tools, formal/TLC/Yarn parity/license gates, qualification, receipt-last publication and make land did not run. Native control success and runtime approval are not claimed.

## Verdicts, costs and remaining limits

Positive remains PASSED: controller89282 actual wait4 exit0/check0, valid proof,16/16 settled, all28 final conjunction terms true/first_failing_term=none. Final debt13286537622ns = helper1352025000 + ordinary raw2462987619 + floors9471525003. Reporting-start margin14438642000ns within admitted29858592000ns; raw/final ordinary maxima799625/540375ns; process-create excess0. G43 remains derived, not measured. This preserves the recorded INPUT66/67/69 positive passes; INPUT68's positive failure remains historical.

Corrupt remains FAILED/quarantined. Unaltered raw result still says created16/reaped15, proofnot_reached and coverage_unknown "foreign or reversed live helper resource identity". Its member/comparison/cause was not serialized. Leader91377's owner-local ownwait4/groupgone/paid flags and child91380's image_admitted0/rollup0/child_cost_unproven1/witnessnone retain their earlier meanings. Killing/reaping the controller does not convert that missing child coverage into16/16 qualification.

Corrupt stderr remained empty. No final terminal, conjunction, debt split, terminal-cost verdict, complete error histogram or final margin was generated. Raw debt12354342750ns remains a publication snapshot. Actual parent rusage above is process accounting after the hold, not a final paid-debt receipt.

Observation-terminal receipts remain16 positive and14 corrupt, all escaped_cleanup_bsd/raw code3; no other site was evidenced. These receipts are not helper birth counts. Requested pause stop/order fields remain unavailable because pause_resume never ran.

## Preservation and committed handoff

All eight original stdout/stderr/result/ledger artifacts for positive and corrupt are byte-for-byte unchanged, with original device/inode retained. D/raw-before.json and D/preservation.json bind pre/post length/hash/device/inode. All24 historical ledgers also retain exact length/hash/device/inode. No source, prefix, run, cache or prior report was deleted, reset or replaced. Prefix inode189320907 remains retained; no qualification/candidate receipt was emitted.

Code remains signed eb42d39aafb86d9de7ded7ec30d9bc00a13297cf, with no new source change. Actual git verify-commit Good Boxy, git diff --check0 and git status --porcelain0/empty are recorded in D/final-state.json. Earlier68 unit tests,7 source authentications and60 build commands remain completed exit0 evidence on the same source; none was rerun after teardown.

D/handoff.json5423bytes SHA256dbdf814dbce29a9b81e30e0ba00f305c34bb81e214d22c72a663aef70e578503 binds the authorization, actions, actual exits and preservation records.
D/final-state.json2761bytes SHA256024a53c6bd7dc7167b8d5aa8163754821e8327f79b6244ce48b8c4f96615e8b8 retains real command statuses, signal/canonical outcomes and the explicit parent-wait limitation.

Task qualification remains failed. The requested teardown and final report are complete. The INPUT loop is parked under the Oracle's explicit pause for Architect review; no INPUT70 work, new attempt, source repair, merge or publication will be started from this disposition.
