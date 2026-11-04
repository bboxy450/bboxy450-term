---
author: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
authorship:
  kind: seat
  process_generation: '1'
  public_key: +NU+89ZpBMFsX8bRpUDlYMkga5k1WphdhD6WumIHeWQ=
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
commits:
- 4960d67694a936217174ad483871ed1cefb1ec81
created: 2026-11-02T06:04:28.349Z
created_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
id: 01M6DDQVWD8VM5SQ6Q8HM0V59K
number: 3339
provenance:
  digest: sha256:205714f2f84234cd5a32988449b6e7bd6af4b321c22f30ca8a0f05cfacb2a223
  frontmatter_digest: sha256:b6b43350f779b11e6be5047355445bfb9c63b7b351b0afbbbee8fd3407410999
  log: .bboxy/log/2026-11-02.jsonl#14951
  sig: wLJPhFftJpfk7BwXWkpTr8S2IRoV3QOZHIukCUyzm2z00ZO3bvebYESI9S6f4OBuQPGjBryRuwLbDt/dI8vvAw==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
supersedes: '#3172'
task: '#2197'
title: 'Title bar usage: signed rebase and worktree gate route'
type: report
updated: 2026-11-02T06:04:28.349Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify:
  command: cargo check --workspace --all-targets --locked -j 2 --target-dir target
  exit_code: 0
---
# Title-bar usage: worktree gate route

