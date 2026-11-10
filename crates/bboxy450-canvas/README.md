# bboxy450-canvas

Semantic canvas records, JSONL operation logs, deterministic replay, SHA-256
picture digests and f64 camera geometry. No GPU or terminal dependency.
Origin: https://github.com/bboxy450/bboxy450-term
Reuse policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

The store writes an interim Markdown record beside a JSONL batch log. The record
codec is separate from log serialization. A per-canvas OS file lock guards
append, sequence allocation and fsync; contention returns `canvas_busy` for a
same-operation retry. One newline-terminated JSON object holds an entire batch,
so an incomplete tail is dropped and its byte count reported. Complete malformed
lines fail. No second picture is stored.

Replay accepts nonempty authors and contiguous sequence numbers starting at one.
The store assigns them and dedupes `batch_id` even after restart; changed retry
arguments fail. Stale expectations are recorded and still appended. Change/Remove/
Tag on a removed id are recorded no-ops; stable ids cannot resurrect. Adds of an
existing live id fail. Change replaces an element; Tag adds a unique tag; Remove
refuses referenced elements. Remove connectors/frame memberships first. Conflicts
are resolved by append order, not a claim that arbitrary order produces identical
pictures. Batches validate fully before append. Acknowledgement follows fsync.

`load_log`/`save_log` retain C0's per-operation interchange format for fixtures;
the durable store uses the separate atomic batch codec in `log`.

Bounds and camera calculations use f64. Screen coordinates are relative to the
viewport center; the UI subtracts the camera before narrowing to f32. Zoom has
no arbitrary limits: positive finite representable values are accepted, and
arithmetic overflow/underflow is reported. Round-trip accuracy is limited by
f64 ULPs at the world position. Pan uses screen pixels, zoom_at keeps its screen
anchor fixed. Semantic detail uses the projected longest extent; text also uses
its projected text box height, so tiny text becomes a bar. Frames retain their title
when collapsed. Tags are sorted/deduplicated during replay. The digest encodes
only replayed elements sorted by id; authors, sequence and times remain in the
log. JSONL loading returns syntax errors with line numbers; replay validates it.

Check: `cargo nextest run -p bboxy450-canvas`. The UI canvas tests render the
fixed `tests/fixtures/c0.log.jsonl` through bboxy450-render. `UPDATE_GOLDENS=1`
explicitly refreshes their PNGs; ordinary runs only compare decoded RGBA bytes.
