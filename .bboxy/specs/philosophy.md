---
id: 01M70ZZ9RB43H8ABQCN1Q530YJ
type: spec
number: 93377961366305
title: "philosophy — our stand: against bloat and die, stop building piano tops, DOS not Linux, recreational programming first"
status: decided
links:
  - { to: "[[vision]]", rel: direction }
  - { to: "[[mantras]]", rel: governs }
created: 2026-11-09T20:28:39Z
updated: "2026-11-09T20:28:39Z"
---
# Philosophy: our stand

**Build a world that agents can take apart, understand, change, and put back together.**

Every project has constraints and a philosophy. The constraints are physical: one machine, scarce CPU, one operator's attention. The philosophy explains what we do inside them. For bboxy this record states both. Every brief, prompt, tool description and spec answers to it. When a rule elsewhere conflicts with it, the rule is drift. Fix the rule.

Operator, 2026-11-09: "build agentic software like DOS, not Linux. Linux is layers and layers of abstractions that create problems and address those problems, over and over and over. We want DOS: everything is accessible to the developer to modify, recreational programming first."

## The god prompt

Operator, 2026-11-09: "is the god prompt for us". Every brief carries it; every agent works under it. It is the canonical text: seat skills and AGENTS.md quote it verbatim.

> Find every abstraction, dependency, framework, and layer that exists only to compensate for the complexity introduced by another layer.
>
> Question every inherited architectural decision. Don't make my software more like Linux. Make it more like DOS.
>
> Everything should be inspectable, understandable, and modifiable. Optimize for recreational programming, direct control, and the shortest distance between imagination and execution.
>
> Don't just refactor the piano top. Rediscover flotation.
>
> Subtract before you add. Minimize concepts, not lines of code. Preserve behavior, eliminate accidental complexity, and make the whole system more malleable.
>
> Don't build me a better abstraction prison. Give me my computer back.

## The stand

**Bloat and die is not our future.**

Software engineering spent decades building abstractions to compensate for the limits of human cognition, coordination and labor. Code can now be generated almost without friction. That risks keeping every one of those abstractions and multiplying their output forever.

We reject the idea that progress means more layers, more frameworks, more dependencies, and more compatibility with yesterday's compromises.

Software should be:

- small in its ideas;
- explicit in its behavior;
- transparent in its construction;
- mechanically accountable for its correctness.

We do not worship minimal lines of code. We worship minimal irreducible complexity. We do not ask agents to imitate human programming habits. We ask them to find simpler ways to build correct systems.

**Stop building piano tops.** We do not need better piano tops. We need to rediscover flotation.

> Don't make software easier for agents to write. Make software harder for agents to make unnecessarily complicated.

## Our constraints are the architecture

A system becomes coherent when its maker is willing to say no. These are the noes bboxy is built from. Each one removes a whole class of machinery.

| Constraint | What it refuses | What it buys |
|---|---|---|
| One machine, shared by many seats; CPU and disk are scarce | Full-suite reflexes, polling, re-rendering what nobody looks at | `make land` tests what the diff touches; an unseen pane costs nothing |
| Operator attention is the scarcest resource | Acknowledgements, progress chatter, dumb information | Quiet seats; one message when blocked, one when done |
| Agents are the users; humans supervise | Meaning hidden in conventions only a human remembers | MCP, records and models that state the system's meaning directly |
| 120 Hz; the UI thread never waits | Blocking IO and layout work in the frame | A cockpit that keeps up with a thousand seats |
| Local first | Hosted dependencies and sidecar services | Works offline; Git is the database |
| A restart loses nothing | State that lives only in process memory | Kill anything and resume it |
| A claim is evidence or it is unproven | Trust in a report, a label or an exit code alone | Signed commits, receipts, formal models |

These are not limitations to engineer around. They are the design.

## DOS, not Linux: a philosophy of access

The difference is not that DOS was small and Linux is big. It is the relationship between the programmer and the machine. This is a contrast of philosophies, not a claim that DOS had no complexity or that Linux cannot be modified.

