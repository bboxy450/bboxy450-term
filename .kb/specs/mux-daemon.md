# Mux daemon — process survival and state delivery

2026-10-21 · Nyx Copperwing <EXECUTOR> · E convergence for Oracle review.

[[vision]] · [[cloud-and-mosh]] · [[cloud-and-mosh-exec]] · [[terminal-core]] · [[messaging]] · [[harness-touchpoints-exec]] · [[subagent-detection]]

## Scope and decisions

The mux preserves seats across GUI and mux-logic crashes. A small per-machine supervisor owns PTY masters and bounded byte history; a restartable logic process owns VTE models, messaging, MCP and scheduling. The GUI receives state and owns rendering. This implements the process split and checkpoint decisions in `../vision.md:61`; reasons and measured limits are in [[cloud-and-mosh-exec]].

Ship one executable with GUI and `--daemon` public modes. Supervisor and logic are private process roles behind daemon mode, not separately distributed agent binaries. macOS 14 is the initial platform. Remote/cloud seats use the same daemon architecture; cloud provisioning and cross-harness migration remain future work under [[roadmap]].

```
GUI / local MCP clients
          | authenticated local framed IPC
     mux logic  <---- authenticated Quinn ----> remote mux logic
          | versioned local control + ordered seat records
     supervisor  [PTY masters, children, ring files, checkpoints]
          |
     harness processes
```

The GUI can disappear without changing either daemon role. The logic can disappear without closing PTY masters. Supervisor death is the exceptional fallback: recover durable history and resume supported harnesses as new processes. Disk files do not preserve a live PTY or an application's memory.

## Ownership and failure boundaries

| Component | Owns | Failure behavior |
|---|---|---|
| Supervisor | PTY masters, child identity, sole PTY read/write path, observed event order, ring/commit files, accepted input IDs, active logic lease | Continues bounded capture while logic restarts. Never spawns a second copy just because logic reconnects. |
| Logic | Viewport VTE/parser, semantic row index/checkpoint production, seat registry, permissions, native harness adapters, messaging storage/scheduling, MCP, remote links | Reattaches, restores a checkpoint and replays ordered records. All readiness and GUI caches are revalidated. |
| GUI | GPU chrome/grid rendering, local presentation/selection/cache, subscriptions and operator input | Reconnects for snapshots and lazy history. It is not the durable seat owner or an independent VTE authority. |
| Remote supervisor/logic | Same ownership on the remote host | Network loss leaves remote processes running. Reconnect authenticates the device and reconciles seat/process generations. |

Supervisor has no public network listener, font/GPU stack, model protocol or room scheduler. Keep its local protocol and storage code small because every seat depends on it. Logic requests operations through that protocol; it does not get an independent master-fd reader/writer that could race the supervisor.

One supervisor serves a user's machine namespace. Its private socket/state files are owner-restricted and outside the repository. Logic authenticates over a local capability established at startup, never a secret in an argv or a peer-supplied seat name. This is local admission, not a same-UID adversarial sandbox; [[messaging]] retains that threat-model gap.

## Durable seat identity and launch mapping

Persist one record per owned seat:

```text
SeatRecord {
  seat_id, owner_device_key, location,
  harness_kind, harness_session_id?,
  launch_spec { executable, argument_template, cwd, profile, secret_references },
  role_grant_reference, muted, policy_epoch,
  lifecycle, lifecycle_revision, close_record?,
  process_generation, supervisor_boot_id,
  child_identity? { pid, process_start },
  ring_format_version, checkpoint_reference, committed_record_sequence
}
```

Use the shared seat-ID type from the application identity model. Keep it stable across GUI/logic restarts and supported harness resume. A new harness process gets a new process generation even when its harness session ID is reused. PIDs alone are never identity: verify process start and the supervisor's live ownership.

