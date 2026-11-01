# Log converter verification inventory

The converter is `crates/bboxy450-log-converter` in this workspace. It has no
product state machine in P1; the IR and canonical CBOR are pure-data contracts.
The workspace native Rust TLC / pinned Java TLC gate owns formal verification;
there is no separate crate formal runner.

| Contract | Source / evidence | Status |
|---|---|---|
| Every IR variant, fixed integer keys, round trip | `src/ir/keys.rs`, `tests/cbor_golden.rs`, 68 vectors | Implementation tests |
| Canonical bytewise CBOR, permissive readers, unknown-field retention | `src/ir/cbor.rs`, `tests/cbor_compat.rs` | Implementation tests with length-first broken control |
| Framed EventId and RecordId | `src/ir/mod.rs`, `tests/cbor_compat.rs` | Fixed vectors and unframed collision control |
| Stateless record DAG joins | `tests/cbor_compat.rs::record_references_join_single_and_multi_event_parents_without_state` | Multi-event and single-event parents; guessed block-0 parent fails |

Paths in the table are relative to the crate. Source/model refinement is not
claimed. Later stateful phases must add their models, controls, bounds and real
API witnesses to the workspace inventory.
