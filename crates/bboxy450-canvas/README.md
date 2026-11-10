# bboxy450-canvas

Semantic canvas records, JSONL operation logs, deterministic replay, SHA-256
picture digests and f64 camera geometry. No GPU or terminal dependency.
Origin: https://github.com/bboxy450/bboxy450-term
Reuse policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

C0 accepts one author, with contiguous sequence numbers starting at 1. Duplicate,
reordered, missing and mixed-author operations are refused. Change replaces an
element; Tag adds a unique tag; Remove refuses referenced elements. Remove their
connectors/frame memberships first. Connectors attach to bounded elements only.
Replay starts empty and applies each operation atomically in slice order. It
performs no IO, scheduling, retries or merging. Multi-writer convergence and
persistent append custody belong to later canvas steps.

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
