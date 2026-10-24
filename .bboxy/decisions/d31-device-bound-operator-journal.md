---
id: "01M5MQ4GCGZVCKVESD8YH71C3P"
type: "decision"
number: 31
title: "GUI operations are journaled under an operator principal bound to the device, in the account object"
status: "amended"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#31]]", rel: "supersedes-as-index" }
created: "2026-10-23T15:47:38Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 31: GUI operations are journaled under an operator principal bound to the device, in the account object

**GUI operations are journaled under an operator principal bound to the
device, in the account object (2026-10-23; unblocks L8 D / decision [[d29-explicit-seat-placement|#29]]).**
Facts at 6582bc7: the control journal binds every op to a full seat
`Actor` (seat, generation, swarm, role, epoch, issuer; `mcp/mod.rs:53`,
`journal.rs:18, 115, 175`) stored under `seat-{id}` (`logic.rs:1020`);
`gui_spawn` bypasses it (`logic.rs:3025`); `gui.sock` admits by version
and capability with no stable principal (`seat_socket.rs:317-339`).
**Choice:** the journal's authority gains a second principal kind,
`Operator { device_id }`: the GUI is the operator acting on this device
(the device identity from ID1, stable across GUI reconnects and
supervisor restarts; the messaging tree already knows `Actor::Operator`
with `controls()` true). Operator ops are journaled in the **account
object** (`account` kv `control/op/{op_id}`) and audited on
`account/control`; seat ops stay under `seat-{id}` and
`seat/{id}/control`. `gui_spawn` goes through the journal like every
other op; a replayed `op_id` from any operator connection on the device
returns the recorded result (the seat id; the GUI then refreshes state);
a result never journaled because the supervisor died mid-launch is
unknown and never repeated automatically ([[d29-explicit-seat-placement|#29]]). Admission on `gui.sock`
stays version plus capability; logic attaches its own device id as the
principal, since the capability already proves the connection came from
the app instance logic spawned, so no new secret and no new handshake.
**Scope for L8 D grows** to `mcp/mod.rs` (principal enum), `mcp/journal.rs`
(location by principal), `logic.rs` (`gui_spawn` through the journal,
principal at admission) and tests. **Rejected:** a synthetic GUI seat (a
seat is a PTY, [[d20-seats-and-jobs|#20]], and it would need a grant chain for a thing that is
not a seat); a separate GUI ledger (two ledgers to reconcile on restart);
binding to the GUI connection id (gone on reconnect); binding to the
logic process generation (gone on restart, and spawn has no target
generation to bind to).
**Amendment (same day, Nyx's finding that `identity/status.rs:193-215`
returns `device_id: None` for LocalDev and unenrolled Pending):** the
device id is **born at first launch, not at enrollment**: the rule, not
yet the code when written (the ULID was minted at enrollment,
`journal.rs:327`, and the custody record had no device id; L8 D fixes
it as its own commit: mint once at first create, persist in the custody
record, upgrade legacy once, key id unchanged). ID1 §1 creates the
device key with its ULID in `DeviceKeyStore` custody and records it in
the account journal before any root certification; enrollment certifies
that key, it does not create it. So `Operator { device_id }`
uses the custody device ULID for every issuer kind, LocalDev and
Pending included, and `identity/status` must expose it from the custody
record, not from the enrollment state. Memory custody (tests, debug
opt-in) has a fresh device per process by [[d17-device-key-custody|#17]], so its journal is
per-process, which is the stated intent. The only case with no device
id is a custody **hold** (ID1d: store unavailable at start); then GUI
operations queue as `Held` exactly like messaging sends and release on
recovery. No nil, random or seat fallback, ever.