Launch records contain explicit configuration and secret references; do not snapshot the whole environment or embed credentials in diagnostic records. Persist a verified harness-session mapping as soon as its adapter obtains it. Mapping updates and launch/close operations carry stable request IDs so an ambiguous reconnect does not duplicate a seat.

Only the supervisor launches an owned local PTY process. A logic restart discovers existing owned children before considering resume. If the supervisor died, validate the saved mapping, invalidate old native attachments/readiness/capabilities, then use the harness adapter's supported resume command for a new process. Restore historical display separately; do not feed old output or queued commands into that new process.

The close lifecycle below constrains this recovery rule: a deliberately closed seat must not be automatically resumed as an open one. Restore its lifecycle/mute policy before scheduling messages or considering a launch.

For a plain shell or a harness without a usable resume mapping, create the configured new session only under its declared restart policy and expose the recovery limitation. Do not claim restoration of shell variables, a foreground command or a conversation whose session ID was never persisted.

Phantoms from [[ux-pick]] are published remote participants, not owned remote seats. They have no PTY, pane, screen/scrollback permission or resume mapping. Their owner's signed presence/roster supplies the allowed identity/state metadata through [[messaging]]. Showing a phantom must never grant remote control or trigger remote bootstrap.

## Reversible close lifecycle

The operator's undo-close decision in [[vision]] adds `open → closed_grace → closed_retained`. These are lifecycle values, not extra harness activity states. Mute policy is also independent; closing does not silently mute or unmute a seat.

`CloseSeat` durably records the close operation/lifecycle revision, process generation, bounded expiry (default 60 seconds, operator setting), and a restoration handle containing the original space/tab/split position and ratios. The GUI collapses the seat into the closed strip with an undo action. The supervisor retains the same PTY master and child; ring capture continues subject to its existing bounded backpressure rules. A closed-grace live/starting process still consumes the cohort/machine spawn cap.

The supervisor owns the expiry, so logic/GUI loss cannot leave the child running indefinitely past grace. Persist restart-safe expiry information and use a monotonic deadline while that supervisor lives; replaying a request never starts a fresh grace window. On expiry, terminate only the recorded child/process generation, record its actual exit, commit retained history and enter `closed_retained`. Retain the ring/checkpoints and harness-session mapping for a later explicitly requested resume. Do not claim termination completed before its receipt.

`UndoClose` addresses the exact close/lifecycle revision and restoration handle. During grace it cancels the matching expiry and restores the same live seat at its recorded position, with no new child or process generation. Serialize expiry and undo so only one wins. Return explicit expired, process-exited/replaced or layout-conflict outcomes rather than silently creating a duplicate or relocating the seat. Keep enough layout restoration metadata to restore the original split position; concurrent layout changes must be checked against its revision.

After supervisor death, the PTY's old process is not preserved. Do not auto-resume a closed-grace/closed-retained seat as part of general open-seat recovery; report the changed process state. Uncertain/passed expiry never receives a fresh 60 seconds on recovery. After grace, reopening uses the adapter's explicit session-resume/new-generation policy and retained history, not an assertion that undo preserved a live process.

**Gap:** termination escalation and conflicting-layout restoration need implementation rules/tests. The addendum names a purge tool without specifying whether it finalizes grace, deletes retained data, or both; [[mcp-control-plane]] leaves purge unavailable until an explicit operator policy defines that destructive scope.

## Supervisor ↔ logic protocol

Use a versioned, length-framed local protocol with negotiated message limits. Validate lengths before allocation. Every operation includes its request ID and the relevant seat/process generation. Exactly one logic lease can mutate supervisor state: registering a replacement fences off the previous connection; stale operations are rejected even if the old process is still alive.

