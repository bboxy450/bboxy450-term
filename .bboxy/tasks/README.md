# Task board

**Top priority (operator, 2026-10-22): the visual foundation**: GPU renderer, component library, visual parity and fidelity. **Direction: cockpit first.** Epic [[2026-10-22-c-cockpit]]: terminal, harnesses, local messaging, tasks, provenance, events, every seat wired with MCP, hooks and skills, in the sexy UI; ssh and cloud are dummies until the cockpit works.

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
| W2.1 | [[2026-10-21-w2-1-mux-supervisor]] supervisor, PTYs, lease protocol | Nyx DONE, verified: f6cab17, 12 tests | DONE |
| W1.2 | [[2026-10-21-w1-2-byte-ring]] byte ring, checkpoints, restart | Rook Ashwhisker <EXECUTOR> | DONE ✔ c69fe4c, W1.2b ✔ b12dfcb; W1.2c ✔ 01d5ea4 |
| W2.3 | [[2026-10-21-w2-3-adapters-claude-codex-zsh]] adapters: Claude, Codex, zsh | Rook Ashwhisker <EXECUTOR> | DONE ✔ d60ee06 |
| W3.2a | [[2026-10-21-w3-2a-ui-model]] ui-model: tokens, model types, layout engine | Nyx DONE, verified: 28 tests, no runtime deps | DONE |
| W3.2b | [[2026-10-21-w3-2b-bboxybook]] bboxybook: draw atoms, chrome pass, modules, native storybook with parity | Nyx DONE at checkpoint ef6c7ff | DONE |
| W3.2c | [[2026-10-21-w3-2c-fixtures-input-fx]] shared fixtures ✔; input/focus, launcher, signals→fx | step 0 DONE (Nyx); steps 1–3 deferred after W3.2d modules | PARTIAL |
| W4.1 | [[2026-10-21-w4-1-transport-trait]] Transport trait, loopback, quinn endpoint auth | — | DEFERRED (cockpit first) |
| W2.2 | [[2026-10-21-w2-2-logic-bus-index]] logic: store traits, registry, event bus, .bboxy index, state streams | Rook Ashwhisker <EXECUTOR> | DONE ✔ 4988ac3 (merged 8484e0a) |
| C | [[2026-10-22-c-cockpit]] cockpit: F1–F5 fidelity backlog → C1 live seats (Sable); 7.9 → 7.10 → W3.2c → L8 → L9 (Nyx); W2.4 messaging → S1 supervisor split (arch #1) → W2.5 MCP + tasks (arch #3) → W2.5b seat wiring (MCP, hooks, skill per seat) → W2.6 provenance/events → D1 dummy ssh/cloud (Rook); lab refs (Wren) | cohort | RUNNING |
| W2.4 | [[2026-10-22-w2-4-local-messaging]] local messaging: envelopes, channel tree, priority, harness delivery | Rook Ashwhisker <EXECUTOR> | DONE ✔ 1f9442d |
| T1–T3 | [[2026-10-22-t-terminal-fidelity]] firehose → R3 → fonts ∥ fidelity → R5 | Aster Cinderwing <EXECUTOR> (exec4) | T1 ✔ dc06057; T2a ∥ T3 RUNNING |
| P1 | parity in CI: geometry failures fail hard, pixel review stays advisory (Architect) | next free Executor | QUEUED |
| PKG0 | stable dev code signing: every dev build of the app and of Keychain-touching test binaries is signed with **"Apple Development: Max Soltan (G2XY6PGFKS)"**, SHA-1 3943AF130A015B36824B7FBC86AA672E7B25F3C4, team UQB3368A84, the identity used for the Tachyon app (operator-approved 2026-10-22 for Apple code signing; git/crates identity stays bboxy450). Verified: codesign signs non-interactively after the operator's partition-list fix, so login-keychain 'Always Allow' survives rebuilds; a cargo runner or post-build step, no portal work. Operator one-time partition-list fix: DONE 2026-10-22 | next free Executor (Velvet after P2) | QUEUED |
| PKG1 | signed app packaging: Developer ID team, keychain-access-groups entitlement for tier-1 device custody (arch #17), notarisation | unassigned. Team: **Wild Honey on the Porch, LLC (UQB3368A84)** (operator, 2026-10-22). On this Mac: 'Apple Distribution' (App Store) for that team plus an 'Apple Development' cert; **missing: a 'Developer ID Application' cert** (for notarised distribution outside the App Store) and an App ID with the keychain-access-groups capability. Operator action, once: Xcode → Accounts → Manage Certificates → + Developer ID Application; App Store Connect → Integrations → Keys → an Admin API key, .p8 into ~/.appstoreconnect/private_keys/ (then agents automate App IDs, profiles and notarisation) | BLOCKED on operator |
| ID1 | full GPG seat chain (arch #4): root → Keychain device key → in-memory seat keys | next free Executor, after Corvin's identity spec | QUEUED |
| O1–O6 | [[2026-10-22-o-open-source]] publish bboxy450-rt, -ui-core, -render, -ui, -book ([[open-source]]) | deferred: publish at a plateau, velocity first | DEFERRED |
| R1–R5 | [[2026-10-22-r-runtime]] runtime: async IO off the UI thread, 120 Hz ([[runtime]]) | Sable Embertail <EXECUTOR> | R1 ✔ 29d5e62; R2 ✔ be604d4; R2b ✔ 82c821b; R4 ✔ 18e0593; R4b ✔ 6f67402; F1 RUNNING; R4c (GPU ≤8.33) queued |
| W3.2d | [[2026-10-21-w3-2d-component-library]] bbt-ui layer cake: d.1 materials → d.7+ modules | Nyx Copperwing <EXECUTOR>, from d.1 | ASSIGNED |

Rule: redundant passes write `-exec.md` next to the subagent's file. The Oracle converges both into `specs/`. Claim by replying on `#bboxy450-term` with `Name <EXECUTOR> CLAIM <Id>`. Finish one, claim the next.
