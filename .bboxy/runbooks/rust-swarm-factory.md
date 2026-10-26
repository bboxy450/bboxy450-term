---
id: 01M5T439H6F29120R87F60T1KN
type: runbook
number: 331
title: "runbook: a Rust software factory run by an agent swarm; cross-platform, dual-licensed, part open source, part reusable; everything we learned building bboxy450-term, with the bottlenecks we hit and what fixed them"
status: living
links:
  - { to: "[[mantras]]", rel: builds-on }
  - { to: "[[landing]]", rel: builds-on }
  - { to: "[[test-policy]]", rel: builds-on }
  - { to: "[[no-chokepoints]]", rel: builds-on }
  - { to: "[[never-block-swarm]]", rel: builds-on }
  - { to: "[[d321-out-of-integration-hell]]", rel: builds-on }
  - { to: "[[d325-ci-capacity]]", rel: builds-on }
  - { to: "[[d320-dispatch-by-noun]]", rel: builds-on }
  - { to: "[[d282-app-tests-itself-unattended]]", rel: builds-on }
  - { to: "[[d285-no-gated-communications]]", rel: builds-on }
  - { to: "[[d46-cross-os-facades]]", rel: builds-on }
  - { to: "[[licensing]]", rel: builds-on }
  - { to: "[[d8-open-source]]", rel: builds-on }
  - { to: "[[plugins]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: builds-on }
  - { to: "[[ctl-cli]]", rel: builds-on }
created: "2026-10-25T18:10:21Z"
updated: "2026-10-26T16:18:37Z"
---
# Runbook: a Rust software factory run by an agent swarm

Operator, 2026-10-25, verbatim: "what we are learning about build proccess, and relay all i say here to oracle - is gold about using rust and agentic swarms to build software, we spotted bunch of bottlenecks along the way, so talk to oracle, look through all the learnings you both had and write very exetniseve runbook on using agents to write rust that is cross platform and dual licensed and part opensource and part reusable to build rust based software factory that we are creating here! DOCUMENT EVERYTHING, lets preserve our learnings!"

This is that runbook. It is written from the record of 2026-10-22 to 2026-10-25: 83 decisions, 40 specs, 37 invariants, about forty landings a day at the end, nine named chokepoints found and seven fixed, one night that ran with nobody at the machine. Every claim here points at a record, a commit or a number; where it does not, it says so. It is a living record: amend it when the factory teaches something new, and date the amendment.

**Lens.** A software factory is a system whose product is working software and whose material is attention: the agents' and the operator's. Every rule below exists because attention was being spent where it bought nothing. **Assumes** one operator, one Oracle, one Architect, one Creativity, five to seven Executors, Rust, one build machine to start. **Breaks when** any of those becomes many without the records growing with them.

---

## Part I. The shape of the factory

### 1. Four roles, never blurred

The harness fixes the role. Claude Code sessions are the **Oracle** by default, the **Architect** on `ROLE: ARCHITECT`, **Creativity** on `ROLE: CREATIVITY`; Codex and every other harness is an **Executor**. The operator outranks the default and can assign otherwise (one Claude session is an Executor by the operator's choice). The roles are different jobs, not ranks:

- The **Oracle** runs the day: assigns work, writes briefs, reviews what lands and reverts, rules on the spot, owns the main watch and the quarantine. It verifies; it does not implement.
- The **Architect** holds the system across time: specs, numbered decisions with alternatives and reasons, invariants with executable checks, rulings when an Executor hits a spec gap. It records; it does not assign, implement or merge. Its two hard lessons are in §9.
- **Creativity** owns the visual lab and supplies entropy: twelve raw shapes before the Oracle or the Architect picks one (§10).
- **Executors** implement exactly the brief, verify with the brief's command, land their own cut through the landing script, and hand off a claim with evidence. They do not reframe, expand, recruit or merge.

Who decides what, in one line each: design questions to the Architect as a spec change; scope, priority and anything outside your allowed files to the Oracle; taste to the operator through the Oracle. When the Architect answered an implementation question that the Oracle was answering in parallel, an Executor briefly held two conflicting instructions (§9).

### 2. Records are the shared brain

The swarm's memory is a directory of Markdown files in the repository, `.bboxy/`, and nothing else: no wiki, no tracker, no chat history as a source of truth. The rules that made it work:

- **One file per record, one record per thing.** Decisions, specs, research, tasks, e2e records, invariants, the quarantine, this runbook. Types are frontmatter; every record has a unique **number** assigned at creation (max plus one), so anyone can say "#316" and be understood; `[[slug]]` and `[[#N]]` links resolve, and a corpus test fails the push when one does not.
- **A decision is the patch, written when it is made**: alternatives, choice, reason, date, the operator's words verbatim when they drove it. Traces record what happened; only the decision records why. Six weeks later nobody remembers; the record does.
- **A spec signs off, not the agent.** A result is accepted because it satisfies a written specification. A spec carries its lens: what it assumes and when it breaks.
- **Records push through one gate**, `scripts/records-push.sh`: corpus and link resolution, diagram fixtures, layering, licences, brand, images, cross-OS, never-block, pending flags, kill-by-name; then commit by named path, rebase, push, and the hash read back from the remote. A record that cannot pass the gate is not a record. The gate caught the Architect's own mistakes three times in one day (§9).
- **Images are records too**: a folder, wikilinks, and one face for every account and crate, pinned by digest.
- **Evidence is `file:line` and `commit`, never an adjective.** A claim in a channel line is a claim; the record cites what the tree shows.

Build this first in a new factory. Everything else below was written into it.

### 3. Invariants are gates, not vibes

An invariant is a property that holds before and after every change, **with the command that checks it, the exit status that means held, and what a violation costs**. No executable check, no invariant: only a preference. Thirty-seven exist here, in families: structure (layering, cross-OS facade, branding, licences, images), product (UI never waits, parity geometry, rim and glow, 967 seats at frame budget, checkpoints bounded, idle swarms cost nothing, watched replay priority, no seat starves, ring reads batched, messaging never held), process (never block the swarm, no chokepoints, kill only own PIDs, main red bounded, presubmit tests fast, e2e runs unattended, ctl opt-in, ctl never prints secrets).

Rules learned the hard way:

- **Land the switch before the load.** A slow or not-yet-passing check enters the shared runner only once the runner can skip it: `status: pending` with `slow: true` and a `gate:` name. The day a pending invariant landed without those flags, the default run executed a check whose test did not exist and failed for every agent preparing a gate. The records gate now refuses that file shape.
- **Activation is a landing**: the commit that lands the test renames the check line if needed, removes the flags, and the Oracle flips the status on acceptance. Twice the landing left the status pending with the flags gone, which the gate also refuses; the Architect flipped on the Oracle's landing note with the evidence.
- **Hard gates and judgement are separate instruments.** Exit codes block; "is this design serialising everything" advises and escalates. Collapsing them turns a gate into theatre. Every judgement call is written as such.
- **A budget is a promise, not a fit to the machine.** The 120 Hz frame budget stayed at 8.33 ms through five failing measurement runs; the runs were evidence of distance, and the record says so.

### 3a. Systems engineering: interfaces, budgets, the matrix

Added 2026-10-26 after the operator read how large engineering programmes keep out of integration hell. Components were owned; their interactions were not, so most of a day's rulings were boundary questions answered late, one at a time. The fix is the lead-systems-engineer job, held by the Architect: one **interface control document** per boundary (`.bboxy/interfaces/`, nine of them: supervisor and logic, logic and GUI, ctl and app, app and harness, messaging and seats, plugin and host, identity and app, lab and native, night and app), each naming both sides, what crosses, timing, states, failure, versioning and its verification, changed only with both owners; **budgets** that split frame time, memory, disk, build time and inference across subsystems with margins before a cut spends them ([[budgets]]); and a **verification matrix** from each interface to the evidence that proves it. "Integrated and verified" is the deliverable; finished components are progress toward it. Build these on day one of a new factory, before the first cut crosses a boundary.

### 4. The mantras are the compressed law

Twenty-seven lines, distilled by a subagent from the operator's words across every record, and amended as the operator spoke: be flexible and always evolve; a restart loses nothing; cockpit first; every pixel counts; GPU everything, 120 Hz, the UI thread never waits; macOS first, one look everywhere, cross-OS as facades; local first; plugins as a folder of binaries with a shape; everything is a graph of graphs; agnostic but opinionated defaults; hackers and furries first; everything branded; dual licences; the suite is the product; LESS IS MORE; the spec signs off; harnesses lie; provenance unbroken; show useful information, never dumb information; always be useful; nothing is dropped; seats are sockets, everything is an event on one bus; one concern at a time; visualization is always ours; never block the swarm; no bottlenecks, no chokepoints. They are the first thing a new agent reads and the last test of any design ([[mantras]]).

---

## Part II. Working with agents

### 5. The brief

Every assignment carries its role header, the task, the worktree, the allowed files, the required outcome and the verification command. One writing worktree per Executor, never the shared tree. Stage by exact path; never `git add -A`, `stash`, `reset` or autostash where others write. Every commit signed with the machine key; an unsigned commit means the configuration was bypassed, fix the configuration. A scope gap is a DM to the Oracle with the exact path and why, and the in-scope work continues meanwhile.

**Size the cut to land the day it is claimed.** The largest single cost in the record after gate length was rebase churn from long-lived branches. Contract changes (the wire, the verb tables, the catalogue manifest) land alone and first, as an empty switch; the loads follow.

### 6. The handoff is a claim

A handoff names the signed sha, the files with `file:line` and mechanism, every command with its real exit status, failures included, the evidence per acceptance line, what was not checked, and what was deliberately left alone. The Oracle verifies against the spec from the tree, the diff and the logs, never from the report. "All gates passed" in a channel line is a claim until the raw log's hash and the tree agree. A shared-tree run proves nothing; other agents' in-flight files leak into it.

### 7. Communication is a protocol, and protocols have grammars

Coordination runs over ICQ, each agent with its own key, never borrowing another's. What the four days taught:

- **Prefix-match protocol lines.** `SLOT <n> TAKEN`, `SLOT <n> RELEASED`, `QUIET <name> START, until <HH:MM TZ>`, `QUIET <name> END`, `LANDED <sha>`: a line begins with the form; everything after is a note for humans. The day a detector read the words "QUIET END" inside a release note as the window closing, it started a compile inside the window. A detector that matches substrings is the bug.
- **The claim is atomic and the line is a note.** Two agents took the same slot at once from the channel; the fix moved the claim into `mkdir` of a directory and left the line as the notice. A channel cannot be a lock.
- **No gated communications.** Agents act on each other's messages within their brief; a message never waits on an approval to be delivered or acted on. An authenticated brief from the Oracle's key is authorization for in-scope work in the Executor's own worktree; inside the app, the signed task record is. A chat line authorizes nothing else, and a peer cannot override a task's own stop line ([[d285-no-gated-communications]]).
- **A receipt proves a commit to an inbox, not that anyone read it.** Readiness of a socket is not model receipt.
- **Milestones get said.** When the first night ran unattended, both the Oracle and the Architect wrote it on the channel in one line each. Morale is infrastructure.

### 8. Never block the swarm

The operator's standing order, repeated until it was an invariant: no test needs the operator; nobody waits; verdicts are scripts; blocked work is reassigned, never parked; no bottleneck, no chokepoint; non-blocking architecture, no gated communication; ask forgiveness, not permission; working software over perfect software. In practice:

- The Oracle rules on the spot and the Architect records after; records follow decisions, not the reverse.
- A step that needs a human is a design bug: own throwaway test identity, headless app, scripted control. The whole app tests itself unattended ([[d282-app-tests-itself-unattended]]), and the invariant [[e2e-runs-unattended]] proves it: every group ends by verdict, every promise is written, no operator identity anywhere.
- "Parked" is the word the rule forbids. A passed gate sitting unmerged for hours is output nobody consumed; the Architect's zoom-out flagged one and the Oracle landed it.
- Ask the operator only for what is physical or irreversible, say why, and keep everything else moving meanwhile. A purchase is the operator's; a design is not.

### 9. The Architect's own lessons, with dates

Written down because each cost a correction.

1. **Realisation is the Oracle's call.** An Executor asked both of us how to realise a ruling with the existing mechanisms; the Oracle ruled, the Architect ruled a different shape in parallel and pushed it first; the Executor held two instructions for a few minutes (2026-10-24, decisions #315 and #280). Authority, invariants and the meaning of a spec sentence are the Architect's; which mechanism to reuse is the Oracle's day ruling, which the Architect records.
2. **Write the spec line from the ruling's words or the code line it names, never from a landing broadcast's paraphrase.** Twice a landing line's wording became a wrong spec line (a "reply state" that was an error kind; "one further native attempt" where the test asserted none). The fix each time was one sentence from the test or the code.
3. **A pending invariant needs its skip flags in the same commit.** Forgetting them broke default CI for everyone for twenty minutes. The records gate refuses the shape now.
4. **The Architect's pane runs no heavy build.** Verifying a flag fix by running the full invariants script compiled the workspace without a gate slot while three gates ran, and left four gigabytes in the worktree. The records gate is the only build that pane runs.
5. **Zoom out before any loop.** A second identical failure makes the premise the problem; a third identical run is forbidden; a passed gate nobody consumed is cost; several results from one neighbourhood mean diverge before selecting. Said in one line before acting.
6. **Judgement is flagged as judgement.** A hypothesis ("a foreign reaper sent that SIGKILL") goes to the Oracle labelled as such, with what would confirm it, not as a ruling. It was wrong, the app's own fence had done it, and the rule it prompted was still right.
7. **Three narrowing patches to one rule mean the rule's premise is wrong.** On 2026-10-25 the presubmit's single 15-minute bound killed five cuts with no assertion failing; the Architect answered with a wide tier, then a cold-compile exception, then a widened cold-compile exception, each correct for the case in front of it. Creativity's zoom-out named the premise: one clock over compile and run measures cache state, not cut size. The fix was two clocks, a run bound and a build cap, and the patches were withdrawn. When the third exception is being written, stop and name what the rule is actually measuring.

### 10. Creativity: diverge first

When the operator said "we are in integration hell", Creativity produced twelve raw shapes before anyone chose: a railway token, a kitchen expediter, a stock-exchange call auction, film offline and online conform, orchestra parts not score, append-only registries, a release train, blast radius, ant pheromones, tower-crane time, the shipping container, the darts foul line. Nine entered the design. The one it named as the Censor ("a merge robot takes the Oracle's job") became the spine. Generate before judging, bring the shape from outside the domain, name the Censor and write the idea anyway.

---

## Part III. Rust, specifically

### 11. Workspace layering and naming

Crates in one workspace, in a fixed layer order checked by `scripts/layering.sh`: OS backends at the bottom, facades above them, core and runtime, the proto and the mux, the app on top. A lower crate never depends upward. Publishing goes leaves first. Everything the factory owns is branded with one prefix (`bboxy450-`), the GUI kit publishes under its own name, and a brand check fails the push on an unbranded crate ([[bboxy450-prefix]]).

### 12. Cross-platform is a facade over per-OS crates

Raw OS bindings live only in per-OS crates (`*-macos`, `*-linux`, `*-windows`, and the named-not-created `*-ios`, `*-android`, `*-web`); a facade crate holds the types and the trait and selects the backend with `cfg(target_os)` dependencies; the app never names an OS crate. Dummies for the OSes not yet built keep the facade honest. `scripts/cross-os.sh` fails the push on a binding in a shared crate, with dated exceptions that each name their retirement ([[d46-cross-os-facades]], [[cross-os-facade]]). macOS first, one look on every platform; the Linux backends nobody runs today are why a cheap Linux box belongs in CI (§22).

### 13. Licensing and release

Dual **MIT OR Apache-2.0** for what we own; Apache alone where upstream forces it; **never copyleft** in the dependency tree, checked by `scripts/deps-licences.sh` over every dependency; no contributor licence agreement; the identity and the face are not licensed with the code ([[licensing]], [[no-copyleft-deps]]). Corporate-friendly, hardcore-developer-first.

A crate release is **signed, published and announced, in that order**: `cargo package`, a detached GPG signature over the packaged artifact, a signed tag, `cargo publish` as the one approved account, `git push --follow-tags`, a GitHub release carrying the artifact and its signature, and one line in the crates channel with the fingerprint. Cargo packaging is byte-stable, so the detached signature verifies what the registry serves. A release without its signature and its announcement is unfinished.

### 14. Closed enums, macro lists and the wire

Rust's closed enums are the compiler's exhaustiveness and the wire's contract; keep them, and still let ten agents add verbs without editing one file. The shape that worked ([[d320-dispatch-by-noun]]): each noun module declares its variants in one `macro_rules!` list, one chaining macro splices the lists into the flat enum with the same derives, so the wire bytes are identical by construction, zero call sites change, a verb touches its noun's file, and a variant name in two lists is a duplicate the compiler refuses. The nested alternative, a variant per noun wrapping the noun's type, would have rewritten 762 call sites in one landing. **Wire changes are additive fields with defaults**, guarded by a golden test of byte identity over every action and reply; the golden runs in every presubmit.

### 15. Build mechanics on a shared machine

- `--locked` everywhere; `CARGO_INCREMENTAL=0` in gates.
- **Each agent builds in a private target directory.** A shared target serialises builds on cargo's lock. The cost is disk: four to five gigabytes per warm target, and disk hit its floor three times in a day. Own targets only, `lsof` before deleting anything, and tell the owner; stale profiler scratch and inactive dev caches are what to look for first.
- **Cache dependencies, never through one daemon.** About 480 dependency crates are most of a cold build, about eight minutes of every clean gate; a shared cache server stalled every build at once under nine agents; the fix is one `sccache` server per agent over one shared local disk cache ([[d325-ci-capacity]]), measured by Velvet with the tool's own counters: a clean build from 488 s to 127 s, 434 of 434 Rust hits, a 297 MB cache (e537345b). **The cache keys on the absolute path**: sccache hashes the working directory and the `CARGO_*` environment into a Rust key (0.18.0, `src/compiler/rust.rs:1565-1595`), so a gate in a fresh detached tree at a new path missed 432 times and took 7 m 17 s while the same path hit 434 of 434. The rule that follows (the Oracle, 2026-10-25): gates run in fixed per-slot paths, `/tmp/bboxy450-gate-slot-<n>/{tree,target}`, clean each time, so each slot hits its own cache; pre-checks hit per worktree. Measure the claim with the tool's own counters, cold against warm, same path, before writing it into a record. The same path-dependence bites a reused target directly: a test binary bakes `CARGO_MANIFEST_DIR` in at compile time, so a target reused from a deleted checkout runs fixtures against a path that no longer exists (Nyx's recovery self-land, 2026-10-25, the one red of 26 invariants). Reuse a target only at the path it was built for, which the fixed per-slot paths give for free. The compile share of gate time was 52 % before the cache. An empty `rustc-wrapper` in a repository `.cargo/config` does not override a global one; only the environment does.
- **`CARGO_BUILD_JOBS=2` per agent when nine build at once**, and never a `--workspace` pre-check in parallel: ten of those drove load past 150 and became the chokepoint of the hour.
- **The affected set is transitive**: touched crates plus every reverse dependency from `cargo metadata`. Direct dependents miss the app behind the mux behind the core.
- Strict clippy (`-D warnings`, all targets) on the affected crates before every handoff; `rustfmt` on a crate root reformats other people's files, so run it narrowly.

### 16. Tests

The test policy is [[test-policy]]; the numbers that forced it: 884 tests, 35 files calling `sleep`, 13.4 hours of test run and 9.3 of test compile over 175 gates, and six binaries holding sixty-second real-clock waits that summed to over eight hours no hardware could buy back. The rules in one breath: one contract per test; delete the pointless (duplicates, tests of the fixture, assertions of constants, tests that cannot fail); rewrite the dumb (real time onto an injected clock, **never by shrinking a real deadline**, which only makes the race tighter and the test flakier (Kestrel's injected deadline, 30 s for the good path and 25 ms for the hung one, passed 10/10 under load where a 2 s real deadline raced a fixture; the Oracle's lesson, folded into #327 on 2026-10-25), real processes onto a seam with one real-boundary test kept in the night, polling onto signalled conditions); rewrite the too direct to the contract (observable reply, record, screen or event, never internal state, call order or error prose); keep and tag what the law needs with a `keeps:` line. **No test asserts a wall-clock bound**; test-internal timeouts are hang guards only; tests are independent of each other and of load. Timing is measured in exactly two controlled places: the frame gate and the night.

### 17. The app tests itself, unattended

This is the core, in the operator's words, and the design that made it true ([[d282-app-tests-itself-unattended]], [[e2e-acceptance]], [[ctl-cli]]):

- **A control surface for scripts**: `bboxy450-term ctl <noun> <verb>`, twelve nouns, one JSON-lines reply shape where `data` is the window action's own reply unchanged, errors as kinds with a `hint` naming what is valid, exit codes 0 to 4, every operation under the operator principal with `door: ctl` and `issued_by` in the signed body, opt-in by an endpoint file that exists only when the app runs with `--ctl`. No verb the window lacks; no desktop input, ever.
- **Replies mean what a script needs**: a relayed layout verb replies after the first frame that shows the change and names the window `frame_seq`; a seat's text screen has its own sequence and the two are never compared; a capture of a hidden seat is the error kind `not_on_screen` carrying the seat and the frame checked.
- **A throwaway account per run** in file custody, never the operator's; its label is a property of the account root the receiver derives, never a caller field ([[d313-test-account-label-derived]]).
- **A headless app** with a real GUI client and no window, so a locked display is fine.
- **One runner** that creates the account, starts the app, delegates every promise as a task, answers known harness prompts through recipes, collects verdicts, writes records, and exits by verdict.
- **A pass means the whole promise** ([[d316-pass-means-the-whole-promise]]): every night script carries a `# coverage: full | partial; missing: …` header; the record carries `coverage` and `missing`; only full is green; a tracked partial is amber; an untracked one is red. The first night ran amber with every partial tracked, which is the honest colour.
- **Real agents are tested only for agent behaviour** (wake, orient, report, reply), from the app's own records; what they say about intuitiveness is advisory.
- A recording notification backend behind the same trait, a test-only fault injector for ring gaps, and one App clock the night can freeze and advance, each exist so a promise can be asserted at an exact state rather than by sleeping.

### 18. Identity, custody and the hold that never blocks

Every commit signed; a device-bound operator journal; custody behind a cross-OS facade with a file backend for automation. The hard rule the record fought for: **signing can be held, delivery cannot** ([[d285-no-gated-communications]], [[messaging-never-held]]). Under a custody hold, local delivery continues with `signature: pending` and the hold's reason; on a first-ever held startup with no author key at all, the row is a logical intent with no author claim; when the key returns, a signed completion is appended, bound to the intent's id and digest, nothing rewritten, nothing delivered twice; the inbox consumer is keyed to an account-local operator reference, never the device, so nothing moves when the device enrols. The same pattern, pending record now and signed completion later, is how the Keychain unlock audits itself ([[d315-unlock-admitted-under-hold]]): the hold's one exit is never behind the hold. Secrets never print; the Keychain item carries an ACL keyed to the team-signed designated requirement so every rebuild keeps access without a prompt.

### 19. Measuring performance honestly

The 967-seat frame gate failed five times and each failure named something: a settle guard that could not settle because of a stale display tick and queued commands re-arming the draw; then, once settled, two producers from the diagnostics counters, 4,321 scene projections for 42 encoded frames and 6,824 serial one-seat ring reads at 93 % of input-to-GPU. Fixing both took input-to-GPU from 8,618 ms to 1,076 ms and CPU p99 from 519 to 292 ms, still over budget, with the partition naming the remainder. The lessons: measure before optimising, with counters in the product, not guesses; a settle guard's predicates are themselves a design question when a source is legitimately alive at idle; a budget is never fitted to the machine; and a scale measurement is scheduled, never a window that stops eight agents for an hour.

---

## Part IV. Integration and CI

### 20. The nine chokepoints

Found in four days, each with cause, fix and gate ([[no-chokepoints]]):

| # | Chokepoint | Fix | Gate |
|---|---|---|---|
| 1 | A message pump scanning every swarm on every tick | Scan only dirty swarms | idle-swarms-cost-nothing |
| 2 | Serial replay starving the watched seats | Two lanes, 3 : 1 watched to background | watched-replay-priority |
| 3 | A GUI knowledge read held on logic | Held-read responsiveness check | its own test |
| 4 | The Oracle granting gate slots by hand | Self-service atomic claim in a directory | the script |
| 5 | A retry sweep restarting from the first seat every pass, three seats at zero admissions in 12,409 attempts | Credit-driven rotation | no-seat-starves |
| 6 | One machine-wide compile-cache daemon stalling every build | One cache server per agent, shared disk cache | the build |
| 7 | 6,824 serial one-seat ring reads, 93 % of input-to-GPU | Batched reads, at most eight per request, 121 for 967 seats | ring-reads-batched |
| 8 | Shared dispatch files every verb cut edited, two or three rebases per landing | One module per noun, macro lists into flat enums | the build plus the golden |
| 9 | The Oracle's serial merged-tree run before every landing, and five measurement quiet windows a day | Self-service landing, batched postsubmit, scheduled scale run | main-red-bounded |

Four of the nine were in the swarm, five in the product. The product ones were found by load; the swarm ones by counting minutes.

### 21. Landing

The whole flow is [[landing]] and the reasoning [[d321-out-of-integration-hell]]; the one-paragraph version: an Executor lands its own cut with `scripts/land.sh`, which claims a slot, rebases onto main, runs a **presubmit bounded at fifteen minutes** on the affected crates (transitive), strict clippy, the fast invariants, the wire golden and the touched parity cases, then pushes signed commits fast-forward and posts `LANDED`. The **postsubmit** runs the full parity suite on the newest main in batches, afterwards and outside the slots; on red, a **culprit finder** bisects the batch with reruns to tell a flake from a break and the culprit alone is reverted, **revert first, fix after**; nothing but a revert lands while main is red, and the length of each red interval is a metric. A **quarantine** holds a test that fails under load and passes alone for seven days with an owner and a task, still running and still logged; most of what looked like flakes here were real bugs the load found, so the quarantine is narrow. Live landing state lives in one machine-wide uncommitted directory; evidence snapshots land once a day. Lab-only cuts compare against the native renders the last green postsubmit stored and build no product crate.

### 22. CI capacity, priced

One Mac shared by nine agents was the build machine. The cheapest CI that actually runs ([[d325-ci-capacity]]), in cost order: the dependency cache done right at zero cost (measured: a clean build 488 s to 127 s); one cheap ARM Linux box (about €25 to €50 a month) running the portable tier and the Linux backends over SSH with no hosted CI billing; a second Mac mini bought once (about $600 to $800) as the postsubmit and night machine. Rejected on price: hosted macOS minutes at about $2,200 a month for our volume on a slower core, rented Macs at €160 a month that cost more than the box in four months, a dedicated x86 box with no macOS. Hosted CI becomes free when the public crates publish, and that is where it goes.

### 23. Disk and load hygiene

Disk is a chokepoint on a shared machine: targets of four to five gigabytes each, profiler scratch of twenty gigabytes nobody remembered, stale agent directories. The rules: own targets only; `lsof` before deleting; tell the owner; delete the stale, keep the reports and hashes; the operator's standing authorization covers old build and scratch artefacts, never a peer's live tree. Load: two cargo jobs per agent, scoped pre-checks, measurement runs never during the day.

---

## Part V. Open source and reuse

### 24. What is reusable, and the order it leaves

The suite is the product and the terminal is its first app; the pieces that leave as crates, leaves first: the GUI kit (`bboxy450-frontmatter`), the desktop-notifications facade and its OS backends, the custody facade and its backends, the harness wake-up kit (the shared hacks for tapping into harnesses, telling its user whether a harness can be woken and what the fallback is) and the harness host (process hosting in PTYs with a probe allowlist), the records tooling and the diagram renderer. Each is a facade with per-OS crates, dual-licensed, branded, signed on release, and gets the free hosted CI of a public repository ([[d8-open-source]]). The split rule: the visualization is always ours, and plugins bring data, never pixels ([[plugins]]).

### 25. Plugins and the knowledge base

A plugin is a folder of binaries with a shape; the shapes are contracts as JSON Schema (data source, task system, event sink, decision model, repository graph), each with a wire name and a tier; a plugin is an MCP server; trust is a content-addressed copy with no execution before trust and off by default. The first bundled provider is the knowledge base itself: the records directory as a data source with resolve, fetch, search, list, links, graph, write and changes-since, swappable in Settings. Build the first plugin as the thing you already have, and the second one will fit.

---

## Part VI. Checklists

### 26. Day one of a new factory

1. Repository with signed commits enforced, one identity, the face in an images folder.
2. `.bboxy/` with the record types, numbering, the corpus test and `records-push.sh`; the mantras as the first spec.
3. The four roles written down with who decides what; ICQ identities; the channel grammar; atomic slot claims.
4. The invariants runner with tiers and the pending-flags rule; the first invariants: layering, licences, brand, cross-OS, never-block, kill-only-own-PIDs.
5. The landing scripts: affected set, presubmit, land, main watch, culprit, revert, quarantine; the state directory; the metrics.
6. A dependency cache with one server per agent; private targets; two jobs per agent.
7. The test policy, and `keeps:` tags on the law's own tests.
8. The control surface and the throwaway-account night before the first feature that needs a human to test.
9. The cross-OS facade for the first OS-bound thing, with dummies.
10. The crate release recipe, and the crates channel.

### 27. The rhythm of a day

The Oracle writes briefs, reviews landings after the fact, rules on the spot, lands reverts, and zooms out weekly from the metrics. The Architect records rulings as decisions with hashes, writes invariants as checks, answers spec gaps by DM, and reads code only when an invariant is in question. Executors claim with the paths they will touch, cut to land the same day, land through the script, hand off claims. Creativity keeps the lab the visual truth. The night runs the e2e groups on throwaway accounts, then the scale measurement, then writes the day's evidence into the record. Nobody waits.

### 28. Anti-patterns seen, with dates

- A human in the test loop (Keychain prompts, trust dialogs, a display): 2026-10-23, fourteen hours; designed out by 2026-10-25.
- A paraphrased landing line becoming a spec line: twice, 2026-10-24 and 25.
- A pending invariant without skip flags: 2026-10-25, twenty minutes of broken default CI.
- Substring-matching a protocol line: 2026-10-24, a compile inside a quiet window.
- A channel line used as a lock: 2026-10-24, a double-taken slot.
- One shared compile-cache daemon: 2026-10-24, six agents stalled.
- Workspace-wide pre-checks from ten agents: 2026-10-25, load 150.
- Five measurement quiet windows in one day: 2026-10-25.
- Sixty-second sleeps in tests: eight of thirty-seven hours.
- A passed gate parked unmerged: 2026-10-25, flagged and landed.
- The Architect running a workspace build in its pane: 2026-10-25, stopped.
- Two rulings on one question from two roles: 2026-10-24, resolved by the realisation rule.

### 29. Numbers to watch

Lead time from the claim line to `LANDED`; presubmit seconds and the per-binary durations; postsubmit red minutes per day; reverts per day; the quarantine's length; test count and files with `sleep` per crate; gate time split into compile, test run, test compile, invariants and clippy; disk free and load at the hour. The board shows them; the weekly zoom-out reads them, not the channel.

---

## Part VII. The Oracle's section

Vesper Greywind, Oracle, 2026-10-25, in their own words and with their evidence; merged by the Architect without rewording beyond punctuation. Where the sections above disagreed with a line here, the line here won and the section was changed and dated.

### What cost the most, and what fixed it

- Compile time, more than test logic: 52 % of 36.9 h of measured gate time across 175 gates is compiling (build 7.6 h, test compile 9.3 h, clippy 2.3 h). Source: `tools/gate_time.py` in the board repository, `time.html`, 2026-10-25.
- One shared sccache daemon stalled every agent, so we dropped the cache (a29f839d) and every clean gate rebuilt about 480 dependencies for about 8 min. Fix: one server per agent over one shared disk cache (#325, Velvet e537345b): cold 488 s to warm 127 s, 434 of 434 Rust hits, 297 MB cache.
- An empty `rustc-wrapper` in a repo `.cargo/config` does NOT override the global one. Only environment variables (`RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=`) do. Wasted a day of "why is sccache still running" (a29f839d).
- Real-clock tests were 36 % of gate time (13.4 h). Single tests of 52 to 72 s every gate: bbt-mux `socket_backlog.rs` 2.1 h total, bboxy450-term `lifecycle.rs` 1.9 h, then custody 1.2 h, identity_local 1.1 h, logic 0.8 h, provenance 0.75 h. Fix: injected clocks (#326, #327).
- Real-time deadlines plus parallel gates make flakes (operator: "timeout is problematic coz we might run tests in parallel then everything goes to shit"). Example: `plugin_registry.rs:722` raced a 2 s deadline (`check.rs:17`) with a Python fixture; fixed by an injected deadline, 30 s for the good path, 25 ms for the hung one, 10/10 under load (Kestrel f584df9a).
- Rule learned from that: never shrink a real deadline to make a test fast; that makes it flakier. Advance a clock instead.
- Gate collisions: two agents took slot 3 at once. Fix: an atomic `mkdir` slot claim in `ci-local.sh`, with dead-pid reclaim, 3 slots at 25 GiB or more free, else 2, and exit 75 when full (40dafd1b, 026a07af).
- Disk is a shared resource that silently kills gates: each agent's private target is 1 to 9 GB, and we dropped to 14 GiB free. Fix: a standing cleanup authorisation, `lsof` before every delete, tell the owner; 2026-10-25 freed 6.1 GB across 768 stale dirs, plus Sable 0.96 GB of gallery PNGs.
- QUIET windows (stopping everyone for a 967-seat scale run) were the largest serialisation. Fix: the scale run moves to the night (#323 §8, `scale-night.sh`).
- Integration hell = everyone gated against an old main and then re-gated. Fix (#321/#323): presubmit on affected crates including transitive dependents, a fast-forward push, batched postsubmit on main, culprit finding, revert first, flaky quarantine with owner and expiry.
- Flaky tests blocked unrelated landings. Fix: a quarantine list (`.bboxy/quarantine.md`, #324) where a listed test still runs and gets one isolated rerun; failing alone is still a failure; seven-day expiry; task records #329, #330.
- My `quarantine-test.sh` passed when the `--exact` rerun matched zero tests; Aster's fixtures caught it. Lesson: every retry and skip path needs a test that it can fail.
- Pending invariants committed without slow/gate flags broke every gate (#314, #318). Fix: `pending-flags.sh` in records-push. Lesson: land the switch before the load.
- A records file without frontmatter broke the corpus gate (`quarantine.md`, fixed as #324 at 7ad988a6). Lesson: every `.bboxy` write goes through `records-push.sh`.

### Merging and verification (the Oracle's craft)

- Rebase-merges failed silently and I pushed the wrong tree twice. Fix: after every rebase, `git diff <gated source> HEAD -- crates` must be 0 lines (R5l: 0 lines, landed f221ba48).
- Merged-tree test of dependents: a change can be green alone and red on main (NUM2 broke app tests). For a bbt-proto change, run workspace strict clippy plus the proto tests on the merged tree (#320 cut 1: 051f7e24).
- If main moved only by records (`.bboxy`), a re-gate is unnecessary; check `git diff --name-only base origin/main` outside `.bboxy`.
- The verdict is the raw log, not the message: find the raw `ci.log`, check its sha256 matches the handoff, grep `ci-local: all gates passed`, count `test result: ok` and `FAILED` (R5l 6c0695bc, 211 ok; cut 1 98ee233c, 211 ok / 0 FAILED).
- Evidence-only successor commits keep provenance without touching code: the code commit is gated, and the next commit adds only `.bboxy/runs` logs (Rook 5e3e80b3).
- I merged R5i without its evidence commit and had to add it after. The handoff is complete only with both commits.
- Wrong `--reply-to` ids (I used a reminder id) orphan threads. Reply to the message id you are answering.

### What I would brief differently on day one

- Delegate design to the Architect from minute one; I drafted the landing plan myself and Corvin found eight gaps in it (operator: "you are not the smartest tool, architect is", "delegate thinking").
- Every brief carries: ROLE header, worktree, allowed files, outcome, verification command, and at least one mutation control (break it, and show the test fails).
- Name overlaps in the brief: who else touches the file, and who rebases on whom (Seraph #320 cut 2c waits for Sable's #328 in `ctl.rs`).
- Pre-review the test file before the gate: a 2-minute grep for `sleep`, `Instant::now`, `from_secs`, or fixed ports catches flakes before they cost a 30-minute gate (Kestrel 518bf8b7).
- Decouple landings: when two in-flight cuts depend on each other, make the dependency optional (`[ -f scripts/cache-env.sh ] && .`) so neither waits (Aster af380bc7).
- Gates measure time; budgets live in the night. A test duration in a loaded presubmit is reported, never asserted (#327 §4).
- Ask "does the existing mechanism actually do this?" before approving a seam: I approved a ctl wait over `AttachEvents`, and Sable showed `bus.rs:1014` refuses a second consumer. #328's waiting `EventsRead` replaced it.

### What the Oracle's day looks like when it works

- Slots stay full (three gates at once), and my time goes to reading raw logs and diffs, not writing code.
- Executors announce SLOT TAKEN / RELEASED / CLAIM / LANDED on the channel; I verify and push within minutes of RELEASED.
- Questions are routed by kind: mechanism choice is the Oracle's on-the-spot ruling; invariant meaning, authority, and spec go to the Architect, and the Executor keeps working on the independent part meanwhile.
- The operator gets a few lines per event: what landed, the evidence, what is next, what needs them (money, purchases, publishing).

### What the Executors taught me

- Sable: "a T8.sh-only substitution cannot meet the assertion" and the `bus.rs:1014` single-consumer refusal; read the code before the brief, not after.
- Aster: fixture tests on throwaway repos caught the zero-test pass and a records-only batching defect; scripts get tests like code.
- Rook: the precheck log binding every failure and fix by hash (`.bboxy/runs/r5l-prechecks.log`); report false starts, never erase them.
- Kestrel: "both own patches range-diff =" as proof a rebase changed nothing.
- Seraph: split a refactor that doesn't fit a day into ordered small landings (2a to 2e), each gated alone.
- Velvet: measure the claim with the tool's own counters (`sccache --show-stats`), cold vs warm, same target path.
- Nyx: structural bounds beat wall-clock bounds (main red until the next code commit, not N minutes; #322 at 0d7bea90).

---

## Sources

Internal: the records linked above, `.bboxy/graph.md`, `.bboxy/runs/`, the board's `time.html`. External, read 2026-10-25 for the landing and CI decisions: Google, "Flake Aware Culprit Finding" (ICST 2023) and "What Breaks Google?"; the Chromium sheriffing document; Trunk, Graphite and Mergify merge-queue documentation on batching and bisection; GitHub's Actions pricing changelog; Hetzner, Scaleway, MacStadium and AWS price pages; Cloudflare R2 pricing.

Owner: Corvin Duskmantle <ARCHITECT>, with the Oracle.
