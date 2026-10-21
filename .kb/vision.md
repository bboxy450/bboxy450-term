# bboxy450-term — vision

Status: DECIDED where marked, ASSUMED where marked, **Gap:** where open. Updated 2026-10-21 by Vesper Greywind <ORACLE> from the operator's answers.

Index: [[README]] · tasks in `tasks/` · research in `research/` · specs in `specs/` · [[ux-sidebar]] · [[roadmap]]

## One line

**bboxy450-term** is a GPU terminal emulator in Rust built as a cockpit for swarms of coding agents. Agents and harnesses are the users. Humans supervise.

## Vocabulary (DECIDED)

| Term | Meaning |
|---|---|
| **seat** | One pane holding one harness session (or a plain zsh). Has an id, a name, a harness, a role, a state, a key. |
| **space** | A group of tabs. One space per project or swarm. Arc-style. |
| **tab** | A member of a space. A tab holds one seat or a split tree of seats. |
| **split** | Vertical or horizontal division of a tab into seats. Binary tree. |
| **cohort** | The set of seats working together under one Oracle. |
| **harness** | Claude Code, Codex, pi, omp, any agent runtime driven through a PTY. |
| **role** | Oracle, Executor, Creativity. Assigned per seat; travels with every message. |

## Layout (DECIDED)

- Arc-style **left sidebar**, simpler and more elegant than the iTerm2 harness plugin on the right. Reference screenshots: `runs/ref-iterm-grouped-tabs.png`, `runs/ref-arc-sidebar.png`, `runs/ref-iterm-session-status.png`.
- Sidebar shows spaces; inside a space, tabs; inside a tab, its split tree. Splits are drawn as a mini-map (riff on Arc's split tile grid), extended to show vertical and horizontal.
- Each seat row: harness icon, generated character name, title, state. Icons, not text badges.
- Seat states: **active**, **idle**, **waiting-on-input**, **errored**. Waiting-on-input: shown, no special escalation yet. Later: an MCP tool for an agent to request the operator's attention.
- `Cmd+T` opens a new tab and asks which harness; zsh is the last option. A separate shortcut opens zsh directly.
- Settings menu and toolbar icon must look good. Logo derives from the operator's identity image `~/.agents/identity/bboxy.png` (electric-blue fox, black hoodie). Logo task: a harness produces a fitting mark from it.

## Roles and control (DECIDED)

- Default roles live in settings: Codex → Executor, Claude → Oracle. Overridable per seat.
- Each role has a system prompt the terminal feeds into the harness at launch (hook, flag, or MCP, whichever the harness supports).
- The terminal runs an **MCP server**, modelled on iTerm2's `it2` binary. The Oracle seat gets the full tool set: list seats, read any seat's screen and scrollback, send text or keystrokes, create seats, split, focus, close, assign roles, set permissions, message. The Oracle sets up its own cohort.
- The Oracle decides per seat which channels and DMs that seat may send to. Permissions are terminal-enforced, not harness-enforced.
- The Oracle may read every seat's scrollback. Verification beats trust.

## Messaging (DECIDED)

- **Rebuild, not reuse.** Take the design of the `icq` crate, its daemon and its NAT puncher as the blueprint. Build on `quinn`. `iroh` only if it is open source under a license we accept and we trust the maintainers (research question).
- Transport: direct QUIC → simultaneous NAT punch → end-to-end encrypted relay fallback. Local and cross-machine, cross-network.
- Global `#channels`, private groups, DMs, group DMs.
- **Identity = key.** Same key, same user. Day one: a key plus an email. Apple and Google sign-in are wanted if cheap enough to add now (research question); otherwise a later login layer on top of the key.
- Message priority derives from the sender's terminal-assigned role. Oracle outranks everything else. Priority orders delivery and badging. Nothing is dropped.
- **Waking a harness:** `icq` already taps Claude, Codex, pi and omp sockets or channels. Reuse those touch points. **Fallback for any harness: type into its PTY and press Enter.** The terminal owns the PTY, so it must watch for interactive input state before injecting.
- Every seat gets a generated character name (fantasy, sci-fi, furry pools) and its own key. **Gap:** whether seats sign git commits with their own key, and how that relates to the operator's signing key.

## Harness touch points (DECIDED, mechanism under research)

Inject deep: PTY, hooks, MCP, skills. All of them. Active/idle detection combines PTY output cadence with harness hooks; hooks win when present. Research: `tasks/2026-10-21-harness-touchpoints-research.md`.

## Persistence and scale (DECIDED)

- macOS first. Linux and mobile devices follow. wgpu keeps that cheap. One unified look and feel on every platform, our own, transcending OS styles.
- Seats survive terminal crashes and restarts. Sessions are recovered and resumed automatically. The terminal keeps a durable seat id and the mapping seat id → harness session id. This implies a mux daemon separate from the GUI process.
- Target scale: swarms of **1000 agents**. Some local, some remote over ssh, some in the user's chosen cloud. The sidebar, the mux and the messaging layer must be designed for that count from the start.

## Future (RECORDED, not v0.1)

- **Cloud harnesses.** Run a harness transparently in a cloud of the user's choice. Reconnect like **mosh**: state-sync, roaming, resumable. Reuse the messaging transport. All tied to the user account. Research: `tasks/2026-10-21-cloud-and-mosh-research.md`.
- **Harness switching.** Move a session's logs from Codex to Claude and resume there, or the reverse.
- **Usage limits.** Tap Claude and Codex account usage limits. Reference implementation: `tachyon` by Gonzih (the operator's other identity). Recorded in [[roadmap]].
- MCP tool for an agent to request the operator's attention.

## v0.1 scope (DECIDED)

Everything above that is not marked Future. Private GitHub repo `bboxy450/bboxy450-term` for now. Crates published under bboxy450.

Work splits into: terminal core (PTY, VTE, grid), renderer (wgpu), frontend chrome (sidebar, tabs, splits, settings), mux daemon (seat persistence, recovery), messaging (transport, identity, rooms, priority), MCP server (Oracle control plane), harness adapters (launch, role prompt, wake, state detection). Task files per area follow the research.

## Rendering and prototyping (DECIDED)

- **Everything is GPU-rendered**, chrome included, not only the terminal grid. Shaders are a first-class UI tool: glow, blur, state animation, seat identity effects. The GUI toolkit choice in Task A must allow custom shaders and share the wgpu device with the terminal surfaces.
- **Storybook-style component lab** before Rust. Every UI element (sidebar row, split mini-grid, state glyph, harness picker, settings, toolbar icon, message badges) is prototyped and iterated in isolation, with all its states, then built in Rust against that reference. The lab is also where the Rust components get mirrored later so the two never drift. **Gap:** lab medium: web (fastest iteration, Lumen builds it) versus a Rust storybook binary on the real renderer. Likely both, web first.

## Process

1. Research everything, written to `.kb/research/`, linked with wikilinks, pushed to GitHub.
2. Dummy UI prototype. A dedicated UI-creator Claude session builds it under Oracle direction. UI iterates for a long time.
3. Implementation by Executors under task briefs.

## Open gaps

- **Gap:** seat commit-signing keys versus the operator's signing key.
- **Gap:** what "same user across machines" proves beyond key possession, once login exists.
- **Gap:** how a seat in a remote ssh or cloud location appears in the local sidebar and who owns its PTY.
- **Gap:** split minimap rendering at 1000 seats.
