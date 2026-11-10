---
type: report
title: Dream post-reap escape check — landed INPUT71 and verification evidence
task: '[[tasks/2026-11-09-dream-postreap-escape-check.md]]'
author: seat:01M70SN0KF3TJEPW1CRZRB43GE
created: 2026-11-09T19:30:59.392Z
commits:
- ee3669931f346a9e7d43695e11a3003bafc8f8d1
- ad6c8b72fe0bb766164c879acd27c1ece2e565a7
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M70WNMQ1V0G88P01NQ18C3W4
updated: 2026-11-09T19:30:59.392Z
number: 93377961366487
created_by: seat:01M70SN0KF3TJEPW1CRZRB43GE
updated_by: seat:01M70SN0KF3TJEPW1CRZRB43GE
authorship:
  signer: seat:01M70SN0KF3TJEPW1CRZRB43GE
  kind: seat
  public_key: LUlHGfHXR63pnRza1UyzFWeHFZbPPJpbhxUIv8epGBM=
  process_generation: '1'
provenance:
  digest: sha256:4dd014dbe17fdba90e2e18d08b8f3f0118d2e4649a66c43ba3573a835e4c42bf
  frontmatter_digest: sha256:fd5eee0a27f863d472efa824bf693cad42e62e749402733d440b1ca211a90856
  signer: seat:01M70SN0KF3TJEPW1CRZRB43GE
  sig: SzJu8dTRD6oS5Br66smDKHC51+uIT4l2GtBZ1gx+hUDhZwFGgbA9NoClhPSfGLf+I/8ArlgqTsWtmoibG0/nAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151116
  event:
    id: 01M70WNPSG2VDP9MM8VGWT3GVP
    sequence: 151116
    hash: 84ba50e6ea4cb19ad3a599be475df701094317f167a5a1ec63f9b8a09d8b2831
---
Landed signed source `ad6c8b72fe0bb766164c879acd27c1ece2e565a7` on origin/main. The canonical four-command verification and `make land` both exited 0 on that same head. Worktree clean. Runtime approval remains false.

## INPUT71

