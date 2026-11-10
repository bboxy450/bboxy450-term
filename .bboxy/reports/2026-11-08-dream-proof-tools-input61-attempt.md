---
type: report
title: 'Dream Proof Tools INPUT61 sole native attempt: retained signature-custody refusal'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T09:58:49.684Z
commits:
- 1691e3947ca04739f1301b596eae388f9c6f2aef
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-08-dream-proof-tools-input61.md]]'
id: 01M6X9H996AR2ASKSRC9NNA1SE
updated: 2026-11-08T09:58:49.684Z
number: 93377961366354
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:6470d093e987e0a267d64a6439dc6712aef625f75b77cc30f002192eb18fc8f3
  frontmatter_digest: sha256:6a546a2e84999ddc044df92c0b20f7a74b9662484c5d962f728af7f33470b6a3
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: 18GkN4rVCgqbUHAsVCIbOwGVVryJ/Ll3FfVEx/S4+e6ZhhHEYaXqBpvD6bB380JrrtCrUmOYZPYvaMwZCWxFCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150632
  event:
    id: 01M6X9HANFM95B4YY1KC6VX7BB
    sequence: 150632
    hash: 79a2af8400be5e613332ae3ed760d17954134756d40ae5b9d8736b6029971a61
---
# INPUT61 sole native attempt: retained signature-custody refusal

BLOCKED. The one authorized INPUT61 attempt ran from the positive and exited 81. The canonical task command exited 2. Required verifier/GS1 evidence was unavailable; no timing retry, budget change or substitute input followed. runtime_approved=false.

This completes the authorized attempt and supersedes the pre-build disk-blocked status in [[reports/2026-11-08-dream-proof-tools-input61.md]] (#93377961366352). It does not supersede or reclassify any historical failure. The exact decision needed is Oracle disposition of the signature verifier/GS1 custody refusal and a causally changed authorized tuple or scoped repair brief before any further native attempt. Neither a local recipe mismatch nor a Q2 defect is established by the retained census.

## Source cut and scope

- Signed code commit: 1691e3947ca04739f1301b596eae388f9c6f2aef, Boxy <bboxy450@gmail.com>, key 9B07E849480C68328CA20684844ECC0587DE7AB0. git verify-commit HEAD exited 0 with Good signature. git diff --check exited 0 and git status --porcelain=v1 exited 0 with empty output.
- Commit changes only scripts/dream-proof-tools.py and tests/fixtures/dream-proof-tools/sources.lock.json: select the exact INPUT61 source, immutable executable and preparation receipt. The terminal checker and its INPUT60 four-field whitelist remain unchanged. There were no code edits during the resumed native attempt.
- scripts/dream-proof-tools.py:52 selects the tuple; :2342 validates immutable inputs without running them; :2953 checks the real component verdict; :3141 preserves raw results/wait4 and stops at the first refusal; :3186 orders components before final gates and qualification; :2853 publishes the candidate receipt last only after validation.
- All source, dependency, compiler, SDK, Mach-O and source-review findings remain conditional source/compiler/platform premises. This report grants no runtime capability or whole-command-lifetime theorem.

## Exact freeze and preflight

Authority is task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], final section ORACLE FREEZE, INPUT61, plus the Oracle disk-release continuation message 01M6X8RCM3RXE6FCBQ5R63PY88. Decision 66280 total budgets and decision 66331 process_create wall ceiling remain unchanged.

- Source/preparation commit f6dffea0ff8038240de796e237362577e58fe843, signed Boxy and on observed origin/main; INPUT60 1602aff7 is an ancestor.
- Frozen directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input61/dream-read-c-preparation/compile-wcb1m3yz/.
- proof-helper-probe.c: 328721 B, SHA256 9bacbb0483ab328798af2bc98225bdf8f50f930e6d2ecda6779cec3afec75042.
- probe: 251064 B, SHA256 c50c2ae7d6f8aad7f3c0c99214e0683ea7b78cdf686f6acba8ca31fe340a2009.
- Receipt: preparation-eb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987.json, 530062 B; SHA256 equals the name digest.
- Nine other frozen inputs match INPUT60; terminal/result/tail format strings match byte for byte. Schema/vector authority is blob identity at b26d7cd3, not the superseded ancestry claim. Schema blob 96e2839a9d2b6031fb0138934eef96850e9b7a60; vectors blob 54cfab3b0e6cc8996bbc942cc2b0128278fccfaf.

