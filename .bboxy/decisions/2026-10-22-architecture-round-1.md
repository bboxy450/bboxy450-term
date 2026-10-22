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

## Specs to touch (Architect, in this order)

[[architecture]] drawn from the code (new) → C1 spec (2, 6, 10, 11) → W2.5 and
W2.5b spec (3) → D1 spec (7) → [[identity]] custody per 4 → [[mux-daemon]] crate
split per 1 → [[open-source]] §1 per 8.

## Operator action pending

`.bboxy/runs/osc7501-codex*.bin` are empty: Codex status is proven by fixtures
only until the trust chooser is authorised in Codex's own pane.
