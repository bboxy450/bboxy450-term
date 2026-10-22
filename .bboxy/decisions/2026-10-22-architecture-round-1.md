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
    ring replayed; the UI shows the restart. Unflushed capture bytes are lost
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
    Gap: the kernel's hard cap on the raise, unverified on this build.

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

## Specs to touch (Architect, in this order)

[[architecture]] drawn from the code ✔ → [[c1-live-seats]] ✔ (2, 6, 10, 11) → [[w2-5-mcp-and-seat-wiring]] ✔ (3) → [[d1-providers]] ✔ (7) → [[id1-seat-chain]] ✔ (4) → [[mux-daemon]] crate
split per 1 → [[open-source]] §1 per 8.

## Operator action pending

`.bboxy/runs/osc7501-codex*.bin` are empty: Codex status is proven by fixtures
only until the trust chooser is authorised in Codex's own pane.
