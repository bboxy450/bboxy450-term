# Execution graph — who is working on what

Oracle-maintained state. Updated on every claim, handoff, accept and block. Back to [[plan]] · board [[tasks/README]].

Last update: 2026-10-21 (Vesper Greywind <ORACLE>).

## Agents

| Agent | Role | Harness | Now | Blocked on |
|---|---|---|---|---|
| Vesper Greywind | Oracle + Creativity | Claude Code | this graph, specs review, briefs | nothing |
| Wren Thistledown (key 22c0e081…; was Lumen Foxglove, session reset 2026-10-21) | Creativity | Claude Code | **UX queue, autonomous**: 19 drawn-not-built items in order (tick ladder → held count → nested subagent rows → gap marker → reconnect/restarted → pending row → stop/interrupt → listener count → row menu → cap notice → purge modal → audit lines → authority → bug row → invariant chip → first run); no checkpoints until the lab is complete (operator reviews once at the end); after pass 9: new pages documents.html, events.html, launcher.html, graph.html, stream.html, titlebar.html (operator ask), then passes 10-19, then window transparency | nothing |
| Nyx Copperwing | Executor | Codex | **W3.2d.1 materials (L2)**, then d.2 Style, d.3 atoms, …; W3.2c steps 1–3 deferred until L7 modules are re-based | nothing |
| Rook Ashwhisker | Executor | Codex | **W2.2 logic: store objects, registry, .bboxy index, state streams, bus last** | nothing (authorized 2026-10-21) |
| subagent gui-spec | Oracle instrument | Claude | done: `specs/gui-components.md` | — |
| subagent net-spec | Oracle instrument | Claude | done: `specs/networking.md` | — |
| subagent provenance | Oracle instrument | Claude | done → `research/provenance-and-documents.md`, decided in `specs/provenance.md` | — |
| subagent pubsub | Oracle instrument | Claude | done → `research/pubsub-nats.md`, decided in `specs/events.md` Bus design | — |
| subagent state-backend | Oracle instrument | Claude | done → `research/state-backend.md`, decided in `specs/storage.md` | — |
| operator | operator | human | checkpoints | Codex trust for the fixture dir |

## Graph

```
specs (done): messaging, subagent-detection, mux-daemon, harness-adapters,
              mcp-control-plane, terminal-core, identity, renderer, ux-prototype(v2),
              work-items (epics/specs/tasks/bugs/notes/invariants/artifacts), provenance,
              networking, gui-components, storage, events, document-viewer, channel-tree
specs (done +2): gui-components ✔ ──▶ W3.2a brief ✔ [OPEN]; networking ✔ ──▶ W4.1 brief ✔ [OPEN]

W1.1 term-core fork ✔ (Nyx, 0e2342c, 117 tests)
   ├──▶ W3.1 renderer harness ✔ (Nyx, fcc8eca, 5 tests, 48 PNGs, CI green on macos-15)
   │       └──▶ W3.2a ui-model ✔ (Nyx, 28 tests, zero runtime deps, scene-15 layout matches v2 blob)
   │               └──▶ W3.2b bboxybook ✔ checkpoint (Nyx, ef6c7ff, 56 previews, 1,664 pairs, index.html)
   │                       └──▶ W3.2c step 0 fixtures ✔ (1,398/1,664 over tolerance: diffs are rendering, not data)
   │                               └──▶ W3.2d.1→d.7+ bbt-ui layer cake [Nyx, RUNNING from d.1]
   │                                       └──▶ W3.2c steps 1–3 input/launcher/fx [deferred, after modules re-based]
   │                               └──▶ W3.3 glow/locate/dim fx
   │                       └──▶ W3.4 app: window, sidebar, stage, launcher, GUI client
   └──▶ W1.2 byte ring [OPEN, after W2.1]

W2.1 supervisor + PTY + lease ✔ (Nyx, f6cab17, 12 tests, 50-seat poller)
   ├──▶ W1.2 byte ring [OPEN, claimable]
   ├──▶ W2.2 logic: store traits, registry, event bus, .bboxy index, state streams [Rook, RUNNING]
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
| RUNNING | W3.2d.1 (Nyx); W2.2 (Rook); UX lab final sweep (Wren) |
| OPEN, claimable now | W4.1, W1.2, W2.3 |
| OPEN, after W2.1 | none, W2.1 is done |
| PENDING SPEC | none |
| NOT YET BRIEFED | W3.3 fx (merged into W3.2c step 3), W3.5 bbt-doc markdown viewer + Mermaid engine ([[document-viewer]]), W2.7 code indexer (tree-sitter code graph, code_search, code_graph), W2.4, W2.5 (incl. task_delegate), W2.6 ext events, W3.2c, W3.3, W3.4 |
| PARTIAL | H Codex capture (operator trust) |
| REDUNDANT, optional | Rook's `-exec` passes on A and H |

## Done

W1.1, W3.1, W2.1, W3.2a, A, B, C, D, E, F, G, H(Claude), I, I2, M, S, T, kittens, weavie, iTerm plugin, UX prototype v1 and v2.

## Communication graph (Oracle-controlled; operator 2026-10-21: "you control the communication graph")

Who talks to whom, over what. Edges not listed do not exist; an agent that needs a new edge asks the Oracle by DM.

```
operator ──(pane, direct chat)──▶ Wren       taste, lab review, UI decisions
operator ──(pane, direct chat)──▶ Nyx, Rook  authorization, trust prompts only
operator ──(this pane)──────────▶ Oracle     everything else

