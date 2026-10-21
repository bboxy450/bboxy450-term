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

- **Rebuild, not reuse.** Take the design of the `icq` crate, its daemon and its NAT puncher as the blueprint. **Build on `quinn`.** DECIDED 2026-10-21 after [[transport-and-identity]]: iroh is MIT/Apache and its relays and DNS are self-hostable, but iroh 1.x runs on `noq`, n0's fork of quinn, so adopting it means a second QUIC stack owned by one VC-funded company. Reversible if the messaging layer only sees `connect(key) -> Connection`.
- Transport: direct QUIC → simultaneous NAT punch → end-to-end encrypted relay fallback. Local and cross-machine, cross-network.
- Global `#channels`, private groups, DMs, group DMs.
- **Identity = key.** Same key, same user. Day one: a key plus an email. **Google sign-in in v0.1**: loopback PKCE, about a week, no server; the ID-token nonce is a hash of the public key so the email claim is bound to the key. **Apple sign-in deferred**: Apple does not allow the native flow for Developer ID (non-App-Store) macOS apps, and the web flow needs a hosted HTTPS callback on a verified domain. Revisit when we have a domain or an App Store build. Source: [[transport-and-identity]].
- Message priority derives from the sender's terminal-assigned role. Oracle outranks everything else. Priority orders delivery and badging. Nothing is dropped.
- **Waking a harness:** `icq` already taps Claude, Codex, pi and omp sockets or channels. Reuse those touch points. **Fallback for any harness: type into its PTY and press Enter.** The terminal owns the PTY, so it must watch for interactive input state before injecting.
- Every seat gets a generated character name (fantasy, sci-fi, furry pools) and its own key. **Gap:** whether seats sign git commits with their own key, and how that relates to the operator's signing key.

## Harness touch points (DECIDED, mechanism found)

Inject deep: PTY, hooks, MCP, skills. All of them. Active/idle detection combines PTY output cadence with harness hooks; hooks win when present. Research: `tasks/2026-10-21-harness-touchpoints-research.md`.