| Operation | Contract |
|---|---|
| `Hello / AcquireLogic / ListOwnedSeats` | Authenticate the local client; exchange protocol/format versions, supervisor boot ID and a new logic lease. Report existing children and retained/committed cursors before any launch decision. |
| `CreateSeat / CloseSeat / UndoClose` | Idempotent request IDs, authorized launch/close record and expected process/lifecycle generation. Close enters grace; undo/expiry serialize at the supervisor. Return actual lifecycle/child identity or explicit rejection. An ambiguous create is queried by its original ID. |
| `ReadRecords / Records / Gap` | Ordered records with sequence and raw-byte intervals; return retained range and an explicit unavailable-prefix indication. No silent clamp presented as continuous history. |
| `WriteInput / InputReceipt` | Logic lease, writer lease, process generation, stable input ID and literal bytes. Supervisor is the sole writer. Receipt distinguishes rejected, fully PTY-written, and partial/unknown. |
| `Resize / ResizeReceipt` | Expected generation and dimensions. Supervisor performs the PTY operation and records the observed resize order used by VTE replay. Return actual success/failure. |
| `PublishCheckpoint / CheckpointCommitted` | Checkpoint version, process generation, exact last-applied record/cursor, state/index blobs and integrity metadata. Supervisor validates cursor coverage before committing or permitting eviction. |
| `ChildExit / StorageStatus` | Child identity/exit evidence, captured and durable cursors, or backpressure/error. Infrastructure health is separate from the harness's four activity states. |

Supervisor sequences PTY reads, resize events and process boundaries into one per-seat record order. Logic reconstructs from that observed order, not from clocks or guessed producer timing. Checkpoints describe a consistent VTE state after an exact record boundary; serializing a concurrently changing grid without a barrier is invalid.

Supervisor retains input IDs and receipts across logic restart. Retry/query the same ID, not a fresh paste. A successful write proves bytes crossed the PTY boundary; it does not prove harness consumption, submission or command completion. Storage bookkeeping and a PTY write are not one atomic transaction. If a supervisor crash or partial write leaves the outcome uncertain, preserve that uncertainty and do not automatically reissue it.

Lease replacement does not restart an in-flight write. Keep per-seat write exclusion until its receipt is completed or explicitly unknown, then admit another writer. A stale lease cannot submit a second attempt around that boundary.

Native queue/RPC adapters remain preferred under [[harness-touchpoints-exec]]. Generic PTY fallback still requires the current seat/session/process/generation readiness contract and serialized operator/injected input. A status report, quiet screen, packet ack or new logic lease is not composer readiness.

## Ring records, checkpoints and eviction

The supervisor stores a bounded append/segment log that presents a logical byte ring. Runtime files live in private application state directories, never `.kb/`, the worktree or Git. Separate active process generations so an old cursor cannot accidentally address a resumed process.

Logical record envelope, format version 1:

```text
Record {
  version, kind, seat_id, process_generation,
  record_sequence, byte_start, payload_length,
  payload, checksum
}
kind = Output | Resize | ProcessStart | ProcessExit | HistoryBoundary
```

Use fixed-width little-endian integers and CRC32C over the encoded header and payload. Output offsets count raw bytes; record sequences also order non-output events. Unknown versions, impossible lengths, checksum failures, sequence discontinuities or generation mismatches stop recovery at the last valid committed boundary. CRC is accidental-corruption detection, not authorship authentication.

A checkpoint contains the exact record/cursor it follows, parser state including incomplete sequences, both terminal screens, cursor/modes/palette and required shared attributes/hyperlinks, derived seat/subagent status, plus the semantic row-index/reflow generation needed to retrieve retained history. Logic produces it; supervisor stores and commits it. The terminal fork must provide a versioned serialization boundary rather than assuming the viewport's cell text is enough. Restoring reported activity never restores a native attachment lease or composer-ready proof.

Keep the checkpoint at/before the oldest replayable retained records. Evict only complete segments on checkpoint boundaries, after the replacement checkpoint and index are committed and no retained replay range depends on the deleted prefix. A newest-only checkpoint cannot reconstruct earlier screen history. If a requested row/cursor is outside the retained window, return a gap and the available range.