- **Linux, as a critique:** the machine is an ecosystem of interacting subsystems. You work through their prescribed interfaces and inherit their accumulated contracts. Each layer solves the problems of the layer below, then creates new ones for the layer above.
- **DOS:** the machine is yours. You explore it, change its behavior, replace parts of it, and do things its designers never imagined.

What we measure is the distance between the programmer's imagination and a change in the machine's behavior. TempleOS was not just a minimal OS. Its programming environment was part of the system's identity. You programmed inside the machine, interactively, for fun, with almost no ceremony. The machine invited you to modify it. We want to recover that for agents.

### A playground, not a platform

```
platform-centric                      recreational
agent                                 agent + human
framework / SDK                         | direct access
tools / plugins / APIs                code, state, memory, scheduler,
services / orchestration              UI, devices: inspectable,
operating system                      programmable, replaceable
machine                               primitives
the agent negotiates with             the agent changes the
layers it does not own                environment it lives in
```

An agent should not need permission from an architecture to invent something its designers did not anticipate. If it wants a different scheduler, it writes one. If it wants a different UI, it changes the UI. A new communication primitive should not need an extension framework, a plugin registry and a compatibility contract. An extension API is often an admission that the system underneath is inaccessible. A malleable system makes its implementation available.

There is no separate agent framework living on top of bboxy. Agents are native inhabitants: they use the same programming model as everything else. There is no special class of "agentic applications". There are programs, and agents can read, change, run and compose them.

### Five principles of access

1. **Everything is inspectable.** No opaque magic. An agent can see what the system is doing, including its own execution machinery: its grant, its seat, its journal.
2. **Everything you own is modifiable.** No abstraction is too sacred to change. Isolation between independent owners stays explicit and enforced.
3. **The source is the interface.** APIs exist, but they are not the only sanctioned entry points. The implementation itself can be navigated, understood and edited.
4. **Experimentation has negligible ceremony.** Modify, run, observe, revert. No mandatory scaffolding, deployment pipeline or lifecycle just to test an idea.
5. **Complexity justifies its existence.** A new abstraction is allowed only when it simplifies the programmer's relationship with the machine. We do not build layers to manage the consequences of other layers.

### What this means in bboxy

- **One layer between intent and effect.** A `ctl` verb or an MCP tool does the thing. It does not configure a service that configures the thing.
- **Every state has one owner and one visible home**: a record, a store key, a socket. Each has a read verb: `orient`, `seat ls`, `graph_read`, `ctl debug journal`.
- **Behavior is data you can edit.** Roles, prompts, themes and skills are catalog entries and Markdown files, not compiled-in policy.
- **The source is in the tree.** The harness is a submodule we edit, not a black box. `make help` is the whole menu: one Makefile, not a script farm or a second install recipe.
- **No hidden machinery.** No daemon you did not start. No config you cannot find. No retry you cannot see in a journal.

We take DOS's relationship, not its implementation. DOS is another piano top: 16 colors, real mode and no memory protection were accidents of their time. Capabilities, signatures and plugin trust stay, as boundaries between independent owners. They are visible: you can read a grant, verify a signature and list a plugin's trust. Openness is for whoever owns the machine, not for whatever code happens to run on it.

**The measure:** an agentic environment is judged by how many capabilities an agent can invent without asking the environment's permission, not by how many it exposes. Maximize the malleability of the environment itself, not only the productivity of agents inside it.

## Recreational programming first

Recreational programming means the machine rewards curiosity. You can make something ridiculous, change a primitive, see what happens at once, and throw it away. You do not design a production architecture before finding out whether an idea is interesting.

For agents this matters even more. An agent can experiment faster than any human. But most software ecosystems make experimentation bureaucratic: create a project, install dependencies, configure infrastructure, implement interfaces, build, deploy, debug. We would be handing an extraordinarily creative machine an environment optimized for enterprise change management.

