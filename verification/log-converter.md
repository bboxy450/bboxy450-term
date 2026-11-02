<!--
Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
-->

# Log converter verification inventory

The converter is `crates/bboxy450-log-converter` in this workspace. The P1 IR and canonical CBOR are pure-data contracts; P2 adds the
append-log state machine.
The workspace native Rust TLC / pinned Java TLC gate owns formal verification;
there is no separate crate formal runner.

| Contract | Source / evidence | Status |
|---|---|---|
| Every IR variant, fixed integer keys, round trip | `src/ir/keys.rs`, `tests/cbor_golden.rs`, 68 vectors | Implementation tests |
| Canonical bytewise CBOR, permissive readers, unknown-field retention | `src/ir/cbor.rs`, `tests/cbor_compat.rs` | Implementation tests with length-first broken control |
| Framed EventId and RecordId | `src/ir/mod.rs`, `tests/cbor_compat.rs` | Fixed vectors and unframed collision control |
| Stateless record DAG joins | `tests/cbor_compat.rs::record_references_join_single_and_multi_event_parents_without_state` | Multi-event and single-event parents; guessed block-0 parent fails |

Paths in the table are relative to the crate. Source/model refinement is not
claimed. Later stateful phases must add their own models, controls and API witnesses.


`LcAppendLog` and three safety controls are registered in the workspace corpus,
contract/source mapping and crate coverage. All 20 variables start at zero:
writer running, three absent frames, durable/index/staged prefixes empty,
reader cursors zero, no rotation, seek-table read/write or recovery. Frames have
absent/writing/complete/synced states (0..3), with two events per abstract frame;
there are two readers and one rotation. Begin/complete/sync, temp-index
write/sync/rename, crash, recover, reader step and seal/rotate are atomic actions.

Safety checks are `NoTornRead`, `NoGapNoDup`, `IndexNeverPastDurable`,
`IndexSound`, `SyncedSurvives` and `RecoveryIdempotent`. Per-reader weak fairness
checks delivery of every complete frame. There is no fair-IO or writer-progress
assumption. Complete unsynced bytes survive this finite local-filesystem
abstraction; recovery syncs validated frames. Kernel power loss, Rust refinement,
byte layout and actual host failures remain implementation-only.

The controls really change transitions: (a) readers yield before validation;
(b) an index publishes before sync; (c) an in-place writer changes old seek-table
halves while a parked reader reads them independently. Control (c) exposes mixed
old/new bytes, rather than setting a verdict flag. The positive protocol appends
new table bytes and keeps the old table immutable.

Implementation witnesses are `tests/bxl.rs`, `torn.rs`, `tail.rs`, `size.rs`,
`compat.rs` and `determinism.rs`. They exercise every byte cut in the last two
frames, checksum corruption, repeated recovery, index publication failure after
fsync, old sidecar descriptors across rename, invalid cache extents, two reader
processes, intra-frame resume, partial-frame waiting and injected deadlines.
The workspace gate compares full native and pinned Java states, action-labelled
edges and verdicts. It rejects omissions, duplicate models, tool failures,
unsupported semantics, timeouts and disagreements; it claims neither complete
TLC compatibility nor arbitrary Rust correctness.

Transport records use existing Event/Session integer maps; blobs use
`{0: blake3 bytes, 1: body bytes}`. Key-0 shapes distinguish these families;
unknown records stay opaque and forwardable. `Decoded<Event>` and
`Decoded<Session>` preserve nested future fields. Cursor payloads remain opaque
CBOR for P7. Unknown physical skippable-frame kinds are skipped.

Flush fsyncs every data frame, then publishes a temporary, fsynced sidecar by
rename and syncs the directory. This also makes `flush` durable. One bounded
command lane feeds one owning worker per log; no global worker/lock exists.
The persistent first-segment descriptor keeps a nonblocking lock across rotation.
The worker has the 250 ms deadline, with an injectable clock and explicit poll.

The index is a disposable cache. Recovery conservatively revalidates complete
frames, truncates only an invalid tail and atomically rebuilds the sidecar.
Readers scan and expose seek-by-seq; no constant-time seek is claimed. Blob
deduplication streams existing records, trading lookup IO for bounded memory.
Frames are at most 1 MiB and 256 events. Segments reserve closing-table bytes
under the 64 MiB bound; index entries are capped at 1 MiB/64 as well.

A sealed segment appends a skippable first-event-seq extension and then standard
zstd-seekable 8-byte entries/9-byte footer. Header/extension skip frames have
zero decompressed-size entries, so standard seek prefix sums include them.
Pinned upstream zstd 1.5.7 is built locally from checksum-verified source, with
no global install. Its `-d` and `-t` witnesses, the 100 MB raw-included size test
and P0 real-client probes run via `make test-log-converter-native` in ci-local
only. Default crate tests contain no native-client probe; missing clients emit
labelled SKIP, which never proves resume.

No provider dictionary or training input is shipped in P2; P9 owns that work.
Caller-supplied dictionaries are content-addressed and checked against their
zstd dictionary ID, with an explicit missing-dictionary error. No real session
content is committed. The root resolution guard checks converter dependency
direction, banned packages/declared copied origins and secret shapes, with
broken controls. Undeclared copying remains a source-review obligation.