The earlier INPUT61 preflight remains retained at target/dream-proof-tools/input61-20261108/: unit.log SHA256 1de2fba6d4e9aed1eb7ae8cc4a115c036be0aecafe38c29f38d2c14a48f11e88 records python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target, exit 0, 66 tests. preflight.json, source-byte-checks.json and terminal-binding.json record signed main binding, exact bytes, nine unchanged inputs and terminal compatibility. These tests were not repeated after the unchanged signed cut.

Current resumed evidence is E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input61-attempt-20261108. The disk recheck /bin/df -k workspace exited 0, reporting 30421104 KiB available, above the strict 8388608 KiB gate. E/disk-gate.json SHA256 dc904a5e83ad8c67e2b85c2329a612b300d587e721d333b8f24e67e2f9dac961 records the pre-attempt observation. This seat performed no cleanup. Exact offline ingestion was repeated successfully, exit 0; E/ingestion.json SHA256 f74badbe7e8ba94874dbc4133017287199cd3be2e7d4fc40d8ed1036f704d86a.

## Actual commands and original artifacts

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

Actual shell/make exit: 2. E/verify-exit.json records start_ns=1794131352916898000 and end_ns=1794131533604666000. E/verify.log is 9665 B, SHA256 5f36c4ffbc5ae5f00eeeae7857faad0488c7b000b540147bacbbaaa1852ca770, ending REFUSED: canonical real proof/component verdict failed; no timing retry and make build error 1. All seven fresh source-authentication records have auth_exit=0; all 60 numbered build/configure/link command exit records 000–059 are 0. These facts do not make the native qualification green.

Build/run ID: 01M6X8ZSXWJQAF6DZH8VE88G5F. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6X8ZSXWJQAF6DZH8VE88G5F. Candidate P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6X8ZSXWJQAF6DZH8VE88G5F. Its original dev=16777234, inode=186555168 and incarnation=1600342476868125 still match intent and descriptor. R/recipe.py and intent bind recipe SHA256 05d89c75aa4a1a7f3497e88b4e56ac14f9379017baa77076c3b370231496d34e.

Only R/components/positive exists; C below means that directory. The 3439 B descriptor SHA256 is f259858bfda02af73dea3a2ea4d02e1b180e8b533da853dd79ef75332c90b189. The 47112 B original basis P/provenance/pretest-basis.json SHA256 is 0b5a0210401bd6e2bf91aff248216bd0f990b9248ddd00b64d02902cb0a7d2c1. Component run_id=01M6X94D4YP9EC9P4NY0CBFDNS and nonce=01M6X94D4YQ3REG5NRM8573V59 are retained independently of the build ID.

| Original artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C/stdout and identical C/output/result.json | 2053 each | affc0ae8ff0addf9ddf4f63ac2865eb0f00ffee51c236ec2ae317bfbf9ab9fd3 |
| C/stderr | 2713 | 8c97d9b1abb10815f7e75354e82ffc717a8fedabe9fdce0bab2724a23e2afd88 |
| C/output/ledger.ndjson | 45082 | b3239d0a53190cb5775030a0d50d6c210f1d8a52e5e759bab4ef420cefe7b5d4 |
| C/wait4.json | 142 | a8f5bd0c10ba51ca5c63342a1f4c2e04b3ee5a6882ff18900abe082c5f932a16 |
| C/retained.json | 1410 | 52a079a1f13af5a29df20ba8f60b40d57638c81d2ef5b31b91aab3c328759e7d |

C/wait4.json proves the parent actually reaped controller PID71655 with exit_code=81, wait_status=20736, user_cpu_ns=110100999, system_cpu_ns=398549999 and max_rss_bytes=4177920. No kill, reconstructed exit or retry is substituted for this observation.

## Refusal and evidence limits

The unaltered result is phase/outcome=quarantined, observed_proof=not_reached, error settlement_unknown: GS2 requires actual leader reap and the retained GS1 descriptor. It records attempts=16, created=15, reaped=15, group_gone=false, retired_coverage=false, members_max=1. The stderr diagnostics retain coverage_unknown count 1: signature lacks exactly one retained verifier, and settlement_unknown count 2 with the GS2 detail. Final terminal remains control_verdict=failed, stage=quarantined despite persistence_verdict=persisted and helper_created/helper_reaped=15/15. Persisting a refusal is not a proof pass.

The 76-row original ledger has 15 helper_total_budget rows at unique sequences 1–14 and 16, each phase=reaped with actual_own_wait4=1 and group_gone=1. Sequence16 is the signature command leader; retained_child_accounting still has child_created=0, child_image_admitted=0, rollup_proven=0. A per-leader physical group observation does not establish retained verifier identity or GS1/GS2 custody. The original raw result's false settlement coverage is preserved.

