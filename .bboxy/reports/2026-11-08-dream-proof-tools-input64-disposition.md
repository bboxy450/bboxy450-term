---
type: report
title: 'Dream Proof Tools INPUT64 void freeze: authorized quarantine teardown and actual exits'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T15:46:00.451Z
commits:
- 4332bd5327906bac33cd94c4e2e17dad904d869e
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-08-dream-proof-tools-input64.md]]'
id: 01M6XXCX40W4CRN6BP1VVJFKFM
updated: 2026-11-08T15:46:00.451Z
number: 93377961366382
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:c0cca0193576695e674d3311e00900f69091fb47aaa5226f468f74f9612725c2
  frontmatter_digest: sha256:a5a3d1cb2291db3c6e96956eb78a0420056f7e3ad06f142bae25700d047af638
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: Ns3PMf2YoNRQ2Yh7x2IT9d45aLeEPQ3RT/4d8vCfkTaQFIP7TLoBjClcl5TcPC30x+gG1Bj1pn3IkzFEjE3gAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150742
  event:
    id: 01M6XXD18HGTEARZ1ZHETJZ49A
    sequence: 150742
    hash: 90e2ea6d01362f29fd13fbe92d8858e0f02ab6379e882015f9a889e434ee0c32
---
# INPUT64 void freeze: authorized quarantine teardown and actual exits

The authorized teardown is complete. Controller89578 received SIGTERM, remained alive after10.007874250s, then received SIGKILL. Original parent41501 actually reaped it with wait_status9/exit_code−9, then exited through the normal recipe refusal path1 reported by Make. The original canonical command exited2. The run remains FAILED/quarantined and the INPUT64 freeze is VOID per Oracle disposition. No other PID was signaled; no retry occurred. Parked pending INPUT65. runtime_approved=false.

This append-only report supersedes the pending-exit status in [[reports/2026-11-08-dream-proof-tools-input64.md]] (#93377961366380). It preserves that report's original source/tuple evidence, raw refusal and unavailable final metrics. Forced signal9 termination is not a canonical native exit81 or a proof/settlement pass.

## Authorization, ownership and source cut

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]]. Oracle message01M6XX6FVXRM391Y7XHDS7Y8M4 voided INPUT64, attributed the missing same-wake image admission to a Q2 phase-split regression and authorized this exact teardown: ps/fd inventory, TERM only89578, after10s KILL the same PID if still alive, parent41501 reaping, no other signals, no retry and parking forINPUT65.

The original wait owner and Popen handle were retained until actual own wait4. No timeout status, detached replacement or dropped ownership was substituted. Implementation commit remains signed4332bd5327906bac33cd94c4e2e17dad904d869e, Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. No code, source pin, cap, checker, model or raw wire changed during teardown. git verify-commit HEAD0/Good, git diff --check0 and git status --porcelain=v1 0/empty.

R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XWN035QSQRFAB367J0AZG0. C=R/components/positive. D=R/quarantine-disposition-20261108. E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input64-20261108. All new private disposition records are in the original run; prior evidence and reports remain unchanged.

## Process/FD inventory before signals

D/ps-before.json records actual ps exit0 and the requested controller/parent/every-child inventory before signaling. No child or descendant of89578 existed at that cutoff. Unrelated ps rows were not retained.

| PID | PPID | Stat | Etime | Numeric FDs |
| ---: | ---: | --- | --- | ---: |
|41501|41498|S|10:08|5|
|89578|41501|SNs|07:14|54|

Separate `/usr/sbin/lsof -nP -a -p <PID> -Ff` commands exited0 for both PIDs. D/fd-counts-before.json preserves raw FD records and timestamps; numeric counts exclude cwd/txt/memory records. Total lsof file-record counts were40 and57. There was no additional child FD command because no child existed.

D/controller-identity-before.json confirms89578's original pid.json identity, parent41501 and exact frozen INPUT64 probe path. R/identity.json independently binds build-recipe PID41501. C/wait4.json was absent before signaling.

## Exact signal/reap statuses

| Action/observation | Actual status | Record |
| --- | --- | --- |
| TERM to89578only | kill syscall0/errno0 | D/sigterm-intent.json and sigterm-outcome.json |
| Wait/fresh ps | same controller/parent/path alive after10.007874250s | D/ps-after-10s.json |
| KILL to89578only | kill syscall0/errno0 | D/sigkill-intent.json and sigkill-outcome.json |
| Disposition script | exit0 | original exec session11708 completed; D/actual-exits.json |
| Original parent wait4 on controller | actual_wait4true/status9/exit−9 | C/wait4.json |
| Parent41501 | normal recipe refusal exit1 reported by its owning Make; no signal sent | E/verify.log, Makefile:14–15, recipe Refused handler |
| Original canonical command | actual exit2 | E/verify-exit.json; completed exec session30398 |

