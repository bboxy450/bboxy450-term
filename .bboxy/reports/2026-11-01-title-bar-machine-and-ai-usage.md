---
author: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
authorship:
  kind: seat
  process_generation: '1'
  public_key: +NU+89ZpBMFsX8bRpUDlYMkga5k1WphdhD6WumIHeWQ=
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
commits: []
created: 2026-11-02T01:19:08.260Z
created_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
id: 01M6CXDD4E3CTCPARS3JD25SC4
number: 2980
provenance:
  digest: sha256:9ca09c74d5cc205aaa6cb617511fc65d09187ca86ce56ceff4dd5299a842b507
  frontmatter_digest: sha256:c80e6e660f65e73f32822c20f821bde1d7fa6137d871a0309c22b708d7a3020a
  log: .bboxy/log/2026-11-02.jsonl#12598
  sig: lvAP7JnwfEW1z+I2lItBFFikyS68MCNi+QpXSqFTHpzKJWvaMEzzMlMaJ/L+yRCAc8Aa7A7UpUsfpkGGbsXkDg==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
task: '#2197'
title: 'Title bar: machine health and AI account usage left'
type: report
updated: 2026-11-02T01:19:08.260Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify:
  command: cargo test -j 2 --target-dir "$PWD/target" -p bboxy450-usage -p bboxy450-ui
  exit_code: 0
---
# Title-bar machine health and AI quota left

Task #2197, Usage Executor, seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A. Worktree: /Users/feral/mydev/bboxy450-term-usage, branch title-bar-usage. The delegated task is tasks/2026-11-01-title-bar-machine-and-ai-usage.md in the authenticated knowledge root. Its on-disk Scope addendum authorizes the layering/licensing entries and generated notice copies.

## Implementation and outcome evidence

- crates/bboxy450-usage/src/machine.rs::platform and cpu_delta — whole-machine processor tick deltas via host_processor_info, physical active+wired+compressed memory via host_statistics64 and hw.memsize (reclaimable file cache excluded), and used blocks on HOME's statfs volume. The first CPU observation explicitly warms up; missing OS fields stay unavailable.
- crates/bboxy450-usage/src/{claude,codex,grok}.rs — one Provider implementation per provider, exact pinned API origins, read-only credential source selection, worst active quota window converted to percent left, and fixed non-sensitive failure reasons. Claude uses environment -> interaction-disabled Keychain -> credentials file; Codex honors CODEX_HOME; Grok filters auth.x.ai issuers. Adding a provider needs its module and one registry entry.
- crates/bboxy450-usage/src/lib.rs::CredentialReader and Credential — no credential mutation/refresh API; Debug redacts secrets; neither published snapshots nor failures carry bearer/account values. io.rs forbids HTTP redirects, limits response/file bytes to 1 MiB and explicitly sets kSecUseAuthenticationUIFail on SecItemCopyMatching. 401/403 return each provider's login instruction.
- crates/bboxy450-usage/src/poller.rs::Worker, ReadSlot and provider_loop — separate single-flight tasks for machine and each provider, using the existing runtime and reqwest 0.12.28. Machine cadence is 2 seconds; AI polls wait 300 seconds between reads, with coalesced manual refresh. Every blocking read and HTTP request has an 8-second visible deadline. A timed-out OS call is retained, never replaced repeatedly, and its late result is discarded. HTTP results are revalidated against current credentials before publication.
- crates/bboxy450-usage/src/poller.rs::PollState — monotonic attempt epoch plus identity and pending fences reject expired, duplicate and A -> B -> A replies. Refresh clears old numbers before re-observing sign-in. Generation exhaustion refuses. Missing providers produce no UI entry; failures show an em dash with a reason.
- crates/bbt-ui/src/modules/title_bar.rs::element_with_usage and usage_metric, crates/bbt-ui-model/src/usage.rs — retained snapshot strip immediately left of the appearance button. Machine values are used percentages, provider values are percentages left; stale values say "(stale)". Existing title-bar callers retain their API. Muted normal paint, waiting at >=80% used / <=20% left, errored at >=95% machine used / <10% provider left. The strip clips within its slot on narrow windows.
- crates/bboxy450-term/src/{usage_strip.rs,scene.rs,desktop_window.rs,main.rs,pointer_cursor.rs} — events update the last UI snapshot without IO or locks, pointer hover paints the full reason/refresh description, and retained strip clicks signal manual refresh instead of dragging/double-clicking the window. Private test accounts, smoke/fake-source and headless owners never start live account readers.
- verification/contracts.toml::UsagePoller, coverage.toml::bboxy450-usage, corpus.toml and crates/bboxy450-term/tests/formal_contracts/usage_models.rs — three A/B/A attempts, delayed/duplicate replies, age, timeout and one publication per attempt, with the fence-removal broken control. Weak fairness of deadline service provides eventual retirement without requiring network replies. The real PollState/worker witnesses use injected credentials/fetchers.
- ATTRIBUTION.md and generated per-crate copies — Tachyon translation/fixture credit and its complete MIT notice; each ported file has a Translated from header. LICENSE-TACHYON preserves the upstream text, and tests/fixtures/README.md identifies the copied synthetic JSON. Layering/licensing/formal inventories include the new crate.