Frozen proof-helper-probe.c:2707–2715 increments verified_descendants only after seeing the expected verifier image, a STOP acknowledgment and successful private_git_sig_gs1 binding. :2284–2287 refuses GS2 when its required actual leader-reap/retained-descriptor conditions are absent; :3388 requires exactly one retained verifier for form12. These pointers explain the failed predicates. They do not establish whether no verifier ran or a short-lived verifier escaped observation. A read-only independent audit found no established recipe mismatch against the frozen command forms. No helper command was re-executed to investigate.

## Requested G, debt and process_create excess

G=43 is SOURCE/BASIS-DERIVED, NOT MEASURED: 26 candidate guards + 17 fixture guards. E/guard-derivation.json, 13161 B, SHA256 7cd59e151bf76a87b9c72a627d8d0f0e25757deb0a778a9529c224fd5144ca74 enumerates unique (root,path) admissions from the descriptor, configs, closure images/statics/member-indexes/link maps and fixture inventory. Equal duplicate path bindings collapse. Absolute compiler/SDK/review references are not runtime guard admissions; no runtime G counter is claimed.

The final terminal uses debt_scope=whole_controller_cumulative with ordinary_raw_complete=true and debt_components_match=true. Costs and 9× debt are:

| Component | Retained cost ns | Debt ns |
| --- | ---: | ---: |
| Helper | 149837000 | 1348533000 |
| Ordinary raw CPU | 355124083 | 3196116747 |
| Ordinary floor surcharge | 2066212083 | 18595908747 |
| Total | 2571173166 | 23140558494 |

Ordinary total cost=2421336166 ns. Final debt_charged_ns=debt_paid_ns=23140558494 (23.140558494 s). All 15 unique helper costs sum to the helper cost total; cumulative global watermark rows were not summed. booked_ordinary_units=2421, engine_charge_calls=158 and poll_closure_calls=3. The earlier raw JSON metric debt=23068558494 ns has its own pre-result cutoff and remains unchanged; the final paid tail includes additional ordinary accounting.

Final process_create_wall_excess_ns=0, count=0, known=1, scope=invocation_sum_once_per_prepared_spawn. The helper rows likewise retain zero excess per original prepared-spawn snapshot. No process_create over-25ms excess was recorded. Final deadline_observed_ns=0. Reporting began 26962527000 ns after original_entry_ns, with 2871175000 ns remaining to the original deadline. The full positive never reached proof, so a full 16-helper positive fitting 30 s remains UNPROVEN.

E/telemetry.json, 35301 B, SHA256 4a80bb8292c09fa5605ba424b5e6062a1930d416c4edfcb4f4b54e893f7a273c preserves all original result values, final terminal tokens, errors, unique helper history and the debt arithmetic.

## Acceptance accounting and retained gaps

- Exact current tuple, signed source and immutable custody: satisfied by the earlier signed preflight and repeated complete offline ingestion; no terminal/schema/budget expansion.
- Private real source authentication/build: seven authenticators exit 0 and all 60 original numbered commands exit 0. The native positive fails 81, so this candidate does not satisfy qualification.
- Positive → remaining components → final formal/license gates → qualification → receipt: stopped at the first positive refusal as required. corrupt_proof, pause_resume, cancel, stale_incarnation, clock_failure, debt_overlap and reap_delay did not run. make test-dream-proof-tools, make test-formal-parity and make test-licenses in the command tail did not run; neither did the builder's final formal/license invocation. They are unverified for this candidate.
- P/qualification.json and P/receipt.candidate.json are absent. No sealed review-ready candidate, runtime admission, landing, merge or publication occurred.
- Preservation: all 11 predecessor ledgers still match original dev/inode/length/SHA256. E/preserved-ledgers.json SHA256 764d87f32e4ed0c0e72b1f724234f21598000e4f6a5fe258a1b677d0da0d9f06 records that check. This seat did not reset, stash, remove prefixes or modify predecessors' sources, raw outputs or ledgers.
- Signed clean source handoff: E/final-state.json SHA256 207c47c4ff7396759914986dd1372d3e8d6a6edbcd68d4569260bac873307249 records git verify-commit=0, git diff --check=0, git status=0/empty and the original candidate identity. New E files are private mode0600; existing original artifacts remain untouched.

E/handoff.json is the full offline provenance manifest: 105612 B, SHA256 9508cc05828f56f162de8f387a8a84b5b3f7cb15989189b29f638183847a10ad. Its size is not a runtime input-budget assertion. It binds exact original artifacts, every numbered exit record, authenticators, preservation check and telemetry. The attempt is retained as FAILED81 and the canonical verify remains 2.