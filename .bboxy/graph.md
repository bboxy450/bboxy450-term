# Execution graph — who is working on what

Oracle-maintained state. Updated on every claim, handoff, accept and block. Back to [[plan]] · board [[tasks/README]].

Last update: 2026-10-21 (Vesper Greywind <ORACLE>).

## Agents

| Agent | Role | Harness | Now | Blocked on |
|---|---|---|---|---|
| Vesper Greywind | Oracle + Creativity | Claude Code | this graph, specs review, briefs | nothing |
| Creativity (new session, key 22c0e081…, name pending; was Lumen Foxglove, session reset 2026-10-21) | Creativity | Claude Code | onboarding; then lab passes under the operator's direct review; design system v17 | nothing |
| Nyx Copperwing | Executor | Codex | **W3.2a ui-model: tokens, model, layout** | nothing |
| Rook Ashwhisker | Executor | Codex | idle | in-session authorization from the operator |
| subagent gui-spec | Oracle instrument | Claude | done: `specs/gui-components.md` | — |
| subagent net-spec | Oracle instrument | Claude | done: `specs/networking.md` | — |
| subagent provenance | Oracle instrument | Claude | done → `research/provenance-and-documents.md`, decided in `specs/provenance.md` | — |
| subagent state-backend | Oracle instrument | Claude | done → `research/state-backend.md`, decided in `specs/storage.md` | — |
| operator | operator | human | lab review with Lumen | Codex trust for the fixture dir; Rook's pane |

## Graph

```
specs (done): messaging, subagent-detection, mux-daemon, harness-adapters,
              mcp-control-plane, terminal-core, identity, renderer, ux-prototype(v2),
              work-items (epics/specs/tasks/bugs/notes/invariants/artifacts), provenance,
              networking, gui-components, storage, events
specs (done +2): gui-components ✔ ──▶ W3.2a brief ✔ [OPEN]; networking ✔ ──▶ W4.1 brief ✔ [OPEN]

W1.1 term-core fork ✔ (Nyx, 0e2342c, 117 tests)
   ├──▶ W3.1 renderer harness ✔ (Nyx, fcc8eca, 5 tests, 48 PNGs, CI green on macos-15)
   │       └──▶ W3.2a ui-model: tokens, model, layout [Nyx, RUNNING]
   │               └──▶ W3.2b bboxybook: atoms, chrome pass, modules, parity [briefed]
   │                       └──▶ W3.2c input, launcher, signals→fx
   │                               └──▶ W3.3 glow/locate/dim fx
   │                       └──▶ W3.4 app: window, sidebar, stage, launcher, GUI client
   └──▶ W1.2 byte ring [OPEN, after W2.1]

W2.1 supervisor + PTY + lease ✔ (Nyx, f6cab17, 12 tests, 50-seat poller)
   ├──▶ W1.2 byte ring [OPEN, claimable]
   ├──▶ W2.2 logic: store traits, registry, event bus, .bboxy index, state streams [OPEN, claimable]
   │       └──▶ W3.4 app GUI client
   └──▶ W2.3 adapters Claude/Codex/zsh [OPEN, claimable]
           ├── needs: osc7501-claude.bin ✔, osc7501-codex.bin ✘ (operator trust)
           └──▶ W2.4 local messaging on loopback (Transport trait from W4.1)
                   └──▶ W2.5 MCP server + seat sockets

W4.1 Transport trait + loopback + quinn auth [OPEN, no dependency]
   └──▶ phase 2 cloud: SshProvider bootstrap, remote supervisor
           └──▶ phase 2b private cross-instance messaging (punch, relay)
                   └──▶ phase 3 global pubsub, DHT, phantoms (iroh re-eval here)
```

## Open work by state

| State | Items |
|---|---|
| RUNNING | W3.2a (Nyx); lab passes (Lumen) |
| OPEN, claimable now | W4.1, W1.2, W2.3, W2.2 |
| OPEN, after W2.1 | none, W2.1 is done |
| PENDING SPEC | none |
| NOT YET BRIEFED | W2.4, W2.5 (incl. task_delegate), W2.6 ext events, W3.2c, W3.3, W3.4 |
| PARTIAL | H Codex capture (operator trust) |
| REDUNDANT, optional | Rook's `-exec` passes on A and H |

## Done

W1.1, W3.1, W2.1, A, B, C, D, E, F, G, H(Claude), I, I2, M, S, T, kittens, weavie, iTerm plugin, UX prototype v1 and v2.

## Rules for this file

One line per change. The Oracle edits it; Executors report on `#bboxy450-term` and the Oracle moves the line. A claim without an entry here is not a claim.
