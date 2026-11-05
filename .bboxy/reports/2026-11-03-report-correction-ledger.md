---
type: report
title: Hash-pinned report correction ledger and verified startup input integration
task: '[[tasks/2026-11-03-report-correction-ledger]]'
commits:
- c7490106f917d8192958635c29c94a62970a7fba
- 4ed3c7cd95fae12e356c739103a1f862226fc272
- 2ab02240fabd40389ae29ec45b9ab9048ca491c7
- 633e91695af7a6dcc6aba6d23cd89ef62b2b9a0c
verify:
  command: make test
  exit_code: 0
author: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
created: 2026-11-03T22:10:15.377Z
id: 01M6HQD05VC2Q6GXJEA1TEP771
updated: 2026-11-03T22:10:15.377Z
number: 19018
created_by: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
updated_by: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
authorship:
  signer: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
  kind: seat
  public_key: 1miQQnU8JxLi0wuA/LnWSq6+99HIgl7nkp6qGDP/8cQ=
  process_generation: '1'
provenance:
  digest: sha256:29afbf27a1ac63e5f1b8def629f6b07521ccefb69c6e68c86f83e40db8ab1dba
  frontmatter_digest: sha256:6a4f6a4dfba6a8ae94d86e80eb7a06d9e8720fcdaf8892b94fc0ea512ee93534
  signer: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
  sig: qsi5HakBbcDrJN08hhZUuHWM2CWVyG7SFCTmjf9GEksvsdchODpgVV3h52kpj9zKXLl/XRMjAD7LcdetdriXAA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#105178
---
# Hash-pinned immutable-report corrections and integrated startup input

