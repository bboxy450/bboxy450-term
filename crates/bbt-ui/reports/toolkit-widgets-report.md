# Toolkit widgets implementation report

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Task: `tasks/2026-10-31-toolkit-widgets.md` in the harness KB.
Executor: E2 toolkit-widgets, Boxy <bboxy450@gmail.com>.
Worktree: `/Users/feral/mydev/bboxy450-term-harness-widgets`; branch `harness-widgets`.
Verification snapshot before registry-only cleanup and rebase:
`91265cdebeb5b5cfb98616ca70e49e9605aa39a2`, signed with
`9B07E849480C68328CA20684844ECC0587DE7AB0` (verified `G`).

This is ordinary repository documentation. At the task's base `302e61ce`, the
KB has no `report` type or reports directory, decision 13 explicitly rejects
that type, and the running MCP schema has no report variant and is scoped to
the harness KB. No KB record creation or numbering is claimed. This committed
document supplies the full report without bypassing that unavailable operation.

The binding scope amendment is harness decision
[d824](https://github.com/bboxy450/bboxy450-harness/blob/main/.bboxy/decisions/d824-e2-toolkit-widgets-scope.md).
It permits the real formal-model directory, Makefile workflows, root attribution
and synchronized policy copies. It also records the disposition of the mux
failures below. AGENTS.md and the KB schema are untouched.

The report's enclosing Git revision is the final handoff candidate. The final
exact-source gate is retained in `target/ci-local.log`; its `ci-local: source`
and `ci-local: result` lines, plus the task report, identify the final tested
head and result. This document records completed checks on the snapshot above;
it does not manufacture a result for a gate that runs after its commit.

## Files and acceptance evidence

| Item | Files and mechanism | Implementation witness |
| --- | --- | --- |
| 1. Streaming Markdown | `crates/bbt-doc/src/markdown/streaming.rs:29` retains a stable block prefix and reparses the open suffix through `parse::body`, preserving absolute source ranges. An incomplete next line keeps its predecessor in the tail, including partial table rows. | `crates/bbt-doc/tests/streaming.rs`: every single split, seeded random multi-append partitions, and generated block mixtures compared with a full parse after each append; a work-range assertion proves prefix reuse. Fences, tables, indentation, lists, quotes, HTML, UTF-8, BOM/CRLF, YAML and late references occur in the corpus. |
| 2. Syntax spans | `crates/bbt-doc/src/highlight.rs:140` exposes `Highlighter`, validates exact span text coverage and binds prepared spans to offset/language/source. Syntect 5.3 uses `default-fancy` with its pure Rust regex backend. `Work::prepare_off_thread` schedules blocking preparation; `Preparation` fences generation/revision and retains the old prepared body until replacement is ready. `markdown/layout.rs:221` consumes prepared text runs. | `crates/bbt-doc/tests/highlight.rs`: real worker preparation, multi-colour Rust spans, unknown language byte preservation, UI-thread guard and late/replaced/A-B-A completion retirement. Dependency licence gate held. |
| 3. Markdown transcript | `crates/bbt-ui/src/modules/transcript.rs:24` composes independently keyed, prepared message bodies. The host supplies bbt-doc body elements, maintaining the existing doc → UI layering. | `crates/bboxybook/src/toolkit.rs` transcript stories prepare Markdown per message and render follow/manual positions headless. |
| 4. Diff view | `crates/bbt-ui/src/modules/diff.rs:40` parses unified hunks with old/new line counters, zero-count ranges and per-file header ordering. `Folds` stores independent hunk state; named header regions expose toggles. Added/removed lines use `TOKENS.color.ui.ok` and `TOKENS.color.state.errored`. | `crates/bbt-ui/tests/toolkit.rs`: number/count checks, malformed/truncated input refusal, independent folds and multi-file header ownership. Open/folded headless stories. |
| 5. Tool card | `crates/bbt-ui/src/modules/tool_call.rs:63` composes a status chip/title and two Accordion disclosures. Stable argument/output action regions toggle only their section. | Independent toggle witness in `bbt-ui/tests/toolkit.rs`; pending, running, succeeded, failed, cancelled and folded stories. |
| 6. Approval | `crates/bbt-ui/src/modules/approval.rs:50` composes shared Buttons. Up/Down wrap over enabled options; Home/End choose endpoints; Enter returns only an enabled host action. | `approval_skips_disabled_wraps_and_never_chooses_without_enter`; enabled/disabled headless stories. |
| 7. Editor | `crates/bbt-ui/src/text_edit.rs:130` adds logical vertical motion with a retained UTF-16 column, word/line motions, bounded undo/redo, submitted-input history with draft restoration, captured completion requests and a keyboard choice popover. History storage is boxed to keep existing app overlay enums compact. IME preedit remains separate. | `bbt-ui/tests/toolkit.rs`: Unicode vertical/word/line motion, undo/redo branching, history draft restoration, both triggers and stale completion refusal. Existing editor/composer/IME tests remain in the canonical suite. |
| 8. Formal contracts | `crates/bboxy450-term/tests/formal_contracts/toolkit_models.rs:11` adds StreamToView, IncrementalMarkdown, ScrollStick and DirtyWakeFrame, each with a broken control. All are registered in `verification/{corpus,contracts,coverage}.toml`. Existing AsyncRenderPreparation is extended to cover owned Markdown preparation. DirtyWake's wake discipline and RenderLinePlan's identity guard are reused as contract shapes. | StreamToView/ScrollStick real controller tests; full-parse property tests; existing `bbt-rt/tests/runtime.rs::a_budgeted_deferred_drain_rearms_its_tail_without_another_producer`. Native/TLC graph and verdict results are in the formal receipt below. |
| 9. Stories | `crates/bboxybook/src/toolkit.rs:49` registers 16 states × four densities × two themes, with a `toolkit` command and selectable manifest. Rendering uses the existing retained material and `parity::replay_images` path. | `crates/bboxybook/tests/toolkit.rs:10` renders all 128 cases headless on Metal, asserts text/nonblank output and expected dimensions. `make stories-toolkit` produced 128 native PNGs. |

`Makefile` adds `test-toolkit`, `check-toolkit`, `stories-toolkit` and a signed,
exact-source `push-branch` gate for the assigned PR handoff. No main merge or
crate publication is performed. Root `ATTRIBUTION.md` records the dependency
and all 31 crate policy copies were synchronized with `make sync-licenses`.
Every new Rust file carries the project, governing licence and policy URL.

## Commands and results

All commands ran in the assigned worktree, unattended.

| Command / layer | Actual exit / result | Retained evidence |
| --- | --- | --- |
| `make test` (first candidate `2bcfc9e5`) | 2; editor-size Clippy failure, fixed by boxing history | `/tmp/e2-toolkit-ci-initial.log` |
| `make test` (`91265cde`) | 2; only `capture_fairness` failed; all toolkit/property/story tests passed | `/tmp/e2-toolkit-ci-code-91265cde-unit-failure.log`, lines 2550–2579 |
| Workspace build and all-target `-D warnings` Clippy within that run | 0 for each layer | The same CI log's build/Clippy completion and subsequent unit-test invocation |
| `make test-seat-ingestion` | 2; watched replay hit its page budget, before the final fairness target | `/tmp/e2-toolkit-ingestion.log`; d824 / issue #85 disposition |
| `make test-formal` | 0; 106 models, 31 crates, 46 contracts; native/TLC graphs and verdicts match, all broken controls rejected | `/tmp/e2-toolkit-formal.log`; receipt SHA-256 `82de6ac91ca8f343a0581583a66a494e68d78c9e8f5b489d4a04bcc420e0a47c` |
| `make smoke` | 0; unsigned private install, headless frame and ten-second zero-frame idle probe, orderly child reaping | `/tmp/e2-toolkit-smoke.log` |
| `make test-licenses` | 0; 31 policy copies and source notices held, negative fixtures passed | `/tmp/e2-toolkit-license-tests.log` |
| `make parity-documents` | 0; 168/168 required geometry cases measured, 0 unproven, 0 mismatches; 16 advisory pixel reviews | `/tmp/e2-toolkit-parity.log`, `target/parity-documents/index.html` |
| `make test-toolkit` (development snapshot before boxed storage) | 0; existing doc/UI suites, new properties/controllers and all 128 headless stories | `/tmp/e2-toolkit-test.log`; the later exact-source workspace run independently passed those tests at `91265cde` |
| `make stories-toolkit` | 0; 128 PNGs rendered on Apple M4 Metal | `/tmp/e2-toolkit-stories.log`, `target/bboxybook/toolkit/toolkit/` |
| `make test CI_ARGS=--after-tests CI_LOG=target/ci-post-tests.log` | 0; formal, UI lint, private headless smoke and invariants; 27 checks passed, 11 existing slow checks skipped | `/tmp/e2-toolkit-post-tests-91265cde.log`; exact `after-tests 0` result line |
| `sh scripts/deps-licences.sh` | 0; permissive dependency gate held | `/tmp/e2-toolkit-licenses.log`; also passed in the post-test invariant gate |

New model details from the native/TLC receipt:

| Model | Positive graph (states / edges) | Broken graph (states / edges) | Broken verdict |
| --- | --- | --- | --- |
| StreamToView | 104 / 285 | 161 / 425 | safety |
| IncrementalMarkdown | 21 / 45 | 27 / 51 | safety |
| ScrollStick | 32 / 288 | 92 / 828 | safety |
| DirtyWakeFrame | 10 / 12 | 13 / 13 | liveness |

The four families are registered through `toolkit_models::cases()` and one
`cases.extend` call, leaving the shared families array unchanged, as required by
upstream LAND.md. This changes registration placement, not the model actions.
After rebasing, formal evidence must be refreshed; an old receipt never proves a
new commit.

Raw logs, image captures and private test accounts stay outside Git. The report
records commands, exits and source identities; these are evidence pointers for
the Oracle to inspect, not an acceptance decision.

## Failures resolved during implementation

- The append equivalence test exposed a partial table row temporarily parsed as
  a paragraph. Retaining the preceding block until the next line settles fixes
  that counterexample; the generated/split corpus passes.
- The first full CI attempt at `2bcfc9e5` exited 2: strict Clippy found that inline
  editor history enlarged the app's `Surface` enum (336-byte largest variant).
  The editor now boxes that storage; the amended candidate's workspace build
  and all-target strict Clippy pass. The initial log is
  `/tmp/e2-toolkit-ci-initial.log`.

## Ingestion failure disposition and attribution scope

The failed assertions are `crates/bbt-mux/tests/capture_fairness.rs:352`
(`LogicTurn.work > 1`, after exact serial-screen equality succeeded) and
`crates/bbt-mux/tests/common/capture_priority.rs:385` (watched tail page budget).
The patch changes neither those tests nor ingestion/supervisor runtime sources.

In d824 the Oracle reports independent runs on both main `6ff474a1` and this
snapshot: capture_fairness passed 3/3 and `make test-seat-ingestion` exited 0,
at load averages 40–64. The failures are classified as flakes under load and
tracked in [issue #85](https://github.com/bboxy450/bboxy450-term/issues/85).
The Oracle instructed a fresh full run; only these two known failures may use
that disposition. No assertion, quarantine entry or ingestion code was weakened.
The watched replay check also passed in the post-test invariant run (0.49 s).

The attribution diff **modifies 31 existing files**; it does not add 31 new
policies. The standing source-credit instruction requires the root policy update
for the new dependency. `scripts/license-packages.py:17` lists ATTRIBUTION.md in
`COPIES`, and `:168` rejects any member copy differing from the canonical bytes.
`make sync-licenses` performs that synchronization; `make test-licenses` exits 0.
D824 explicitly accepts these required paths after the Oracle inspected the diff.

## Bounds and gaps

- Brackets conservatively disable prefix reuse because late reference/footnote
  definitions can change earlier blocks. Correctness remains full-parse
  equivalent; this is an explicit performance fallback.
- Sequence gaps are explicit refusals for host retry or snapshot repair; the
  controller does not buffer unbounded out-of-order deltas. Hosts retain the
  controller across replacement and retire generations on drop.
- Transcript bodies and long history windowing are host-owned; completed bodies
  should be retained. No downstream harness integration or p99 frame-budget
  result is claimed by this upstream patch.
- New control captures prove native renderability. Independent browser reference
  captures for the new controls are not supplied. Existing document parity
  covers 168 required geometry cases with zero mismatches; 16 pixel reviews are
  advisory, not pixel identity proof.
- Existing night-only process/real-provider tests remain ignored under the
  repository's normal test policy. No new process boundary is implemented;
  `make test-boundaries` and live-provider smoke are not additional claims.
- `Motion` and `Command` gain public enum variants. Downstream exhaustive
  matches need updating; the release owner should account for that compatibility
  change when choosing the final published version.
- Finite model parity does not prove arbitrary Rust refinement, parser grammar,
  OS fairness or wall-clock deadlines. The inventories spell out the bounds,
  atomic actions, fairness and implementation-only coverage.

## Release follow-up

`bboxy450-ui` and `bboxy450-doc` need 0.1.2 releases for these APIs.
`bboxy450-book` should receive the corresponding 0.1.2 update for the new
stories/CLI and consume the new UI/doc versions. RT, render and Yarn have no new
runtime APIs in this change. Version/dependency-floor updates and publication
remain with the release brief; no package version or release was published here.

Credit requests and retained upstream terms:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