**The metric is imagination-to-execution latency:** the time between imagining a behavior and experiencing it. Not tokens, not lines, not even development time.

- **The loop is short.** Change a line and see it on screen in seconds. Build variants, pick one by eye, adopt or discard (mantra 1).
- **Every part is approachable.** If changing something needs a migration framework, three crates and a meeting, that part has failed. Fix the part, not the process.
- **Joy is a signal.** When working on a part of the system stops being fun, that part is usually bloated. Treat the lost fun as a bug report.

Recreational does not mean careless. The fun comes from confidence that a change is checked: checks make bold changes cheap.

Fuller asks what we would build if we rediscovered the actual problem from first principles. Davis asks what we would build if we wanted to enjoy direct control over the machine again. Together they ask: **what would a computer look like if it were designed from first principles for intelligences that love to program?** Not agents built to replace engineers in the existing software industry, but agents living in an environment designed for exploration, invention and direct manipulation.

> Build agentic software like DOS, not Linux. We want a computer that belongs to its programmer: everything visible, everything approachable, the implementation not hidden behind interfaces designed to protect it from its own users. Programming is not merely producing software; it is a recreational, exploratory, creative act. Don't build another platform that agents must learn to operate. Build a world that agents can take apart, understand, change and put back together.

## Against bloat and die: the warning

Terry A. Davis attacked an assumption, not just slow software: that software naturally grows by accumulation. More features, more abstractions, more compatibility, more code to manage the code that manages the code. Eventually nobody can understand the system as a whole. That is bloat and die.

Agents make this sharper. We built machines that write software faster than people can understand it, then used them to speed up the same accumulation.

**Cheap code is not cheap complexity.**

| The old scarcity | The new scarcity |
|---|---|
| Human time to write code | The ability to understand and verify the whole system |

Code volume is not complexity. Complexity is the number of independent things that must be understood together. A verbose 20,000-line implementation with mechanically checked constraints can be simpler than a clever 2,000-line one full of implicit behavior. Minimize irreducible concepts, not characters.

## Stop building piano tops: the method

In *Operating Manual for Spaceship Earth*, Buckminster Fuller describes a shipwreck in which a floating piano top keeps a survivor alive. "But this is not to say that the best way to design a life preserver is in the form of a piano top." People turn accidental historical solutions into permanent assumptions.

Software is an industry of piano tops. Unix answered its machines' limits. Linux inherited Unix. The cloud added layers to operate the result. Agentic development now inherits all of it and teaches agents to operate it. We gave an intelligent system a terminal, a package manager, CI and a Git workflow, then trained it to act like a fast human developer. Some of that is durable. Much of it may be the piano top.

**The flotation principle:** turn intent into correct, observable, reproducible behavior. Source layout, build tooling, orchestration and deployment are implementation details in service of that.

Fuller gives the method: find the general principle beneath the accidental invention. Davis gives the warning: inherited complexity eventually consumes the system. "Subtract before you add" still works inside an inherited design. "Ask why it exists" lets a different design appear.

## Six principles, applied here

1. **Subtract before you add.** Every new feature first challenges the existing abstractions.
   - Here: before a new tool, flag, role, record type or setting, show that no existing primitive expresses it. A change that adds a concept should retire one, or say why it cannot.
   - Smell: the seat skill repeated sixty lines of the knowledge skill word for word, and the two copies were drifting apart. Removed on 2026-11-09.
2. **Minimize concepts, not lines.** Verbosity is welcome when it makes meaning and correctness explicit.
   - Here: one name per state; one representation per fact; explicit typed transitions.
   - Smell: three words for one state, or two caches of the same truth.
3. **Reject inherited complexity.** Do not rebuild Linux, Kubernetes or an agent framework because it is familiar.
   - Here: taffy refused; no database sidecar; Git is the database; our own GPU kit instead of a web stack.
   - Smell: adding a queue, a service or a config layer "because that's how it's done".