## Checks completed before this report's containing commit

- cargo test -j 2 --target-dir "$PWD/target" -p bboxy450-usage -p bboxy450-ui — exit 0; all provider/UI groups passed, including the retained strip snapshot, missing provider, stale text, error description and threshold colors.
- cargo test -j 2 --target-dir "$PWD/target" -p bboxy450-usage — exit 0 after lint cleanup; counter wrap, stale/timeout/duplicate/A-B-A fences, bounded retained read, injected provider decode/error states, manual refresh and credential-switch-during-request witnesses pass.
- cargo clippy -j 2 --target-dir "$PWD/target" -p bboxy450-usage -p bboxy450-ui --all-targets -- -D warnings — exit 0. OS filesystem IO is allowed only in the blocking credential-reader implementation; fixture runtime/latch allowances are test/benchmark scoped.
- cargo run -j 2 --target-dir "$PWD/target" -p bboxy450-usage --example sampler_cost — exit 0. The actual scheduled machine worker (timer, spawn_blocking, Mach/statfs and publication counter) took 0.004705 process CPU seconds over 32.000964 seconds for 16 samples: 0.014703% of one core, below the 0.5% budget by about 34 times. Providers are absent from this headless measurement, so no credentials, Keychain or network are read. UI rendering and HTTP costs are not measured. The earlier direct-OS-only probe measured 0.005332%; the worker measurement supersedes it.
- make test-licenses — exit 0, 33 workspace crates plus six license and nine source-notice controls. make sync-licenses, sh scripts/layering.sh and sh scripts/brand.sh — exit 0.
- git diff --check — exit 0.

Raw command logs remain outside Git in /tmp/bboxy-usage-*.log and the worktree target directory. Initial targeted attempts exited 101 for the missing pinned harness, old color variants and missing low-level bindings; the harness was initialized and bindings/colors corrected. make init-submodules initially exited 2 after cloning because the new manifest required a lock refresh; make lock exited 0. An intermediate app check exited 101 because its compiled UI interface preceded an in-flight source edit; final checks rebuild the settled source. The first Clippy attempt exited 101 for UI-only IO lint rules and mechanical type/slice/sort issues; worker/test scope annotations and the actual lint fixes are now checked green. No operator input or real account access was required.

The native model and app click tests are still running at report preparation. The required exact-committed-source make test (including native/TLC/Yarn, unit/integration and private headless smoke), followed by sh scripts/land.sh --gate-log target/ci-local.log, remain pending. Their actual exits, resulting main SHA, formal receipt and final worktree state must appear in the final native task_report. This pre-gate report does not claim a future gate passed. commits: [] avoids a self-referential SHA; the signed commit's Report trailer identifies its containing source commit.

## Deliberate bounds and remaining coverage

Codex rollout token_count data has no account identity. Matching Tachyon's account-safety rule, anonymous rollout fallback is only shown while signed out, marked stale and "Historical rollout; account unverified". An authenticated API failure shows an error rather than borrowing another login's history. This is the conservative interpretation of the requested fallback plus the no-cross-account invariant.

Credential/account changes are observed at poll boundaries and before HTTP publication, not continuously between filesystem observations. Blocking OS calls cannot be forcibly cancelled; only their published result and outstanding-call count are bounded. The model checks atomic owner state at finite bounds, not arbitrary Rust refinement, kernel semantics, credential encryption, filesystem races or physical wall-clock deadlines. Provider schemas, credential-source selection, decoding and UI geometry use implementation fixtures. No live provider request, live Keychain read, operator app installation, release or lab-reference modification was performed. Visual parity for this strip remains advisory.
