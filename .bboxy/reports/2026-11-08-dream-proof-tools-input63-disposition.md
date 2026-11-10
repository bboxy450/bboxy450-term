---
type: report
title: 'Dream Proof Tools INPUT63 disposition: positive passed, corrupt-proof controller reaped after authorized kill'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T14:04:30.622Z
commits:
- 9ff0f6acbe32c7dfb0b06dcb870783defc8489f7
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-08-dream-proof-tools-input63.md]]'
id: 01M6XQK4T11263DX6EXNSC3X0S
updated: 2026-11-08T14:04:30.622Z
number: 93377961366375
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:5d41b324a5d7a5d64d217e7c70ac71f2a9b87ca0777e0935ce124a3c58adc920
  frontmatter_digest: sha256:61ba4c6e478f7a316efc3fb62e16738e335629e999032fd2ad0624b6636e91c2
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: Kui85Zx0S6e2sfjSwrELwzCUk2674TZezKFMNykicrHwuhK/2UtQ8kvmTQVr0KR52taRELoGWpe1c+9x2KQPAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150712
  event:
    id: 01M6XQK6414YB4A7SZPVP3RGBQ
    sequence: 150712
    hash: 4c5b61304bc50a5a3691ce53c62c7ae51b7960dc33dfe71b149cb89c08e99cd1
---
# INPUT63 disposition: positive remains passed; corrupt-proof controller reaped after authorized teardown

The authorized teardown is complete. Positive remains PASSED: actual controller exit0/check0, proofvalid, settled16/16. The corrupt-proof controller53612 received TERM, remained alive after10.008976375s, then received KILL. Original parent5130 actually reaped it with status9/exit−9 and exited through the normal recipe refusal path1 reported by Make. The original canonical command exited2. The whole run remains FAILED/quarantined. No retry or signal to another PID occurred. Parked pending INPUT64; runtime_approved=false.

This append-only report supersedes only the pending-exit status in [[reports/2026-11-08-dream-proof-tools-input63.md]] (#93377961366372). That report and its original manifest retain the exact freeze, passing positive, failed ordinary unit, both original wire cutoffs and source evidence. A forced corrupt controller exit−9 is not a canonical native exit81, expected corrupt rejection or successful child-cost settlement.

## Authority and unchanged source

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]]. Oracle disposition message01M6XQD30GHZ27WVPNT0MCQ908 authorized ps/fd inventory, TERM exactly53612, after10s KILL the same PID if alive, parent5130 reaping normally, no other signals, no retry and parking for INPUT64. The original owned handle was held until actual wait4; no timeout status or dropped ownership was substituted.

Implementation commit remains signed9ff0f6acbe32c7dfb0b06dcb870783defc8489f7, Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. No code, pins, budgets, checker, model or raw result changed during teardown. git verify-commit HEAD0/Good, git diff --check0 and git status --porcelain=v1 0/empty.

R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XPVSR73HYNHFAB3ECZSZX8. C=R/components/corrupt_proof; C+=R/components/positive. D=R/quarantine-disposition-20261108. E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input63-20261108. All new disposition evidence is private under the original run; prior reports/manifests are unchanged.

## Pre-signal observations and exact actions

D/ps-before.json records ps exit0, sampled before signaling, filtered to controller53612, wait parent5130 and every controller child/descendant. No children or descendants existed at that cutoff. The rows were:

| PID | PPID | Stat | Etime | Numeric FDs |
| ---: | ---: | --- | --- | ---: |
|5130|5127|S|10:08|5|
|53612|5130|SNs|07:10|56|

Separate `/usr/sbin/lsof -nP -a -p <PID> -Ff` commands exited0 for each PID. D/fd-counts-before.json retains raw records and numeric FD counts, excluding cwd/txt/memory entries. Total lsof file-record counts were40 for5130 and59 for53612. No additional child FD command was needed because no child existed.

D/controller-identity-before.json binds53612 to its original pid.json record, parent5130 and the exact frozen INPUT63 probe path. R/identity.json identifies build-recipe parent5130. The original corrupt wait4 record was absent before signaling.

| Action/observation | Actual status | Evidence |
| --- | --- | --- |
| TERM only53612 | kill syscall0, errno0 | D/sigterm-intent.json and sigterm-outcome.json |
| Wait and fresh ps | after10.008976375s, same controller/parent/path still alive | D/ps-after-10s.json |
| KILL only53612 | kill syscall0, errno0 | D/sigkill-intent.json and sigkill-outcome.json |
| Disposition script | exit0 | original exec session22879 completed; D/actual-exits.json |
| Original parent wait4 on53612 | status9, exit−9, WIFSIGNALEDtrue/WTERMSIG9 | C/wait4.json |
| Recipe parent5130 | normal refusal exit1 reported by its owning Make, no signal sent | E/verify.log, Makefile:14–15, recipe Refused handler |
| Original canonical chain | actual exit2 | E/verify-exit.json, completed exec session61032 |

Each signal had an exclusive fsynced intent before its syscall and an outcome afterward. TERM intent time_ns1794146530061207000; KILL intent time_ns1794146540070621000. Fresh same-parent/same-executable checks preceded signaling; no other PID was signaled.