**Primary state channel (found 2026-10-21, [[iterm-harness-plugin]], spec [[subagent-detection]]):** Claude Code (since 2.1.295) and Codex (0.163) emit **OSC 7501, the Program Status Protocol**, on their own PTY once the terminal answers the probe `ESC]7501;?ESC\`. Wire form: `ESC]7501;state=<idle|working|blocked|done|error|clear>:app=<name>[:id=<agent id>][:kind=permission|question|auth][:progress=N][:title=<base64>][:msg=<base64>]ESC\`. Each running Claude subagent is its own report with `id=` and `title=` the Agent tool description; that is how iTerm2 nests subagent rows under a seat. Codex also emits OSC 21337 status. Because we own the PTY, we answer the probe and parse the stream: zero install, no hooks needed for the baseline. Hooks (`bboxy-hook` shim via a per-launch settings overlay writing a namespaced OSC 7502 to `/dev/tty`, seat id from `BBOXY_SEAT_ID`) add the events 7501 lacks. Unix socket fallback, PTY heuristic for unknown harnesses.

## Persistence and scale (DECIDED)

- macOS first. Linux and mobile devices follow. wgpu keeps that cheap. One unified look and feel on every platform, our own, transcending OS styles.
- Seats survive terminal crashes and restarts. Sessions are recovered and resumed automatically. The terminal keeps a durable seat id and the mapping seat id → harness session id. This implies a mux daemon separate from the GUI process.
- **VTE runs in the mux daemon, not the GUI** (DECIDED 2026-10-21 from [[cloud-and-mosh]]): mosh, tmux, zellij, wezterm all do this; it gives crash survival, remote parity, and lets the Oracle read any seat's screen without the GUI. The GUI receives screen state diffs mosh-style, scheduled per seat by RTT and visibility.
- **Memory model at 1000 seats**: an `alacritty_terminal` grid with 10k scrollback measured 15–20 MB per seat, so 1000 seats would be 15–20 GB. Not viable. The daemon keeps a viewport-sized grid per seat plus a bounded raw-byte scrollback ring with a monotonic cursor; scrollback lines are re-parsed lazily on demand. Hidden seats send the GUI only a state bit for the sidebar.
- Target scale: swarms of **1000 agents**. Some local, some remote over ssh, some in the user's chosen cloud. The sidebar, the mux and the messaging layer must be designed for that count from the start.

## Working rules (operator, 2026-10-21)

- **One concern at a time.** Layout first, then style, then behaviour, then performance. A brief names one concern.
- **Do not optimise pre-emptively.** 120 fps is the eventual target; nobody measures or tunes for it until layout and style are nailed.

## Future (RECORDED, not v0.1)

- **Cloud harnesses.** Run a harness transparently in a cloud of the user's choice. Reconnect like **mosh**: state-sync, roaming, resumable. Reuse the messaging transport. All tied to the user account. Research: `tasks/2026-10-21-cloud-and-mosh-research.md`.
- **Harness switching.** Move a session's logs from Codex to Claude and resume there, or the reverse.
- **Usage limits.** Tap Claude and Codex account usage limits. Reference implementation: `tachyon` by Gonzih (the operator's other identity). Recorded in [[roadmap]].
- MCP tool for an agent to request the operator's attention.

## v0.1 scope (DECIDED)

Everything above that is not marked Future. Private GitHub repo `bboxy450/bboxy450-term` for now. Crates published under bboxy450.

Work splits into: terminal core (PTY, VTE, grid), renderer (wgpu), frontend chrome (sidebar, tabs, splits, settings), mux daemon (seat persistence, recovery), messaging (transport, identity, rooms, priority), MCP server (Oracle control plane), harness adapters (launch, role prompt, wake, state detection). Task files per area follow the research.

## Stack (DECIDED by the operator, 2026-10-21)

| Layer | Decision |
|---|---|
| Terminal model | **Fork `alacritty_terminal`** into this monorepo as our own subcrate and maintain it. The operator reads alacritty as abandoned; bundling the upstream crate is a dependency on a dead project. The fork gets our viewport-grid plus byte-ring memory model. Task A reports its last-release date for the record. |
| PTY | **`portable-pty`** (from wezterm). |
| wezterm mux | **Read only.** Design reference, no code reuse. |
| Chrome and renderer | **Raw wgpu.** Our own retained UI, invested in and polished over time. No gpui, no iced. A **dedicated renderer subagent** builds it under an architecture the Oracle writes. **Option kept open by the operator (2026-10-21): resurrect the whole alacritty codebase, renderer included, and sprinkle on top.** Oracle read: alacritty's renderer is OpenGL via glutin (deprecated on macOS since 10.14, no Metal path), so the renderer fork would mean porting it to wgpu anyway; what is worth lifting is its design (cell instancing, `crossfont` glyph rasterization through CoreText, atlas handling), not its GL code. Task A costs both: port alacritty's renderer to wgpu vs write ours lifting its design. |
| Text rendering | **The sexiest text possible.** Study what ghostty does and take it. Research: `tasks/2026-10-21-text-rendering-research.md`. |
| Terminal protocol baseline | **Full 24-bit colour** plus everything listed: OSC 8 hyperlinks, OSC 52 clipboard, bracketed paste, mode 2026 synchronized output, Kitty keyboard protocol. Images deferred. |
| Repo shape | **Monorepo managed by Nix** using the operator's **`weavie`** crate (crates.io 0.2.0, github.com/bboxy450/weavie: Rust graph-repo workspaces from Nix inputs and isolated Git worktrees). We improve weavie as we use it. |
| Binaries | **One binary, two modes**: GUI and `--daemon`. |
| macOS floor | **Aggressive.** Support roughly the last four years: floor is **macOS 14 Sonoma**, raised every autumn. Metal and wgpu features from that floor are fair game. |

Defaults not yet contested (ASSUMED): grayscale antialiasing, ligatures on with system font fallback, render on change with the glow shader as the only continuous animator, 10k lines or 4 MB scrollback per seat.

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
