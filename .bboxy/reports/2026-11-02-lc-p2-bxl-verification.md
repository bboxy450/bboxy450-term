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
  "created": "2026-11-02T03:10:45.960Z",
  "created_by": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "id": "01M6D3ST0BVK0Q95N2H2W70AGH",
  "number": 5139,
  "provenance": {
    "digest": "sha256:7ff415b1c05b52b96d2b4723a5a9109df563b039df39b901c10b8212495c43e2",
    "frontmatter_digest": "sha256:d1d9a017d37ec3c1551cb5bb5e88b1581d6d02abf916b67280ae5eabc2b3cda7",
    "log": ".bboxy/log/2026-11-02.jsonl#34045",
    "sig": "r60wDHTKvUyv78Zjdzj7UwkUEbro0FBmZ9pW5aT7kjquRabAmsmX9SjY1TXOpSwmjFawpaNsxksU9gZQlx0aAg==",
    "signer": "seat:01M6CKGYEPHYP25GFF6H7B1X23"
  },
  "task": "[[tasks/2026-11-01-lc-p2-bxl]]",
  "title": "P2 bxl: append log, recovery and native/Java verification",
  "type": "report",
  "updated": "2026-11-02T03:10:45.960Z",
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

# P2 bxl implementation handoff

Implementation source is signed
`1067033e5b20a0728c9928f28ca0fe13b1f45364` in bboxy450-term,
`crates/bboxy450-log-converter`, branch log-converter, private worktree
/Users/feral/mydev/bboxy450-term-log-converter.
Draft PR: https://github.com/bboxy450/bboxy450-term/pull/91.
The delegated port task #5096 and d5087 supersede the retired standalone
repository/worktree instructions in the original #5084 brief. No merge occurred.

P2 code and its separately run verification stages pass. Canonical make test
remains failed (exit 2 on 503197ba) at the unchanged resource-ledger signal
startup race; this task is not presented as accepted or ready to merge.
The later full workspace tests and --after-tests run on 1067033e are
supplemental evidence, not a normal full receipt.

## Implementation and witnesses

Paths below are relative to crates/bboxy450-log-converter.

- src/bxl/frame.rs and wire.rs emit canonical CBOR in standard checksummed
  zstd frames with a skippable version/dictionary header. Record envelopes
  carry Decoded projections and complete wire values; unknown header, event,
  cursor, record and skippable-frame parts remain readable or forwardable.
- src/bxl/writer.rs:358 fsyncs frame data; publish_index at :50 writes/fsyncs a
  temporary sidecar, renames it at :64, then syncs the directory. The mutable
  index is a cache, never an in-place trailing seek table.
- A nonblocking first-segment lease owns one log across rotation. The per-log
  worker has a bounded command lane and the 250 ms flush deadline. Injected
  clocks test deadline, count, size and turn-end frame closure.
- src/bxl/reader.rs:79 yields only validated complete frames; :163 resumes an
  intra-frame cursor, :186 seeks events, and :298 tails concurrent writes.
  The whole-frame cursor regression at tests/bxl.rs:216 includes the old
  zero-index replay control. Two reader processes follow a writer and one
  resumes without duplicates (tests/tail.rs:76); partial bytes do not move a
  cursor (:109).
- recover at src/bxl/writer.rs:424 scans validated frames, truncates the torn
  tail and atomically rebuilds the index. tests/torn.rs:39 tries every byte cut
  across the last two frames and repeats recovery. :74 rejects checksum damage.
  tests/bxl.rs:104/122/156 exercise idempotence, old sidecar descriptors across
  rename, failure after data fsync and index hints outside durable extents.
- Seal and rotation append immutable tables. The closing standard zstd seek
  table accounts for skippable-header/first-seq-extension bytes. The pinned
  upstream zstd 1.5.7 CLI -d/-t witness and seek-table offsets pass at
  tests/compat.rs:176; no global tool install or product Java process exists.
- tests/compat.rs:36 forwards unknown newer-minor parts through an old typed
  reader. :114 refuses an unknown major and reports a missing dictionary.
  Caller dictionaries are hash- and dictionary-ID-checked.
