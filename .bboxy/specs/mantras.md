---
id: 01M5NKND80QY0J7D1TRW6ZQM6R
type: spec
number: 209
title: "mantras — the design's core rules, each tied to the gate that holds it or named as judgement"
status: decided
links:
  - { to: "[[vision]]", rel: direction }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[d53-evolve-and-restart]]", rel: decides }
  - { to: "[[work-items]]", rel: uses }
created: 2026-10-24T00:06:11Z
updated: "2026-10-24T04:24:45Z"
---
# Mantras

Owner: Corvin Duskmantle <ARCHITECT>. Distilled 2026-10-23 by a read-only subagent from all 156 records under `.bboxy/` plus `AGENTS.md`, under the Architect's brief and the operator's list of that evening (verbatim where quoted). Twenty-five mantras. Each names where it already binds, the executable gate that holds it today, the candidate gate where none exists, and the mantra it trades against. A mantra with no gate is judgement, and says so: no command, no invariant ([[invariants/README|README]]). The operator's words that lived only in chat are recorded here for the first time and marked **chat**.

Order: the operator ranked 1 and 2 first ([[d53-evolve-and-restart]]). The rest is grouped: product, surface, system, law.

## 1. Be flexible and always evolve

Build variants, pick by eye, pair native, discard or adopt. Operator: "invariance #1 BE FLEXIBLE AND ALWAYS EVOLVE, fast iteration, improve iterate experiment test experience discard or adopt!" ([[d53-evolve-and-restart]]). Binds: the lab loop (variants → operator pick → decision with the words → native under strict parity, [[rim-glow]] in one evening), the RFC loop ([[work-items]]), [[runbook-web-to-native]]. Gate: none; judgement, read by ZOOM OUT. Candidate: age of the oldest `draft` or `proposed` record against a budget, advisory. Tension: 4 and 17, the strict-geometry pairing is what makes iterating cheap and what slows a pick.

## 2. A restart loses nothing

No working state exists only in process memory; state lives in frontmatter the operator can see. Operator: "fast reload without loosing working state, even though native app on GPU rendering, state lives FRONTMATER that user sees restarts" ([[d53-evolve-and-restart]]). Binds: #11 restart and resume, #27 `layout_revision`, #38 (c) read marks in the account object, [[c1-live-seats]] §3, "Git is the database" ([[work-items]]). Gate: [[restart-keeps-state]], pending its test (RS1). Tension: 16, #11 accepts that unflushed capture bytes are lost.

## 3. Cockpit first; the end goal is the cockpit of an agentic planetary system

Operator: "Priority right now is a working terminal, with harnesses, local messaging, local tasks, a local provenance system and local events. That's it, but a sexy, sexy UI." ([[vision]]); **chat**: "cockpit of an agentic planetary system, that's the end goal". Binds: [[vision]] (agents and harnesses are the users, humans supervise; a thousand seats per swarm), #19 and #20 (a seat is a PTY, the cap is a product fact), epic [[2026-10-22-c-cockpit]]. Gates: [[seats-1000-bus]], [[seats-400-memory-idle]], [[seats-967-frame]] (pending a settled run). Tension: 16, v0.1 cut down against "design for it from the start".

## 4. Every pixel counts; the visual foundation outranks everything

Operator: "Visual parity and visual fidelity and our visual component library and GPU renderers: top priority." ([[vision]]); "Every pixel is precious" ([[ux-sidebar]]); "they should be centered, to be sexy! fix that!" ([[2026-10-23-mk1-centred-marks]]). Binds: [[ux-prototype]], [[gui-components]], [[rim-glow]] (padding does not grow), #35. Gates: [[parity-geometry]], [[glow-never-moves-layout]], [[rim-never-covers-text]], [[rim-size-is-layout]] (pending G3). Tension: 16, fidelity residue is the one place "super good enough" is refused.

## 5. Render everything on the GPU; shaders are a first-class UI tool; blocks are sexy out of the box

