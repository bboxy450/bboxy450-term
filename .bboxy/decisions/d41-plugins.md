---
id: "01M5N5BWN0SNZ22DE28C1QJ7BA"
type: "decision"
number: 41
title: "Plugins: planned and logged, not built"
status: "decided"
decided: "2026-10-23"
operator:
  - "there is a folder of binaries for bboxy450-term, called plugins, each plugin is named <app name, bboxy450-term in this case>-<system it plugs in, eg knowledge_base>-<provider name, notion, linear, gitkb>, this binary has shape, so if we run just binary it outputs json shape, saying here is how i talk to the world, for each system bboxy450-term has contract, that needs to be fulfilled by plugin, shape says here are my commands, we dont care what happens inside binary, can work locally, can have login system, not up for us, login is a step on setup of plugin, and then term can ask, hey dear plugin are you logged in -> no then we will run login command, but basically its deterministic way of dealing with plugins, we can tap into decision models like Jev in that implementation, so then we can use any external system to plug into, we write adapters for NATS messaging lets say, or for Notion, or for Obsidian, you name it, we have a guide people feed into their harness saying i want X to work as a plugin in system Y for bboxy450-term and done!"
  - "task system can be plug in, event system can have sink plugin for external messaging system as well, external sessions, external data sources"
  - "we can have plugins for repo management, mono repo, weavie repo based on nix, meta cli by my friend matt, meta repo he calls them, whatever people want to use to structure their codegraph, documentgraph, workgraph, executiongraph … we just render harnesses and plug them in, and we expose plugins via our MCP and via our hooks as well, they can tap into that and say in shape, hey i want to be run on this hook before or after, please run me! we have this hot sexy registry of all the plugins, and we have flamegraphs and timings and we can tell users hey this is taking 10s on each hook, its not ok"
  - "beauty of plugins we have very fixed contract of how we talk to them, and that way we can iterate on plugins very very very fast, without having them monolithically bundled with us"
links:
  - { to: "[[2026-10-22-architecture-round-1#41]]", rel: "supersedes-as-index" }
  - { to: "[[plugins]]", rel: "decides" }
  - { to: "[[work-items]]", rel: "decides" }
created: "2026-10-23T19:56:20Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 41: Plugins: planned and logged, not built

**Plugins: planned and logged, not built (operator, 2026-10-23, verbatim
in [[plugins]] §0).** A folder of binaries named `<app>-<system>-
<provider>`, each printing its JSON shape when run bare, fulfilling
app-owned versioned contracts per system (task_system, event_sink,
session, data_source, repo_graph, provider), a deterministic protocol
(shape → status/login → health → calls) over stdio JSON-RPC or a named
socket, hooks before/after with budgets, exposure through MCP and hooks,
a registry with timings and flame graphs, a guide any harness can
follow. Built-ins stay built in and opinionated; every pluggable system
gets its contract now and the built-in is the contract's reference
implementation, the task system first. Status: draft until the operator
and the Architect talk. Rejected for v1: in-process dynamic libraries
(no stable Rust ABI, a crash is ours), WebAssembly plugins (sandboxing we
do not need yet, language and IO limits we would pay for), PATH-prefix
discovery (a folder is the operator's shape and is auditable). The "Jev"
reference resolved by the research: TypeSafe's decision model, so plugin
selection and routing may use a decision model while contract,
protocol and authority stay deterministic.
**Document types (operator, same day):** `research`, `proposal` and
`comment` join the work-item types (supersedes [[d13-document-types|#13]]'s rejection of
research; the operator's call), and the RFC loop is defined in
[[work-items]]: proposal → red-team comments by a Claude Executor seat
→ decision → spec, every record signed and digest-pinned so the whole
argument lives in provenance. The plugin research and spec are the first
red-team subject once the research lands; it ran the same day: Wren
filed 19 comments at 16b4c98, 18 are folded into revision 2 of
[[plugins]] with a reply comment each (host in logic plus a
`bbt-plugin` kit crate, acceptance never crossing a contract,
`--shape`, the system in tool names, a call-and-hook timeline with
opt-in trace spans, MCP-style framing and MAJOR.MINOR versions, hook
semantics per phase, a bare login seat, trust records before any run,
shape caching, trust tiers per contract), and one, the suite-wide
prefix, is the operator's: closed the same evening, **per app**
(`bboxy450-term-<system>-<provider>`; a second suite app has its own
folder and prefix, the contracts in `bbt-plugin` are shared; rejected:
a suite-wide prefix, two prefix rules). The operator also corrected
the Jev reading: Jev is TypeSafe's hosted decision primitive already
used by kittens and paperhands, so `decision_model` is a plugin
system with Jev its first provider, optional and never load-bearing.
Wren's two follow-ups (a817a9d) close in revision 3: trust records
live only in the operator's store, never read from a repository; the
diagram matches the text. Outside the cockpit,
comments and proposals are commit-signed files with an inferred author
grade; seat-leaf signing and a claim upgrade them to reported when the
reviewer runs as a seat (Oracle's gap, same day). Rejected: a second
signing path for ICQ sessions (the cockpit is the path).
