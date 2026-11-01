# Toolkit widgets implementation and rework report

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Task: `tasks/2026-10-31-toolkit-widgets.md` (#50) in the harness KB.
Executor: E2 toolkit-widgets, Boxy <bboxy450@gmail.com>.
Worktree: `/Users/feral/mydev/bboxy450-term-harness-widgets`; branch `harness-widgets`.
PR: https://github.com/bboxy450/bboxy450-term/pull/86 (review branch only).

This report supersedes the rejected implementation at signed `b81d3c38`.
That commit's full gate passed, but source review found an overly broad parser
fallback and models that did not establish the intended properties. A green
check did not resolve those defects. The rework below changes both implementation
and the verification contracts, with failing controls against the original code.

The binding scope amendment is harness decision
[d824](https://github.com/bboxy450/bboxy450-harness/blob/main/.bboxy/decisions/d824-e2-toolkit-widgets-scope.md).
It authorizes the real formal-model directory, Makefile workflows and synchronized
attribution copies. The task's recorded rejection supplies the rework criteria.
AGENTS.md and the term KB schema are untouched. This is ordinary repository
documentation, excluded from the UI crate package by `exclude = ["reports/"]`.

The enclosing signed Git revision is the handoff candidate. The final full gate
runs after this document is committed: `target/ci-ready.log` records its exact
`ci-local: source` and `ci-local: result` lines; `task_report` records that result
and the fresh formal receipt. Results below are actual completed development
checks, not a predicted result for a future commit. Raw logs remain outside Git.

## Rework acceptance evidence

| MUST | Files and mechanism | Witness / result |
| --- | --- | --- |
| 1. Narrow Markdown fallback | `crates/bbt-doc/src/markdown/streaming.rs:23` classifies only definition lines matching `^\s{0,3}\[[^\]]+\]:`; the scanner retains the unfinished-line offset and never scans the stable prefix again. Only definition documents keep full-parser reference context. Inline links, task lists, JSON and indexing retain the prefix. | `crates/bbt-doc/tests/streaming.rs:137` counts bytes across 100 appends: **16,461 reparsed versus 785,890 full-parse bytes**. `:76` keeps full-parse equivalence at every scalar append, with only one eighth of seeds adding definitions; **96/96 seeds** use prefix reuse for most appends. Dedicated every-split fence/table/late-definition corpus remains. |
| 2. Stream ledger | `crates/bboxy450-term/tests/formal_contracts/toolkit_models.rs:11` tracks each applied sequence count and its captured origin. Invariants require exactly one entry per sequence below `next`, none at/above it, current-generation origins and an empty dropped view. No `bad` flag exists. | `:387` removes the generation guard from the good model and requires the specific `AppliedOriginsBelongToCurrentGeneration` violation. Broken also bypasses order/identity checks, corrupting actual ledger fields. The real API witness remains `bbt-ui/tests/toolkit.rs:17`. |
| 3. Package exclusion | `crates/bbt-ui/Cargo.toml` excludes `reports/`; the document remains committed without shipping local diagnostic paths in a crate. | `make list-toolkit-package` exited **0**. It runs `cargo package --list --allow-dirty --locked -p bboxy450-ui` with the local target, retains `target/toolkit-package-list.txt`, and fails if any `reports/` path occurs. |
| 4. Syntect feature surface | `crates/bbt-doc/Cargo.toml` enables only `parsing`, `default-syntaxes`, `default-themes`, `regex-fancy`. | `make lock` exited **0**. Lock diff removes eleven packages: base64 0.23.1, deranged, linked-hash-map, num-conv, plist, powerfmt, quick-xml 0.42, time, time-core, time-macros, yaml-rust. Existing quick-xml 0.41 remains for wayland-scanner, outside syntect's dependency chain. `Cargo.lock` is the reviewable diff; `/tmp/e2-rework-lock.diff` retains the command view. |
| 5. Resource reuse | `crates/bbt-doc/src/highlight.rs:33` holds SyntaxSet and ThemeSet in a static OnceLock, initialized after the worker-thread guard. Each highlighter borrows these same resources. | `highlight.rs:87` checks pointer identity across dark/light highlighters; owned-worker spans, unknown language bytes, UI-thread refusal and late-generation tests pass in `make test-toolkit`. |
| 6. Diff robustness | `crates/bbt-ui/src/modules/diff.rs:40` accepts empty context and degrades one damaged line to a visible Note while advancing the remaining side counters. `DiffView::element` handles measurement failure without expect/panic. Header/range/truncation failures remain explicit. | `bbt-ui/tests/toolkit.rs:124` checks blank context, preservation of a damaged raw line and correct subsequent numbering. All diff/open/fold stories render headless. |
| 7. Scroll agreement | `toolkit_models.rs:155` now re-sticks at `offset >= maximum`; jump resets requested/manual intent to zero, matching `transcript.rs:88`. The model includes `jump ~> following`; restoration is atomic, needing no fairness assumption. README describes endpoint and reset semantics. | `toolkit_models.rs:405` compares reachable endpoint/jump edges with the actual API; `transcript.rs:103` checks actual reset state. Existing growth/shrink/nonfinite tests and visible transcript-scroll story assertion pass. |
| 8. Coalesced frame wake | `toolkit_models.rs:233` extends upstream DirtyWake's bits/pending/producer_pc/drain_pc with a queue, admitted/consumed ledger, coalesced frame-ready bit and budget-tail rearm. A frame can consume two deltas, so arrivals need not each allocate a frame. | `bbt-ui/tests/toolkit.rs:38` applies new StreamToView mutations through actual DirtySet/ViewRing APIs: three deltas, two frames. Omitting complete_drain is the real negative control: one delta remains without a wake. Broken model omits rearm and fails liveness; `toolkit_models.rs:437` requires upstream wake fields and a two-delta frame transition. |
| 9. Definition invalidation | `toolkit_models.rs:96` distinguishes ordinary/inline-bracket appends from definition-line appends. A definition changes semantic identity and explicitly invokes invalidation before returning; Broken skips it. | Native good passes and Broken has a stale returned cache. Contracts record internal classification/invalidation as one externally atomic append and map the full-parse equivalence and work-range tests. |

## Red to green

Red commands ran against the original production/model implementation at
`b81d3c38` with the new regression witnesses added. Failures were assertions,
not compilation failures. Make exits are recorded separately from Cargo's 101.

| Required item | Red command / actual result | Green command / actual result |
| --- | --- | --- |
| 1 | `make test-toolkit-rework` **2**: append 1 reparsed `0..308`, losing the stable prefix | Same target **0**; 16,461/785,890 byte measurement and scalar equivalence/coverage assertions pass |
| 2 | `make test-toolkit-models` **2**: deleting the good model's generation guard produced no violation | Same target **0**; the specific ledger-origin invariant detects the deletion |
| 6 | `make test-toolkit-rework` **2**: empty context returned `unprefixed hunk line` | Same target **0**; empty context and a damaged line preserve the rest of the hunk |
| 7 | `make test-toolkit-models` **2**: model jump retained nonzero manual intent | Same target **0**; endpoint/jump edges agree with Rust, and jump liveness is registered |
| 8 | `make test-toolkit-models` **2**: upstream wake/queue state was absent | Same target **0**; upstream state and coalescing transition exist; the new real API witness passes and its omitted-rearm control loses the tail |

Red logs: `/tmp/e2-rework-red-widgets.log`, `/tmp/e2-rework-red-models.log`.
Green logs: `/tmp/e2-rework-test-toolkit-rework.log`,
`/tmp/e2-rework-test-toolkit-models.log`. The earlier
`/tmp/e2-rework-green-widgets.log` and `/tmp/e2-rework-green-models.log` also
record the first green pass; the named test-toolkit logs include the stronger
coverage and ledger-origin assertions added afterward.

After the rework gate passed on signed `6ee43422`, GitHub reported a conflict
with newer main `2e7a6791`. Read-only merge-tree inspection located only the
adjacent module declarations and cases.extend calls in formal_contracts/models.rs.
The follow-up commit moves this workstream's declaration and single append away
from those shared insertion points, retaining the same model code and families
array. No merge, rebase or history rewrite is performed. The final full gate is
rerun on that signed handoff commit rather than treating the parent result as a
new result. Parent evidence: `/tmp/e2-rework-ci-6ee43422.log`, normal exit 0;
formal receipt SHA-256
`b2c34d3109d3cb2f494d7e3519845ac43e82a89d3bad191628711125b43407d6`.

## Completed commands

| Command | Exit / evidence |
| --- | --- |
| `make test-toolkit-rework` | **0**: four parser properties and eleven UI controller/paint/queue tests; byte and seed counters retained in `/tmp/e2-rework-test-toolkit-rework.log` |
| `make test-toolkit-models` | **0**: four independent model/code checks, including every positive/broken verdict; `/tmp/e2-rework-test-toolkit-models.log` |
| `make test-toolkit` | **0**: doc/UI unit, integration and doc tests; all 128 headless Metal theme/density stories; `/tmp/e2-rework-test-toolkit.log` |
| `make check-toolkit` | **0**: strict all-target toolkit Clippy; `/tmp/e2-rework-check-toolkit.log` |
| `make lock` | **0**: eleven dependencies removed; `/tmp/e2-rework-lock.log` and Cargo.lock diff |
| `make list-toolkit-package` | **0**: no reports path; `/tmp/e2-rework-package-list-checked.log` and `target/toolkit-package-list.txt` |
| `make test-licenses` | **0**: synchronized notices and package/license fixtures; `/tmp/e2-rework-test-licenses.log` |
| `make parity-documents` | **0**: 168/168 required geometry cases measured, 0 unproven, 0 mismatches; 16 advisory pixel reviews. `/tmp/e2-rework-parity-documents.log`, `target/parity-documents/index.html` |

Native model graphs from the focused rework check (full native/TLC parity is
refreshed by the final exact-source gate):

| Model | Good states / edges | Broken states / edges | Broken verdict |
| --- | --- | --- | --- |
| StreamToView | 104 / 285 | 704 / 2164 | safety |
| IncrementalMarkdown | 33 / 45 | 48 / 63 | safety |
| ScrollStick | 32 / 288 | 38 / 342 | safety |
| DirtyWakeFrame | 142 / 245 | 129 / 212 | liveness |

## Original widget outcomes retained

- Streaming Markdown and syntax spans are in bboxy450-doc; worker preparation
  fences generation/revision and layout consumes owned prepared spans.
- `modules/transcript.rs` accepts keyed prepared Markdown body elements without
  introducing a UI-to-doc dependency cycle. StreamToView rejects retired,
  duplicate and out-of-order effects; gaps require host retry/snapshot repair.
- `modules/diff.rs` supplies unified line numbers, themed additions/removals and
  independent hunk folding. `modules/tool_call.rs` supplies status/title and
  independent Accordion disclosures for arguments/output.
- `modules/approval.rs` supplies keyboard enabled-choice selection. Editor
  logical vertical/word/line motions, undo/redo, history/draft restoration and
  captured slash/mention completion hooks remain covered in `tests/toolkit.rs`.
- All four formal contracts remain registered in corpus/contracts/coverage.
  Models remain in their own module, appended through one cases.extend call.
- Sixteen story states across four densities and two themes still render headless.
  Story assertions now prove folded lines disappear while the other hunk remains,
  rendered code has multiple colours, and manual transcript scroll changes pixels.
  The UI paint witness proves moving approval selection changes painted focus.

## Bounds, remaining gaps and disposition

Definition-bearing documents deliberately use full-parser reference context;
only matching definition lines activate that path. The finite corpus and
random mixtures do not prove all Markdown grammar. Prefix/tail work is measured
in bytes; no downstream harness p99 frame budget is claimed.

The formal abstractions state owners, atomic steps, finite bounds and fairness
in verification/contracts.toml. DirtyWakeFrame has at most three arrivals and
fair posted-wake/drain/frame servicing, with weak memory and wall-clock deadlines
outside the model. Finite native/TLC parity is not arbitrary Rust refinement or
complete TLA+ semantics proof. Real API witnesses bridge the stated abstractions.

Remaining SHOULD: approval Escape cancellation, option descriptions and distinct
allow/deny tones. These add option/event API and state/paint behavior beyond the
current prompt contract. The four requested semantic story/paint checks are done.
Keyed/virtualized transcript reuse, Plan/Image widgets, ANSI/tail truncation and
intra-line diff remain explicitly deferred by the Oracle's rework brief.

Independent browser reference captures for the new controls and downstream
history windowing/frame-budget measurements remain outside this patch. Existing
normal-policy night/slow tests remain skipped by the canonical gate. No new
process boundary is implemented, so no extra boundary or live-provider claim is
made. All tests run with private unattended fixtures, without operator prompts.

The two mux flakes are tracked in issue #85 and dispositioned by d824. No mux
runtime/test assertion or quarantine is changed here. The prior b81 full run
passed both. Only a repeated #85 failure may use that disposition; the final
rework gate's actual outcome is recorded separately, never relabeled as green.

The original attribution diff updates 31 existing copies: root syntect credit
must be copied byte-for-byte under scripts/license-packages.py COPIES. D824
accepts those paths. No additional policy copy change is needed for feature
removal; origin/license notices stay attached.

## Release follow-up

bboxy450-ui and bboxy450-doc need the 0.1.2 API releases, with the corresponding
bboxy450-book update for its gallery/CLI and dependency floors. RT/render/Yarn
have no new runtime API in this rework. Public editor enum additions still
require downstream exhaustive matches to be updated. No version, crate publish,
main merge or release tag is performed by this Executor.

Project policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