Operator: "We need our own wgpu-based component library: tables, blocks, divs, buttons that we can later style; style with shaders and sizing." ([[gui-components]]); "each block in our UI GPUGUI system should have ability to define this type of dynamic border and border swell … our blocks should be sexy out of the box!" (#43); "we are a pure GPU rendering library" (#46). Binds: [[renderer]], [[gui-components]], #43, [[rim-glow]] §7 and §8. Gates: [[parity-geometry]] and the three glow invariants. Tension: 6, an effect costs frame time and the budget decides (the 13.4 ms blur in [[graph]]).

## 6. Render at 120 Hz; the UI thread never waits; optimise from the meter, never ahead of it

Operator: "IO async … it should never block the UI thread, and we should render at 120hz." ([[runtime]]); "Do not optimise pre-emptively; optimise from the meter." ([[vision]]). Binds: [[runtime]] (8.33 ms; CPU ≤ 4, GPU ≤ 6), #10 budgets from the first measurement, [[c1-live-seats]] §5. Gates: [[ui-never-waits]], [[clippy-clean]], [[seats-967-frame]] (pending; blocked on logic ingestion, not the renderer). Tension: 5 and 22 spend the same 8.33 ms.

## 7. macOS first, one look on every platform; everything cross-OS is a facade over per-OS crates

Operator: "as a GENERAL RULE, everything cross-OS is done this way, with an eye on eventually running in wasm, on iPad and on Android" (#46); **chat**: "cross platform crate design"; "One unified look and feel on every platform, our own, transcending OS styles" ([[vision]]). Binds: #45, #46, [[desktop-notifications]], [[architecture]]. Gates: [[cross-os-facade]] (three dated exceptions), [[layering]]. Tension: 16, the exceptions are not refactored now.

## 8. Local first; cloud designed in, dummied in the UI, built later; no local-only shortcuts either

Operator: "We can have dummy ssh support and dummy cloud support, as if it works without it actually working, so we have everything in the UI." ([[vision]]); "for now simple on disk storage system in git works … KISS for now" ([[d47-decision-nodes]]); **chat**: "local first". Binds: [[vision]] (no local-only shortcuts in the mux or messaging boundaries), #7, #15, #25, [[d1-providers]], [[networking]] (never depend on a hosted service), "No database, no sidecar JSON. If git has it, the terminal has it." ([[work-items]]). Gate: [[local-first]] (new, held). Tension: cloud from day one against KISS.

## 9. Plugins are a folder of binaries with a shape; planned, not built; the built-ins stay opinionated

Operator: "i dont want this to derail our work, but lets start planning plugin system … for now we have built in shit, and opinionated built in shit" ([[plugins]]). Binds: #12, #41, [[plugins]] (trust before any run, contracts, the registry), per-app prefix. Gate: none; candidate `bboxy450-term plugin check <binary>` (P-0). Tension: 3, cockpit first against the composability test of [[vision]].

## 10. Everything is a graph, a graph of graphs; records are nodes, wikilinks are edges, every record has a number

Operator: "like one big graph of thinking" ([[d47-decision-nodes]]); "everything needs to have a # to it" ([[d48-every-record-has-a-number]]); "everything all the graphs and how they tie together! … endless zoom canvas" ([[d49-swarm-canvas]]); "wikilinks is the way", "codegraphs!" ([[work-items]]); **chat**: "everything is a graph, graph of graphs!". Binds: the seven graphs of [[work-items]], [[channel-tree]], `graph_read` in [[mcp-control-plane]], the Graph tab in [[ux-flow]], [[provenance]]. Gates: [[frontmatter-parses]], [[mermaid-fixtures]]; the sweep adds number presence and uniqueness and `[[#N]]` resolution. Tension: 16, the canvas is logged, not briefed.

## 11. Agnostic but opinionated defaults; opinionated, not locked; every default overridable

**Chat**: "agnostic but opinionated defaults!". Binds: "opinionated, not locked" (#12, [[architecture]]), roles overridable per seat ([[vision]]), numbers are defaults in settings not protocol ([[messaging]]), theme is data (#5), "the API takes a theme so others can bring their own" ([[open-source]]), [[cross-os-facade]]. Gate: none. Candidate: a `bbt-ui` gallery rendered under the neutral default theme, proving defaults are data. Tension: 7, our own look against bring-your-own theme.

## 12. Hackers and furries first; the electric-blue fox is the face of everything

Operator: "Hackers and furries first" (#12); "use my identity logo, png version everywhere" ([[ux-flow]]); **chat**: "FURRY! I AM A FURRY!"; "our profile identity picture as part of invariance" ([[d52-brand-and-face]]). Binds: [[vision]] (fox, furry name pools), [[d52-brand-and-face]], identity is not licensed ([[d50-licence-when-mit-when-apache]]), [[document-viewer]] images. Gate: [[images-resolve]] (hash-pinned face). Tension: none found.

## 13. Everything is branded bboxy450-; the GUI kit publishes as bboxy450-frontmatter

Operator: "everything is branded as bboxy450- prefix, everything!" ([[d52-brand-and-face]]); "bboxy450-gui library should be called bboxy450-frontmatter unless its too much to rename it" ([[d51-frontmatter-is-the-gui-kit]]). Binds: [[licensing]] §1 publish names, [[plugins]], [[desktop-notifications]], the channels. Gate: [[bboxy450-prefix]]. Tension: 16, the internal rename waits for the first release.

## 14. Licences: dual MIT OR Apache-2.0 for what we own, Apache alone where upstream forces it, never copyleft; corporate friendly, hardcore developer first

Operator: "MIT and Apache licenses, when to use what", "corporate friendly, hardcore developer first" ([[d50-licence-when-mit-when-apache]]); "lets do dual license, be smart about this" (#34). Binds: [[licensing]] §0a and the map, #34, #36, #50. Gates: [[licenses-present]], [[no-copyleft-deps]] (new, held over 483 dependencies). Tension: provenance forces `bbt-core` Apache-only; forks are kept honest by the brand, not the licence.

## 15. The suite is the product; the terminal is its first app; the GUI kit is open-sourced and reused

Operator: "all those crates are reusable part of #bboxy450-* suite, term is just a way we glue them together, later on we will build more apps using our GUI kit that all connects together!" ([[vision]], #39); "designed so we open-source them independently of the term app … document them so rustdoc gives agents nice docs." ([[open-source]]). Binds: #8, #39, #51, [[architecture]], "nothing in the app" across the specs. Gates: [[layering]], [[modules-through-components]]. Candidate: `cargo package --list` dry-run per publishing crate. Tension: 16, nothing is split or renamed before its plateau.

## 16. LESS IS MORE; KISS; super good enough, not perfect

Operator: "LESS IS MORE, KISS, super good enough, not perfect." (#1 to #12 round); "KISS for now so we can keep developing our system" ([[d47-decision-nodes]]); v0.1 cut down ([[vision]]); "high velocity is the goal" ([[tasks/README|README]] of tasks). Binds: every spec's lens line, taffy refused ([[gui-components]]), #24. Gate: none; judgement only, a line or dependency budget invites gaming; #39 keeps the app's line count against the crates as an advisory drift signal read at each architecture round. Tension: 3, 4, 8 all pull the other way, and the operator overruled the Architect once to lean in on identity (#4 of round one).

## 17. The spec signs off, not the agent; hard gates block, judgement advises; no command, no invariant

Operator: "embody it" (#16); the Architect's rules recorded as direction in [[vision]]. Binds: [[work-items]] rules, #28 (`verify_result`), `task_accept` refusals ([[mcp-control-plane]], [[w2-5-mcp-and-seat-wiring]]), [[invariants/README|README]] of invariants, every spec's "Acceptance, the spec signs off". Gate: the invariant set itself through `scripts/invariants.sh` on every handoff. Candidate: a named test that `task_accept` refuses the two cases #28 rejects. Tension: 1 is outside the gate on purpose.

## 18. Harnesses lie; a report is a claim; verification beats trust; a harness reads the images

Binds: "Verification beats trust." ([[vision]]), "An Executor's report is a claim." (`AGENTS.md`), "proven by pictures, not prose" ([[renderer]]), "Do not fabricate certainty from a blinking cursor" ([[harness-adapters]]), grade rules ([[subagent-detection]]). Gates: [[signed-main]] (refuses an empty log), and every `grep -q 'ok. 1 passed'` check, because a cargo filter that matches nothing exits 0. Tension: 16, clean-worktree gates cost disk and slots.

## 19. Provenance unbroken; every record signed, digest-pinned, numbered; a decision nobody can find is a rumour

Operator: "provenance in that system is very important" ([[provenance]]); "lets treat files as objects" ([[d47-decision-nodes]]). Binds: [[provenance]], the RFC loop with `on_digest`, #30 (an owner is never invented), [[architecture]] trust boundaries. Gates: [[signed-main]], [[frontmatter-parses]]; the sweep adds the number rule. Tension: 16, GPG checkpoints wait while the root is held; comments outside a seat are inferred grade.

## 20. Show useful information, never dumb information; every refusal, state and dialog names what it is about

**Chat**: "show user useful information, never show user dumb information", the 1Password dialog that asks for a fingerprint without saying what for. Binds: "If a row needs two lines of explanation it is wrong." ([[ux-sidebar]]), the four questions at a glance ([[ux-prototype]]), "unknown is never a failure" and evicted history never invents a count ([[ux-flow]]), the notification body is the first line of the question ([[attention]]), "refused with the reason named" (#19), "a refusal name never lies" (#40 (e)), the OS error verbatim ([[desktop-notifications]]), `KTRnnn <consequence>. Repair:` ([[yarn]]). Design rule from `AGENTS.md`: no eyebrow text, ever. Gate: none. Candidate: a scan that every refusal variant in `bbt-proto` carries a non-empty reason, and a parity case asserting the unknown ladder is never red. Tension: 4, icons over text against saying enough.

## 21. Always be useful; a seat can orient, take work, report and claim on its first turn; the primitives compose anything

**Chat**: "always be useful". Binds: [[vision]] (agents are the users; the composability test: if a loop needs a feature we do not have, the primitives are wrong, not the loop), `orient` is one call and enough to resume cold ([[mcp-control-plane]]), [[w2-5-mcp-and-seat-wiring]] (first turn calls `orient`), agent-readable docs ([[open-source]]). Gate: none. Candidate: w2-5 §5 acceptance 1 and 3 as named tests; a fixture swarm running one autoresearch iteration end to end. Tension: attention budget, useful is not chatty ([[messaging]]).

## 22. Nothing is dropped, nothing is deleted; store everything, every size; unknown is a state, never a failure

Operator: "store everything, every size; enterprise-grade compatible; durable objects" ([[storage]]); "Nothing is dropped." ([[vision]]). Binds: purge moves to cold blobs ([[storage]]), lower priority delays and never deletes ([[messaging]]), "Busy is backpressure, never fatal" ([[mux-daemon]]), "never drop without a `gap`" ([[events]]), the terminal never deletes a work item ([[work-items]]). Gate: [[seats-1000-bus]] (every message delivered). Candidate: a scan that no `DELETE` or `UPDATE` touches the events log, and a replay test that every `gap` names its range. Tension: 3, never-delete at a thousand seats is a cold-storage bill.

## 23. Seats are sockets; everything is an event on one bus; messages, tasks and docs are the same kind of thing

Operator: "EVENT DRIVEN SYSTEM. Tasks and markdown docs live side by side with the event system: touch a doc, whoever subscribed gets the update." ([[events]]); "routing becomes symlinking sinks" ([[channel-tree]]); "messages are just an abstraction on top of the event bus". Binds: [[events]], MCP is one client of the bus ([[mux-daemon]], [[mcp-control-plane]]), seats are sockets ([[vision]]). Gate: none. Candidate: a test that a seat-socket request and the equivalent MCP call write identical control-journal rows. Tension: 16, our own bus in two thousand lines, no clusters.

## 24. One concern at a time; one layer per brief; fix a problem at the layer it comes from

Operator: "we are building from scratch, so it is a layer cake of abstractions; we need more layers of granularity" ([[gui-components]]); "Layout first, then style, then behaviour, then performance." ([[vision]]). Binds: a diff at layer N is fixed at layer N ([[gui-components]]), a scaled reference is a lab bug fixed at the source ([[renderer]]), every spec's "one concern each" brief order. Gate: [[modules-through-components]]. Tension: 16, a vertical slice ships faster than a layer; the repo chose the layer and pays in brief count.

## 25. Visualization is always ours; plugins bring data, never pixels; everything pluggable with opinionated defaults

Operator: "I WANT VIZUALIZATION TO ALWAYS BE OURS, and we plug into data sourcse! like document bases, code hosting, git or what not, everything pluggable and with opionated defaults!" ([[d255-visualization-is-ours]]); earlier: "opinionated not locked" (#12), "for now we have built in shit, and opinionated built in shit" ([[plugins]]). Binds: [[plugins]] §1 (contracts first, MCP only the wire; a contract returns records, resources and results, never UI; built-ins are the default providers), #249, [[d41-plugins]], the one look on every platform ([[vision]]). Gate: none yet. Candidate: `plugin check` refuses a shape whose result schemas carry a UI type (P-0). Tension: 9, a plugin that could draw would ship faster for its author; the cockpit's look is the product, so it does not get to.

## What this record is for

Briefs cite a mantra by number alongside the decision; a new spec's lens line names the mantras it serves and the one it trades against; a candidate gate above becomes an invariant when someone writes its command, and the mantra's line here changes from candidate to gate the same day. Twenty-five is the count (the twenty-fifth earned its place the next day, in the operator's words); the next one earns its place or does not get in.

Owner: Corvin Duskmantle <ARCHITECT>.
