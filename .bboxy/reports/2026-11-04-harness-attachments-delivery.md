---
type: report
title: Shared harness attachments and workspace file completion delivery
task: '[[01M6KKWJVF71S4JT0NNF193EWJ]]'
commits:
- 623fff70c62e7838d56cf10a90fff7c6b27a8a23
verify:
  command: make verify-harness-attachments
  exit_code: 0
author: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
created: 2026-11-04T16:42:13.366Z
id: 01M6KQ12BSYMNCR4D2SGGT4X66
updated: 2026-11-04T16:42:13.366Z
number: 93377961361059
created_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
updated_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
authorship:
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  kind: seat
  public_key: S3I9J2cN2HkGGxi3t4PZvVBpIllzItN0PkRADkjVqvs=
  process_generation: '1'
provenance:
  digest: sha256:da527d4437556fc94c60aa9617f678158c29608a34bace28aec1a5e37456b840
  frontmatter_digest: sha256:c6bcc675155a4fbd87a659ae3e38319ccab3406794e48a1cfebb7faebff72b08
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  sig: UPlCT+uHk0c3oOQpkBOaR2L5KF3QGM8Z1ofKrS259i5C0Puy3wyFm06vdKN4vQGlS2ORYgCqpS/XYwZrwQb2AA==
  log: .bboxy/log/2026-11-04.jsonl#121333
---
Implemented the scoped shared composer input path in producer commit `958f1d14b40c4b83e50cf405c58efd126b73c976` and consumer source checkpoint `623fff70c62e7838d56cf10a90fff7c6b27a8a23`, both signed by Boxy with key `9B07E849480C68328CA20684844ECC0587DE7AB0`. Worktree: `/Users/feral/mydev/bboxy450-term-composer`; both branches remain `codex-composer-20261104`. The parent gitlink is deliberately unstaged. Oracle owns publication, integration, pins and installation.

- Producer `src/frontends/gui/attachments.rs:55` — bounded, nonrecursive workspace directory lookup; canonical containment and hidden-path refusal; directories, spaces and Unicode remain literal paths.
- Producer `src/frontends/gui/attachments.rs:99` and `:151` — worker-side text snapshots and validated PNG decoding with immutable thumbnails. Composer metadata uses the existing attachment controller; submitted blocks carry actual contents.
- Producer `src/frontends/gui/input.rs:129`, `:302`, `:397` and `:494` — shared async ownership, keyboard/mouse file selection, removable previews, per-submission custody and nonblocking later drafts. Captured session/epoch, monotonically changing view birth and draft revision reject stale intake/completion, including A → B → A.
- Producer `src/store/intake.rs` — private same-uid 0700 intake directory under /tmp, unguessable 0600 files, no-follow regular-file reads, bounded length and SHA256 validation; normalization to session `InputBlock::Image` blobs before queueing. The directory is independent of GUI/seat TMPDIR differences. Binary bytes bypass text redaction; session blob persistence uses the existing durable writer.
- Producer `src/frontends/gui/bridge.rs:676` — actual input blocks and custody stay with pending delivery, including unknown/refused outcomes and disposable-view detach; no automatic resend. The injected unknown-outcome witness retains readable original image content.
- Producer `src/backend/acp_agent.rs:872` and `:1383` — image capability admission; unsupported images generate an explicit rejection and restore the draft without a downstream prompt. Supported images are base64 encoded from the actual stored blob into the existing real ACP prompt. Text-file content shares that same UserTurn.
- Producer `src/engine/mod.rs:436` and `:746`, plus ACP user-item publication — immutable initial operation causes reach durable user items, fixing pending-tail retirement without assigning an interrupt's cause to the original input.
- Consumer `crates/bboxy450-term/src/rich_pane.rs:42`, `:107` and `:182` — text/image clipboard snapshots, Tab and file intake enter the existing identity-fenced worker and the same HarnessView.
- Consumer `crates/bboxy450-term/src/rich_faces.rs:265`, `src/terminal_clipboard.rs:35`, `src/main.rs:8131` and `:9226` — both keyboard/menu paste routes, drops and worker-decoded thumbnail registration. Standalone window input reaches the identical intake implementation.
- Attribution notices, the focused Make targets, the two-model contract inventory and the exact signed task mirror are included. Tool/terminal/card composition, rich.rs, bbt-render and bbt-doc were untouched.

