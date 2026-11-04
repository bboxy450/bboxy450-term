---
author: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
authorship:
  kind: seat
  process_generation: '1'
  public_key: +NU+89ZpBMFsX8bRpUDlYMkga5k1WphdhD6WumIHeWQ=
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
commits:
- 50e4a22dd44a14bf5965b4bf330a347284a4104d
created: 2026-11-02T05:50:55.934Z
created_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
id: 01M6DCY4FCWBTDA8C5GPP73B4M
number: 3172
provenance:
  digest: sha256:41681b49c8f165d5c1339bf14d90f14ed4ada886af7eefaa24e857ab0220f86b
  frontmatter_digest: sha256:cb2498e4af89280c62324d87136eef9804b52021948fb98463bec4600d3b524a
  log: .bboxy/log/2026-11-02.jsonl#14044
  sig: LAgBYCJNuxMxAmaAapOwkyYfI2MciGLBeQT3icyXaK0gq/KS8Sh2oxm9+jAc4CwbAED2zJ1nm6ctf1WjULvZDA==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
supersedes: '#3165'
task: '#2197'
title: 'Title bar usage: authorized retry stopped on app-binary preflight'
type: report
updated: 2026-11-02T05:50:55.934Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify:
  command: make land-full
  exit_code: 2
---
# Title-bar usage: authorized retry handoff

