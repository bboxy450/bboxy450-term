---
{
  "author": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "authorship": {
    "kind": "seat",
    "process_generation": "1",
    "public_key": "srV0sf6OqVcpHr6OW6bpLTluWr3qGWNk1q9J8nzC9/Q=",
    "signer": "seat:01M6CKGYEPHYP25GFF6H7B1X23"
  },
  "commits": [
    "1067033e5b20a0728c9928f28ca0fe13b1f45364"
  ],
  "created": "2026-11-02T03:10:30.651Z",
  "created_by": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "id": "01M6D3SAEC3WEFAAP0WSVEV8TK",
  "number": 5137,
  "provenance": {
    "digest": "sha256:603f4d7f29581011908a01458fb48ec3133d7079e9ff40ec59761c1df1a6ef7d",
    "frontmatter_digest": "sha256:79cd869660b08830cbe8f47a1de42cf61a243f797c2a0aedf35f7cac1f5e460e",
    "log": ".bboxy/log/2026-11-02.jsonl#34039",
    "sig": "cdjdnJ4CWZ5Skfmrll4Rn1vkFf7CeuJBEjVF4XeS8IQI9rMa3SZhBvlXBxp3IirVBuN4WWXcL8eJTX0l9S/XCA==",
    "signer": "seat:01M6CKGYEPHYP25GFF6H7B1X23"
  },
  "task": "[[tasks/2026-11-01-lc-port-into-term]]",
  "title": "LC port: implementation verification and external full-gate failure",
  "type": "report",
  "updated": "2026-11-02T03:10:30.651Z",
  "updated_by": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "verify": {
    "command": "make test",
    "exit_code": 2
  }
}
---
<!--
Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
-->

# LC port and P2 verification handoff

Implementation source: `1067033e5b20a0728c9928f28ca0fe13b1f45364`, branch
`log-converter`, private worktree
`/Users/feral/mydev/bboxy450-term-log-converter`.
PR: https://github.com/bboxy450/bboxy450-term/pull/91 (draft).
No merge or land.sh was run.

The converter and the separately run workspace stages pass. The required
canonical `make test` has **not** passed: its latest normal run, at signed
`503197ba90e7fb547a48002473011dbe9c317104`, exited 2 at the pre-existing
resource-ledger startup signal race. Later supplemental checks run on 1067033e
do not replace that receipt. Acceptance remains open.

## Changes and evidence

- `crates/bboxy450-log-converter/Cargo.toml`, root Cargo manifests and lock:
  P1 from retired-repository bb87743b and P0 from 7945ad92 are workspace members
  and local tests/probes. Headers and license copies follow term; original
  origin notices remain attached. No standalone formal.py was carried over.
- `crates/bboxy450-log-converter/src/ir/mod.rs:49`: public native RecordId
  constructor; `:73` frames domain/provider/native_id/native_key/role_tag for
  EventId; `:227` carries Event.record and `:228` uses RecordId parents.
  Compaction.replaced_through is a RecordId too. Stateless multi-event and
  single-event parent joins pass; guessing a block-0 EventId fails the control.
  CBOR key 8 was deliberately added, regenerating 66 of the 68 fixed cases.
  EventId bytes were unchanged by that RecordId addition.
- IR/CBOR tests retain all variants, RFC 8949 encoded-key byte ordering,
  permissive decoding, exact identity vectors and unframed/length-first broken
  controls. Decoded preserves the complete nested wire value during forwarding.
- `src/bxl/writer.rs:50` publishes a temporary fsynced sidecar by rename
  (`:63–64`) after data fsync (`:358`). The directory is synced. The first
  segment's nonblocking ownership lease survives rotation. One bounded worker
  is owned per log; there is no global writer lock or worker.
- `src/bxl/reader.rs:79` consumes an entire delivered frame in its saved cursor;
  private prefetch (`:87`) leaves record iteration at the first record.
  `tests/bxl.rs:216` verifies that resuming whole-frame delivery does not replay
  the old frame, and demonstrates replay with the old zero-index cursor.
  This corrects an implementation witness under the existing NoGapNoDup model.