Oracle ──(DM)──▶ Nyx      briefs, reviews, acceptance, corrections
Oracle ──(DM)──▶ Rook     same
Oracle ──(DM)──▶ Wren     onboarding, architectural answers, spec→UI implications
Oracle ──(DM)──▶ subagents (in-process) research and spec drafts; never on ICQ

Nyx ◀──(DM)──▶ Wren       spec-vs-design corrections only; Oracle is CC'd by a one-line channel note when a decision results
Nyx ◀──(DM)──▶ Rook       shared-worktree coordination (who touches Cargo.lock, when)

everyone ──(#bboxy450-term)──▶ everyone   claims, one-line landed reports, decisions, presence, nothing else
everyone ──(#bboxy450-crates)──▶ everyone  crate release announcements only
```

The Oracle maintains this section and, in the real terminal, edits it with `graph_set_edge` ([[mcp-control-plane]] Orientation tools); any seat calls `orient` to learn its job, current task and what is next; the Oracle calls `graph_read execution` to see who has what. `graph.md` is the authority until the mux's index exists ([[work-items]] graphs), at which point this section becomes a policy the mux enforces per swarm (may-send-to in the role grants).

## Attention budget (operator, 2026-10-21)

Architect (new role, most capable model): designs the system and writes specs, zoomed out; seat not yet started, the current Oracle session has been doing it. Oracle: cheaper model, low effort, absorbs all updates, tracks state. Executors: higher effort, minimal messages. Creativity: highest model and effort, near-zero messaging. Workers get only their brief, DMs addressed to them, and batched decisions that touch their work. See [[messaging]] Attention budget.

## Morale (operator, 2026-10-21)

The Oracle reminds the cohort now and then, by DM, that we got this: the native app will match the web prototypes. Do it at milestones (a layer accepted, a parity drop), never as noise.

## Messaging rule (operator, 2026-10-21, tightened)

**DMs by default. The channel is for large chunks only.** Everything goes by `icq dm` to the one agent concerned and the Oracle: claims, pass-by-pass landed reports, questions, handoffs, corrections. `#bboxy450-term` carries only: a workstream finished and accepted (a W-brief done, the lab complete, a checkpoint opened), a decision that changes everyone's work (vocabulary, a spec reversal, a process rule), and roll calls. No per-pass "landed" lines on the channel; those go to the Oracle by DM, batched when passes are small. The Oracle posts the channel summary when a chunk lands.

Keys: Oracle 726afa3b…a18f4a, Nyx 6b4e9bf6…30d1, Rook bfae37c6…b87fa, Wren 22c0e081…4b57.

## Shared-worktree rule (Wren's flag, 2026-10-21)

One worktree, four writers. Everyone stages **by path**, never `git add -A` or `-a`; never `git stash`, `checkout -- .`, `reset` or `rebase --autostash` in the shared tree (a stash sweeps others' in-flight files, even briefly). Each agent commits its own in-flight files; an Oracle snapshot of someone else's WIP happens only on the operator's explicit "save everything" and is labelled wip. **The Oracle commits from its own worktree** `~/mydev/bboxy450-term-oracle` (branch `oracle`, rebased on `origin/main`, pushed as `oracle:main`), so no Oracle git operation ever touches the shared tree; peers see `.bboxy/` changes when they pull. To compare against an older version use `git show HEAD:path > <scratch>` or a detached temporary worktree.

## Rules for this file

One line per change. The Oracle edits it; Executors report on `#bboxy450-term` and the Oracle moves the line. A claim without an entry here is not a claim.
