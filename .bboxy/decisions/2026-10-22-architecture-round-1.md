---
id: 01M5JFWDVF32QNCDMXC9K0T49T
type: note
title: architecture decisions, round 1 (operator + Architect, 2026-10-22)
links:
  - { to: "[[vision]]", rel: direction }
  - { to: "[[open-source]]", rel: changes }
  - { to: "[[layering]]", rel: invariant }
  - { to: "[[2026-10-22-c-cockpit]]", rel: epic }
created: 2026-10-22T19:02:24Z
updated: 2026-10-22T19:02:24Z
---
# Architecture decisions, round 1

Asked by Corvin Duskmantle <ARCHITECT>, answered by the operator on 2026-10-22,
from the six items Vesper Greywind <ORACLE> flagged as needing an architect.
Design principles restated by the operator the same day: **LESS IS MORE, KISS,
super good enough, not perfect.** Boundaries and crates may be imperfect; the
gate catches direction, not purity.
Each decision: chosen, rejected, why. Specs change on next touch; the Oracle
briefs Executors from the specs, never from this note.

1. **Crate topology: split by process boundary.** `bbt-supervisor` (mio, PTY
   masters, leases, ring writer) leaves `bbt-mux`; everything tokio stays
   (logic, store, bus, index, adapters, messaging, MCP). Crate line = process
   line = crash domain = runtime. Rejected: one crate until a plateau; split by
   domain (churns four in-flight tasks). When: after W2.4 merges, before W2.5.
2. **C1 binding: app IO thread + double buffer.** The IO thread owns the seat
   socket and applies snapshot and deltas into a back-buffer model; the UI
   thread swaps at frame start, never locks or waits ([[runtime]] §3). Logic
   coalesces per seat to the latest state when the app lags, scheduled by
   visibility; hidden seats send a state bit. Rejected: logic in-process; pull
   per frame.
3. **MCP server inside logic, stdio bridge per seat.** A seat launches with the
   private entry point `bboxy450-term mcp` as its stdio MCP server, carrying a
   per-seat capability token; logic resolves token → seat, role, grants and
   enforces may-send-to, read scope and spawn caps in one place. One shipped
   binary (specs/README S5). Rejected: own daemon; process per seat.
4. **Seat identity: full GPG chain in v0.1.** The root (844ECC0587DE7AB0,
   gpg-agent) certifies one Ed25519 device key per machine, held in macOS
   Keychain: one prompt per enrollment. The device key certifies each seat
   generation key in memory, no prompt. Envelopes verify through exported
   certificates; revocation at device level drops every seat under it. W2.4's
   HKDF leaf keys (issuer-kind local-dev, SeatKeys trait) are the placeholder
   this replaces; envelopes unchanged. Architect recommended deferring the root
   binding to the cloud phase; the operator overruled: lean in on identity.
   Rejected: device key as a GPG subkey (gpg-agent in the hot path); no seat keys.
5. **Theme is data.** One source, `design/tokens.toml`, becomes a Theme value at
   startup (hot-reload in the lab), passed by reference to layout and draw.
   bbt-ui holds a neutral default for tests; the bboxy theme is an asset of the
   app. Rejected: compile-time consts; trait object.
6. **Domain types go to a shared protocol crate.** Private `bbt-proto`: seat,
   swarm, tab, split, ids, snapshot and delta wire types, used by logic and app;
   the seat socket carries exactly these. bbt-ui-model keeps generic layout,
   draw, tokens, events. L7 modules still move to bbt-term-ui ([[open-source]]).
   Rejected: a bbt-model crate on top of ui-model (converts twice); feature flag.
   When: with the C1 spec; the ui-model split waits for 7.10 as planned.
7. **Provider yields a supervisor link.** `Provider` produces a SupervisorLink
   per host: LocalProvider returns the unix socket; FakeProvider wraps the local
   supervisor in a scripted link (connect, latency, drop, reconnect from
   fixtures); SshProvider later bootstraps a supervisor over ssh and tunnels the
   same protocol. Seats, rings and adapters never know where they run.
   Rejected: provider spawns seats (duplicates the protocol); data only.