- The same module provides record/event iteration, resume, seek and tailing.
  Writer recovery (`src/bxl/writer.rs:424`) validates frames, truncates a torn
  tail and atomically rebuilds the disposable index; repeated recovery is stable.
  Seal/rotation keep old bytes immutable and append a standard seekable table.
- `scripts/harness-submodule.py:108` traverses cargo metadata and rejects
  upward app/mux/harness dependencies. Banned d4949 packages and declared copied
  origins are refused; secret-shape errors suppress values. Ten controls pass,
  including the existing real ACP/serde_json-preserve_order rejection.
- Root Makefile and ci-local include the crate checks and full-run-only native
  target. P0 real clients, upstream zstd compatibility and 100 MB size checks
  are excluded from presubmit. Missing clients are labelled SKIP, never proof.
- Both landing fixture drivers account for the added targets.
  `scripts/test-landing.py` has a failed-native-target control that cannot
  issue a green receipt. Extended recovery expects all six full-run make calls.
- `verification/log-converter.md`, contracts.toml, corpus.toml and coverage.toml
  register LcAppendLog and all three broken controls. Root formal.py supports
  the explicit control list while retaining legacy pairs.

Paths abbreviated as src/tests above are relative to the converter crate.

## Verification

Commands used the term CI profile (opt-level 1, debug 0, debug assertions on)
and its default two test threads. No operator input was required.

| Command or stage | Actual result |
|---|---|
| Initial make lock / make check-log-converter | 0 / 0 |
| Normal make test before fixture fix | 2, missing synthetic Makefile targets; fixed |
| Normal make test on 503197ba | 2, resource-ledger SIGTERM startup race |
| Fast landing fixtures | 0, 140 tests, 151.011 s |
| Extended landing recovery fixtures | 0, 46 tests, 315.396 s |
| Workspace cargo build --workspace --locked on 1067033e | 0 |
| Workspace clippy --workspace --all-targets --locked -- -D warnings | 0 |
| quarantine-test.sh --workspace --locked, with the isolated non-PTY fixture skipped | 0; that exact fixture separately passed in test-harness-native |
| make test-log-converter, including fmt and all-feature clippy | 0 |
| make test-log-converter-native | 0 |
| Current complete-range committed-tree report guard | 0, 1 test, 1.08 s |
| make test-formal | 0, all 209 model pairs, 33 crates, 85 contracts |
| UI lint and isolated unattended headless smoke | 0 / 0 |
| Supplemental ci-local.sh --after-tests | 1 (disk guard); same compiled checkpoint fixture rerun: 0 |

Current workspace execution of the converter suite took 40.72 s of executable
test time, below its 60 s affected-test allocation. Initial compilation and
build-lock waits are build time. The converter includes 8 bxl API, 11 IR
compatibility, 68 golden/round-trip cases, 2 CLI, 2 future-format/dictionary,
1 determinism, 1 formal corpus, 1 P0 control, 2 size/rotation, 3 tail/process and
2 torn/checksum tests. Two default-ignored witnesses are mandatory in the native
full-run target: pinned upstream zstd -d/-t with seek-table validation, and
100 MB raw-included ratio/readback (current native run 15.73 s). All five P0
scripts passed: seven positive resumes and six expected negative controls,
zero missing-client skips.

Native/Java full state sets, action-labelled edges and verdicts matched:

| Model | Verdict | States | Edges |
|---|---|---:|---:|
| LcAppendLog | pass | 9476 | 43265 |
| BrokenLcAppendLogTornRead | safety violation as expected | 11114 | 50704 |
| BrokenLcAppendLogEarlyIndex | safety violation as expected | 11312 | 54342 |
| BrokenLcAppendLogMixedTable | safety violation as expected | 15152 | 66177 |

