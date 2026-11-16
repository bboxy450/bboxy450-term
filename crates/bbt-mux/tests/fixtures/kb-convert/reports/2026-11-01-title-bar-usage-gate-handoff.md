---
author: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
authorship:
  kind: seat
  process_generation: '1'
  public_key: +NU+89ZpBMFsX8bRpUDlYMkga5k1WphdhD6WumIHeWQ=
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
commits:
- 4f3e1fa89302b0323cbc9d16a7a6ddd2e6885ae8
created: 2026-11-02T04:37:26.777Z
created_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
id: 01M6D8RFZ7BX5HZDWTY8Q5F8XT
number: 3165
provenance:
  digest: sha256:30a999995491435b342774c09eef63a5be2ccbda4da3f4e34bb04a4eb6583e37
  frontmatter_digest: sha256:b3c8e0c0aa8e574687a5bd0ebb4bfc923b4381c8844d87b315e189ea16b0845f
  log: .bboxy/log/2026-11-02.jsonl#13939
  sig: P8AOrlZfmVvX+nBKnel0nl72w9tguAVB3r+AdazIvbySVoAQZSDwC4RRPXN0WEC+SsSfY9o61nNbxuCyqPUlBg==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
supersedes: '#2980'
task: '#2197'
title: 'Title bar usage: implementation and stopped landing gate'
type: report
updated: 2026-11-02T04:37:26.777Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify:
  command: make land-full
  exit_code: 2
---
# Title-bar usage implementation and stopped landing gate