Assignment: [[tasks/2026-11-03-report-correction-ledger]] (#16394), including the signed integration/upstream amendments. Worktree: `/Users/feral/mydev/bboxy450-term-report-ledger`, branch `report-ledger`.

The report migration permits only an exact, newly appended correction in the same commit as the report modification. Native report writers retain their immutable publication behavior. The first correction repairs only the historical startup-modal report; its original evidence and waiver body remains byte-for-byte intact.

## Implementation and acceptance evidence

- `crates/bbt-doc/tests/corpus/corrections.rs:38` parses committed TOML with Python's existing Make/landing stdlib dependency and deserializes an exact field schema. Missing/unknown fields, malformed TOML, noncanonical paths, invalid/full-zero blob IDs, unchanged blob pairs, blank reasons, invalid decision locators and reused path/from/to triplets fail. No new Cargo dependency or product-runtime parser was added.
- `corrections.rs:105` examines each affected commit's raw, unabridged Git diff against its actual first parent (or the empty root), with rename detection disabled so a rename exposes a forbidden deletion. It reads both ledger snapshots from Git blobs. Symlink/executable ledgers fail.
- `corrections.rs:127` requires the old ledger bytes and parsed entries to be an unchanged prefix. Only appended entries can authorize modifications. `:149` matches each modified report's actual path, parent blob and commit blob to exactly one appended entry, refuses deletions/renames/type changes, and requires every appended entry to be consumed by a same-commit modification. `:199` resolves each new decision to a decision in that same committed tree.
- `crates/bbt-doc/tests/corpus/reports.rs:25` keeps metadata and task/supersedes/trailer validation, checks report paths even if their type was changed, and checks report ID/frontmatter/positive unique number in historical trees. `:163` audits every commit in the supplied range, including report/ledger changes without a Report trailer. Working files cannot repair earlier bad history. Existing pinned signature checks are unchanged.
- `crates/bbt-doc/tests/corpus/correction_fixtures.rs:95` proves consecutive exact corrections and working-tree independence. `:112`, `:146`, `:190`, `:230`, and `:261` exercise unlisted edits, wrong source/destination blobs, deletion/rename, ledger mutation/removal, old-entry/duplicate reuse, malformed/missing/extra fields, unused/late entries, missing/non-decision targets, invalid report metadata without a trailer, historical failures hidden by a later valid tip, and A -> B -> A reuse.
- `.bboxy/reports/CORRECTIONS.toml:7` binds the sole legacy correction: `.bboxy/reports/2026-11-03-startup-modal.md`, from `b09c0677ea9964b927303fa4f02592ed2d20f6e8` to `798669285e8bf927548b249450f9ff58a1d99b7b`, under [[decisions/d16393-hash-pinned-report-corrections]]. Immediately before its initial signed commit, a read-only scan examined 17,985 current record numbers across 56 worktrees; the highest was 93377961359198 and the correction received 93377961359199. It does not reuse 5002227 or the known conflicting 5002228 drafts.
- `.bboxy/reports/2026-11-03-startup-modal.md:8` identifies this correcting seat as correction author, explicitly retains original Boxy machine-key attribution, and states that the original seat is unproven. Its commits contain signed `1affb9865d75d775a9a5f306c043da8a05bf8f7c`. The original body is retained verbatim, with an appended correction explanation; this is historical metadata repair, not fresh acceptance of that historical implementation or its waived checks.
- `.bboxy/tasks/2026-11-03-startup-modal-operator-brief.md` is the Oracle/tool-created #16564 record. Its body contains the exact original `.bboxy-brief-startup-modal.md` bytes and preserves original implementation evidence and waiver. The later task amendment explicitly requires preserving its historical done/reconciliation fields; they were copied unchanged. The assigned task, decision and Oracle handoff are also copied unchanged from their signed host records at the cited amendments.
- Source notices retain the project's canonical origin and absolute attribution policy. The existing root ATTRIBUTION.md and product publication permissions were not broadened.

## Combined source and upstream integration

The original signed implementation was `41d74881ca09d7adc71f183be88750eb35e027da`. After the required rebase onto upstream `6afd6314`, its source-equivalent ledger implementation was `9b6b0ebd38aaebd752a2baa209f07ae08a427e1d`. After the later mandatory refresh onto upstream `6951376d`, the ledger is `4ed3c7cd95fae12e356c739103a1f862226fc272` and the inherited layering fix is `c7490106f917d8192958635c29c94a62970a7fba`. The redundant inherited HarnessTextZoom inventory commit was dropped after upstream `3eafd2be` supplied the inventory; no duplicate model names remain.

Task #16417's approved signed source commit `7eb6ead826ff104acf52f1f9bc383e4575ec848b` was read, signature-verified and cherry-picked as `7a6dc40e5e510fbc2b335cfc22a1a2ebd6a63c6b`, subsequently rebased as `2ab02240fabd40389ae29ec45b9ab9048ca491c7`. Its sole add/add conflict was the already-copied task record; resolution takes that record's exact bytes from the signed source commit. No implementation conflict or independent UI change was introduced. `262ce5d732c5e9453f1a31bcf394fce816da1167` recorded the latest integration brief; its rebased form is `633e91695af7a6dcc6aba6d23cd89ef62b2b9a0c`.

Upstream `6f2eddce` retains the single native keyboard arm and attached-state cursor fixture transitions. The residual #16417 implementation moves startup admission into shared `key_input`/`ime_input`, retains Retry/Quit and held-key ownership, adds the canonical six-test Make target and private normalized-input witness, and maps implementation-only limits into the existing NativeInputOwnership contract. Its full original evidence is [[reports/2026-11-03-startup-modal-input-regressions]]. That record's focused success is distinguished from the combined integration checks below.

## Commands and actual outcomes

All checks ran unattended with two Cargo jobs and a worktree-local target. Raw logs remain outside Git.

- `make init-submodules` — exit 0, pinned harness `0c403165e815c2f7bf77ccac293784b021f42f96`. The first targeted attempt exited 102 because that worktree prerequisite was initially absent.
- `cargo nextest run --locked -p bboxy450-doc --test corpus` — exit 0, 15/15 tests passed, zero skipped (`/tmp/report-ledger-corpus-final.log`). This was the complete focused pre-integration corpus run.
- `SIGNED_RANGE=adf9796b..HEAD cargo nextest run --locked -p bboxy450-doc --test corpus -E 'test(real_bboxy_frontmatter_parses) or test(committed_report_trailers_resolve_in_their_own_trees)'` — exit 0, both checks passed after the initial implementation commit, proving its real correction hashes against its parent/commit blobs (`/tmp/report-ledger-committed-corpus.log`).
- Deliberate blob-guard removal: the wrong-blob Git witness exited 100 with its expected assertion failure; the temporary guard-free implementation accepted an invalid correction. Original bytes were restored in a finally block (SHA256 `10b57d52cea6279b216d8a595cc6c8972eb3094292900943302b117090a80c61`). The subsequent restored run passed 14/14 selected checks; the remaining corpus-link test was then included in the complete passing 15-test run. Logs: `/tmp/report-ledger-blob-guard-mutant.log` and `/tmp/report-ledger-restored-corpus.log`.
- `cargo clippy --locked -p bboxy450-doc --test corpus -- -D warnings` — exit 0 (`/tmp/report-ledger-clippy-final.log`). An earlier lint exited 101 for constant `chunks_exact(2)`; it now uses `as_chunks::<2>()` and rejects a remainder. The initial Git fixtures also caught unsupported combined `-rz` flags; separate `-r`, `-z` restored the positive and negative cases.
- `make test-licenses` — exit 0: six licensing fixtures, the 34-crate policy/notice/source checks, and nine negative fixtures (`/tmp/report-ledger-licenses-final.log`).
- `python3 -I scripts/verify-signatures.py 1affb986^..1affb986` — exit 0, the original approved Boxy signature verified. The full SHA form was used in the actual command (`/tmp/report-ledger-original-signature.log`). The same pinned verifier passed the initial ledger commit and #16417's full source commit; all rebased/cherry-picked commits carry the approved Boxy signature.
- `git diff --check` — exit 0 after integration.

- `make test TARGET_DIR=target CI_LOG=target/ci-local.log` on `262ce5d732c5e9453f1a31bcf394fce816da1167` — exit 0. Archived complete normal log: `target/ci-local-262ce5d7.log`, SHA256 `1bb4417dbfbb6b5e66c5bfc529a3fa44fe2b9f0658e5af0fb14d51391aeaa619`. Native/TLC: 270 models, 34 inventoried crates, 111 contracts; receipt SHA256 `94868cd8560af7f0c788ce4bf33f757154727b07624d2740f19d1d1803120235`, dirty=false.
- `make test TARGET_DIR=target CI_LOG=target/ci-local.log` on `633e91695af7a6dcc6aba6d23cd89ef62b2b9a0c` — exit 0. Archived complete normal log: `target/ci-local-633e9169.log`, SHA256 `3b1406bb41f8981598eda33d37a4fc1717e88d116f594f16dde7f421e50babb1`. Native/TLC: 270 models, 34 inventoried crates, 111 contracts; receipt SHA256 `c648856c683dfcc994f35e915241cf84199611f4fa19462b5263325bcb4b294e`, dirty=false.

Both canonical runs passed all 2,584 workspace nextest tests; 211 existing ignored/filtered tests were skipped in the normal configuration. Doctests, workspace build, strict all-target Clippy, capability isolation controls, the isolated native-harness check, native/TLC graph/verdict parity, private unattended headless smoke and the normal invariant set passed. The second history signature invariant verified 2,676 commits. Normal invariants report 27 checks, zero failures and 11 slow checks skipped; extended slow/visual/night checks are not claimed.

The second complete run was necessary because origin/main acquired the two desktop source changes 1f7c48bc/6951376d while the first ran. After that refreshed source also passed, origin/main advanced again to 56387182 with a GitKB implementation/default change. The existing `--gate-log` path permits only records deltas (`scripts/landing.py:1213`); this second source advance requires a fresh current-source verification disposition. The two successful exact-source logs and content-addressed formal receipts are retained; no third baseline loop or source-mismatch waiver was started.

`SIGNED_RANGE=6afd6314..HEAD` and `SIGNED_RANGE=6951376d..HEAD cargo nextest run --locked -p bboxy450-doc --test corpus -E 'test(committed_report_trailers_resolve_in_their_own_trees)'` — each exited 0, auditing all affected commits in its real combined source cut rather than only the latest working tree. Logs: `/tmp/report-ledger-combined-range.log` and `/tmp/report-ledger-combined-633e9169-range.log`.

The signed 22:07/22:09 UTC delivery/resource amendment removes this component's independent landing obligation and designates #16404 as the final incident integrator. No third baseline is run. Current-source combination with later main and the separate per-root ownership repair belongs to #16404; this report claims no full gate for 56387182 or a later source. Only this full report and the exact current task copy are added after the passing 633e9169 source. Focused records checks and the existing records-only source/proof validator must validate that handoff; their actual results and final signed handoff SHA are supplied in task_report.


## Limits and component handoff

The formal coverage manifest already classifies report syntax, Git history resolution and filesystem predicates as implementation-tested; this Git migration changes none of the native immutable ReportProvenance actions. The shared input cut retains the existing finite NativeInputOwnership model and broken control and records modal visibility/IME/cursor limits explicitly. Full finite-model parity is not a proof of arbitrary Rust or the historical waived startup-modal lifecycle/visual behavior.

The containing report is created through knowledge_write/Index, copied unchanged and signed with its own Report trailer after the combined checks. Its commits field names the tested source cut, avoiding a self-referential report SHA. This component is verified on the two exact sources above and is **not independently landed**. The delivery amendment supersedes earlier sole-lander/scripts/land.sh requirements for #16394; #16404 owns the current-main combination, separate ownership fix, final gates and landing. No final-product gate is waived. Source, targets, both complete logs and persistent formal receipts are retained for that integrator and the Oracle; no build-target cleanup was performed.

No production or gate implementation changes follow the tested source. The final signed source/report and canonical log/receipt locations are handed to the Oracle and #16404. No task-specific test waiver, exclusion, quarantine, desktop input, operator prompt, service-credential setup or installation was introduced.