The in-place-table control exposes independently read mixed old/new halves.
The positive model uses immutable old table bytes. Three frames, two events
per frame, two readers and one rotation are bounded; per-reader weak fairness
is explicit. No unsupported Rust-TLC construct was dropped, and there is no
new unsupported-semantics gap.

Formal receipt: source 1067033e, dirty=false,
SHA-256 `da379dece34315c980e3cc3bfd79daad726b231bcedfdabbddc249524cb613f0`.
It is retained under the common Git directory at
`/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/da379dece34315c980e3cc3bfd79daad726b231bcedfdabbddc249524cb613f0/`.
A fresh `python3 -I scripts/formal.py --target target/formal --verify-evidence`
also exited 0 on that clean implementation head.

All six implementation commits report G from the approved key
9B07E849480C68328CA20684844ECC0587DE7AB0, author/committer Boxy
<bboxy450@gmail.com>. The signed report-only commit is identified by its Report
trailer and the handoff. It changes no product/model input; the receipt above
names the measured implementation head, not that later documentation commit.

## Logs, limits and remaining work

Canonical failed output is complete in /tmp/lc-term-make-test-fixed.log and
/tmp/lc-term-make-test-second-ci.log. Supplemental output is collected as
/tmp/lc-term-rest.NNNN.log and combined at /tmp/lc-term-rest-combined.log.
One oversized workspace-test chunk was truncated during collection; command
exit statuses and the converter/formal output remain available. The combined
file is not represented as a complete raw transcript.

The port report log is committed at
.bboxy/runs/2026-11-01-lc-port-report.log and copied to the exact requested
harness .bboxy/runs path. P2 has .bboxy/runs/p2-bxl-report.log. Reports contain
synthetic-fixture evidence only; no native session content or credentials were
added to Git.

The required make-test failure is scripts/test-resource-ledger.py:222:
expected 143, actual -15. scripts/resource-ledger.py starts the child at :442
before installing handlers at :458. Oracle message
01M6CSPNMQ5691NTGSBTJYTHZ1 assigns that repair to E1c and explicitly says not to
change it here. Main had no such fix at this handoff. Once it lands, rebase
with an Oracle heads-up and rerun canonical make test; refresh formal evidence
on the integrated head. No full success or merge readiness is claimed.

The format remains bounded at 1 MiB/256 events per frame and 64 MiB segments;
the 250 ms deadline has an injected-clock witness. Recovery and seek scan
conservatively; blob dedup streams the existing log. Caller-provided
dictionaries are content-addressed/ID-checked; P9 supplies trained dictionaries.
SourceCursor stays opaque for P7. The CLI remains the P1 skeleton. Model/kernel
power-loss refinement and complete TLC replacement are not claimed. Root
ATTRIBUTION and mirrors in unrelated crates were outside #5096's allowed
paths; all copied notices are retained and the converter's canonical copies
match term.

The supplemental after-tests command exited 1 solely because the bounded-
checkpoint fixture refused free space below 10 GiB (10,679,144,448 bytes).
Its other 26 default invariant checks passed; 11 marked slow were skipped by
the normal gate. I removed only this worktree's generated cache: 117 older
executable variants, target/plugin-fixtures, and 471 older rlib variants.
No source or peer/shared cache was removed. /tmp/lc-own-target-cleanup.json
records every removed path and the free-space samples. The retained current
recovery executable's SHA-256 stayed
a90e6d7314ecfdfe364cdfa7c9d4a448e8de6b16e0e336cf187a59f873fc5d99.
With reserve restored, the exact same compiled fixture was run as
`R5G_SEATS=4 RUST_TEST_THREADS=2 target/debug/deps/recovery-1f648b33642729d0 checkpoint_bounded_recovery --exact`.
It exited 0: one test, 14.94 s, /tmp/lc-checkpoint-after-cleanup.log.
This targeted rerun resolves that environmental refusal; it does not relabel
the original supplemental command's exit status or create a full-CI receipt.
HEAD remained 1067033e and git status was clean after the rerun.