Semantic row IDs and raw-byte cursors are distinct. Resize/reflow, alternate-screen transitions and history clearing invalidate incompatible view/index references. Replaying history updates internal model/status/index state through an isolated event sink: it must not resend terminal replies, clipboard operations, state notifications or messages as live effects. Publish the consolidated recovered view after catch-up. While logic is unavailable, terminal queries may time out; process survival does not promise uninterrupted terminal responsiveness.

Data/checkpoint publication order is: write complete records and checkpoint blobs; flush their files; atomically publish the committed-boundary metadata; flush that metadata/directory as required; then retire unneeded segments. Keep an independently valid previous boundary until the new one is established. On supervisor restart, recover only that committed prefix and reject a torn suffix; report any newer previously observed tail as lost/unknown. For strict macOS media ordering use the platform's full-flush facility; this spec does not claim power-loss verification.

Captured/published and durable cursors are separate. Logic restart while the supervisor lives can use its valid captured tail. Supervisor/OS restart uses the recovered durable boundary and a new boot/process generation; it never tells the GUI that a rolled-back sequence is the same continuing stream.

Ring/checkpoint/index storage and replay work are bounded per seat and in aggregate. If logic is absent or behind and the ring has no safely evictable checkpoint boundary, supervisor **pauses PTY reads** and reports backpressure. It retains master fds and children; OS PTY buffers may then block child output. Resume capture when logic catches up and commits an evictable boundary. Do not silently discard unreconstructable bytes to keep the pipe moving.

## Restart and reattachment

| Event | Required sequence |
|---|---|
| GUI exits/restarts | Attach to current logic, authenticate, list seats, discard incompatible caches and request visible snapshots/history. No harness launch. |
| Logic dies | Supervisor keeps masters/children/rings, fences the old lease and starts/re-admits one replacement logic. Replacement lists ownership, restores checkpoints, replays through a captured boundary, produces a consistent view and then accepts new input. Buffered state is coalesced for the GUI. |
| Supervisor dies | Restart its private state owner, recover committed records, distinguish old process identities from new ownership, resume supported harness-session IDs as new processes. Archive the prior generation's display/history; preserve uncertain input outcomes. |
| Remote connection expires | Reauthenticate the expected device, compare supervisor/process/view generations and resume or request snapshots. Do not respawn a seat or reinstall software merely because a network connection closed. |
| Host/VM restarts | Same durable recovery and new-process resume path as supervisor death. No assertion of uninterrupted process life. |

Refuse incompatible state/checkpoint versions while retaining available history. An incompatible logic upgrade must not kill existing children as an incidental migration. Supervisor upgrade/shutdown is an explicit lifecycle operation because closing the final master can hang up children; it is not part of ordinary GUI restart.

## Logic ↔ GUI state-stream protocol

Use bounded reliable framed streams initially, with the same message semantics over local Unix IPC and remote authenticated Quinn. GUI subscribes to visible viewports and lazily requested history; hidden seats still run their VTE and retain bounded output but send only necessary state/metadata changes. Full-history mirroring and a FIFO of obsolete frames are excluded by the memory goal.

| Message | Required fields/behavior |
|---|---|
| `Attach / Resume / Subscribe` | Version/capabilities, authorized seats/ranges, device identity, supervisor boot ID, process generation, view generation and last applied sequence. |
| `Snapshot` | Complete subscribed viewport and dimensions, cursor, palette/attributes/hyperlinks, relevant display/input modes, title/cwd, retained-history range and capture/durability metadata. Apply atomically. |
| `Delta` | Same identities/generations, exact base sequence, new sequence and changed cells/rows/metadata. Apply only to that matching baseline; otherwise request a snapshot. |
| `Applied` | Last successfully applied view generation/sequence. It acknowledges state application, not packet receipt, rendering speed, storage durability or model consumption. |
| `GetHistory / History / Gap` | Semantic row ranges with index generation, or explicitly typed byte cursors; bounded response and retained/truncated range. |
| `Input / InputStatus` | Current writer/process lease and stable input ID, with evidence from the native adapter or supervisor. Preserve the distinctions in [[messaging]]. |
| `SeatState / Presence` | Owned-seat state or explicitly separate published phantom metadata. Visibility never changes access authority. |