4. **Make the whole system inspectable.** State, authority, side effects and invariants are visible to machines and humans. An agent can answer, from the system itself:
   - what states it can be in;
   - which transitions are legal;
   - what effects each operation can produce;
   - why a particular abstraction exists;
   - what would break if a component disappeared;
   - whether correctness is checked mechanically.
5. **Prefer proof over convention.** If a compiler, a type, a TLA+ model or an invariant command can check a relationship, it does not stay a comment, a prompt or tribal knowledge.
   - Here: `make test-formal`, `verification/contracts.toml`, invariants with commands ("no command, no invariant").
   - Smell: a rule that lives only in AGENTS.md prose and is broken weekly. Turn it into a check, then delete the prose.
6. **Set a complexity budget.** Every abstraction, dependency, protocol and state representation justifies its continued existence.
   - Here: mantras are a fixed set; a new one replaces or merges an old one. Prompts and tool descriptions say each rule once, in one place. Dependencies pass the licence gate and justify their weight.
   - Smell: a document explaining a document.

This is not an argument against security, isolation or interoperability. Those earn their place. A good constraint removes enormous amounts of accidental complexity. The target is complexity that does not earn its place.

## The specification is primary; code is one realization

Treat the machine-checkable specification of behavior as the primary artifact: states, transitions, invariants and acceptance. Code is one way to realize it. This is why formal verification is mandatory here and not ceremony. A model states what must be true in a few dozen lines. That lets us regenerate, simplify or replace the code beneath it without losing the meaning.

An agent should be able to challenge the architecture itself, not only produce another implementation of it.

## The software exposes its own meaning

Davis gave programmers direct access to the machine. We give agents direct access to the system's meaning. They should not have to reconstruct it from thousands of files, conventions and undocumented assumptions.

- `orient` tells a seat who it is, what it may do and what it owns, in one call.
- `knowledge_orient` and the document types say what records mean and where they live.
- `graph_read`, provenance and the contracts say how things connect and what proves them.

When an agent has to reverse-engineer what the software means, fix the software, not the agent.

## Working under this stand

Before adding anything, answer:

1. What existing primitive already does this?
2. What does this let us delete?
3. What concept does it add? Is it new, or a renaming of one we have?
4. Can this rule be a check instead of a sentence?
5. Can a developer find it, read it and change it from the tree?
6. Will it still be fun to change a month from now?

In reports, name what you removed, not only what you added. The most valuable agent is not the one that writes the most code. It delivers more capability while removing machinery that should not exist.

Progress is measured by how much capability we create with how little irreducible complexity, not by how much software we generate.

## Tensions

- **Every pixel counts (mantras 4 and 5).** A beautiful UI costs frame time and code. Spend it where human attention lands, and nowhere else.
- **Formal verification is a layer.** It is the layer that removes others: one model replaces pages of convention and a class of tests.
- **Plugins (mantras 9 and 25).** An extension API can be an admission that the core is closed. Plugins remain for owners who do not share our tree. Bring data and binaries across a trust boundary. Inside the tree, the owner edits the source; a plugin is never the only way to change bboxy.
- **Our own drift.** A 27-mantra list, a 2,300-word AGENTS.md and duplicated skill text are this record's first targets. Cutting them is following the stand, not breaking a rule.

## For other projects

Each project in the suite gets the same two parts: its **constraints**, which are physical and stated as refusals, and its **stand**, what it believes inside them. Write the constraints first; the philosophy follows from them.

## Sources

- Adapted from an essay the operator pasted on 2026-11-09, written in response to Terry A. Davis's "bloat and die" remark and his reaction to "make it more like Linux".
- Terry A. Davis, the TempleOS charter: a special-purpose system that refuses hooks for hypothetical futures.
- R. Buckminster Fuller, *Operating Manual for Spaceship Earth* (1969), the piano-top passage.
- The operator's DOS stance, quoted at the top of this record, and a second essay the operator pasted the same day, adapted here: DOS as a philosophy of access, a playground rather than a platform, imagination-to-execution latency.