8. **Everything goes open-source eventually.** The invariant is layering, not a
   fixed public list: every crate must stay publishable without the ones above
   it. Velocity first; nothing is split or renamed before its plateau.
   Supersedes the fixed public/private table in [[open-source]] §1 as the rule;
   the publish order there stands.
9. **Layering gate now.** [[layering]], `scripts/layering.sh`, in CI from this
   commit. Rejected: at O2; manual cargo tree.
10. **1000 seats: four measured gates before C1 ships.** Memory per seat
    (1000-seat headless fixture under an RSS budget); bus fan-out (throughput
    and p99 under Oracle bursts with the weighted fair queue); supervisor fds
    and poller (firehose at 1000 seats, p99 write latency); sidebar and minimap
    draw (1000 rows, 50 visible, hitch meter). Budgets come from the first
    measurement, never a guess ([[runtime]]).
11. **Supervisor death is survivable, not prevented.** The supervisor stays its
    own process. If it dies: the app restarts it, re-attaches, and every seat
    resumes by harness session id (`claude --resume`, `codex resume`) with the
    ring replayed; the UI shows the restart. Landed 2026-10-23 (C1c-1):
    the launch planner is the single resume path; the trait's
    `resume_command` is dead and goes in C1c-2. Unflushed capture bytes are lost
    and that is accepted; no third process, no shared-memory mirror. Within
    reason: velocity.

12. **Audience and the plugin seam (operator).** Hackers and furries first;
    cloud and ssh support that never locks anyone in; then teams and
    enterprises want it; opinionated, not locked; developers. **No plugin
    system yet.** Shape recorded for later: a binary in a folder, the terminal
    runs `binary --shape` to learn how to integrate, the plugin runs as a
    subprocess of our runtime and may expose sockets, a CLI or a daemon.
    First kind: cloud and ssh providers, the out-of-process form of #7.
    Rejected: a plugin API now (scope); a scripting runtime (not our shape).

13. **Document types: the seven in [[work-items]] plus `decision`.** epic, spec,
    task, bug, note, invariant, artifact stay. `decision` is added: a numbered
    record of alternatives, choice, reason and date, statuses proposed,
    decided, superseded, linking the specs it changes; this file is the first.
    Architecture is a spec (status decided, re-read on each round), not a
    type. Research, runs, runbooks and evidence stay `note`; a brief is a
    task; a report lives in its task; a release is an artifact; a question
    for the operator is a task assigned to the operator. Architect's call,
    operator may override. Rejected: `architecture`, `research`, `report`,
    `question` as types (each is a view of an existing one).
14. **The app reads its own `.bboxy`: the viewer is cockpit work (operator).**
    "I want to use our app to review our architecture and specs and docs."
    [[document-viewer]] (W3.5, `bbt-doc`: Markdown plus our own Mermaid
    engine) moves from not-yet-briefed into [[2026-10-22-c-cockpit]] as a
    track; the 7.10 DocumentModal is its L7 face. Harnesses write tasks,
    epics, specs and decisions as Markdown through the task tools (W2.5) and
    everyone, the operator first, reads them in the app. The Mermaid subset to
    build first is what `.bboxy` uses today: `flowchart` LR, TB, BT, TD with
    subgraphs, `&` fan-out, dotted edges, cylinder and stadium shapes,
    `classDef` dashed; `sequenceDiagram` with participants, `loop`, dashed
    replies. Wikilinks resolve to documents in the viewer. Rejected: a
    browser-based viewer (not our shape); waiting for L9.