Each signal had an exclusive fsynced intent before its syscall and an outcome afterward. TERM intent time_ns1794152600784274000; KILL intent time_ns1794152610792600000. Fresh same-parent/executable checks preceded signaling. No other PID was signaled.

C/wait4.json is139B, SHA25617bc9704074996cb9413c3a3a19b4b8ea074181624c11a3efd9427b57e3595d0. It records WIFSIGNALEDtrue/WTERMSIG9, user_cpu_ns292304000, system_cpu_ns2343796000 and max_rss_bytes4243456. These include the retained quarantine interval and certify no original CPU or lifetime budget. C/retained.json now binds the actual forced-exit record to the original raw artifacts.

Post-disposition `ps -p 89578,41501 -o pid=,ppid=,stat=,etime=` exited1 with no rows, preserved in D/ps-after.json. That is expected absence, not an additional parent wait4 claim. Make's Error1 and the recipe Refused return1 provide the normal recipe-parent exit evidence; this seat did not directly reap41501.

## Original failed proof and completed canonical command

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

This is the same single original invocation, not a rerun. Actual exit2, started_ns1794151969573438000, ended_ns1794152610822255000. E/verify.log is9878B, SHA25611820ab5450ca6a206bbabc588d37db886298196cb4b69fc5caaa41a9d44852a. It retains OWNED PENDING, then the post-reap canonical real proof/component verdict refusal and Make build Error1.

Raw stdout remains byte-identical to persisted result.json, quarantined/coverage_unknown: retired child lacks qualified nonduplicated same-leader rollup. created16/reaped15/group-gonefalse/retired-coveragefalse, proofnotreached. Ledger child_exit_witnesskqueue_note_exit, child_image_admitted0, mask30/proven0/unproven1 and the held queue snapshot remain unchanged. Physical forced controller termination does not reconstruct missing GS1 admission or rewrite those snapshots.

| Original artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C/stdout = C/output/result.json |2048each|26a8531436559432fdc252610b04730e7a6e2beacb7129e8a616941dd1336ad4|
| C/stderr |0|e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855|
| C/output/ledger.ndjson |50507|8fabd6b5e8974ea5f50019f90c6bb7abcd2393da7104c08ca047046340b551d9|

No final terminal/debt split exists. Original snapshot paid debt12.647472372s, raw ordinary_max_ns660041 and retained process_create excess0/count0/all-known1 remain observations at their original cutoffs. Final margin remains unavailable; the15.309174s partial margin at the last retained leader-RI sample is not a completed-proof margin. corrupt_proof never ran, so all its requested metrics remain unobserved. Historical INPUT63 positive stays PASSED and is not substituted for this failed INPUT64 run.

## Verification, preservation and remaining work

- Requested pre-signal ps/children/FD inventory: complete, ps/lsof0, no controller children at the recorded cutoff.
- Authorized teardown: TERM0, wait>10s, fresh live original identity, KILL0 to89578only, actual own-parent wait4 status9/−9, parent normal recipe exit1, canonical2. Independent read-only audit found no status discrepancy.
- Earlier66unit tests,7fresh authenticators and60numbered build commands exited0; none was repeated during disposition. The source stays signed and clean.
- D/preserved-evidence.json confirms7current component files and all15historical ledgers retain original dev/inode/length/SHA256. No reset, stash, source/prefix deletion, peer cleanup or raw rewrite occurred.
- Only positive exists and remains unchecked/failed. corrupt_proof and all later cases, final formal/license gates, command-tail tests, qualification, receipt and landing did not run. Qualification/receipt files are absent; no runtime admission or task-completion claim.
- D/final-state.json SHA25609daa8df1fb985ae355a0f610b56f0ea1e7b2b5dd5c16e5412ca4bc330cded5b records signed clean code, actual failure, missing final terminal, void INPUT64 freeze and parking forINPUT65.

D/handoff.json is6514B, SHA256998fb9f9cc5e9d4e36d51b2f94ea95620dc8a0e25b86d73dbbef36ce15414d08. It binds every disposition record, actual exits, the unchanged pending manifest and completed verify log. Authorized teardown is complete; the qualification task remains incomplete. No further attempt occurs before the INPUT65 freeze.