Supersedes [[reports/2026-11-01-title-bar-usage-gate-handoff]] (#3165). Task #2197, Usage Executor, seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A. Worktree /Users/feral/mydev/bboxy450-term-usage, branch title-bar-usage.

Source commit for this retry: 50e4a22dd44a14bf5965b4bf330a347284a4104d, signed by Boxy <bboxy450@gmail.com>, key 844ECC0587DE7AB0 (signature G). No implementation changed after that commit. No landing occurred. Before the reporting-only commit, the worktree was clean, ahead of origin/main by two commits and behind by twelve.

## Outcome evidence

- crates/bboxy450-usage/src/machine.rs:22,61,125,134 — whole-machine CPU tick deltas using host_processor_info, active+wired+compressed memory using host_statistics64/hw.memsize, and HOME-volume usage using statfs. CPU explicitly warms up on the first observation.
- crates/bboxy450-usage/src/{claude,codex,grok}.rs — separate Provider modules, quota converted to percent left using the most constrained active window. Claude credential precedence is environment, interaction-disabled Keychain, then file; Codex honors CODEX_HOME; Grok filters auth.x.ai-issued auth entries. The common trait and registry support another provider module.
- crates/bboxy450-usage/src/lib.rs:39,51 and io.rs — injected read-only credentials/fetchers, redacted Credential Debug, fixed non-sensitive failures, no refresh/write API. The existing reqwest 0.12.28 client disables redirects and bounds responses/files at 1 MiB. Keychain explicitly requests kSecUseAuthenticationUIFail. Auth failures give provider login instructions.
- crates/bboxy450-usage/src/poller.rs:46,59,84,135,200,275 — independent machine/provider workers, 2-second machine and 300-second AI cadence, coalesced click refresh, 8-second visible deadlines. UI only receives snapshots. ReadSlot retains one timed-out blocking call and discards its late result; epoch/identity/pending guards reject timeout, duplicate and A -> B -> A completions. HTTP publication rechecks credentials.
- crates/bbt-ui-model/src/usage.rs and crates/bbt-ui/src/modules/title_bar.rs:126,238 — compact retained strip immediately left of appearance, machine used percentages and provider percentages left. Missing providers are hidden; errors render an em dash plus reason in semantic/hover text; stale data says "(stale)". Normal paint is muted, waiting starts at 80% used/20% left, errored at 95% machine used or below 10% AI left.
- crates/bboxy450-term/src/usage_strip.rs:9,32, scene.rs:525,559,564 and desktop_window.rs:215 — runtime/event wiring, click refresh and tooltip reason. Snapshot inheritance preserves early worker results when GPU preparation replaces the scene and clears layout cache. Private test accounts, smoke/fake-source and headless owners skip live readers.
- verification/coverage.toml:297, verification/contracts.toml:5, verification/corpus.toml and tests/formal_contracts/usage_models.rs — crate inventory and bounded poller model with account/attempt fences, delayed/duplicate replies, timeout, age and one publication per attempt. The broken control removes guards and must violate safety; deadline-service weak fairness provides retirement. Native implementation witnesses use fake credentials/fetchers.
- ATTRIBUTION.md, generated notices, source headers and tests/fixtures/README.md — Tachyon translation and copied fixture credit, upstream MIT notice and LICENSE-TACHYON retained. Layering, licensing and formal inventories include the crate. No lab reference changed.


## Authorized retry and CPU condition

The Oracle classified the first resource-ledger result as a load flake after its own 3/3 isolated runs. This is the Oracle's ruling, not an additional Executor test claim. The on-disk #2197 Verify section's Retry ruling, recorded after report #3165, authorized one more make land-full on 50e4a22d under the same CPU cap and required stopping on any further failure.

The retry used the unchanged requested commit. A watcher began at 04:44 UTC. At 05:05:41 UTC it observed three actual ci-local.sh processes; the immediate 05:06:03 UTC preflight confirmed PIDs 17760, 27179 and 60631, an unchanged signed HEAD and a clean worktree. make land-full was then invoked once. No additional full run or source repair followed.

target/usage-retry-cpu-wait.log records that condition, SHA256 0a3b653e0df9622d38b23b07950c7a8002e1f1069642d5c87af4f338d3ea9f0d. target/usage-retry-authorized-brief.md preserves the on-disk ruling read before the attempt.

## Current-source gate facts

make land-full exited **2**; scripts/land.sh returned **77**, presubmit_failed. No push or landed main SHA resulted.

- Layering held; all 140 landing fixtures passed.
- All 34 resource-ledger tests passed, including the signal test that stopped the previous attempt.
- cargo build --workspace --locked passed in 6m 34s.
- cargo clippy --workspace --all-targets --locked -- -D warnings passed in 4m 55s.
- The full workspace Rust test/doctest command ran to completion under the canonical quarantine wrapper. One target failed: bboxy450-term's app binary tests. Its result was 202 passed, one failed and six ignored; the wrapper identified the single failure as unlisted or expired. This is not a green full gate.
- The current-source usage click, snapshot event, UI strip fixture, provider fixture/worker tests and project_contracts_and_broken_controls native model test passed in this run. Yarn compile-time controls and doctests also ran; the complete pinned Java TLC/Yarn reference-verification target was not reached.
- Existing canonical quarantine/night-only exclusions were retained. No operator input or real provider credentials were used.

The failing test was live::tests::knowledge_provider_events_publish_the_status_and_reread_tasks. The log at lines 2828-2840 records its panic at crates/bboxy450-term/src/live.rs:3771:9:

"build the app before its real-process test"

live.rs:3763-3774 derives the app path by taking current_exe's grandparent and appending bboxy450-term, then asserts is_file before launching the real app. The log's test executable is under /Users/feral/mydev/bboxy450-term-usage/target/private-build/debug/deps; the gate's build target is /private/tmp/bboxy450-gate-slot-1/target. The assertion failed before app launch. live.rs has no diff across the two feature/report commits (git diff 50e4a22d~2 50e4a22d -- crates/bboxy450-term/src/live.rs is empty). The path discrepancy was not repaired or retested; the brief required stopping on any further failure.

Evidence retained outside Git:

- /Users/feral/.local/state/bboxy450-term/landing/presubmits/50e4a22dd44a14bf5965b4bf330a347284a4104d-2693859f/commands.log — source-bound complete retry transcript, SHA256 3237cd8ad1f7a67a095633f1fe96b3e2a295982395c7c77d35ee1459a6fc2fbe.
- target/land-attempt-1-50e4a22d-retry-failed.log — landing refusal, SHA256 af1d73919960160d0868ac6a4ba7bb8c62a7e53e9210cf730a8f4a50eeee0b8f.
- target/usage-land-full-authorized-retry.log — make exit wrapper, SHA256 743fc0973fcd6c807d12940cabdc95537d963212fdda8a76776e2990135ee0e8.
- The previous failed source-bound gate and its hashes remain in report #3165; they were not overwritten.

## Earlier targeted verification and sampler measurement

Report #3165 retains the exact targeted commands and logs: provider/UI tests, focused strict Clippy, app click/startup snapshot witnesses, native models and broken controls, licenses, attribution, layering and brand checks all exited zero. The final record graph on source 50e4a22d also passed make test-kb-reports (nine corpus tests plus four native task/report/CLI witnesses), log target/usage-final-record-check.log, SHA256 cc1e57a75529e5e4c6a5a10b8b10489e5f0f9f760f0e07a9e693f6ef8ade217b.

The scheduled machine worker benchmark exited zero: 16 samples over 32.000964 seconds, 0.004705 process CPU seconds, **0.014703% of one core**, below the 0.5% budget. It includes timer, spawn_blocking, Mach/statfs and publication counter, excluding UI rendering and HTTP. No provider credentials/Keychain/network were read. /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c.

## Remaining coverage and disposition

No green exact-source full gate, complete TLC/Yarn formal receipt, final smoke result or landed main SHA exists. Native model success does not substitute for the reference gate. No live provider API/Keychain test, operator app installation, release or lab-reference change occurred; visual parity for the strip remains advisory and was not run.

Codex anonymous rollout fallback remains signed-out-only, stale and account-unverified; authenticated failures do not borrow history. Credentials are re-observed at poll boundaries and before HTTP publication, not continuously. A blocked OS read cannot be forcibly cancelled; its publication and outstanding-call count are bounded. The finite model is not arbitrary Rust/kernel/filesystem refinement or a proof of physical deadlines.

The Executor stopped as the recorded Retry ruling requires. The containing signed report commit changes only handoff records. Its final record-check result and clean worktree state are supplied in the native task_report; the Oracle must disposition the missing-app preflight before another landing attempt.
