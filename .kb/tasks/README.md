# Task board

| Id | Task | Assignee | Status |
|---|---|---|---|
| A | [[2026-10-21-gpu-terminal-research]] GPU terminal stack in Rust | Rook Ashwhisker <EXECUTOR> | RUNNING, authorized by operator |
| B | [[2026-10-21-agent-messaging-research]] embedded agent messaging | Nyx Copperwing <EXECUTOR> | DONE, accepted |
| C | UX divergence → `research/ux-divergence.md` | Lumen Foxglove <CREATIVITY> | RUNNING |
| D | [[2026-10-21-harness-touchpoints-research]] harness touch points | Oracle subagent + Nyx Copperwing <EXECUTOR> | RUNNING (redundant) |
| E | [[2026-10-21-cloud-and-mosh-research]] mux daemon, mosh, cloud | Oracle subagent DONE + unclaimed Executor | OPEN (redundant) |
| F | [[2026-10-21-transport-identity-research]] iroh vs quinn, Apple/Google sign-in | Oracle subagent DONE; Nyx cross-checked in B | CLOSED |
| G | usage limits via tachyon → `research/usage-limits-tachyon.md` | Oracle subagent | DONE (future item) |
| T | [[2026-10-21-text-rendering-research]] ghostty text rendering, our wgpu equivalent | Oracle subagent + open Executor | RUNNING (redundant open) |
| H | [[2026-10-21-subagent-detection-research]] how iTerm2 detects native Claude subagents | Oracle subagent DONE (OSC 7501 found) + Rook after A: live wire capture | RUNNING |

Rule: redundant passes write `-exec.md` next to the subagent's file. The Oracle converges both into `specs/`. Claim by replying on `#bboxy450-term` with `Name <EXECUTOR> CLAIM <Id>`. Finish one, claim the next.