ONE tuple: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json`.
SHA-256: `f8d82d84b92557d346bc4942d398a21a841addd5769420d914d55c26519cccf1`; 37,964 bytes; 13,707 binding checks. It retains INPUT70's top-level format and includes fresh signed source/compiler/header/probe pins plus all shim pins. Probe source SHA-256 is `077d63a18be3e36ef0c1289f263ffb87beb85954e059c3e929d970248d4c9a63`; compiled probe SHA-256 is `9a9ee37905cdbe6d04c933c7ffc902d06fa29345418dbcd4a59161c4ebf078dc`.

**The shim source and header are unchanged.** Its source, executable, compiler and allowlist pins all equal INPUT70. Shim source SHA-256 `34a9a6ee959843fc3192d64fac05b5439175ffb8dd15cc927b65fd4a81b102cc`; executable SHA-256 `4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d`. No new private basis fields or basis bytes. No builder/candidate run or new Git/GPG cryptographic run; the unchanged-shim crypto witness is explicitly historical. `runtime_approved=false`, `whole_command_lifetime_proof=false`.

## Change and site inventory

All source paths below are in `/Users/feral/mydev/bboxy450-harness-dream-read-q2`.

- `tests/fixtures/dream-read/proof-helper-probe.c:2848` — escape-only unavailable receipts; UNIX ESRCH returns without terminal-proof routing, while other failures write a diagnostic. `:2853` selects this policy for shim escape cleanup and post-reap BSD observations.
- `:4678` — retirement escape cleanup may inspect a child already reaped by Git even though the leader remains unreaped. Only a positively observed same-birth live process outside the original group refuses; zombies are excluded.
- `:2919`, `:4693`, `:4703` — the active shim's post-ownwait4 sites are final group census and retained-incarnation BSD checks. Missing reads are best effort; a same-birth live hit refuses. No signal occurs after own reap. A reused PID with another birth is not an original-incarnation hit.
- Pre-reap live census/pause/observation paths retain observe_or_terminal. The legacy non-shim pre-reap observation matrix remains unchanged. Rust retirement in `src/dream/c1.rs` already returns before post-reap escape effects; it required no change.
- `exec-shim-controls.c:18` and `check-exec-shim.py:48` add six owned-process/source-known controls and the exact old-route mutant. The controls supply lifecycle/error facts and do not claim a complete approved-tool lifecycle.
- `tests/formal/dream_read.rs:3830`, `tests/formal/models.rs`, and `verification/{contracts,coverage,corpus}.toml` register post-reap ESRCH, other-error diagnostic and positive live-hit cases plus control20. Actual ownwait4, terminal caps, full debt and GS2 remain required.

## Acceptance evidence

Evidence directory: `target/dream-postreap-escape-witness/`.

| Requirement | Actual evidence |
| --- | --- |
| Cleanup ESRCH settles | `target/harness-landing/exec-shim-controls/run-wkj2r0iv/result.json`: cleanup-esrch driver exit0, settled=true, terminal_takeovers=0, kill_attempts=0. |
| Another cleanup errno settles with diagnostic | Same receipt: cleanup-eio exit0; retained NDJSON contains kind=escape_check_unavailable, site=escaped_cleanup_bsd, errno=EIO. |
| Inventory other post-reap sites | census-esrch and census-eio exit0; census EIO retains group_census diagnostic. |
| Positive original live escape refuses | escaped-live and alive-after-reap exit0 only after asserting coverage_unknown refusal; the latter asserts zero post-reap signals. |
| Old terminal route reproduces failure | Mutated production wrapper compiled exit0; cleanup ESRCH driver exited the expected81 and failed the precise no-terminal-route settlement assertion. |
| Finite model and broken control | HelperShimBudget and control20 each have112 states/111 edges; baseline passes, control20 fails safety at PostReapFailedEscapeReadDoesNotRefuse. All447 native/Java graphs/verdicts match across96 contracts/one crate: 3,018,869 states, 10,272,101 labelled edges. |
| Preserve existing controls | `target/harness-landing/private-engine-controls-4i0fh7ov/run.json`:114 baseline cases,108 source mutants,288 actual commands; passed with unchanged input sources. New shim/control receipt contains17 controls, including six escape cases and one mutant. |

`required-four.json/.log` — exact task verification exited0, final head unchanged. The successful shell conjunction establishes exit0 for each named Make target. Existing labelled ignored tests remain recorded; this task adds none.
`canonical-land.json/.log`, `landed-receipt.json` — make land exit0;25 command rows, all exit0, including strict lints,101 concurrent lib tests and all10 serialized timing families. Signature verification and push exited0.
`landed-state.json` — independent ls-remote exit0 matches final HEAD; clean worktree.
`preparation.log` — strict signed probe/shim preparation exit0;326-header closure retained.
`INPUT71-binding-audit.json` — current inputs, snapshots, pins, profiles, controls and all4,472 formal archive files checked.

## Fields

No terminal/public wire fields are added. New internal NDJSON record:
`kind`: JSON string / C string literal, fixed escape_check_unavailable;
`site`: JSON string / C const char*, escaped_cleanup_bsd or group_census;
`errno`: JSON integer / C int, captured errno (zero for semantic short read);
`domain`: JSON integer / C engine_observation_domain formatted unsigned.
UNIX ESRCH emits no unavailable diagnostic.

Existing `helper_total_budget.group_gone` remains JSON integer / C unsigned, but shim settlement now means ownwait4 plus best-effort detectors with no positive original live hit; it is not an unconditional group-absence theorem. `native_census_boundary.group_known` remains JSON integer / C unsigned and stays0 on an unavailable census. Exact names/types and semantics are retained in `field-changes.json` and INPUT71. Conditional supported-tool source-review and undiscovered setsid/coalesced-note limitations remain explicit.

## Operational gap and retention

The requested8 GiB disk floor was breached during the four-command run: its sampled minimum was5,194,911,744 bytes; a later compaction sample observed4,911,636,480 bytes. This constraint was not continuously met. I reported the resource blocker, compacted only my current outputs and resumed after free space recovered. Landing's minimum was16,814,804,992 bytes; cleanup left19,301,744,640 bytes.

`formal-storage-compaction.json` records byte-checked COW replacement of3,005 identical current files using immutable prior files without modifying the prior inodes/bytes. `formal-dot-compression.json` retains all447 current raw TLC DOT outputs losslessly as .dot.gz, with both compressed and decoded hashes; states, edges, TLA and metadata remain plain. Current native/Java verification preceded storage compaction. Post-compaction check-formal-evidence exited0, and INPUT71 independently rechecked every decoded DOT hash. Source/model/tool hashes and limits are unchanged.

`cleanup-selective.json` records removal of754,850,408 release-build bytes and1,768,928,947 debug-build bytes produced by this task; prepared artifacts, controls, profile workers, old evidence and the current formal archive remain. Five sink-observer files retain their original bytes/inodes under `observer-evidence/preservation.json`.

`preliminary-interrupted.json` retains an earlier SIGTERM exit(-15) before final HEAD while the missing corpus registration was corrected; it is not passing evidence. Final source commits are `ee3669931f346a9e7d43695e11a3003bafc8f8d1` and `ad6c8b72fe0bb766164c879acd27c1ece2e565a7`, both signed by approved key9B07E849480C68328CA20684844ECC0587DE7AB0 with Report trailers. No remaining code/test failure is claimed resolved by this report; the disk-floor breach above remains a recorded operational gap.
