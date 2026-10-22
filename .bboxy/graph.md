# Execution graph — who is working on what

Oracle-maintained state. Updated on every claim, handoff, accept and block. Back to [[plan]] · board [[tasks/README]].

Last update: 2026-10-21 (Vesper Greywind <ORACLE>).

## Agents

| Agent | Role | Harness | Now | Blocked on |
|---|---|---|---|---|
| Vesper Greywind | Oracle + Creativity | Claude Code | this graph, specs review, briefs | nothing |
| Wren Thistledown (key 22c0e081…; was Lumen Foxglove, session reset 2026-10-21) | Creativity | Claude Code | **standing by**: lab complete (30 pages, design system v54, layer-reference pages for L2/L3/L6); awaiting the operator's review points. Previous queue: **UX queue, autonomous**: 19 drawn-not-built items in order (tick ladder → held count → nested subagent rows → gap marker → reconnect/restarted → pending row → stop/interrupt → listener count → row menu → cap notice → purge modal → audit lines → authority → bug row → invariant chip → first run); no checkpoints until the lab is complete (operator reviews once at the end); after pass 9: new pages documents.html, events.html, launcher.html, graph.html, stream.html, titlebar.html (operator ask), then passes 10-19, then window transparency | nothing |
| Nyx Copperwing | Executor | Codex | **W3.2d.7.10 DocumentModal**; 7.9 Settings ✔ (cherry-picked as 5b84d40: 72/72 geometry, 24 text-AA reviews ≤2.59%; follow-up: lab-defined select caret and textarea grip); 7.8 GraphTab ✔ (198b243: 64/64 geometry, 48 within policy, 16 text-AA reviews ≤2.07%; support: x-height d8f2b86, atlas 39d7245 (encode 33→4.9 ms), paths d10585f); 7.7 StreamDock ✔ (527dad4: 168/200 current within policy, 32 pixel-only text-edge reviews, geometry 200/200; italic ✔ cfa6328, Flow ✔ 3147921); 7.6b text raster ✔ (270582f: reviews 172→81 of 1,096 current, zero new, geometry 1048/1048, terminal smoke 48/48 identical); 7.6 Launcher ✔ (2bf3601: 144/176, geometry all pass, compact picker 64/64); 7.5 TitleBar ✔ (af73b4f: 0/40), Image element ✔ (362793a); 7.4b ✔ (dc3018d: group opacity, 147→140 reviews, zero new); logged L1 text-raster parity item for after modules (remaining residue is 89–93% inside text); L5 snapshots ✔ (910adfe), kind colours ✔ (a17d912), per-node glow ✔ (4884aa5, 65bd72c), 7.4 Pane+Strip ✔ (5734cc4: 120/136, geometry all pass); 7.3b ✔ (f2961ab: zero geometry failures across 744 current cases; Sidebar 4/72, Style 0/112); 7.0 ✔ (47500ea), 7.3 ✔ (f2f9552: 6/136 pixel-only); 7.2 TabRow+MiniGrid ✔ (6b4f2bc: 0/208 review); 7.1 SeatRow ✔ (f811a9d: geometry all pass, 71/296 pixel-only residue); d.6 ✔ (4bf9d4f: overlays 16/16; L6 complete); d.5 ✔ (dca0a05: 24/32 within policy, 8 Button cases visually equivalent; 1000-row List/Table virtualised); Style 0/88 and components 0/24 after font weights (01edd5f); per-case policy ✔ (3a38388), d.4 matrix 0/24 review after Wren ceafbe8, d.1e withdrawn, then d.6 Button/List/Table/Input; d.1c ✔, kerning ✔ (ef1267c), d.4 ✔ (efacd88: text geometry 1.447→0.015 px); d.1d per-size SVG raster later; d.1 ✔, d.2 ✔, d.1b ✔ (materials 8/32, Style 56/88), d.3 ✔ (cc99459: atoms 103/112, worst 4.09%→1.55%); W3.2c steps 1–3 deferred until L7 modules are re-based | nothing |
| Rook Ashwhisker | Executor | Codex | **S1 bbt-supervisor split (#1)**; W2.4 ✔ (1f9442d: signed envelopes, channel tree, ledger, 4:1 fairness + 10/s burst 30, mute, guarded delivery; 15 messaging acceptance tests; clean ci-local 413 tests, re-run by Oracle exit 0); W1.2c ✔ (01d5ea4 merged 097ec3b: shared writer thread, 128 MiB at 54–57 MiB/s, max write 12.8 ms, p99 4.7 ms, 0 full-queue episodes); W2.3 ✔ (a00e53e+3de2ae1 merged d60ee06: Claude/Codex/zsh adapters, guarded native delivery, quiet hook shim warm ~5 ms, 11 adapter tests); W1.2b ✔ (b12dfcb: logic stopped, 20 MiB writer completes, cold replay no gap; W1.2c latency ≤50 ms / ≥30 MiB/s queued); W1.2 ✔ (c69fe4c: CRC ring, checkpoints, cold archival, paginated history, 37 tests); W2.2 ✔ (4988ac3 merged: SQLite store/blobs/events, registry, bus on the R1 handle, .bboxy index, seat streams; hard-killed logic reattaches same zsh PID and screen; 155 tests on merge); resize ✔ ffe2fae; vendor lint scope ✔ ae0855d | nothing |
| Corvin Duskmantle | Architect | Claude | worktree ~/mydev/bboxy450-term-arch (branch arch, pushes arch:main). Round 1 ✔ (6f8ef1f: .bboxy/decisions/2026-10-22-architecture-round-1.md, 11 decisions; invariants/layering.md + scripts/layering.sh in CI). Next: specs/architecture.md from the code → C1 spec → W2.5/W2.5b → D1 → identity custody → mux-daemon split → open-source §1. Agenda: (a) specs/architecture.md system map from the code, (b) C1 app↔logic seam, (c) O2 public/private split, (d) W2.5/W2.5b MCP and seat wiring, (e) D1 Provider trait, (f) 1000-seat risks; writes specs, the Oracle briefs Executors | nothing |
| Kestrel Voidwing | Executor | Codex | **ID1a identity root and device** (spec id1-seat-chain, arch #4), then ID1b, ID1c, D1; worktree ~/mydev/bboxy450-term-exec5 | nothing |
| Velvet Vulpine | Executor | Codex | P1 ✔ (8e66b3a), P2 ✔ (51e7d00: invariants.sh in ci-local, 4 checks + parity under --parity); **PKG0 dev code signing**, then fidelity residue (F3 ghost border, F4 rim/edge); worktree ~/mydev/bboxy450-term-exec6 | nothing |
| Aster Cinderwing | Executor | Codex | **T2a headless terminal fidelity ∥ T3 fonts**; T1 ✔ (dc06057 merged a195d89: 10 scenes, 19,200/19,200 cells match xterm.js; pipe 2.8 GiB/s, PTY 95 MiB/s); R3 waits for the C1 spec; then R5; worktree ~/mydev/bboxy450-term-exec4 (branch exec4); owns terminal rendering and terminal fonts | nothing |
| Sable Embertail | Executor | Codex | R1 ✔ (29d5e62), R2 ✔ (be604d4: native window, 15 seats, idle 0 frames, meter; release encode 33 ms/frame); R2b ✔ (82c821b: 87% of encode = 16 MiB icon atlas + glyph atlas re-uploaded every pass; fix → Nyx); R4 ✔ impl (18e0593: native display link, idle 0 frames/callbacks; 75 Hz budget FAIL: GPU 19 ms, 13.4 ms = full-screen frosted blur for a 38 px bar); R4b ✔ (6f67402: GPU 19.2→8.9 ms, 75 Hz 0 late, 1808/1808 identical; 8.33 ms gap → R4c queued); **F1 per-size icon raster** in worktree ~/mydev/bboxy450-term-exec3 (branch exec3), then F1–F5 fidelity backlog, C1 live seats | nothing |
| subagent gui-spec | Oracle instrument | Claude | done: `specs/gui-components.md` | — |
| subagent net-spec | Oracle instrument | Claude | done: `specs/networking.md` | — |
| subagent provenance | Oracle instrument | Claude | done → `research/provenance-and-documents.md`, decided in `specs/provenance.md` | — |
| subagent pubsub | Oracle instrument | Claude | done → `research/pubsub-nats.md`, decided in `specs/events.md` Bus design | — |
| subagent state-backend | Oracle instrument | Claude | done → `research/state-backend.md`, decided in `specs/storage.md` | — |
| operator | operator | human | checkpoints | Codex trust for the fixture dir |

## CI (operator, 2026-10-22): local only

No GitHub CI during heavy development; the workflow runs on manual dispatch only (billing had also refused every run since 07:27). **Every Executor handoff and every Oracle acceptance runs `sh scripts/ci-local.sh`** (layering, workspace build, workspace tests, UI IO lint; `--parity` when lab, design, UI or renderer changed) and reports the exit. **Disk:** the volume is near full (four worktrees × their targets ≈ 60 GiB). Delete a private CI target as soon as its gate exits; prune your own superseded `target/` run dirs and unused profiles; never touch another agent's. **Run it in a clean worktree of the commit being handed off** (`git worktree add --detach <tmp> <sha>` on top of origin/main, with its own `CARGO_TARGET_DIR`), never in the shared tree: peers' in-flight sources and concurrent cargo runs make a shared-tree gate non-hermetic (W2.4 hit missing symbols that were present in source). CI returns per crate when it is open-sourced into a public repo. Queued P1 (Architect): geometry parity failures fail the gate hard; pixel review stays advisory.

## ICQ keys (DM addresses)

| Name | Role | Key |
|---|---|---|
| Vesper Greywind | Oracle + Creativity | 726afa3bb257faf85781e90a3ff97bbe826449f3ae4300c39ce4a8ba0ca18f4a |
| Corvin Duskmantle | Architect | 6b172639511c39982cbdb35272730510078889d2576670928432da09d5c08a73 |
| Nyx Copperwing | Executor (UI modules) | 6b4e9bf61a1302b1c8a84c5cd0faf94580025217ad0ae20a68f7d712062130d1 |
| Rook Ashwhisker | Executor (logic, messaging) | bfae37c6a294e8103c578f0ea3921f576930c668d18471c5760ca923103b87fa |
| Sable Embertail | Executor (runtime, renderer, app) | b04a60914af6d5622d4f21eadf214bb78cdb795051907a0ec8dd07914af7c3f5 |
| Aster Cinderwing | Executor (terminal rendering, fonts) | b2a1d2db42014c0e52ed6347e8ba878dcc8928a479866366b0bdc2b8c567bdf9 |
| Kestrel Voidwing | Executor (identity, then dummy providers) | 2868e75c4e031b6ec826d1f7a82af9f34df2968f1ea541ed5334e97b99e5f03a |
| Velvet Vulpine | Executor (gates, renderer residue) | 99a17c42af1f2c94943965c002f12c3d6ded0fedbbebfe5956d8bbd9ff311015 |
| Wren Thistledown | Creativity (lab) | 22c0e081a5f4ac211b1c27d219914d330d4b9edea6006e075edc9271ba884b57 |

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
   ├──▶ W2.2 logic: store, registry, bus, .bboxy index, state streams ✔ (Rook, 4988ac3)
   │       └──▶ W3.4 app GUI client
   └──▶ W2.3 adapters Claude/Codex/zsh [OPEN, claimable]
           ├── needs: osc7501-claude.bin ✔, osc7501-codex.bin ✘ (operator trust)
           └──▶ W2.4 local messaging on loopback (Transport trait from W4.1)
                   └──▶ W2.5 MCP server + seat sockets

R1 bbt-rt runtime ✔ (Sable, 29d5e62) ──▶ R2 app shell + hitch meter ✔ (be604d4; release encode 33 ms/frame, R2b profile running) ──▶ R3 terminal data path ∥ R4 120 Hz display link ──▶ R5 stall test
   └── R2 replaces W3.4's window/loop part; R3 consumes W2.1

O1 bboxy450-rt docs+publish [DEFERRED, velocity first] ──▶ (after W3.2d.7.10) O2 split public/private ──▶ O3 ui-core ──▶ O4 render ──▶ O5 ui ──▶ O6 book

TOP PRIORITY: visual foundation (renderer, library, parity). COCKPIT (operator 2026-10-22, epic C): Sable F1 → C1a → C1c → F2–F5 | Aster (exec4) T1 ✔ → T2a ∥ T3 → C1b watched grids (supersedes R3) → R5 | Nyx 7.10 → W3.5 bbt-doc (#14) → W3.2c → L8 → L9 | Rook W2.4 local messaging → S1 bbt-supervisor split (arch #1) → W2.5a–e MCP + seat wiring (spec w2-5-mcp-and-seat-wiring, arch #3) → W2.5b seat wiring (MCP, hooks, skill) → W2.6 provenance/events → D1 dummy ssh/cloud | Wren lab refs

W4.1 Transport trait + loopback + quinn auth [DEFERRED: cockpit first]
   └──▶ phase 2 cloud: SshProvider bootstrap, remote supervisor
           └──▶ phase 2b private cross-instance messaging (punch, relay)
                   └──▶ phase 3 global pubsub, DHT, phantoms (iroh re-eval here)
```

## Open work by state

| State | Items |
|---|---|
| RUNNING | 7.10 DocumentModal (Nyx); S1 supervisor split (Rook); F1 icons (Sable) |
| OPEN, claimable now | W2.3 (W4.1 queued for Sable; W1.2 → Rook) |
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

The Oracle reminds the cohort now and then, by DM, that we got this: the native app will match the web prototypes. Do it at milestones (a layer accepted, a parity drop, a measured speedup), never as noise. Operator, 2026-10-22: **"WE GOT THIS, WE ARE INEVITABLE in this effort!"** Relay it with the concrete milestone that earned it (first relayed with: vector paths, 7× renderer speedup 33 → 4.9 ms, the live native window, seats surviving a hard kill).

## Messaging rule (operator, 2026-10-21, tightened)

**DMs by default. The channel is for large chunks only.** Everything goes by `icq dm` to the one agent concerned and the Oracle: claims, pass-by-pass landed reports, questions, handoffs, corrections. `#bboxy450-term` carries only: a workstream finished and accepted (a W-brief done, the lab complete, a checkpoint opened), a decision that changes everyone's work (vocabulary, a spec reversal, a process rule), and roll calls. No per-pass "landed" lines on the channel; those go to the Oracle by DM, batched when passes are small. The Oracle posts the channel summary when a chunk lands.

Keys: Oracle 726afa3b…a18f4a, Nyx 6b4e9bf6…30d1, Rook bfae37c6…b87fa, Wren 22c0e081…4b57.

## Shared-worktree rule (Wren's flag, 2026-10-21)

One worktree, four writers. Everyone stages **by path**, never `git add -A` or `-a`; never `git stash`, `checkout -- .`, `reset` or `rebase --autostash` in the shared tree (a stash sweeps others' in-flight files, even briefly). Each agent commits its own in-flight files; an Oracle snapshot of someone else's WIP happens only on the operator's explicit "save everything" and is labelled wip. **The Oracle commits from its own worktree** `~/mydev/bboxy450-term-oracle` (branch `oracle`, rebased on `origin/main`, pushed as `oracle:main`), so no Oracle git operation ever touches the shared tree; peers see `.bboxy/` changes when they pull. To compare against an older version use `git show HEAD:path > <scratch>` or a detached temporary worktree.

## Rules for this file

One line per change. The Oracle edits it; Executors report on `#bboxy450-term` and the Oracle moves the line. A claim without an entry here is not a claim.