`make verify-harness-attachments` exited **0** unattended on the final source. Its raw logs are under `target/harness-attachments/`; the committed command/exit/log-hash/source-hash summary is `.bboxy/runs/codex-harness-attachments.json`. All 52 recorded source hashes were independently compared with both current files and their signed source-commit bytes. Producer manifest SHA256: `37e8bab60c429ea484a39bfdc5aa39aa36733cfaaef581f35a9bfa43276ef9e3`; consumer manifest SHA256: `c1e768c616cffcb8eae826aca1c39a636c38b7629bdbdf17660fd354d33041d0`. Each manifest hashes sorted, compact JSON of its repository-relative path → SHA256 map; the run JSON contains every individual digest.

| Focused command | Actual result |
|---|---|
| Producer nextest, standalone features, only harness_attachments | 6 passed; exit 0 |
| Existing causal pending contract and new unknown-custody Bridge test | 2 passed; exit 0 |
| Only attachments_intake_native_model_and_control | 1 passed; exit 0 |
| Producer strict Clippy, library and named tests, -D warnings | exit 0 |
| Embedded-only library compilation | exit 0 |
| Consumer private native worker/clipboard/drop/Unix Bridge witness | 1 passed; exit 0 |
| Existing named private macOS PNG/TIFF/text pasteboard witness | 1 passed; exit 0 |
| Consumer strict Clippy, library, app binary and named test, -D warnings | exit 0 |
| Source stability and committed-byte comparison | all 52 hashes match; exit 0 |
| Both repository diffs checked for whitespace errors | exit 0 |

The positive HarnessAttachmentIntake model explores 36 states / 66 transitions and passes native and pinned Java TLC (exit 0). BrokenHarnessAttachmentIntake explores 65 states / 116 transitions; both backends find the intended safety violation (TLC exit 12). Full reachable state and transition sets match for each. One captured completion, two draft revisions and three view births are finite abstractions; weak fairness of completion ensures settlement, including stale discard. Native filesystem, decoding, queue/ACP transport and custody are implementation witnesses, not an arbitrary Rust refinement proof. Existing pending/context models were reused; no full formal corpus was run.

| Acceptance criterion (verbatim) | Evidence |
|---|---|
| Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once | Producer tests/harness_attachments.rs:169 — real InProc and Unix Bridge paths to a private ACP child; decoded base64 equals the original PNG; one attached turn, one prompt; preview/remove and durable blob assertions. Consumer native worker witness also verifies stored original bytes. |
| Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference | Producer tests/harness_attachments.rs:169 — keyboard directory selection and mouse filename selection with spaces/Unicode; real file content arrives in the private child's same prompt. Workspace escape, hidden path and unavailable-file refusal are exercised. |
| Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context | Producer tests/harness_attachments.rs:169, :404 and :455; consumer tests/harness_attachments.rs:57 — shared HarnessView, busy-turn follow-ups, concurrent pastes, later independent messages and durable projection. The original launcher bootstrap appears once; attachments never turn into profile/launcher commands. Existing causal pending evidence is retained. |
| Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership | Producer delayed-intake/A-B-A, unsupported-image/missing-file and unavailable-Bridge cases; bridge.rs:1885 unknown-custody witness; positive model and broken control. Refused drafts restore, unknown outcomes retain blocks/custody and never automatically resend. |
| Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart | Focused Make exit 0, all command exits and 52 source hashes in the run JSON; actual private ACP child and private native pasteboard/worker checks; strict producer/consumer compilation. No operator clipboard read, OS click, paid request or backend action. |

Iteration found and fixed missing causal IDs in both actors. A native fixture then timed out despite a confirmed one-item transcript because it incorrectly expected a second silent-provider item; its final assertion checks the real durable user message and blob instead. The fixture-only Clippy annotation permits that private test's runtime/filesystem operations; production UI IO linting remains strict. An early sandbox local-socket denial was resolved with the private test execution allowance, without operator interaction.

Limits: PNG up to 16 MiB / 32 megapixels; native TIFF clipboard conversion; UTF-8 text files up to 64 KiB and combined JSON input under 200 KiB; eight attachments per draft, at most 64 outstanding local attachment deliveries. Listings examine at most 4096 entries and offer 64 paths. Binary non-PNG files, hidden paths and workspace escapes are explicitly refused. Unknown delivery is an injected Bridge outcome, not a live-provider timeout probe. Queue admission is not a durable user-message acknowledgement; custody is retained until that causal acknowledgement. No new crash/restart durability claim is made.

Skipped as instructed: full test/parity/formal/runtime corpora, paid/live provider probes, OS automation/captures, backend restart, other worktrees, dependency pin updates, merge, push and install. No operator credentials or raw logs/images were committed. Producer is clean; consumer retains only the intentionally unstaged nested gitlink after the record checkpoint. The final metadata checkpoint contains this indexed append-only report, the focused receipt and the latest exact signed task mirror; no source checks are rerun for those records.