Post-disposition `ps -p 53612,5130 -o pid=,ppid=,stat=,etime=` exited1 with no rows. D/ps-after.json preserves that expected absence, without claiming an additional parent wait4 observation. The parent was not directly reaped by this seat: Make's Error1 and the recipe's Refused return1 provide its normal-exit evidence.

C/wait4.json is139B, SHA2568142049213720a8cb69b211f17f8efb1362945e60f5fbe8109d368514f47f36b. It records actual_wait4true, PID53612, wait_status9/exit−9, user_cpu_ns337414999, system_cpu_ns2686371000 and max_rss_bytes4243456. These include the live quarantine interval and certify no original CPU budget or native proof. C/retained.json now binds those actual forced-exit bytes alongside the unchanged raw report/ledger.

## Passing positive and failed whole invocation remain distinct

The positive's original wait4 and checked records remain actual0/check0. Its final terminal remains settled16/16, error_count0, with the verifier retained via fork_event and child_exit_witnesskqueue_note_exit, rollup_mask31/proven1. GS1→GS2 binds the same inode/hash. Its final paid debt remains24.863725881s: helper1.376091s + ordinary raw4.634007003s + floors18.853627878s. Final process_create excess0/count0/known1. G43 remains explicitly source/basis-derived, not measured.

Positive reporting started29.144471s after entry within the admitted29.834517s, leaving690.046ms at reporting start; actual0/check0 prove this one positive passed the original deadline gate. This is not a general timing guarantee or runtime approval.

Corrupt-proof remains the original resource_limit refusal. Ledger87 records engine_checkpoint actual1020958ns versus the unchanged1ms law. Raw settlement remains created16/reaped15/group-gonefalse/retired-coveragefalse; witnessnone/mask0/unproven1 is preserved. Killing the controller does not rewrite those fields or establish the missing child-cost coverage.

The original raw artifact hashes are unchanged in dev/inode/length/bytes:

| Artifact | SHA256 |
| --- | --- |
| C+/stdout=result.json |cf1907c0d043e8697c54d453cabd3cffbf249f3e283657c7261aa2a3de8f0694|
| C+/ledger.ndjson |4b3e368ba8b576bd7152f56ce12b524324eaf0f68323855fabc624b707ec9342|
| C+/wait4.json |a2ffe0207e9505f0a3b97e12c36b802ac2ddadf0be59af7c59b1b0c355fb31b3|
| C/stdout=result.json |710e4d73263c0e3a920e8dfda89428ad2fe57485f1301f13d84c463afad985bb|
| C/ledger.ndjson |131ccb59fd58e1ae7c4b48b50bc7a9917aa771f89c255d9aea22f51619246feb|
| C/stderr |45c9f50451d03efb143255ab96f881bcc7446fe200af0a860fb4a81aa79afd25|

No corrupt final terminal or final debt split exists. Its earlier paid snapshot23.530964994s is preserved; no final helper/raw/floor values are invented. Its retained helper snapshots show process_create excess0/count0/all-known1.

## Canonical verification, preservation and remaining gates

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

This is the same single original invocation, not a rerun. Actual exit2, started_ns1794145901562682000, ended_ns1794146540102912000. E/verify.log is9947B, SHA25632752bbe57cb75e806253cb5f8912920a43056b5976c4d848a7edeab6c41e823. It records positive actual_exit0/check_exit0, corrupt OWNED PENDING, then post-reap canonical verdict refusal and Make build Error1.

- Requested ps/children/fd inventory: complete before signals, actual ps/lsof0; no controller children at the cutoff.
- Authorized escalation/reaping: TERM0, wait>10s, fresh live identity, KILL0 to53612only, actual own-parent wait4 status9/−9, parent normal recipe exit1, canonical2. Independent read-only audit found no discrepancy.
- Positive remains PASSED; corrupt and whole qualification remain FAILED/quarantined. No retry or later case ran. pause_resume,cancel,stale_incarnation,clock_failure,debt_overlap,reap_delay, final formal/license gates, command-tail tests, qualification, receipt and landing remain unrun. Qualification/receipt files are absent.
- Earlier66unit tests,7fresh authenticators and60numbered build commands all exited0; none was repeated during teardown. Conditional source/compiler/platform assumptions remain unchanged; runtime_approved=false.
- D/preserved-evidence.json verifies16current component files and all13historical ledgers against the pre-disposition dev/inode/length/SHA256. No reset, stash, removal, peer cleanup or source/prefix rewrite occurred.
- D/final-state.json SHA2561b32d8c8b738f491bf0f1dc0423ec5211dce422894b5f5a3361d2804dcd4c13b records signed clean source, both case verdicts, missing qualification/receipt and parking forINPUT64.

D/handoff.json is7240B, SHA2565eace0c561b8bec73866c820bc551abc48a2d2b5d8c2aca17f94bc326988f192. It binds every disposition record, actual exits, unchanged positive records, original pending manifest and completed verify log. Teardown is complete; the assigned qualification is not complete. No further attempt occurs before the INPUT64 freeze.