Supersedes [[reports/2026-11-01-title-bar-machine-and-ai-usage]] (#2980). Task #2197; Usage Executor, seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A. Worktree /Users/feral/mydev/bboxy450-term-usage, branch title-bar-usage.

Implementation source commit: 4f3e1fa89302b0323cbc9d16a7a6ddd2e6885ae8, signed by Boxy <bboxy450@gmail.com>, key 844ECC0587DE7AB0 (git signature status G). No landing occurred. The containing report commit adds handoff records; it does not change implementation source.

## Outcome evidence

- crates/bboxy450-usage/src/machine.rs:22,61,125,134 — whole-machine CPU tick deltas using host_processor_info, active+wired+compressed memory using host_statistics64/hw.memsize, and HOME-volume usage using statfs. CPU explicitly warms up on the first observation.
- crates/bboxy450-usage/src/{claude,codex,grok}.rs — separate Provider modules, quota converted to percent left using the most constrained active window. Claude credential precedence is environment, interaction-disabled Keychain, then file; Codex honors CODEX_HOME; Grok filters auth.x.ai-issued auth entries. The common trait and registry support another provider module.
- crates/bboxy450-usage/src/lib.rs:39,51 and io.rs — injected read-only credentials/fetchers, redacted Credential Debug, fixed non-sensitive failures, no refresh/write API. The existing reqwest 0.12.28 client disables redirects and bounds responses/files at 1 MiB. Keychain explicitly requests kSecUseAuthenticationUIFail. Auth failures give provider login instructions.
- crates/bboxy450-usage/src/poller.rs:46,59,84,135,200,275 — independent machine/provider workers, 2-second machine and 300-second AI cadence, coalesced click refresh, 8-second visible deadlines. UI only receives snapshots. ReadSlot retains one timed-out blocking call and discards its late result; epoch/identity/pending guards reject timeout, duplicate and A -> B -> A completions. HTTP publication rechecks credentials.
- crates/bbt-ui-model/src/usage.rs and crates/bbt-ui/src/modules/title_bar.rs:126,238 — compact retained strip immediately left of appearance, machine used percentages and provider percentages left. Missing providers are hidden; errors render an em dash plus reason in semantic/hover text; stale data says "(stale)". Normal paint is muted, waiting starts at 80% used/20% left, errored at 95% machine used or below 10% AI left.
- crates/bboxy450-term/src/usage_strip.rs:9,32, scene.rs:525,559,564 and desktop_window.rs:215 — runtime/event wiring, click refresh and tooltip reason. Snapshot inheritance preserves early worker results when GPU preparation replaces the scene and clears layout cache. Private test accounts, smoke/fake-source and headless owners skip live readers.
- verification/coverage.toml:297, verification/contracts.toml:5, verification/corpus.toml and tests/formal_contracts/usage_models.rs — crate inventory and bounded poller model with account/attempt fences, delayed/duplicate replies, timeout, age and one publication per attempt. The broken control removes guards and must violate safety; deadline-service weak fairness provides retirement. Native implementation witnesses use fake credentials/fetchers.
- ATTRIBUTION.md, generated notices, source headers and tests/fixtures/README.md — Tachyon translation and copied fixture credit, upstream MIT notice and LICENSE-TACHYON retained. Layering, licensing and formal inventories include the crate. No lab reference changed.

## Completed verification

The targeted commands used two Cargo jobs and the worktree-local target; tests used injected credentials, Keychain and HTTP, never real account access.

- cargo test -j 2 --target-dir "$PWD/target" -p bboxy450-usage -p bboxy450-ui — exit 0; provider fixtures and UI snapshot, missing/error/stale/threshold cases passed.
- cargo test -j 2 --target-dir "$PWD/target" -p bboxy450-usage — exit 0, including credential switch during request, refresh, timeout/duplicate/A-B-A and bounded outstanding reads.
- cargo clippy -j 2 --target-dir "$PWD/target" -p bboxy450-usage -p bboxy450-ui --all-targets -- -D warnings — exit 0.
- Native project_contracts_and_broken_controls model test — exit 0; /tmp/bboxy-usage-native-formal.log. This includes UsagePoller and its negative control.
- App usage_ tests — exit 0; /tmp/bboxy-usage-app-tests.log, both click-routing and startup-snapshot/cache witnesses passed.
- make test-licenses, make sync-licenses, sh scripts/layering.sh, sh scripts/brand.sh, git diff --check — exit 0.
- make test-kb-reports with canonical optimization/debug settings — exit 0 on c535deb44abb765845ef0d3b1f203219324d0ad4; nine corpus checks and the four report/task/CLI witnesses passed after authorized byte-for-byte task/artifact imports. target/usage-task-import-check.log SHA256 d2c56cf5ca6d216451eab7b1678b3affadae5c9e7036d15385c8bdba08c0a4d5.
- cargo run -j 2 --target-dir "$PWD/target" -p bboxy450-usage --example sampler_cost — exit 0. Actual scheduled machine worker: 16 samples over 32.000964 seconds, 0.004705 process CPU seconds, **0.014703% of one core** (budget below 0.5%). Includes timer, spawn_blocking, Mach/statfs and publication counter; excludes UI rendering and HTTP. /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c.

## Full gate and required stop

The first full make test on 3dc86e9283d228964624879c2e3654926b25e830 exited 2 after workspace build, strict Clippy and Rust tests ran: two bboxy450-doc corpus/report checks failed because report #2980's task was absent from the committed tree. Oracle scope addenda authorized copying the task and its linked artifact #2984 byte-for-byte. The later targeted corpus/report check passed. The original failed log is unchanged at target/ci-local-3dc86e92-failed.log, SHA256 4c32c90d4735cf04124e2d9e00312a67cffff6ece37b95f09bb8e78233e542f1.

The Oracle changed the route to make land-full, with one run and a stop on unrelated failure. Its CPU rule initially required zero other full runs; the automatic waiter began at 03:39 UTC. New full runs replaced old ones. The Oracle then amended #2197 to allow starting after 45 minutes regardless. The waiter was stopped by its recorded PID, the latest task copied byte-for-byte and the commit signed again. No full gate ran during the wait.

make land-full ran once on signed 4f3e1fa89302b0323cbc9d16a7a6ddd2e6885ae8, using the canonical isolated slot checkout. **Actual exit: 2**; scripts/land.sh returned 77 (presubmit_failed). Layering held and all 140 landing fixtures passed. The existing resource-ledger suite then failed one of 34 tests:

scripts/test-resource-ledger.py:222 — test_signal_reaches_only_the_recorded_command_tree expected child.returncode 143; actual -15.

Neither scripts/test-resource-ledger.py nor scripts/resource-ledger.py differs in the feature commit. The cause of that signal-status discrepancy is unproven. It was not changed or retried, per the brief's stop line. This gate stopped before its Rust, TLC/Yarn, integration and smoke phases; there is no green exact-source full/formal receipt and no landed main SHA.

Evidence retained outside Git:

- /Users/feral/.local/state/bboxy450-term/landing/presubmits/4f3e1fa89302b0323cbc9d16a7a6ddd2e6885ae8-aaa0e376/commands.log — source-bound full gate; SHA256 676534aa6bd8a750953bdfcbaa610b1bd20a5f1a9b70f5deed62a5201b3bb19b.
- target/land-attempt-1.log — landing refusal, SHA256 0ea0103bae99bf4db16f216cac64dd12bc473e99a6321366363922e7d8dd9305.
- target/usage-land-full.log — make exit wrapper, SHA256 743fc0973fcd6c807d12940cabdc95537d963212fdda8a76776e2990135ee0e8.
- target/usage-land-wait.log — CPU-condition observations before the amended rule.

## Limits and handoff

Codex rollout history has no account identity: it is only used while signed out, marked stale and "Historical rollout; account unverified"; authenticated failures cannot borrow another login's history. Credential changes are observed at poll boundaries and before HTTP publication, not continuously. OS calls cannot be forcibly cancelled; their publication and outstanding-call count are bounded. The finite model is not arbitrary Rust/kernel/filesystem refinement or a proof of wall-clock deadlines.

No live provider API/Keychain test, operator installation, release or lab modification occurred. Visual parity was advisory and not run. The native formal model passed, but the complete TLC/Yarn/formal gate and final-source smoke are unverified. The implementation is ready for Oracle inspection; landing remains incomplete. The final native task_report records the signed handoff commit and worktree status.