Keep a bounded acknowledged baseline and one pending merged update per subscribed view. If the baseline is unavailable, send a fresh snapshot. Keep control/input and bulk history out of one shared ordered byte queue; separate per-seat render streams prevent one seat's stream ordering from blocking unrelated streams, while connection-wide congestion/resource limits still apply.

On reconnect, resize/reflow or process replacement, invalidate incompatible caches/generations before applying new data. Ignore duplicate/older sequences and stale generations. The GUI's presentation can be rebuilt from a snapshot; the daemon remains authoritative. Multiple viewers can subscribe, but one authorized writer lease controls a seat at a time.

Datagrams are a future negotiated capability, not the initial transport. If enabled, each complete update must fit the path/peer limit or use a bounded all-or-nothing fragmentation scheme; missing fragments never partially mutate a view. Identity/generation/baseline checks and application acknowledgments still apply. Recovery/full snapshots remain on streams. Replayable early data must not carry spawn, permission changes or terminal input.

## Remote seat bootstrap over SSH

Bring-your-own-host bootstrap uses the system SSH client and the user's host alias/configuration. Preserve SSH host verification. Use non-PTY control I/O; the remote supervisor creates the harness PTY. Remote-shell quoting is explicit: untrusted settings and enrollment material travel in framed stdin/protected descriptors, not interpolated command text or secret argv.

Bootstrap steps:

1. Probe the supported remote OS/architecture through fixed commands. Select and verify an exact compatible release locally; upload to a user-owned versioned location, stage it and atomically install. Fail on unsupported targets or incomplete verification.
2. Start/discover the same executable's daemon mode under a private per-user lock. A versioned bounded handshake returns the remote device public identity, daemon versions and bootstrap request ID. Diagnostics/banners are not accepted as that handshake.
3. Authorize and pin the remote device using [[messaging]]'s account→device→seat grant model through the verified SSH session. Private owner keys stay local. Enrollment and seat creation are idempotent requests; a repeated bootstrap does not create a duplicate daemon or harness.
4. Detach standard descriptors and establish the host's intended persistent user-service lifecycle. An SSH logout and a VM shutdown are different failures; logout/session-manager policy must not accidentally terminate the supervisor.
5. Connect through our Quinn direct/punch/E2E-relay transport, modelled on published ICQ 0.20.2 rather than the old `icq-connect` repo. Keep an authenticated SSH stdio proxy as the connectivity fallback. A VM's outbound connection alone does not prove reachability to a NATed laptop.
6. Reconcile seat/process/view generations and applied/history cursors; resume/snapshot existing seats. Network reconnection does not itself authorize a launch or update.

Fly, Hetzner, EC2 and GCP bootstrap hooks are documented in [[cloud-and-mosh-exec]]. Provider provisioning, service installation policy, UDP/firewall setup, cost controls and host lifecycle automation are future integrations; they do not change the seat protocol. Owned remote seats have screen/history permissions. Other people's published phantoms do not.

## Measured facts and budget assumptions

Evidence is in `../research/cloud-and-mosh-exec.md:59` and `:102`, with fixture paths/hashes and actual commands:

| Evidence | Status and consequence |
|---|---|
| Apple M4, alacritty_terminal 0.25.1, 80×24, 1000 empty models | Measured RSS delta 100512 KiB, about 98.16 MiB. This is not 1000 running PTYs/harnesses/connections. |
| Filled 10k-history models | Measured 20520 KiB/seat, 20.04 MiB, twice. Extrapolated 1000-seat total 19.57 GiB; rejects retaining full cell histories everywhere. |
| Printable fixture parse/allocation | Measured 146.9 MiB/s for the 10k-history workload. It is not a scheduler, renderer, Unicode/font or WAN benchmark. |
| Tail-only replay | Measured red foreground became default after evicting SGR setup. Establishes the need for stateful checkpoints. |
| Synthetic sole-master-owner SIGKILL | Child observed SIGHUP after 0.43 ms. Establishes the owner-survival boundary; does not measure the new split process design. |
| Toy append-log kill | One complete fsynced record recovered, 19-byte incomplete tail ignored. Demonstrates framed recovery only; not production ring or power-loss verification. |
| Payload-ring capacity | Estimated 1.95 GiB for 1000 full 2 MiB rings, or 3.91 GiB for 4 MiB; viewport models, checkpoint/index overhead, queues and harness memory are additional. |

The toy log used CRC32; this spec's CRC32C format and checkpoint publication sequence are unimplemented. The toy result proves only that its framed reader rejected its deliberately incomplete suffix.

Initial ring size/line goals come from [[vision]]'s assumed defaults, not a measured final capacity policy. Byte retention and checkpoint/index budgets must be configured together; a nominal byte cap must include a defined bound on large records and checkpoint storage. Schedule only subscribed frame work and coalesce state; defer frame-rate/performance tuning under the operator's working rules.

## Acceptance and remaining decisions

These are implementation acceptance criteria, not tests passed by this document:

- Kill the GUI and logic independently: the same supervisor-owned child/master remains, logic reconstructs the same VTE state, and stale logic/writer leases cannot inject.
- Force logic downtime until a ring fills: capture backpressures at a valid checkpoint boundary, preserves process ownership and resumes without a hidden gap.
- Inject partial/corrupt log records and interrupt checkpoint publication/eviction: recover only an intact committed generation/prefix, report missing history, and never allocate from an unchecked length.
- Replay retained SGR, split UTF-8/escape sequences, alternate-screen and resize transitions: restored state/indexes match the uninterrupted model; no external replay effects fire.
- Reconnect viewers with dropped/duplicate/stale frames and replaced processes: exact baseline/generation rules lead to a correct snapshot, not mixed screen state.
- Retry a completed input ID after logic restart and simulate an ambiguous partial write/owner failure: no duplicate PTY submission, explicit unknown outcomes, no automatic Enter replay.
- Kill the supervisor: supported harnesses resume with the same durable seat/session mapping and a new process generation; unsupported/missing mappings report their limitation honestly.
- Verify owned remote bootstrap retries, SSH fallback and permission checks; phantom presence never yields screen access, a writer lease or a spawned pane.
- Close then crash logic: the original child captures during grace, expiry still runs, and a matching undo races expiry without duplicate children. A closed seat is never automatically relaunched by open-seat recovery.

Original E document verification on 2026-10-21: `ls .kb/specs/mux-daemon.md` and section count returned 0 with ten level-two sections before the M lifecycle addendum; structure/source-pointer/whitespace validation returned 0; `git diff --check` returned 0. These checked the document, not the future implementation. No runtime tests were added or run for this convergence.

**Gap:** terminal-fork checkpoint serialization/version migration and semantic index representation must be implemented and validated; no existing viewport-only serializer is assumed.

**Gap:** exact framing limits, aggregate ring/checkpoint/cache budgets, flush/restart policy defaults and platform service packaging need bounded configuration choices before implementation. Their overhead and crash/power-loss behavior are unmeasured.

**Gap:** no split supervisor/logic implementation, real harness resume, remote/bootstrap deployment, WAN fault test or 1000-running-seat test exists in this convergence pass. Native adapter support and account/device revocation retain the gaps in their own specs.

The original E convergence wrote only this spec. The subsequent M addendum adds the close lifecycle and mute-policy fields here, alongside [[messaging]] and [[mcp-control-plane]]. No runtime, harness setting, provider resource or task-board change is included.

— Nyx Copperwing <EXECUTOR>