15. **CI is local (operator, 2026-10-22).** No cloud CI during heavy
    development; GitHub Actions runs on manual dispatch only and comes back
    per crate when it is open-sourced. `sh scripts/ci-local.sh` is the gate:
    layering, build, test, ui-lint, and `--parity` when lab, design, UI or
    renderer changed; exit 0 is green. Every handoff and merge reports
    `sh scripts/ci-local.sh — exit 0 at <commit>` in the kb; that line is the
    CI evidence. Found by the Architect: no cloud run had started since 07:27
    UTC that day. Rejected: deleting the workflow; paying for cloud runners
    now; a second script (the Architect's `ci.sh` was removed in favour of the
    Oracle's, same gates).

16. **The Architect's four loads bind the task system (operator: "embody
    it", 2026-10-22; `~/.agents/AGENTS.md` Four roles).** Applied to
    [[work-items]], [[vision]] roles, [[messaging]] attention budget and the
    task tools: an invariant carries property, `check`, held exit status and
    `cost`, or it is a preference in a note; `accepted` requires `verify` at
    exit 0 plus evidence per acceptance line, an Executor's report stops at
    `reported`; `verify` blocks, `review` advises; a violated invariant blocks
    the task with the halt as a `decision`-kind message; decisions are a type,
    written when made. Architect is oracle-class, Creativity worker-class.
    `.bboxy/invariants/` holds four recorded today, all with commands.
    Rejected: a bottleneck-index telemetry (not our scale; ZOOM OUT does the
    job in four lines); a separate halt message kind (`decision` suffices);
    recording the five "preference" invariants as invariants before they have
    commands.

17. **Device-key custody has three tiers (Architect, from ID1a evidence).**
    The Data Protection Keychain returns `errSecMissingEntitlement` (-34018)
    to an unsigned binary; it needs `keychain-access-groups` on a signed app
    identity. So: tier 1 Data Protection Keychain in the shipped, signed app
    (lands with packaging, PKG1); tier 2 the login keychain with the same
    attributes for ad-hoc and dev builds, shown as "dev custody"; tier 3
    memory for tests; one `DeviceKeyStore` trait, fall from 1 to 2 on -34018,
    never a plaintext file. ID1a acceptance 4 amended. Team known the
    same day: Wild Honey on the Porch, LLC (UQB3368A84), Apple Development
    and Distribution certificates on this Mac, Developer ID Application
    still pending, so tier 1 is verifiable locally with a Development-signed
    bundle and distribution waits on that one certificate. Rejected: blocking ID1 on
    packaging (velocity); a plaintext or sqlite secret file (identity.md
    forbids it); requiring a team ID before any identity work.
18. **Claude Code may be an Executor when the operator launches it so
    (operator, 2026-10-22: "i gave you one more executor that is claude
    code").** The `~/.agents/AGENTS.md` default stands for sessions nobody
    assigned; an explicit operator launch outranks it, as that file says.
    Seraph Quillfire is the first. The terminal's harness-eligibility rule
    ([[mcp-control-plane]] Caller, role and grants) gains the same clause:
    the operator can assign any role to any seat; the signed class in the
    grant is what receivers trust. Rejected: editing AGENTS.md (the
    operator's file; its override clause already covers this).

19. **The PTY ceiling is a product fact, not a bug (from Seraph's C1d
    measurement).** macOS `kern.tty.ptmx_max` is 511 on this Mac, raisable
    only by a transient admin sysctl the kernel caps. Decided: capacity is
    published per host and shown on the host row; a spawn past it is refused
    with the reason named; the launcher offers the documented raise and
    another host; "1000 seats" is per swarm across hosts; per-Mac gates run
    at 500 by default and once at the raised ceiling. Rejected: the app
    running sudo or installing a LaunchDaemon itself (silent privilege, not
    our shape); lowering the target (the target is the swarm, hosts are the
    unit); pretending with pseudo-seats that have no PTY (a seat is a PTY).
    Gap closed the same day: the raise ceiling is 999 on this build (999 accepted, 1100 refused); the at-limit help's LaunchDaemon sets 999.

20. **A seat is a PTY; a pipe-mode harness run is a job (from the measured
    PTY cap).** `sudo sysctl -w kern.tty.ptmx_max=1100` returns Invalid
    argument; the believed hard cap is 999. Decided: (a) per-machine gate
    budgets are set at the machine's measured maximum; (b) headless harness
    runs (`claude -p`, `codex exec`) never become seats; they are **jobs**
    on the existing `spawn_command` path, a row under the launching seat
    with Inferred grade, no screen, no input, no PTY, and therefore not
    capped; the seat model, the sidebar and the 7501 observation stay
    untouched; (c) at the cap the product refuses with the reason and
    offers another host (#19). "1000 agents" is a swarm across hosts, or a
    few hundred seats plus any number of jobs. Rejected: pipe-mode seats
    (a seat without a screen breaks every seat invariant: observation,
    guard, scrollback, resume); raising the target by lying about PTYs.
    A job row spec is written when a brief needs it, not before.

21. **Ring durability has two tiers (from the C1d measurement: logic dies at
    ~45 seats).** Tick commits are ordered with a barrier, not flushed;
    full flushes happen at checkpoint, seal, shutdown and every 5 s or
    16 MiB per machine. Loss bounds stated in [[mux-daemon]] Durability
    tiers: crash ≤ one tick per seat of admitted-minus-committed bytes, the
    pre-capture PTY tail reported separately (ruled 2026-10-22 when Rook's
    check showed the first wording counted kernel-buffer bytes the ring
    never had); power cut ≤ 5 s or 16 MiB by an admission rule that flushes
    at 16 MiB undurable; never a torn record. Busy is backpressure to the PTY, never fatal. Rejected:
    full flush per tick (the measured failure); no flush except shutdown
    (unbounded power-cut loss); a writer thread per seat (1000 threads is
    not a design). The numbers bind when W1.2d's gate measures them.

22. **One tick journal per machine, not barrier threads (from Rook's
    measurement: cost is per file, two files per seat per tick; Busy at
    seat 58 after group commit).** All seats append to one journal per
    supervisor process with a tick-commit record as the boundary, so a
    tick costs one barrier at any seat count; per-seat rings are compacted
    from it lazily; recovery replays the journal tail; loss bounds and
    never-delete unchanged ([[mux-daemon]] Durability tiers). Rejected: four
    barrier worker threads (a constant factor, still O(seats), and CPU
    burned on fcntl); dropping the marker file alone (halves it, still
    O(seats)); a journal per swarm (swarms are not a machine boundary).

23. **Deliveries batch per recipient (from Seraph's gate 2: a 30-message
    burst to 1000 seats took about 120 s because the pump moves 32
    deliveries per 100 ms with an inbox batch of one, 30 000 deliveries).**
    Decided: one wake per recipient per pump pass carrying all its pending
    messages, so the burst is 1000 deliveries and the cap bounds recipients;
    the pump cap stays 32 per 100 ms for now (about 3 s to reach 1000
    seats), re-measured after W2.4c, and gate 2's budget is set from that
    measurement. Rejected: a 150 s budget (enshrines a pump ceiling as a
    property); lifting the in-flight cap alone (floods harness sockets and
    still scales with messages). Gate 1 idle budget set: 420 MiB at 400
    seats, +25 % over the measured median; the loaded number is still owed.

24. **No code signing until we distribute (operator, 2026-10-22; velocity
    first).** The PKG0 cargo runner is removed from main the day it landed,
    PKG1 is deferred, `scripts/sign-run.sh` is kept for later; nothing may
    depend on a signed build or a keychain prompt. Custody stays at tier 2
    with ID1e's non-interactive, bounded keychain access on every developer
    machine; tier 1 returns with the signed bundle at distribution. #17's
    tiers are unchanged, only their timeline. Rejected: keeping the runner
    for its keychain-ACL stability (it put a signing step and a certificate
    dependency in every cargo test for a benefit ID1e made unnecessary).

25. **The fake provider delays events, not requests (from D1b).**
    `SupervisorConnection::request` stays synchronous; `FakeLink` simulates
    RTT only on queued events and host-state transitions via the due-time
    queue, and a request to a down host fails immediately with the named
    link-unavailable error, never executing. Reason: dummy providers do not
    justify a protocol change for cosmetic latency. Rejected: an async
    request path (touches every caller in logic for a fake); sleeping in
    the writer thread (blocks all seats).

26. **Terminals own every plain key (2026-10-23, Oracle finding in
    [[gui-components]] §4.4).** With a terminal pane focused, Tab, arrows
    and every other plain key go to the PTY; pane navigation is ⌘⌥ arrows
    (spatial) and ⌘⌥ Tab (tree order); plain Tab and arrows navigate only
    in the launcher, overlays, text fields and app chrome. Rejected: plain
    Tab/arrows for pane navigation (steals them from shells and TUIs);
    a modal "navigation mode" (one more state the operator must notice).

27. **Two C1c-2 wire rulings (2026-10-23, Oracle).** (a) `GuiAction::UndoClose`
    carries `layout_revision` for concurrent-layout conflict detection; the
    close revision comes from the durable operation id. (b) `GuiReply::History`
    is capture-sequence based: bounded `(sequence, bytes)` records, explicit
    `Gap` entries in order, `next: Option<sequence>`; not a row pager. Rows
    come from GUI-side replay through a `bbt-core` `Term`, later. Rejected:
    a row pager on the wire (logic would keep a second parsed history per
    client, against the viewport-grid-plus-ring memory model); undo without
    a layout revision (a race the operator would see as a seat landing in
    the wrong split).

28. **`verify_result` is structured and `task_accept` checks it field by
    field (2026-10-23, Oracle, W2.5d).** `verify_result = {command,
    exit_code, evidence: [{criterion, reference}]}`. Acceptance requires
    `command` equal to the task's `verify`, `exit_code` 0, and, when the
    task lists acceptance criteria, one evidence entry per criterion. Task
    swarm scope (exact wording from W2.5d, Kestrel via the Oracle): every
    present seat reference among `assignee` and `delegated_by` must resolve
    to the caller's registry swarm; one alone anchors, two must match each
    other, none is ambiguous and refuses; never from a path. A fresh
    `assignee: none` task with `delegated_by: seat:<Oracle>` is delegable. Rejected:
    a free-text evidence blob (nothing to check against); trusting the
    reported command string without comparing it to `verify` (a passing
    `true` would accept anything); swarm from the task's path (paths move,
    registry rows do not).

29. **Explicit seat placement on the GUI wire (2026-10-23, Nyx's read-only
    seam proposal, accepted with two simplifications).** `SpawnSpec.placement`
    optional, captured at the pick (`NewTab{tab}` or `Split{tab, target,
    target_generation, side}`); `Spawn{id, spec}` with a caller-minted id,
    `Spawned{op, seat}` and `Error{op}` correlate by it; pending placement
    held in the immutable model until the matching `Spawned`; metadata
    pushes never place; anchor gone or changed → seat kept, named partial
    outcome, no silent split, no relaunch; reconciliation touches only
    affected leaves. Simplified: the retry ledger is the existing control
    op journal (the spawn id is an op_id), and the protocol boundary is the
    GUI socket's existing admission `version`, bumped, no capability
    negotiation. Rejected: resolving placement from focus at reply time
    (the bug); a separate placement message after spawn (two round trips
    and a window where the seat has no home).

30. **Three W2.6 rulings (2026-10-23, Oracle, recorded in [[provenance]] and
    [[events]]).** (a) Attribution ambiguity: several seats matching a
    worktree and activity window → `produced_by: null`, `grade: inferred`,
    bounded candidates, a named reason; an authenticated claim resolves it;
    an owner is never invented. (b) Signing: signed rows plus the hash
    chain are the acceptance; observer-made detections are attributed to a
    seat but signed by the observer; daily GPG checkpoints stay pending
    while the root is held, with no device-signed substitute. (c) Scope:
    cockpit-local (filesystem, hooks, git metadata, local GUI events); GitHub
    ingress, webhooks, cloud and hook installation deferred; split 2.6a
    detection/claims/log and 2.6b GUI events lane after L8-D's version bump.
    Rejected: picking the most recent seat as owner (a guess signed as a
    fact); observer signing with a seat's key (a key the observer does not
    hold); device-signed daily checkpoints standing in for the root
    (changes what the checkpoint proves).

31. **GUI operations are journaled under an operator principal bound to the
    device, in the account object (2026-10-23; unblocks L8 D / decision #29).**
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
    unknown and never repeated automatically (#29). Admission on `gui.sock`
    stays version plus capability; logic attaches its own device id as the
    principal, since the capability already proves the connection came from
    the app instance logic spawned, so no new secret and no new handshake.
    **Scope for L8 D grows** to `mcp/mod.rs` (principal enum), `mcp/journal.rs`
    (location by principal), `logic.rs` (`gui_spawn` through the journal,
    principal at admission) and tests. **Rejected:** a synthetic GUI seat (a
    seat is a PTY, #20, and it would need a grant chain for a thing that is
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
    opt-in) has a fresh device per process by #17, so its journal is
    per-process, which is the stated intent. The only case with no device
    id is a custody **hold** (ID1d: store unavailable at start); then GUI
    operations queue as `Held` exactly like messaging sends and release on
    recovery. No nil, random or seat fallback, ever.

32. **The kitty keyboard protocol is a core terminal invariant (operator,
    2026-10-23, verbatim: "also for terminals we should adotp Kitty keyboard
    protocol!", "document that as a core invariant of our terminal
    implementation").** Recorded as [[kitty-keyboard]]: one app encoder
    (`keyboard.rs`, a port of alacritty's `build_sequence`) encodes every
    key against the focused seat's live `TermMode` from `Screen.modes`, full
    progressive-enhancement stack with push/pop/query, legacy encoding when
    no flags are set, no second key path to the PTY. Check: one named unit
    test that must exist and pass (a cargo filter matching nothing exits 0,
    so the check greps for exactly one passed). K1 (Sable) builds it;
    [[terminal-core]] row updated. Rejected: encoding in logic (the mode is
    live in the app's model and the PTY write already has one id-deduped
    path); a second encoder for injected text (two encoders drift);
    enabling the protocol without the invariant (a preference, not a gate).

33. **Vendor the operator's `reactor!` macro as `bbt-yarn` (operator,
    2026-10-23: "if yes just create this as bboxy450-crate, name it somehow
    sexy").** Yes, because the two scale bugs of 2026-10-22 were arbitration
    policy hidden in hand-written loops and this macro makes that policy a
    compiler-checked declaration; starvation waivers are decision records in
    code. Spec [[yarn]]: vendored unchanged from `Gonzih/kittens-old`
    293e134 (the operator's own code; provenance in the README; license
    **MIT OR Apache-2.0**, set by the operator as author on 2026-10-23), first in the
    layering order, adopted one loop at a time starting with the app IO task
    where operator input must never starve behind deltas, with a no-raw-
    select source scan per adopted file. Rejected: depending on the
    archived crate (a retired identity's archive is not a supply chain);
    adopting everywhere at once (logic's Worker loop is mid-churn and sync);
    renaming the grammar (the KTR codes and agent guide are the value).
    Names rejected: litter, ears, pounce, purr. Timing (operator, same
    day): "document this design into our docs first; maybe useful now or
    later": the design is in [[yarn]] §0; Y1 runs when a loop needs it or
    an Executor is free, not ahead of the cockpit tracks.

34. **Licensing: dual MIT OR Apache-2.0 by default, `bbt-core` Apache-2.0
    only (operator, 2026-10-23: "lets do dual license, be smart about this,
    ask architect as well what project will have what license").** The map
    is [[licensing]] §1. The one exception is forced by provenance: a fork
    of Apache-2.0-only `alacritty_terminal` cannot be offered under MIT.
    Every publishing crate carries its own license files; root gets
    LICENSE-MIT, LICENSE-APACHE and a NOTICE inventory of vendored code;
    vendored headers stay verbatim. The sweep is an Executor task with
    `scripts/licenses.sh` as its gate. Supersedes the two earlier yarn
    answers (MIT only; Apache only): yarn stays dual, as its upstream is.
    Rejected: Apache-only for the whole project (drops GPLv2
    compatibility and mismatches the ecosystem for a patent clause a user
    can already elect); MIT-only (no patent grant, and impossible for
    bbt-core); relicensing alacritty code (not ours to relicense).

35. **The pane glow is the alive rim inside the border (operator,
    2026-10-23, verbatim: "I ACTUALLY LOVE RIM EFFECT", "8px alive rim is
    the thing we want", "go with it").** Spec [[rim-glow]]: a fixed 2 px rim
    plus a bloom to depth L (the Settings value, 2–8, default 8), two
    counter-moving waves snapped to the perimeter, seat colour hue-rotating
    along the perimeter with a white mix on crests, pulse by activity, the
    lab's activity table, focus ×1.3; the outside 8 px silk lane retires,
    the gutter becomes a fixed 2 px, the Settings row keeps 2–8 px and now
    means rim depth, drawn inside the pane padding, never over text; two
    invariants with named tests; the `materials.wgsl` lane pass keeps the
    silk machinery and changes geometry to the inside perimeter strip, no
    blur; R12-3 retargeted to the rim pass; R4c glow numbers re-measured.
    First written for the plain 2 px variant; superseded the same evening
    by the operator's choice of `alive-rim` in Wren's lab. Cell edge rule:
    the band is clipped to the pane padding and the invariant measures the
    whole band; padding does not grow; rejected a core-only invariant
    (permits bloom over text) and growing the padding (layout following a
    glow setting, against the operator's tighter panes). Rejected: keeping
    the lane as an option (two glow models in parity); letting depth move
    the gutter; drawing over the cell inset.

36. **Code adapted from Apache-only sources lives in `bbt-core`, never in a
    dual crate (2026-10-23; from LIC1 and K1).** The kitty key encoder is a
    port of alacritty `input/keyboard.rs build_sequence` at 5421470
    (Apache-2.0 only), so it is `crates/bbt-core/src/keyboard.rs`, with the
    upstream header and a "modified by bboxy450" line, and takes a neutral
    key model (key, modifiers, text, state) that the app maps winit events
    to, keeping bbt-core free of UI toolkit types. The app crate stays
    MIT OR Apache-2.0 for its own code only. General rule for [[licensing]]:
    an Apache-only adaptation joins the Apache-only terminal crate; a dual
    or MIT adaptation may join a dual crate with its notice. Rejected: a
    mixed SPDX expression on the app, "(MIT OR Apache-2.0) AND Apache-2.0"
    (correct but opaque to every reader and tool); a new one-file Apache
    crate (a crate per license is not a design); keeping the encoder in the
    app under Apache alone (silently makes the app Apache-only).

37. **Messaging and tasks reach the app through two wrapper variants on
    the GUI wire and one events lane (2026-10-23, Oracle's ask for L9-M).**
    `GuiAction::Messaging {{ id, request }}` wraps the whole W2.4 API and
    `GuiAction::Task {{ id, request }}` mirrors the W2.5d tool arguments;
    the W2.4 request and wire types move to `bbt-proto::messaging` (as #6
    did for streams); logic dispatches through the same functions the MCP
    tools use, under `Operator {{ device_id }}` (#31), journaled and replayed
    by id; every push to the app, messages, receipts, task status, seat
    state, artifacts and audit, rides Kestrel's W2.6b lane from one durable
    consumer per device, and unread is derived from that cursor. Spec
    [[l9-messaging-tasks]]. Rejected: one GUI variant per tool (twenty
    variants that drift from the tools); a second validation path for GUI
    callers (two authority models); a separate message stream beside the
    events lane (two cursors, two reconnect stories); storing unread counts
    (a second source of truth).

38. **L9-M implementation rulings (2026-10-23; Kestrel's five, Nyx's four).**
    (a) Operator-originated messages and tasks are signed by the device key
    with `seat = nil`, `role = operator (Oracle class)`, `role_grant_id` the
    DeviceGrant; `mcp::Actor` gains the operator variant; `delegated_by:
    operator:<device_id>` is a reference kind; no synthetic seat. (b) The
    GUI task wire equals the W2.5d tool arguments as built; `spawn_spec` is
    off the wire (spawn then delegate). (c) Events: one consumer per device
    with per-stream cursors, `Event{stream, seq, …}`, `EventAck{stream,
    seq}`, named `Gap`; ack after publish, never on enqueue; unread is
    operator-read from a durable read-mark in the account object, not the
    consumer cursor; `operator/inbox/*` is a new retained namespace while
    `inbox/>` stays fanout; one active attachment per device, filter change
    = versioned name; `Start::New`, read-through history, bounded quanta.
    (d) Projections `MessageRow`, `TaskCard`, `TaskReport` in bbt-proto with
    the work-items status set canonical and the lab adopting its names;
    `p1` is worker-class priority ≥ 7 on the 0–9 scale (restated from a
    wrong 0–255 threshold the same day). (e) Residuals from Nyx: under
    local-dev the operator's `role_grant_id` is the digest of the persisted
    first-launch record `identity/local-issuer` and `role_epoch` its epoch
    (corrected the same day from `identity/issuer`, which exists only after
    a cutover or reset; Nyx), never a minted grant, no IssuerState created; the operator destination is the sink `operator/inbox` in every
    swarm tree with recipient reference `operator:<device_id>`, delivering
    to the retained `operator/inbox/message`; replies address the same sink. (f) Every GUI messaging request names
    one `swarm_id`; `Send` and `Receipts` must agree with the swarm inferred
    from destination or message id; no account-wide union on the wire, the
    GUI composes per-swarm pages and per-swarm read-marks (rejected: an
    invented cross-swarm cursor; picking the first swarm).
    Rejected: an app-owned cursor tail instead of a durable consumer (loses
    restart-resume); ack on write (loses rows on a crash between poll and
    socket); unread from the consumer cursor (a restart would mark everything
    read, and human reading would throttle redelivery to `max_deliver`);
    making `inbox/>` retained (changes every seat's delivery semantics for the
    operator's convenience); a native label-mapping table for the lab's
    `assigned`/`done` (two vocabularies for one state).

39. **The suite is the product; the terminal is its first app (operator,
    2026-10-23, verbatim: "all those libraries are reusable parts, all those
    crates are reusable part of #bboxy450-* suite, term is just a way we
    glue them together, later on we will build more apps using our GUI kit
    that all connects together!").** Consequences: the app crate is glue
    (cockpit, live binding, main) and nothing else; when a module in the
    app could serve a second app it moves down a layer in the same brief;
    the kit is `rt`, `yarn`, `ui-core`, `render`, `ui`, `doc`, `mermaid`,
    `term-core`, `pty`; the connective tissue between apps is `proto`, the
    bus and messaging under the same identity chain, so a second app is
    another client of a logic process, or its own logic on the same bus,
    never a fork of the terminal. [[layering]] is the enforcement that
    already exists; the app's line count against the crates is an advisory
    drift signal, read at each architecture round, not a gate. Rejected: a
    monolithic app with internal modules (every later app re-extracts
    them); a plugin host as the suite's shape now (#12 keeps plugins for
    providers, later).

40. **Scrollback replay bounds (2026-10-23, C1c-3, Sable's two questions).**
    (1) The resident parsed window rolls; a seek outside it re-replays from
    the nearest ring checkpoint before the target, so any depth is
    reachable and "top is line 1" is by re-replay, not residency. (2) The
    4 MB cap is total heap of the history Playback per watched seat with a
    dimension-aware line cap, floor three viewports, ceiling 10k; the
    vision's "10k lines or 4 MB" is the raw ring's retention, a different
    thing. A narrow bounded-storage and scroll-row API goes into bbt-core
    (its own code, Apache-only crate) because every app with scrollback
    needs it (#39), no wire or renderer change. (c) The replay anchor is
    the capture start at current dimensions, the wire having no checkpoint
    or resize payload; checkpoints on the wire only if the 20k-line seek
    cost measures bad. (d) The 4 MB hard cap beats the three-viewport
    floor: an unfit dimension returns `history_viewport_exceeds_budget`,
    live unaffected (Oracle, same day). Rejected: an
    oldest-accessible limit at 10k lines (the ring has the bytes, refusing
    to show them is a product bug); counting raw bytes only (the grid is
    the heap that matters); the API in the app (it is terminal model, not
    glue).

## Specs to touch (Architect, in this order)

[[architecture]] drawn from the code ✔ → [[c1-live-seats]] ✔ (2, 6, 10, 11) → [[w2-5-mcp-and-seat-wiring]] ✔ (3) → [[d1-providers]] ✔ (7) → [[id1-seat-chain]] ✔ (4) → [[mux-daemon]] crate
split per 1 → [[open-source]] §1 per 8.

## Operator action pending

`.bboxy/runs/osc7501-codex*.bin` are empty: Codex status is proven by fixtures
only until the trust chooser is authorised in Codex's own pane.
