# Toolkit widgets: second rework report

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Task: `tasks/2026-10-31-toolkit-widgets.md` (#50), harness KB.
Executor: E2 toolkit-widgets, Boxy <bboxy450@gmail.com>.
Worktree: `/Users/feral/mydev/bboxy450-term-harness-widgets`; branch `harness-widgets`.
PR: https://github.com/bboxy450/bboxy450-term/pull/86 (unmerged review branch).

This supersedes the reports at b81d3c38 and 18ff123b. The second review accepted
the ledger model, package exclusion, syntect trim, OnceLock, coalesced wake model
and explicit prefix invalidation. This revision preserves those mechanisms and
fixes the remaining Markdown classifier, local diff recovery and ScrollStick
witness. A passing gate on an earlier revision did not establish these fixes.

Binding scope/flake disposition:
[d824](https://github.com/bboxy450/bboxy450-harness/blob/main/.bboxy/decisions/d824-e2-toolkit-widgets-scope.md).
It accepts the actual formal-model directory, Makefile and synchronized
ATTRIBUTION copies. AGENTS.md and the term KB schema remain unchanged.
`crates/bbt-ui/Cargo.toml` excludes `reports/`, so this document is not packaged.

The enclosing signed commit is the candidate. The exact-source full gate runs
**after** committing this document, with no RUST_TEST_THREADS override. Its
command, actual exit, source/result lines and formal receipt are handed off
through `task_report`; `target/ci-round2.log` retains the full output. This
pre-gate document does not predict a successful result. Raw logs stay outside Git.

## Second review: mechanisms and evidence

| Requirement | Source and mechanism | Witness |
| --- | --- | --- |
| Container/multiline definitions | `crates/bbt-doc/src/markdown/streaming.rs`, `definition_label` removes quote and bullet/ordered-list markers before classifying a leading label. `definitions` recognizes a colon after a closing label even across lines. An unfinished label retains its original scan offset and temporarily uses full parsing; a closed ordinary link resumes caching. Confirmed definitions retain document-wide reference context. | `crates/bbt-doc/tests/streaming.rs`, `quoted_definition_seed_resolves_a_retained_prefix` fails on 18ff123b and passes now. The every-split/random-append corpus includes `> [r]: u`, `- [r]: u`, `- [^f]: x` and `[foo\nbar]: u`; scalar mixtures include all four. A separate ambiguous-label test proves caching resumes after a nondefinition closes. |
| Local diff recovery | `crates/bbt-ui/src/modules/diff.rs`, `Diff::parse` retains malformed rows as Notes. Incomplete hunks get a diagnostic at the next boundary or EOF. Invalid hunk headers create a visible unnumbered region. File/hunk headers take precedence over erroneous counters, including bare/tab-separated headers; orphan notes remain visible. The Result signature is preserved for compatibility. | `crates/bbt-ui/tests/toolkit.rs` tests short hunks at another hunk and EOF, orphan no-newline notes, bad headers, and next-file headers after zero/short/long counts. Earlier blank-context, bad-row numbering, multifile and folding witnesses remain. Rendering has no expect/panic path. |
| ScrollStick claim/witness | `crates/bboxy450-term/tests/formal_contracts/toolkit_models.rs`, `scroll_stick` removes the vacuous response and jump marker. `verification/contracts.toml` makes no asynchronous jump liveness claim. The original safety controls still check retained manual intent and following projection. | `scroll_model_matches_the_real_endpoint_and_jump_policy` reconstructs the real controller from each reachable jump source, calls `jump_to_latest`, and compares its following flag and layout projection to the model successor. The implementation unit test checks private reset state. |

The Markdown classifier is conservative rather than a second Markdown parser.
Possible definitions can temporarily trigger full parsing, including a leading
open bracket inside a code block. It does not permanently disable caching for
ordinary inline links, task lists, JSON or indexing. Grammar and measured work
remain implementation coverage, explicitly outside the finite identity model.

## Red to green

New regression witnesses ran against the unchanged production code at signed
18ff123b before the fixes. `make test-toolkit-rework` exited **2**, with actual
assertion failures, not compiler failures. `/tmp/e2-round2-red.log` records:

- `quoted_definition_seed_resolves_a_retained_prefix`: the retained `[link][r]`
  stayed Text after `> [r]: u`, while full parsing produced a Link.
- The every-split equivalence test failed on that same quoted-definition corpus.
- `diff_short_hunk_recovers_at_the_next_hunk` and
  `diff_short_hunk_at_eof_stays_visible`: whole-diff truncation errors.
- `diff_orphan_note_and_bad_header_do_not_discard_valid_hunks`: orphan-note error.
- `diff_file_headers_win_over_incorrect_hunk_counts`: the next file's region
  incorrectly contained a preceding hunk row.

After the fixes, `make test-toolkit-rework` exited **0**:
**6 streaming tests and 15 UI tests** pass (`/tmp/e2-round2-green.log`).
The byte witness reparses **16,461 bytes versus 785,890** for repeated full
parsing, across 100 appends with inline links/task lists/indexing. **95/96**
scalar-append seeds reuse a prefix for most appends; only one eighth introduce
explicit definitions. Every append is compared with full parsing.

The first rework's required red-to-green evidence remains in
`/tmp/e2-rework-red-widgets.log` and `/tmp/e2-rework-red-models.log`, with green
`/tmp/e2-rework-test-toolkit-rework.log` and
`/tmp/e2-rework-test-toolkit-models.log`: removing the good StreamToView generation
guard now violates its ledger-origin invariant; the original Scroll model
retained stale manual intent; the original wake model lacked upstream wake/queue
state. These were real assertion failures against b81d3c38. The second review
specifically accepted the ledger, wake shape and invalidation action.

## Evidence for the nine original outcomes

| Item | Files and mechanism / evidence |
| --- | --- |
| 1. Streaming Markdown | `bbt-doc/src/markdown/streaming.rs` retains stable blocks and the unsettled tail; tests cover scalar/random/single splits, fences, tables, UTF-8, YAML and late definitions. Work bytes and incremental-path coverage are asserted above. |
| 2. Highlighting | `bbt-doc/src/highlight.rs` prepares owned spans on a guarded worker; SyntaxSet/ThemeSet share OnceLock resources. `bboxybook/src/documents.rs` uses Work::prepare, generation/revision fencing and AsyncRenderPreparation. Syntect enables exactly parsing/default-syntaxes/default-themes/regex-fancy. Pointer reuse, unknown syntax, exact byte coverage and UI-thread refusal have implementation witnesses. |
| 3. Transcript | `bbt-ui/src/modules/transcript.rs` accepts keyed prepared Message bodies; the host supplies bbt-doc body elements without a UI-to-doc cycle. StreamToView rejects duplicates/gaps/retired generations and A-B-A callbacks. |
| 4. Diff | `bbt-ui/src/modules/diff.rs` supplies independent hunk folding, line numbers and theme-token addition/removal colours, now with local malformed-input recovery. |
| 5. Tool card | `bbt-ui/src/modules/tool_call.rs` composes title/status and independent Accordion arguments/output. The controller test toggles only the requested disclosure. |
| 6. Approval | `bbt-ui/src/modules/approval.rs` handles enabled-choice keyboard selection; paint assertions show selection changes visible focus. |
| 7. Editor | `bbt-ui/src/text_edit.rs` and its modules support logical Up/Down, word/line motions, undo/redo, history/draft restoration and captured slash/mention completion. Unicode, stale draft and completion-caret tests exercise the real controller. |
| 8. Formal | `toolkit_models.rs` registers four good/Broken families through one append, with contracts/source mapping in verification. StreamToView uses an exactly-once current-generation applied ledger; IncrementalMarkdown explicitly classifies definition appends and invalidates; ScrollStick checks manual intent/following; DirtyWakeFrame extends upstream bits/pending/drain_pc with queue accounting, coalesced frames and budget-tail rearm. Its real DirtySet/ViewRing witness applies three deltas in two frames, while omitting complete_drain leaves the tail unwoken. |
| 9. Stories | `bboxybook/tests/toolkit.rs` renders 16 states × 4 densities × 2 themes = 128 headless stories. Assertions check folded lines disappear, syntax has multiple colours and manual transcript scrolling changes pixels. Approval focus has a separate real paint witness. |

## Previously completed checks retained by the second review

These are historical development results, not substitutes for the new exact-head
full gate. No RUST_TEST_THREADS override is needed by any new test.

| Command | Actual exit and evidence |
| --- | --- |
| `make test-toolkit` | 0; doc/UI unit/integration/doc tests and all 128 headless stories, `/tmp/e2-rework-test-toolkit.log` |
| `make check-toolkit` | 0; strict all-target Clippy, `/tmp/e2-rework-check-toolkit.log` |
| `make lock` | 0; lock diff removes base64 0.23.1, deranged, linked-hash-map, num-conv, plist, powerfmt, quick-xml 0.42, time, time-core, time-macros and yaml-rust. quick-xml 0.41 remains for wayland-scanner, outside syntect. `/tmp/e2-rework-lock.diff` |
| `make list-toolkit-package` | 0; Cargo package --list omits reports/, `/tmp/e2-rework-package-list-checked.log` |
| `make test-licenses` | 0; synchronized notices and license fixtures, `/tmp/e2-rework-test-licenses.log` |
| `make parity-documents` | 0; 168/168 required geometry cases, no unproven/mismatched cases; 16 advisory pixel reviews. `/tmp/e2-rework-parity-documents.log` |

The package-list and model checks are refreshed for this revision; exact commands
and exits accompany the final task report. Native/TLC parity and every full-gate
layer are refreshed on the enclosing signed source. The 18ff123b full green result
used RUST_TEST_THREADS=1 after an issue-85 failure; that serial retry is historical
and is not offered as evidence for this review's default-concurrency requirement.

## Bounds, gaps and release follow-up

Formal bounds, atomicity and fairness are in `verification/contracts.toml`.
DirtyWakeFrame admits three deltas with a two-delta frame budget and weakly fair
wake/drain/frame service. Weak memory, wall-clock timing and unbounded streams
remain outside these models. Finite native/TLC parity is neither arbitrary Rust
refinement nor complete TLA+ semantics proof.

Remaining SHOULD: approval Escape cancellation, descriptions and separate
allow/deny tones. The four requested semantic story/paint checks are implemented.
Oracle-deferred follow-ups remain keyed/virtualized transcript reuse, Plan/Image
widgets, ANSI handling/tail truncation and intra-line diff. Downstream harness
frame/history budgets and independent browser captures for the new controls are
not claimed. Normal-policy night/slow checks remain skipped. There is no new
process boundary requiring a separate boundary suite.

Issue #85's two mux flakes have the d824 disposition. No mux code, assertions,
quarantine or concurrency setting changes here. Any repeated failure is reported
with its actual exit; it is never relabeled as green.

The original attribution diff synchronizes 31 existing copies because
`scripts/license-packages.py` COPIES requires byte identity with the root syntect
credit. D824 and the second review explicitly accept these copies, including
plugins/gitkb/ATTRIBUTION.md.

For 0.1.2, bump bboxy450-ui and bboxy450-doc for the new APIs, and bboxy450-book
for its gallery/CLI and dependency floors. RT/render/Yarn have no new runtime API
here. Editor enum additions require downstream exhaustive matches to update.
No version bump, publication, merge or release tag is performed.

Project policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