- tests/size.rs:23 checks compression including retained raw records and
  structural frame limits; :63 keeps sealed segments immutable and under the
  segment limit. The separate 100 MB raw-included ratio/readback test at :106
  passes and runs only in the full native target.
- tests/determinism.rs:23 pins reproducible segments/indices for identical IR,
  clock and flush schedule. Blob dedup streams the existing log and verifies
  matching content, including collision/mismatched-body refusal.

Maximums are 1 MiB and 256 events per frame, 250 ms buffering and 64 MiB segments
with closing-table space reserved. Oversized records fail explicitly. Recovery
and seek scan conservatively; no constant-time index claim is made. P9 owns
trained provider dictionaries; P7 owns SourceCursor interpretation. The CLI is
still the P1 skeleton. No native-log parser/lowerer or session content is added.

## Formal verification

verification/log-converter.md and root contracts/corpus/coverage map the Rust
source, API/process tests, explicit bounds, assumptions and remaining gaps.
LcAppendLog explores writer, temp-index sync/atomic rename, crash/recovery,
tail readers and rotation. It checks NoTornRead, NoGapNoDup,
IndexNeverPastDurable, IndexSound, SyncedSurvives and RecoveryIdempotent.
The three required controls alter real transitions: early reader visibility,
early index publication, and an in-place table rewrite exposing mixed halves.

All four native/Java full graph and verdict pairs match on 1067033e:

| Model | Verdict | States | Edges |
|---|---|---:|---:|
| LcAppendLog | pass | 9476 | 43265 |
| BrokenLcAppendLogTornRead | safety violation | 11114 | 50704 |
| BrokenLcAppendLogEarlyIndex | safety violation | 11312 | 54342 |
| BrokenLcAppendLogMixedTable | safety violation | 15152 | 66177 |

The model bounds three frames/two events each, two readers and one rotation;
per-reader weak fairness is explicit. No unsupported construct was discarded,
and there is no new unsupported-semantics gap. Filesystem/kernel power loss,
implementation refinement and general TLC completeness are not proved.

The complete workspace formal run exited 0: 209 model pairs, 33 crates,
85 contracts. Receipt source 1067033e is clean; evidence SHA-256
da379dece34315c980e3cc3bfd79daad726b231bcedfdabbddc249524cb613f0 is retained at
/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/da379dece34315c980e3cc3bfd79daad726b231bcedfdabbddc249524cb613f0/.
Fresh --verify-evidence exited 0 before the later report-only commit.

## Commands, provenance and open gate

Workspace build, strict default clippy, workspace unit/integration/doc tests,
converter fmt/all-feature clippy/tests and full native target exit 0 on the
implementation head. Converter executable tests total 40.72 s at term's
default two test threads (60 s allocation). Pinned zstd and 100 MB witnesses
pass; the current 100 MB native run took 15.73 s. The default-ignored native
witnesses are mandatory in full CI. All five P0 scripts pass with seven
positive resumes, six expected negatives and no missing-client skips.
UI lint and the private unattended headless smoke exit 0.
The final supplemental --after-tests result is 1 (disk guard); the same compiled checkpoint fixture subsequently exited 0.

All six implementation commits have valid signatures from
9B07E849480C68328CA20684844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>.
Source files start with term policy headers and preserve original notices.
Deny/dependency/provenance/secret-shape controls pass. The report log is
.bboxy/runs/p2-bxl-report.log. Full port context is
[[reports/2026-11-02-lc-port-verification]].

The full make-test gate exited 2 on 503197ba:
scripts/test-resource-ledger.py:222 expected 143, observed -15. The child
starts before SIGTERM handlers are installed. Oracle message
01M6CSPNMQ5691NTGSBTJYTHZ1 assigns the fix to E1c and says not to change it
here. Main lacked that fix at handoff. After it lands, rebase with a heads-up
and rerun normal make test, refreshing current-head formal evidence.

Complete canonical failure logs: /tmp/lc-term-make-test-fixed.log and
/tmp/lc-term-make-test-second-ci.log. Supplemental stdout is
/tmp/lc-term-rest-combined.log; one oversized workspace-test chunk was
truncated during collection, so this is not called a complete raw transcript.
The command statuses, converter witnesses and formal receipts are retained.
No unsanitized logs, secrets, peer worktrees, merge or release were involved.

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
