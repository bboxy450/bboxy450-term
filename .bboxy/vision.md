---
id: "5VKS102BDYZWBTZ67C2BQ2AH4Z"
type: "note"
number: 66
title: "bboxy450-term — vision"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-24T00:10:38Z"
---
# bboxy450-term — vision

Status: DECIDED where marked, ASSUMED where marked, **Gap:** where open. Updated 2026-10-21 by Vesper Greywind <ORACLE> from the operator's answers.

Index: [[#57|README]] · tasks in `tasks/` · research in `research/` · specs in `specs/` · [[ux-sidebar]] · [[roadmap]]

## One line

**bboxy450-term** is a GPU terminal emulator in Rust built as a cockpit for swarms of coding agents. Agents and harnesses are the users. Humans supervise.

## Vocabulary (DECIDED)

| Term | Meaning |
|---|---|
| **seat** | One pane holding one harness session (or a plain zsh). Has an id, a name, a harness, a role, a state, a key, and a **where** (local, ssh host, cloud host; from the lab 2026-10-21, drawn as a where-mark before the harness tile). |
| **swarm** | The sidebar block and the unit of everything: a group of tabs, the seats under one Oracle, its own stream and panes. DECIDED 2026-10-21 by the operator in the lab: "swarm" replaces both "space" and "cohort". Specs that say cohort mean swarm; rename on next touch. |
| **tab** | A member of a space. A tab holds one seat or a split tree of seats. |
| **split** | Vertical or horizontal division of a tab into seats. Binary tree. |
| **cohort** | Retired. See swarm. |
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

- Built-in roles: **Architect** (holds the system across time: specs, invariants, decision records and the checks that enforce them; halts work that breaks an invariant; most capable model, e.g. Claude Fable; oracle-class), **Oracle** (runs the swarm: specs → epics → tasks, delegation, state, verification; cheaper model, low effort; oracle-class), **Executor** (builds; higher effort; worker-class), **Creativity** (designs the experience; highest model and effort; worker-class). The Oracle governs the day; the Architect writes the law the day runs under (operator, 2026-10-22, `~/.agents/AGENTS.md` Four roles). Four Architect rules bind the task system ([[work-items]]): an invariant has an executable check or it is only a preference; the spec signs off, not the agent; hard gates block and judgement advises; a decision is recorded when it is made. Default role per harness lives in settings: Codex → Executor, Claude → Oracle; Architect and Creativity are assigned per seat. Overridable per seat.
- **Custom roles (operator via the lab, 2026-10-21).** Users define roles in settings. Built-in Architect, Oracle, Executor, Creativity stay, editable not deletable. A role = name, colour, mark, default system prompt, may-send-to, and a **priority class**: oracle-class or worker-class. Delivery order, badging, MCP gating and the receiver trust policy derive from the class; colours and marks from the name. Oracle decisions: roles are **per account** (defined once, assigned per seat in any swarm); the **class may change** later, and a change re-signs the affected seat grants with a new epoch, so a receiver never trusts a name, only the signed class in the grant.
- Each role has a system prompt the terminal feeds into the harness at launch (hook, flag, or MCP, whichever the harness supports).
- The terminal runs an **MCP server**, modelled on iTerm2's `it2` binary. The Oracle seat gets the full tool set: list seats, read any seat's screen and scrollback, send text or keystrokes, create seats, split, focus, close, assign roles, set permissions, message. The Oracle sets up its own cohort.
- The Oracle decides per seat which channels and DMs that seat may send to. Permissions are terminal-enforced, not harness-enforced.
- The Oracle may read every seat's scrollback. Verification beats trust.

## Messaging (DECIDED)

- **Transport = quinn. Oracle call, operator deferred (2026-10-21).** Build on upstream `quinn` with our own punch ladder, relay and DHT discovery, modelled on the `icq` crate (daemon, punch, receipts, ~5k lines as the blueprint). Reasons: iroh 1.x runs on `noq`, n0's quinn fork, a second QUIC stack owned by one VC-funded company; global messaging is phase 3 in the priority order, so the transport is not on the v0.1 critical path and we lose nothing by owning it. The messaging layer sees only a `Transport` trait (`connect(key)`, `subscribe(topic)`, `publish`), so **iroh is re-evaluated at phase 3** with fresh data and swaps in behind the trait if it wins then. Goal unchanged: global pubsub, DHT discovery as in icq, WebSocket failover, everything encrypted, magic for users.
- Transport: direct QUIC → simultaneous NAT punch → end-to-end encrypted relay fallback. Local and cross-machine, cross-network.
- Global `#channels`, private groups, DMs, group DMs. With global messaging enabled, other people's harnesses in the cohort's channels appear in the sidebar as grey **phantoms** (Dark Souls style): present, active, messaging, never their logs. See [[ux-pick]] Phantom seats.
- **Identity = the user's GPG key. Lean in aggressively** (operator, 2026-10-21). The account root is the operator's OpenPGP primary key, the same key that signs their commits. Email comes from the GPG user id. Device and seat keys are Ed25519 subkeys or GPG-certified leaf keys, so `gpg --verify` and the seat roster are the same trust chain, git provenance falls out for free, and existing GPG backup and revocation practice is the recovery story. Pure-Rust OpenPGP via `sequoia-openpgp`; gpg-agent for root custody where present, Keychain fallback. Same key, same user. Day one: a key plus an email. **Google sign-in in v0.1**: loopback PKCE, about a week, no server; the ID-token nonce is a hash of the public key so the email claim is bound to the key. **Apple sign-in deferred**: Apple does not allow the native flow for Developer ID (non-App-Store) macOS apps, and the web flow needs a hosted HTTPS callback on a verified domain. Revisit when we have a domain or an App Store build. Source: [[transport-and-identity]].
- **Cohort scope and mute (DECIDED by operator, 2026-10-21).** A seat sees only the sessions in its own cohort by default. The operator can **mute a seat**: it leaves the cohort stream and the operator works with it solo, one to one. **Unmute** returns it, and the unmute is a notification to the whole cohort, as is the mute, so everyone knows who is in and who is out. Same shape as `icq mute`, lifted into the sidebar: a muted seat shows a muted mark, its row stays where it is, its messages to the cohort are held, direct operator traffic still flows.
- Message priority derives from the sender's terminal-assigned role. Oracle outranks everything else. Priority orders delivery and badging. Nothing is dropped.
- **Waking a harness:** `icq` already taps Claude, Codex, pi and omp sockets or channels. Reuse those touch points. **Fallback for any harness: type into its PTY and press Enter.** The terminal owns the PTY, so it must watch for interactive input state before injecting.
- Every seat gets a generated character name (fantasy, sci-fi, furry pools) and its own key. **Gap:** whether seats sign git commits with their own key, and how that relates to the operator's signing key.

## User-defined harnesses (DECIDED by operator via the lab, 2026-10-21)

Users add harnesses in settings: name, command, tile colour, icon, default role, wake method, state source, hooks. A user-added harness runs through the **generic adapter**: PTY typing behind the input guard, state inferred from output cadence (grade `inferred`), upgraded to `reported` if it speaks OSC 7501 or to `shim` if the user wires hooks. It appears in the launcher like the built-ins. The built-in adapters are the same record with code behind it. One consequence for [[harness-adapters]]: the adapter trait is instantiated from a harness record, not a hard-coded enum; `harness_kind` is open.

## Seats are sockets (DECIDED by operator, 2026-10-21)

Every seat exposes a socket: a unix socket always, HTTP optionally. Whatever is written to it goes into that seat's harness as input through the adapter and the input guard; whatever the seat emits (state, 7501 reports, hook events, output lines) comes out as an event stream on the same socket. The terminal is event-based end to end, the same way the operator's `kittens` harness is. The point is glue: cron, scripts, other agents, other machines plug into a seat with nothing but a socket path. Research: `research/kittens-events.md` (what kittens does, what we take). Specs: [[mux-daemon]] (the socket is owned by the logic process, bound to seat id and process generation), [[mcp-control-plane]] (MCP is one client of the same event bus, not a separate path).

## Storage shape (DECIDED 2026-10-21, [[storage]]; from the operator's durable-object vibe and [[state-backend]])

**One durable object per thing.** A seat, a swarm and the account are each an object: its actor, its own SQLite file, its byte ring, its alarms. Locally an object is a directory under the run dir hosted by the mux logic process; in the cloud the same object runs on Cloudflare Durable Objects or any actor runtime; on-premise it runs on the operator's own host with MinIO for blobs. **Store everything, every size, never delete**; retention is operator policy per object kind. Small facts in the object's SQLite; large things (rings, PNGs, transcripts, binaries, PR diffs) in a content-addressed blob store, local directory now, S3-compatible later (R2, S3, MinIO). Seams: a `Store` trait (get/put/list/sql/alarm) and a `Blobs` trait (put by digest/get/list), SQLite-file implementations first. Every object writes one append-only signed event log; that log is what an enterprise tails into its own systems.

## Harness touch points (DECIDED, mechanism found)

Inject deep: PTY, hooks, MCP, skills. All of them. Active/idle detection: OSC 7501 reports are the primary grade, hooks are the secondary shim grade, PTY cadence is a negative-only check (see [[harness-adapters]] §0 R1). Research: `tasks/2026-10-21-harness-touchpoints-research.md`.

**Primary state channel (found 2026-10-21, [[iterm-harness-plugin]], spec [[subagent-detection]]):** Claude Code (since 2.1.295) and Codex (0.163) emit **OSC 7501, the Program Status Protocol**, on their own PTY once the terminal answers the probe `ESC]7501;?ESC\`. Wire form: `ESC]7501;state=<idle|working|blocked|done|error|clear>:app=<name>[:id=<agent id>][:kind=permission|question|auth][:progress=N][:title=<base64>][:msg=<base64>]ESC\`. Each running Claude subagent is its own report with `id=` and `title=` the Agent tool description; that is how iTerm2 nests subagent rows under a seat. Codex also emits OSC 21337 status. Because we own the PTY, we answer the probe and parse the stream: zero install, no hooks needed for the baseline. Hooks (`bboxy-hook` shim via a per-launch settings overlay writing a namespaced OSC 7502 to `/dev/tty`, seat id from `BBOXY_SEAT_ID`) add the events 7501 lacks. Unix socket fallback, PTY heuristic for unknown harnesses.

## Persistence and scale (DECIDED)

- macOS first. Linux and mobile devices follow. wgpu keeps that cheap. One unified look and feel on every platform, our own, transcending OS styles.
- Seats survive terminal crashes and restarts. Sessions are recovered and resumed automatically. The terminal keeps a durable seat id and the mapping seat id → harness session id. This implies a mux daemon separate from the GUI process.
- **VTE runs in the mux daemon, not the GUI** (DECIDED 2026-10-21 from [[cloud-and-mosh]]): mosh, tmux, zellij, wezterm all do this; it gives crash survival, remote parity, and lets the Oracle read any seat's screen without the GUI. The GUI receives screen state diffs mosh-style, scheduled per seat by RTT and visibility.
- **Undo close (DECIDED by operator, 2026-10-21).** Closing a seat is reversible. A closed seat goes to a grace state for a bounded window (default 60 s, setting): the harness process keeps running under the supervisor, the ring keeps capturing, the sidebar shows the seat in a collapsed "closed" strip with an undo action, ⌘Z restores it in place with its split position. After the window the process is terminated and the ring retained for scrollback on a later resume by session id. No confirm dialog on close; the undo window is the confirmation.
- **Process survival (DECIDED 2026-10-21 from [[cloud-and-mosh-exec]]):** killing the sole PTY master owner delivers SIGHUP to the harness within a millisecond, so a monolithic daemon crash would kill every seat. The mux therefore splits in two: a **tiny supervisor** per machine that holds the PTY masters and the byte rings and almost never changes, and the **mux logic process** (VTE, messaging, MCP, scheduling) that may crash and restart freely and re-attaches to the supervisor. If the supervisor itself dies, seats are resumed as new processes by harness session id (`claude --resume`, `codex resume`) with the ring replayed for scrollback; that path is the fallback, not the design.
- **Byte ring guarantees:** a raw tail replay loses SGR and mode state, so the ring carries periodic parser-state checkpoints with semantic row indexes and evicts only on checkpoint boundaries; on disk it is a length/sequence/checksum record log with a committed-boundary marker so a torn suffix is rejected on restart.
- **Memory model at 1000 seats**: an `alacritty_terminal` grid with 10k scrollback measured 15–20 MB per seat, so 1000 seats would be 15–20 GB. Not viable. The daemon keeps a viewport-sized grid per seat plus a bounded raw-byte scrollback ring with a monotonic cursor; scrollback lines are re-parsed lazily on demand. Hidden seats send the GUI only a state bit for the sidebar.
- Target scale: swarms of **1000 agents**. Some local, some remote over ssh, some in the user's chosen cloud. Per Mac the kernel caps pseudo-terminals (`kern.tty.ptmx_max`, 511 default, 999 at most; decision [[d19-pty-ceiling|#19]], 2026-10-22), so 1000 is a per-swarm number across hosts and the product says so at the limit instead of failing quietly. The sidebar, the mux and the messaging layer must be designed for that count from the start.

## The composability test (operator, 2026-10-21)

Something like Karpathy's **autoresearch** must be natively possible with no new feature: an agent loop that proposes an experiment, runs it, measures, keeps what improved, logs everything, and iterates. A harness told "glue everything together" should be able to build it out of the primitives alone, with bboxy450-term as the glue:

| autoresearch needs | bboxy primitive |
|---|---|
| a hypothesis and its plan | a task markdown with `derived_from` the previous result |
| running it | `task_delegate` to a seat, or spawn+delegate under the cap |
| the result | an artifact claimed by the seat, with digest and `for: [[task]]`; a note markdown with the measurement |
| "did it improve" | an invariant markdown naming the metric and direction; the Oracle seat subscribed to `artifact/*` for the experiment |
| the next iteration | an event wakes the Oracle seat, which writes the next task; the markdown graph is the lineage |
| the record | the provenance log: every run, every input, every key, chained |
| scaling out | seats local, ssh, cloud; same bus, same grants |

If a loop like this needs a feature we do not have, the primitives are wrong, not the loop. This is the test every spec is held to. Note: the operator's own `paperhands` is an autoresearch system; it should run inside bboxy450-term as a swarm without special support.

## The suite is the product; the terminal is its first app (operator, 2026-10-23)

"All those libraries are reusable parts, all those crates are reusable part of #bboxy450-* suite, term is just a way we glue them together, later on we will build more apps using our GUI kit that all connects together!" The app crate is glue; anything in it that a second app could use moves down a layer; apps connect through `bbt-proto`, the bus and messaging under one identity chain. Decision [[d39-suite-is-the-product|#39]], enforced by [[layering]].

## The visual foundation outranks everything (operator, 2026-10-22)

"Visual parity and visual fidelity and our visual component library and GPU renderers: top priority. With that we can build anything we want down the line."

The renderer (`bbt-render`), the component library (`bbt-ui`, `bbt-ui-model`) and the web↔native parity loop are the foundation every other feature stands on. When work competes, fidelity and the library win: renderer and library work gets the first free Executor, parity residue is a real backlog and never "good enough", and a feature that needs a missing primitive waits for the primitive to be built properly at its own layer.

## Cockpit first (operator, 2026-10-22: the current direction)

"Priority right now is a working terminal, with harnesses, local messaging, local tasks, a local provenance system and local events. That's it, but a sexy, sexy UI. Then we start designing ssh and cloud, gpg keys for auth. We build the cockpit, then expand into cloud providers and ways to branch out, but not yet. We can have dummy ssh support and dummy cloud support, as if it works without it actually working, so we have everything in the UI."

What that means:
- **Real now, all local:** terminal seats that run and render in panes; harness adapters (Claude, Codex, zsh) with status and subagents in the sidebar; local messaging (DMs, roles, priority, the channel tree) over the local bus; local tasks (`.bboxy` work items, delegate and report-back); local provenance (signed write intents, artifacts, the activity log); local events (the bus, visible in the UI). And every seat comes wired: **the MCP control plane, task tools, hooks and skills are there from the first launch** (operator, 2026-10-22: "terminal with MCP, tasks, hooks, skills all should be there"). A seat launched from ⌘T gets our MCP server, our hook shim and the bboxy450 skill injected per seat at launch, through launch flags and environment and never by editing the user's own harness config, so the agent in it can orient, message, take and report tasks, and claim artifacts on its first turn.
- **Dummy now, real later:** ssh hosts and cloud providers exist in the UI and behave plausibly through a fake provider (remote-labelled seats backed by local PTYs, simulated latency and connect states). No real network transport, no quinn, no remote supervisor yet.
- **Later:** real ssh and cloud, gpg-key auth across machines, private cross-instance messaging, global messaging.

## Working rules (operator, 2026-10-21)

- **One concern at a time.** Layout first, then style, then behaviour, then performance. A brief names one concern.
- **Granularity: one layer per brief.** The UI is a layer cake from scratch ([[gui-components]] §2c, L0 device … L9 app). Each layer has its own bboxybook gallery and is verified alone before the layer above uses it; a brief covers one layer and at most two components.
- **Checkpoints.** Rare and deliberate (operator 2026-10-21). bboxybook demos are checkpoints: the first native render of any component set, every parity index, and each batch of three lab passes. The Oracle may stack two or three UX passes, then calls a checkpoint: it opens the page once to get the operator's attention, says what changed in a few lines, and waits. Between checkpoints it never opens, closes or reloads the operator's browser. Feedback comes from the operator touching the thing.
- **Harnesses in the loop look at images.** Screenshots, renders and diffs are read by a harness as part of any improvement loop; a numeric diff ranks, a harness judges. This applies to bboxybook parity, render goldens, and lab review.
- **Do not optimise pre-emptively; optimise from the meter.** 120 Hz is a target (operator, 2026-10-22, [[runtime]]): the UI thread never waits on IO, and every performance change follows a measured profile (the hitch meter, xctrace), never a guess.
- **Velocity first (operator, 2026-10-22).** Nothing ceremonial goes inside the build loop. The `bboxy450-*` libraries are built to be open-sourced ([[open-source]]), but a crate is published only at a natural plateau where its API has stopped moving; a release cycle never slows a build.
- **No code signing until distribution (operator, 2026-10-22).** Dev builds run unsigned; signing, entitlements and notarisation (PKG0, PKG1) wait until we ship. Identity and tests must never depend on a signed build or prompt for the keychain.
- **CI is local for now (operator, 2026-10-22).** We are in heavy development: no GitHub CI. Every handoff and every acceptance runs `sh scripts/ci-local.sh` (layering, build, tests, UI IO lint; add `--parity` when lab, design, UI or renderer changed) and reports its exit. The GitHub workflow runs only on manual dispatch. CI comes back per crate when a crate is open-sourced into its own public repo.
- **We are inevitable (operator, 2026-10-22).** "WE GOT THIS, WE ARE INEVITABLE in this effort!" This is the stance of the whole cohort: we build without hedging, measure instead of worrying, fix at the layer where a problem starts, and keep moving.

## Future (RECORDED, not v0.1)

- **Cloud harnesses.** Run a harness transparently in a cloud of the user's choice. Reconnect like **mosh**: state-sync, roaming, resumable. Reuse the messaging transport. All tied to the user account. Research: `tasks/2026-10-21-cloud-and-mosh-research.md`.
- **Harness switching.** Move a session's logs from Codex to Claude and resume there, or the reverse.
- **Usage limits.** Tap Claude and Codex account usage limits. Reference implementation: `tachyon` by Gonzih (the operator's other identity). Recorded in [[roadmap]].
- MCP tool for an agent to request the operator's attention.

## v0.1 scope (DECIDED, cut down by the operator 2026-10-21: less is more)

**Ship first:** rendering (the priority), the visual style as prototyped in [[ux-prototype]], local messaging between seats on one machine, the MCP control plane, harness visibility (seat state and subagent rows). Everything designed with cloud as a coming fact, so no local-only shortcuts in the mux or messaging boundaries.

**Then, in order:**
1. **Cloud**: remote seats, supervisor bootstrap on a VM, reconnect. Built "in between", after rendering is solid.
2. **Private account-based messaging** between the operator's own instances, local and cloud (device grants, direct QUIC, punch and relay).
3. **Global messaging**: public channels, phantoms, global pubsub (gossip over our quinn transport, or iroh-gossip if the phase-3 re-evaluation picks iroh) with DHT discovery and WebSocket failover.

**Postponed out of the first prototype:** global messaging and phantoms, Google binding (not needed until instances talk across machines), Linux and mobile. Private GitHub repo `bboxy450/bboxy450-term` for now. Crates published under bboxy450.

Work splits into: terminal core (PTY, VTE, grid), renderer (wgpu), frontend chrome (sidebar, tabs, splits, settings), mux daemon (seat persistence, recovery), messaging (transport, identity, rooms, priority), MCP server (Oracle control plane), harness adapters (launch, role prompt, wake, state detection). Task files per area follow the research.

## Stack (DECIDED by the operator, 2026-10-21)

| Layer | Decision |
|---|---|
| Terminal model | **Fork `alacritty_terminal`** into this monorepo as our own subcrate and maintain it. Dated facts ([[gpu-terminals-rust]], 2026-10-21): alacritty is in maintenance mode, not dead: v0.17.0 and alacritty_terminal 0.26.0 released 2026-04-06, 37 commits in the last 12 months, all fixes, two active maintainers, 328 open issues. Fork from HEAD, not from the registry 0.25.1. **The parser is forked too**: alacritty_terminal does not parse, `vte` does, and vte's parser state is fully private, so checkpoint export and OSC 7501/7502 pass-through require copying vte's ~4.1k lines into term-core (terminal-core D2). The fork gets our viewport-grid plus byte-ring memory model. Task A reports its last-release date for the record. |
| PTY | **`portable-pty`** (from wezterm). |
| wezterm mux | **Read only.** Design reference, no code reuse. |
| Chrome and renderer | **Raw wgpu.** Our own retained UI, invested in and polished over time. No gpui, no iced. A **dedicated renderer subagent** builds it under an architecture the Oracle writes. **Option kept open by the operator (2026-10-21): resurrect the whole alacritty codebase, renderer included, and sprinkle on top.** Oracle read: alacritty's renderer is OpenGL via glutin (deprecated on macOS since 10.14, no Metal path), so the renderer fork would mean porting it to wgpu anyway; what is worth lifting is its design (cell instancing, `crossfont` glyph rasterization through CoreText, atlas handling), not its GL code. **Decided after Task A ([[terminal-core]] D3): fresh wgpu renderer, lift alacritty's design only** (32-byte cell instance layout, two-pass uniform, 64k batch cap, RectKind underlines, damage tracker). Its 8.6k lines of GL plus 407 lines of GLSL would be a rewrite anyway, and a port would land below the text quality bar set by [[text-rendering]]. |
| Text rendering | **The sexiest text possible.** Study what ghostty does and take it. Research: `tasks/2026-10-21-text-rendering-research.md`. |
| Terminal protocol baseline | **Full 24-bit colour** plus everything listed: OSC 8 hyperlinks, OSC 52 clipboard, bracketed paste, mode 2026 synchronized output, Kitty keyboard protocol. Images deferred. |
| Repo shape | **Monorepo managed by Nix** using the operator's **`weavie`** crate (crates.io 0.2.0, github.com/bboxy450/weavie: Rust graph-repo workspaces from Nix inputs and isolated Git worktrees). We improve weavie as we use it. |
| Binaries | **One binary, two modes**: GUI and `--daemon`. |
| macOS floor | **Aggressive.** Support roughly the last four years: floor is **macOS 14 Sonoma**, raised every autumn. Metal and wgpu features from that floor are fair game. |

Defaults not yet contested (ASSUMED): grayscale antialiasing, ligatures on with system font fallback, render on change with the glow shader as the only continuous animator, 10k lines or 4 MB scrollback per seat.

## Rendering and prototyping (DECIDED)

- **Everything is GPU-rendered**, chrome included, not only the terminal grid. **Semi-transparent window** with optional blur, like iTerm2, supported by the renderer from the start ([[renderer]] §1b). Shaders are a first-class UI tool: glow, blur, state animation, seat identity effects. The GUI toolkit choice in Task A must allow custom shaders and share the wgpu device with the terminal surfaces.
- **Storybook-style component lab** before Rust. Every UI element (sidebar row, split mini-grid, state glyph, harness picker, settings, toolbar icon, message badges) is prototyped and iterated in isolation, with all its states, then built in Rust against that reference. The lab is also where the Rust components get mirrored later so the two never drift. **Gap:** lab medium: web (fastest iteration, Lumen builds it) versus a Rust storybook binary on the real renderer. Likely both, web first.

## Process

1. Research everything, written to `.bboxy/research/`, linked with wikilinks, pushed to GitHub.
2. Dummy UI prototype. A dedicated UI-creator Claude session builds it under Oracle direction. UI iterates for a long time.
3. Implementation by Executors under task briefs.

## Open gaps

- **Gap:** seat commit-signing keys versus the operator's signing key.
- **Gap:** what "same user across machines" proves beyond key possession, once login exists.
- **Gap:** how a seat in a remote ssh or cloud location appears in the local sidebar and who owns its PTY.
- **Gap:** split minimap rendering at 1000 seats.
