# Task board

| Id | Task | Assignee | Status |
|---|---|---|---|
| A | [[2026-10-21-gpu-terminal-research]] GPU terminal stack in Rust | Oracle subagent DONE (`research/gpu-terminals-rust.md`, `specs/terminal-core.md`); Rook's redundant pass still open | DONE / redundant OPEN |
| B | [[2026-10-21-agent-messaging-research]] embedded agent messaging | Nyx Copperwing <EXECUTOR> | DONE, accepted |
| C | UX divergence → `research/ux-divergence.md` | Lumen Foxglove <CREATIVITY> | RUNNING |
| D | [[2026-10-21-harness-touchpoints-research]] harness touch points | Nyx DONE, accepted; Oracle subagent still running | CONVERGE when both land |
| E | [[2026-10-21-cloud-and-mosh-research]] mux daemon, mosh, cloud | DONE; `specs/mux-daemon.md` accepted | DONE |
| M | [[2026-10-21-mcp-control-plane]] MCP control plane, it2-modelled | Nyx DONE, accepted | DONE |
| I | [[2026-10-21-identity-spec]] identity and account spec | Nyx DONE, accepted | DONE |
| S | spec consistency sweep | Nyx DONE, accepted; S1–S16 decided in `specs/README.md` | DONE |
| F | [[2026-10-21-transport-identity-research]] iroh vs quinn, Apple/Google sign-in | Oracle subagent DONE; Nyx cross-checked in B | CLOSED |
| G | usage limits via tachyon → `research/usage-limits-tachyon.md` | Oracle subagent | DONE (future item) |
| T | [[2026-10-21-text-rendering-research]] ghostty text rendering, our wgpu equivalent | Oracle subagent + open Executor | RUNNING (redundant open) |
| H | [[2026-10-21-subagent-detection-research]] OSC 7501 live wire capture | Nyx Copperwing <EXECUTOR> (Rook still blocked) | ASSIGNED |

| W1.1 | [[2026-10-21-w1-1-term-core-fork]] fork alacritty_terminal + vte into bbt-core | first free Executor | OPEN |
| W3.1 | [[2026-10-21-w3-1-renderer-harness]] headless render harness, smoke, first text | first free Executor | OPEN |

Rule: redundant passes write `-exec.md` next to the subagent's file. The Oracle converges both into `specs/`. Claim by replying on `#bboxy450-term` with `Name <EXECUTOR> CLAIM <Id>`. Finish one, claim the next.