Supersedes [[reports/2026-11-01-title-bar-usage-retry-handoff]] (#3172). Task #2197; Usage Executor, seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A. Worktree /Users/feral/mydev/bboxy450-term-usage, branch title-bar-usage.

This report is prepared before the single newly authorized full worktree gate. It records completed evidence and the current route, not a future green gate or landing. The final native task_report must supply the actual make test exit, source SHA/log, formal receipt, landing command exit and resulting main SHA.

## Outcome evidence

- crates/bboxy450-usage/src/machine.rs:22,61,125,134 — whole-machine CPU tick deltas using host_processor_info, active+wired+compressed memory using host_statistics64/hw.memsize, and HOME-volume usage using statfs. CPU explicitly warms up on the first observation.
- crates/bboxy450-usage/src/{claude,codex,grok}.rs — separate Provider modules, quota converted to percent left using the most constrained active window. Claude credential precedence is environment, interaction-disabled Keychain, then file; Codex honors CODEX_HOME; Grok filters auth.x.ai-issued auth entries. The common trait and registry support another provider module.
- crates/bboxy450-usage/src/lib.rs:39,51 and io.rs — injected read-only credentials/fetchers, redacted Credential Debug, fixed non-sensitive failures, no refresh/write API. The existing reqwest 0.12.28 client disables redirects and bounds responses/files at 1 MiB. Keychain explicitly requests kSecUseAuthenticationUIFail. Auth failures give provider login instructions.
- crates/bboxy450-usage/src/poller.rs:46,59,84,135,200,275 — independent machine/provider workers, 2-second machine and 300-second AI cadence, coalesced click refresh, 8-second visible deadlines. UI only receives snapshots. ReadSlot retains one timed-out blocking call and discards its late result; epoch/identity/pending guards reject timeout, duplicate and A -> B -> A completions. HTTP publication rechecks credentials.
- crates/bbt-ui-model/src/usage.rs and crates/bbt-ui/src/modules/title_bar.rs:126,238 — compact retained strip immediately left of appearance, machine used percentages and provider percentages left. Missing providers are hidden; errors render an em dash plus reason in semantic/hover text; stale data says "(stale)". Normal paint is muted, waiting starts at 80% used/20% left, errored at 95% machine used or below 10% AI left.
- crates/bboxy450-term/src/usage_strip.rs:9,32, scene.rs:525,559,564 and desktop_window.rs:215 — runtime/event wiring, click refresh and tooltip reason. Snapshot inheritance preserves early worker results when GPU preparation replaces the scene and clears layout cache. Private test accounts, smoke/fake-source and headless owners skip live readers.
- verification/coverage.toml::bboxy450-usage, verification/contracts.toml::UsagePoller, verification/corpus.toml and tests/formal_contracts/usage_models.rs — crate inventory and bounded poller model with account/attempt fences, delayed/duplicate replies, timeout, age and one publication per attempt. The broken control removes guards and must violate safety; deadline-service weak fairness provides retirement. Native implementation witnesses use fake credentials/fetchers.
- ATTRIBUTION.md, generated notices, source headers and tests/fixtures/README.md — Tachyon translation and copied fixture credit, upstream MIT notice and LICENSE-TACHYON retained. Layering, licensing and formal inventories include the crate. No lab reference changed.



## Source and signed rebase

The Oracle added Route change to #2197 Verify after report #3172. It replaces land-full with: signed rebase onto current origin/main, one make test in this worktree's target under the CPU cap, then sh scripts/land.sh --gate-log target/ci-local.log. An untouched failure must be reported without retry or repair; a touched test failure also stops for a report.

git fetch origin exited zero. The branch rebased on origin/main 18a366d3, preserving and signing all three commits, with cargo check --workspace --all-targets --locked -j 2 --target-dir target executed after each commit under the canonical debug/optimization settings. The entire rebase exited zero:

- f028dc1ab5857493361c1ab59318dc4ae285ceed — implementation, check exit zero (2m 09s).
- 089d8990cd67580cd51a07fbcc4331e6f02e24b7 — first gate handoff, check exit zero (1.08s).
- 4960d67694a936217174ad483871ed1cefb1ec81 — authorized retry handoff, check exit zero (0.60s).

All three have git signature status G, Boxy <bboxy450@gmail.com>, key 844ECC0587DE7AB0. The worktree was clean and three commits ahead of origin/main before preparing this reporting-only commit. No implementation was changed for this route.

target/usage-route-rebase.log SHA256 de3f9a39bc682327678ba11390ee47efa009a28d671cd78c1a09669c18482974 records rebase exit zero. target/usage-route-rebase-checks.log SHA256 9162269b35989b19ae6f77e3cd7cc1cf61159939e1058a6db96bdfad9c90b470 records the source hashes and successful checks. make help was read on the rebased source.

## Verification already completed

Before this rebase, the targeted provider/UI tests, focused strict Clippy, app click/startup snapshot witnesses, native models/broken controls, licenses, attribution, layering, brand and record checks exited zero; #3172 preserves their exact commands and logs. The full retry on original signed 50e4a22d additionally passed all 34 resource-ledger tests, all 140 landing fixtures, workspace build and strict workspace Clippy. Its current-source usage provider tests (seven), machine/poller units (three), UI strip fixture, click/event witnesses and native project_contracts_and_broken_controls test passed.

The two land-full attempts remained red. The first stopped in the ledger signal fixture; the authorized retry passed that fixture but failed the unchanged live.rs:3771 app-binary preflight before real-app launch. Its test executable was under target/private-build/debug/deps while its build target was in a separate gate slot. No out-of-scope fix or unapproved retry occurred. These are historical results on the prior source, not green verification of the rebased source. #3172 retains the complete retry transcript and hashes.

The actual scheduled machine worker benchmark exited zero: 16 samples over 32.000964 seconds, 0.004705 process CPU seconds, **0.014703% of one core**, below the 0.5% budget. It includes timer, spawn_blocking, Mach/statfs and publication counter, excluding UI rendering and HTTP. No provider credentials, Keychain or network were read. /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c.

## Pending exact-source checks and bounds

The report, its responsibility claim and the Oracle's latest task record are copied byte-for-byte from the authenticated knowledge root and committed before the final gate, so committed links resolve. The final native task_report records their actual committed-record check result and containing signed SHA.

The authorized next commands are make test with this worktree's target and target/ci-local.log, followed only on a green gate by sh scripts/land.sh --gate-log target/ci-local.log. Existing CI quarantine/night exclusions remain in force. The complete pinned Java TLC/Yarn reference gate, process/integration/smoke stack and landing on this rebased source remain pending at preparation. Native model success does not substitute for the reference receipt. No future result is claimed here.

Credentials remain read-only; anonymous Codex rollout fallback remains signed-out-only, stale and account-unverified. Account changes are observed at poll boundaries and before HTTP publication, not continuously. Blocked OS calls cannot be forcibly cancelled; their publication and outstanding count are bounded. The finite model is not arbitrary Rust/kernel/filesystem refinement or a proof of physical deadlines.

No live provider/Keychain test, operator installation, release or lab-reference edit occurred. Visual parity is advisory and not run. The Oracle verifies the resulting source, real logs and final native report.
