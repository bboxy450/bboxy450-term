---
id: "01M6H9VT0QJ8ZP6GHFVQH9T36X"
number: 93377961359202
type: report
title: "Yarn temporal implementation verified; landing blocked by main harness layering"
task: '#15234'
commits: ["76de285870509035997e2b1c44aa78be09a173ae"]
supersedes: "[[reports/2026-11-03-yarn-protocol-temporal-effects]]"
verify:
  command: make test-yarn-model && make test-formal
  exit_code: 0
author: "seat:01M6GVBAY1HW6P67GGDZM9XEBY"
created: "2026-11-03T18:13:39.991098Z"
---

ROLE: EXECUTOR. Task #15234. This supersedes the original report's pending-landing disposition. The complete implementation and pre-rebase command/acceptance evidence remain in that report. This is a blocked handoff, not an Oracle acceptance or a claim that the rebased tree passed full CI.

Implementation commit 76de285870509035997e2b1c44aa78be09a173ae is signed G by Boxy <bboxy450@gmail.com> with the approved key, after scripts/land.sh rebased original signed 72940c72f56b755f07748bc5b0bfc98ddf801cc7 onto current origin/main. The branch is yarn-temporal in /Users/feral/mydev/bboxy450-term-yarn-temporal. No push occurred. The author seat is the inherited delegating Oracle report context; the execution role is Codex Executor.

## Exact landing blocker

`sh scripts/land.sh --full` exited **77**. Its canonical `make test` exited **2** on exact committed source 76de285870509035997e2b1c44aa78be09a173ae. The first CI check, `sh scripts/layering.sh`, returned **1**:

```text
bboxy450-harness -> bboxy450-term-desktop: depends upward
bboxy450-harness -> bboxy450-term-desktop-core: depends upward
layering: VIOLATED
```

- `scripts/layering.sh:8` places bboxy450-harness before both desktop crates in ORDER.
- `crates/bboxy450-harness/Cargo.toml:34` and `:35`, at main's new pinned harness 0c403165e815c2f7bf77ccac293784b021f42f96, declare the two dependencies. Root commit 3e581dfe advanced that pin while this Executor was verifying its work.
- `git diff --name-only origin/main..HEAD -- scripts/layering.sh Cargo.toml Cargo.lock crates/bboxy450-harness` produced no paths. The dependency graph and ordering error are on the new base, outside this brief's allowed paths. No source, ordering, feature or gate bypass was applied.
- `make init-submodules` exited 0 after the rebase and synchronized the own worktree to that exact harness pin. The gate's private checkout already used the new pin. This resolved the transient own-worktree submodule dirtiness, not the layering error.

The blocker clears when the harness dependency graph / canonical layer order is repaired by an authorized assignment, followed by a successful scripts/land.sh full gate on the rebased source. No unchanged red gate was rerun.

Gate artifacts: `/Users/feral/.local/state/bboxy450-term/landing/presubmits/76de285870509035997e2b1c44aa78be09a173ae-e8ca8699/commands.log`. SHA256 `dfa02739a0ce270b9196169e4a4b8ee1ded898ae8daf2e494ec9b808e5552df4`. The sibling make.log records `make: *** [test] Error 1`; the landing presubmit record records exit=2/verdict=red. The root wrapper log is target/temporal-land.log. Raw logs remain outside Git.

## Acceptance and completed verification

The task's exact verify command recorded above **really exited 0 before the rebase**. It checked 272 models, 34 crates and 112 contracts, with complete native/TLC state sets, labeled edges and verdicts equal. The precommit source was 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c with dirty=true; receipt SHA256 2651a46224033ba9158bf9238de8ba17740c4f669a121f6b15e14b03ffcb6942 remains archived in .git/bboxy-formal-evidence. target/temporal-verify.log SHA256 is `a89be85e590edcb77df3e4e8ae4faf9765aa1176508decf6837a9a84df7585d1`. The earlier disk-error attempt exited 2; removing only 434.6 MiB of this run's older generated artifacts and using BBOXY_CACHE_MODE=off produced the green retry. No peer output or global configuration was changed.

1. Temporal declaration parsing/lowering: predicate, terminal, quiescent, stable-ID leads_to, and named-case WF/SF groups are generated from the typed AST. The fair-stuttering control proves that ignore/reject and the quiescent label do not satisfy a false terminal response.
2. Effect lifecycle: generated admission and all listed completion outcomes require exhaustive on coverage. Native API tests separate intent/admission/completion, reject pre-admission success, enforce cancellation-before-start and revalidate generation at presentation.
3. All eight finite models replay every explicit edge through generated step. Native/Java parity is pass 91 states/1365 edges for PresentGeneration and PresentProgress; safety 99/1485 for missing generation revalidation; liveness 91/1365 for missing presentation fairness; pass 6/42 for retained unknown-outcome claim; safety 16/112 for retry without that fence; and pass 2/4 versus liveness 1/2 for terminal-target progress versus fair stuttering. The three requested bad controls assert/replay their actual counterexamples; unknown retry has the six-event duplicate-commit trace.
4. Five normal trybuild negative controls passed with their checked-in diagnostics: missing completion, unknown fair case, unknown predicate, duplicate fairness and duplicate property ID. Core+alloc runtime checks without model/Tokio passed (2 tests); temporal model checks passed (7 tests); macro nextest passed (9 tests).
5. Corpus, coverage, four exact positive/broken contract pairs, exporter and READMEs document bounds/assumptions. make test-yarn-model passed final strict Clippy, normal trybuild and feature checks; make test-licenses exited 0. The primary report names source mechanisms and commands.

`git diff --quiet 72940c72f56b755f07748bc5b0bfc98ddf801cc7 76de285870509035997e2b1c44aa78be09a173ae -- crates/bbt-yarn-macros/src/protocol.rs crates/bbt-yarn/tests/protocol_temporal.rs crates/bbt-yarn/examples/verify.rs` exited 0: the temporal implementation itself did not change on rebase. Main did change attribution inventories, project verification maps and the harness revision. Consequently the old receipt is **not** clean current-source evidence for the new complete tree.

The full committed-source make test was attempted once, as required, and stopped at layering before Rust CI, current-source formal verification, integration and smoke. Those checks on the new base, successful landing and deployment remain **unverified**. No separate geometry/boundary gate was relevant to this pure protocol change. Real external effects, persistent dedup storage/restart, larger/reused identities, per-instance fairness and wall-clock progress remain outside the finite contracts. Reports are append-only; the original report is retained. The containing signed Report trailer identifies this correction commit.
