---
allowed_paths:
- crates/bboxy450-log-converter/
- Cargo.toml
- Cargo.lock
- Makefile
- scripts/
- verification/
- tests/
- .bboxy/
assignee: seat:01M6CKGYEPHYP25GFF6H7B1X23
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0Yj/k2rOV2tLHQm2aES5d/roSsYDRC/E70ukAxvovgo=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-01T23:15:33.879Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6CPB56WQKJ9P5H47YTS117D
number: 5096
provenance:
  digest: sha256:4eb8b35357b2824c55583640cce31dfcdd12213ac0eb114e289e5cde7df65a11
  frontmatter_digest: sha256:6132dd56c93f8a8eb008260e4f085af0e0d3c9ffbca4cd8336f34ce7da1f953e
  log: .bboxy/log/2026-11-01.jsonl#33436
  sig: VAkbGR/1JOlmDemnrEvnD9v/ifRDANf8LG+ThjJcpbRHxTkgFgdQCTNFM5rguK72JVzwxLmX/JTsBfOJ3QiABw==
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
role: executor
status: claimed
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'LC-port: move log-converter P1+P0 into bboxy450-term crates/bboxy450-log-converter, then P2'
type: task
updated: 2026-11-01T23:15:45.742Z
updated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
verify: make test
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-log-converter -b log-converter origin/main`. Push `log-converter` early once the crate builds, because E5's P4 bases on it. Open a bboxy450-term PR. Do not merge or run land.sh.

Binding:
- d5087 (converter in bboxy450-term, no new repos), which supersedes the old-repo parts of #4976 and #5084.
- The spec at harness f3e04a6 or later: specs/log-converter.md §2/§4/§7.
- lc-ir at 8c44ce3 or later.
- d5089 (user-facing name "bboxy450-harness").

Scope:
1. Port P1 (bboxy450-log-converter bb87743b) and P0 probes (7945ad92) verbatim in logic into `crates/bboxy450-log-converter`, as a workspace member.
2. Its tests run under `cargo test -p bboxy450-log-converter` within bboxy450-term's make test, as a ≤60 s share of d5002's affected-test allocation. Its models (none yet beyond controls) join bboxy450-term's formal inventory and verification/coverage.toml, and the crate's own formal.py is dropped.
3. The real-CLI probes become `make test-log-converter-native`, run in ci-local's full run like test-harness-native and never in presubmit. An absent client gives a labelled SKIP that never counts as proof. Unit parts stay in make test.
4. Add a cargo-metadata assertion to `scripts/harness-submodule.py --check-resolution` that refuses any edge from bboxy450-log-converter to bbt-mux, bboxy450-term or bboxy450-harness, with a broken control. Keep the serde_json preserve_order guard green.
5. The deny check (ctx/casr/…) and secret scan follow bboxy450-term's mechanisms.
6. Then implement P2, the bxl format, per the #5084 brief, on this branch or a follow-on branch.

Acceptance:
- Full bboxy450-term `make test` exits 0 at default concurrency.
- All commits are signed, with headers matching bboxy450-term's ATTRIBUTION.
- The report goes in bboxy450-harness `.bboxy/runs/2026-11-01-lc-port-report.log` (tell me the commit).
- Send one task_report with verify_result.command exactly `make test`.