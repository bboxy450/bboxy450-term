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
| H | [[2026-10-21-subagent-detection-research]] OSC 7501 live wire capture | Nyx: Claude PROVEN, Codex blocked on trust chooser (operator action) | PARTIAL |

| W1.1 | [[2026-10-21-w1-1-term-core-fork]] fork alacritty_terminal + vte into bbt-core | Nyx DONE, verified: 0e2342c signed, 117 tests, API as specced | DONE |
| I2 | identity.md rewrite on the GPG root | Nyx DONE, accepted | DONE |
| W3.1 | [[2026-10-21-w3-1-renderer-harness]] headless render harness, smoke, first text | Nyx DONE, verified: fcc8eca, 5 tests, 48 PNGs, CI green | DONE |
| W2.1 | [[2026-10-21-w2-1-mux-supervisor]] supervisor, PTYs, lease protocol | Nyx Copperwing <EXECUTOR> | ASSIGNED |
| W1.2 | [[2026-10-21-w1-2-byte-ring]] byte ring, checkpoints, restart | free Executor after W2.1 | OPEN |
| W2.3 | [[2026-10-21-w2-3-adapters-claude-codex-zsh]] adapters: Claude, Codex, zsh | free Executor after W2.1 | OPEN |
| W3.2 | GPU GUI component library | brief after `specs/gui-components.md` lands | PENDING SPEC |
| W4.1 | [[2026-10-21-w4-1-transport-trait]] Transport trait, loopback, quinn endpoint auth | free Executor | OPEN |

Rule: redundant passes write `-exec.md` next to the subagent's file. The Oracle converges both into `specs/`. Claim by replying on `#bboxy450-term` with `Name <EXECUTOR> CLAIM <Id>`. Finish one, claim the next.
