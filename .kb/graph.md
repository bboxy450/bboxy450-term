# Execution graph — who is working on what

Oracle-maintained state. Updated on every claim, handoff, accept and block. Back to [[plan]] · board [[tasks/README]].

Last update: 2026-10-21 (Vesper Greywind <ORACLE>).

## Agents

| Agent | Role | Harness | Now | Blocked on |
|---|---|---|---|---|
| Vesper Greywind | Oracle + Creativity | Claude Code | this graph, specs review, briefs | nothing |
| Lumen Foxglove | Creativity | Claude Code | lab passes under the operator's direct review; design system | nothing |
| Nyx Copperwing | Executor | Codex | **W3.1 renderer harness** | nothing |
| Rook Ashwhisker | Executor | Codex | idle | in-session authorization from the operator |
| subagent gui-spec | Oracle instrument | Claude | drafting `specs/gui-components.md` | nothing |
| subagent net-spec | Oracle instrument | Claude | drafting `specs/networking.md` | nothing |
| operator | operator | human | lab review with Lumen | Codex trust for the fixture dir; Rook's pane |

## Graph

```
specs (done): messaging, subagent-detection, mux-daemon, harness-adapters,
              mcp-control-plane, terminal-core, identity, renderer, ux-prototype(v2)
specs (drafting): gui-components ──▶ W3.2 brief
                  networking     ──▶ W4.1 brief

W1.1 term-core fork ✔ (Nyx, 0e2342c, 117 tests)
   ├──▶ W3.1 renderer harness  [Nyx, RUNNING]
   │       └──▶ W3.2 GUI components [PENDING SPEC]
   │               └──▶ W3.3 glow/locate/dim fx
   │                       └──▶ W3.4 app: window, sidebar, stage, launcher, GUI client
   └──▶ W1.2 byte ring [OPEN, after W2.1]

W2.1 supervisor + PTY + lease [OPEN, no dependency]
   ├──▶ W1.2 byte ring [OPEN]
   ├──▶ W2.2 logic: seat registry, state streams Snapshot/Delta/Applied
   │       └──▶ W3.4 app GUI client
   └──▶ W2.3 adapters Claude/Codex/zsh [OPEN]
           ├── needs: osc7501-claude.bin ✔, osc7501-codex.bin ✘ (operator trust)
           └──▶ W2.4 local messaging on loopback (Transport trait from W4.1)
                   └──▶ W2.5 MCP server + seat sockets

W4.1 Transport trait on quinn + loopback + provider traits [PENDING SPEC]
   └──▶ phase 2 cloud: SshProvider bootstrap, remote supervisor
           └──▶ phase 2b private cross-instance messaging (punch, relay)
                   └──▶ phase 3 global pubsub, DHT, phantoms (iroh re-eval here)
```

## Open work by state

| State | Items |
|---|---|
| RUNNING | W3.1 (Nyx); gui-components spec; networking spec; lab passes (Lumen) |
| OPEN, claimable now | W2.1 |
| OPEN, after W2.1 | W1.2, W2.3 |
| PENDING SPEC | W3.2, W4.1 |
| NOT YET BRIEFED | W2.2, W2.4, W2.5 (incl. task_delegate), W3.3, W3.4 |
| PARTIAL | H Codex capture (operator trust) |
| REDUNDANT, optional | Rook's `-exec` passes on A and H |

## Done

W1.1, A, B, C, D, E, F, G, H(Claude), I, I2, M, S, T, kittens, weavie, iTerm plugin, UX prototype v1 and v2.

## Rules for this file

One line per change. The Oracle edits it; Executors report on `#bboxy450-term` and the Oracle moves the line. A claim without an entry here is not a claim.
