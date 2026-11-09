# bboxy450-term: working instructions

Read `~/.agents/AGENTS.md` first. Codex is an Executor. A direct operator request is the brief; implement its scope in your own worktree.

## Our stand

**Build a world that agents can take apart, understand, change, and put back together.**

The god prompt, verbatim; every brief carries it:

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

DOS, not Linux: no layers built to manage the consequences of other layers. Everything
you own is visible and modifiable; the source is the interface. Recreational programming
first: the measure is how long it takes to go from imagining a behavior to seeing it.
Against bloat and die: cheap code is not cheap complexity. Stop building piano tops:
ask why a thing exists before copying its shape. The full stand is the knowledge record
`specs/philosophy.md` (`knowledge_fetch`); every rule below answers to it.

- **Subtract before you add.** First show that no existing primitive does the job. A
  change that adds a concept retires one, or says why it cannot.
- **Minimize concepts, not lines.** One name per state, one home per fact. Verbose
  and explicit beats clever and implicit.
- **Prefer proof over convention.** A rule that a type, a model or a check can hold
  becomes that check, and its prose goes.
- **Say it once.** A rule lives in one place; everywhere else links to it.
- **Report what you removed**, not only what you added.

## Quiet by default

This is one machine shared by many seats; every message wakes a model and costs CPU.

- **Executor:** no acknowledgements, plans, progress, status or "still working" messages.
  Send exactly one message when blocked (evidence and the exact decision needed) and
  one when done (`task_report` with the report locator). Silence means working.
- **Solo:** owns one task end to end and lands it alone. It reports once, when landed or
  blocked, and does not recruit seats.
- **Oracle/Architect:** brief once, then wait for the report. Do not poll a working seat
  or ask for status; read the worktree and `git log` instead.
- Run one build or test at a time per seat, and only the checks the diff can affect.

## Respectful slopping — keep attribution attached

Read [ATTRIBUTION.md](ATTRIBUTION.md) before copying, adapting, translating or
deliberately borrowing a project-specific idea. Project-owned code uses the
[Respectful Slop License 1.0-Draft](LICENSE), with upstream terms and prior
grants preserved. Keep an accurate “Copied from”, “Adapted from” or “Translated
from” comment beside reused protected material, naming its project, canonical
URL and affected source. Known independent inspiration gets an “Inspired by”
reference as a project norm, not a new condition on unprotected ideas.
Do not invent sources, remove upstream notices, imply endorsement or label an
adaptation as independent. Use compact source headers and adjacent origin
comments so extracts carry their credit. Formats without comments use a
companion attribution map. Run `make test-licenses` and preserve license/notice
documents in source and binary packages. Slop respectfully. Keep the credit attached.

## Naming — the bboxy450 suite

This repository is the bboxy450 suite: every crate here, and every repository
split out of it, carries the `bboxy450-` prefix.

| Thing | Rule | Example |
|---|---|---|
| Cargo package / crates.io name | `bboxy450-<name>`, kebab-case | `bboxy450-keychain` |
| Rust import name | the package name in snake_case | `bboxy450_keychain` |
| Directory | `crates/<package name>`, identical to the package | `crates/bboxy450-keychain` |
| Terminal-app-only crate | `bboxy450-term-<name>` | `bboxy450-term-mux` |
| Reusable outside the terminal | `bboxy450-<name>`, no `term` | `bboxy450-render` |
| Platform backend | `<base>-macos`, `-linux`, `-windows` | `bboxy450-keychain-windows` |
| Test fixture | normal name plus `publish = false` | `bboxy450-term-plugin-fixture` |
| GitHub repository | `bboxy450/bboxy450-<name>`; a suite crate stays in `bboxy450/bboxy450-term` until it is split out | `bboxy450/bboxy450-harness` |

The `bbt-*`, `bboxybook` and `bboxy-firehose` directories are legacy: their
packages already follow the rule. Do not rename them as a side effect, and do
not create new directories in that style. A new crate also gets an entry in
`verification/coverage.toml` and a workspace version. It is published as
**bboxy450** only through `make package-crate CRATE=<package name>` and an
authorized release brief.

## Use the Makefile

**The Makefile is the canonical interface for building, tests, installation and release work. Run `make help`. Do not invent Cargo/codesign/copy/publish sequences or keep a second installation recipe in a script or handoff. Add a missing workflow to Make.**

| Command | Result |
|---|---|
| `make build` | Debug app |
| `make build-release` | Optimized app |
| `make bump-harness [REV=sha]` | Move and stage the harness submodule and changed Cargo.lock; defaults to harness `origin/main` |
| `make test` | Full local CI, with exact source/result lines in `target/ci-local.log` |
| `make test-parity` | Full local CI plus the existing visual parity gate |
| `make test-boundaries` | Explicit shell PATH and real process/lifecycle regressions |
| `make test-swarm` | Swarm working folder, override, restart and quit-key regressions |
| `make smoke` | Private unsigned install, throwaway identity, headless smoke; no operator |
| `make install` | Release app signed with the approved Apple Development identity, installed in `~/Applications` |
| `make install-unsigned` | Ad-hoc-only debug app in `target/install-unsigned`, for tests; never replaces the trusted app by default |
| `make release` | Identity-signed macOS ZIP, SHA256 and detached GPG signature |
| `make package-crate CRATE=name` | Package/sign one crate without publishing |
| `make publish-release` | Signed tag and GitHub app release; only on an authorized release brief |
| `make land` = `make push` | **Lean land, always the default.** `scripts/land-fast.sh`: rebase, signed-commit check, clippy and the lib/bin tests of the crates the diff touches plus each changed integration-test file, `cargo check` of their dependents, push. `LAND_MODE=wide` also tests every dependent. Only when push is authorized. **Read [LAND.md](LAND.md) first** |
| `make land-full` | Full signed gate (`make test` + formal receipt) in a warm, queued gate slot; on a release, toolchain or explicit brief |
| `make tools` | Install cargo-nextest and sccache once per machine |

Overrides live in `make help`: `PROFILE`, `TARGET_DIR`, `INSTALL_DIR`, `UNSIGNED_INSTALL_DIR`, `CI_LOG`. Build caches are selected by `scripts/cache-env.sh`; Make sources it. Never change global Cargo configuration for a task.

**Rust workflow, every crate.** `rust-toolchain.toml` pins the compiler for every worktree.
Tests run through cargo-nextest (`.config/nextest.toml`): binaries run concurrently, each test
is its own process, hung tests die after 5 min; doctests run after through `cargo test --doc`.
`scripts/quarantine-test.sh` is the one entry point. sccache shares compiled dependencies
between worktrees. `.cargo/config.toml` keeps intermediates in the worktree's `target/`, overriding
a machine-wide `build-dir`. Iteration builds use two Cargo jobs; `make land` uses four, niced, and two
test threads; on a new machine any build/test/land target installs the tools first.
While iterating, test one crate: `cargo nextest run -p <crate>` (add `-E 'test(name)'` to narrow).

Signed installation never silently falls back to unsigned. `install-unsigned` uses an ad-hoc executable signature needed by arm64 macOS, with no Apple identity/Keychain access. It is a test install, not distribution signing. Notification authorization tests must use the appropriate signed build; ordinary smoke uses file custody and no OS prompt.

## Mandatory formal verification

TLA+/TLC and Yarn verification are mandatory for this codebase and every crate
(operator decision [[d365-mandatory-formal-verification]]). Run `make test-formal`;
`make test` includes it. Every workspace crate must occur in
`verification/coverage.toml` with its model/source mapping or an explicit account
of behavior still checked only by implementation tests. A new crate cannot pass
by omission. Project contracts, assumptions and implementation witnesses live in
`verification/contracts.toml`; `make test-formal-contracts` runs their native
checks. Changes to concurrency, lifecycle, persistence, ownership, retries
or event-loop arbitration update the relevant model and its negative controls.
Yarn is the constrained frontend; native Rust and pinned Java TLC run side by
side. Java remains the reference until separately recorded conformance evidence
authorizes retirement. Finite model parity is not a proof of arbitrary Rust or
complete TLA+ support. See `verification/README.md`.

**Think in TLA+ when designing feature tests.** Name the state variables, their
initial values and finite bounds, the owners, and the actions that change them.
State what must always hold (safety), what must eventually happen (liveness),
and the scheduling/fairness assumptions needed for progress. Explore delayed,
reordered and duplicate completions, cancellation, restart, dependency failure,
and generation/identity changes, including A → B → A. Mark which operations are
atomic and which can interleave. Check a small instance exhaustively before
trusting the happy-path test; add a broken control that removes the relevant
guard and must fail. Connect the property to an actual Rust/API or process test,
and record abstractions and unmodeled behavior in `verification/contracts.toml`.
Pure parsing, geometry and performance still need their specialized tests.

Landing requires a clean, current-source formal receipt as well as the Rust CI
result. `make land-full` (`scripts/land.sh`) records its evidence hash and corpus counts and
refreshes formal verification after a rebase. `make land` runs `make test-formal` when a
model or `verification/` changes. `make check-formal-evidence` verifies the stored
receipt. Land with `make land`;
[LAND.md](LAND.md) has the procedure, the measured timings behind it and the rules that
keep a landing to one pass.

## Scope and evidence

- Read the task, `.bboxy/vision.md`, relevant specs/decisions/interfaces and the source/Git state before editing. `.bboxy/tasks/README.md` is the historical task board; GitHub issues track current release findings.
- One writing worktree per Executor. Do not edit the shared checkout, stage broadly, stash, reset, or change someone else's work. Stage exact paths. Every commit is signed with its author's exact approved key and identity from `.bboxy/invariants/signed-main.md`; Boxy and Maksim are approved.
- Run the Make targets appropriate to the change. Before push, `make land` runs the lean gate on the exact rebased source; do not run a wider suite first; run the full `make test` only for a release, a toolchain or `Cargo.lock` change, or when the brief asks. A real process-boundary change also runs `make test-boundaries`; UI geometry/reference changes require the existing parity check.
- Use the push gate to inspect and reuse source-bound verification already completed. Run only a missing or stale required check once; do not run the entire suite separately and repeat it for the push. Report the actual coverage and any operator-authorized exception accurately.
- **CPU is shared and scarce** (operator, 2026-11-01: several swarms and many workers on one machine). While iterating, run only the targeted tests for the crates you touch. When a full `make test` is required, run it once, on the final committed source. Before starting it, count active runs with `pgrep -f ci-local.sh`, and start only when three or fewer are running. Recheck every few minutes. This is a cap, not a queue: nobody waits on a single holder. Verifiers reuse the author's exact-HEAD CI log instead of rerunning the suite.
- Write the full report in `.bboxy/reports/YYYY-MM-DD-slug.md`, commit it with the work and a `Report: [[reports/YYYY-MM-DD-slug]]` trailer, then call `task_report` with a one-paragraph summary and the `report` locator. Corrections create a new report with `supersedes:`.
- Report commands and actual exits, paths/lines and mechanisms, what was skipped, the commit and worktree status. An agent report or dashboard label is not proof.
- Keep raw logs and fixture accounts outside Git. In `.bboxy/runs`, commit only a `.log`/`.json` command/exit/hash summary. Markdown under `.bboxy` is a numbered record and must follow its frontmatter format.
- Preserve live work: window close/Cmd-Q detach; Cmd-Option-Q and `ctl app stop` explicitly stop backend owners. Cmd-Shift-Q belongs to macOS logout. Supervisor/PTYS remain independent of ordinary UI restart.

## Autonomous operation

Operator, 2026-11-02: "yes land it just merge it, auto merge, work autonomously, dont ask me for permissions". The same rule holds in bboxy450-harness.

Never ask for permission. Verified work is landed (`make land`), merged and deployed by the agent that did it, then reported. Verification still decides: signed commits, the gates above, unattended tests. A red gate is fixed or reported, never bypassed. Ask only for what is physical or irreversible, say why, and keep everything else moving.

## Unattended operation

Every unit, smoke, e2e and agent test runs with nobody at the machine. Use private accounts, homes, sockets and scripted controls. No operator Keychain prompt, login, click, pane authorization or physical probe may be required.

Drive this app through its flags, `ctl`, sockets and captures. Never inject OS keystrokes/clicks or screen-capture the operator's desktop. Kill only exact PIDs you started and recorded; delete only directories you created. Never use broad process kills or clean up peers' targets/worktrees.

## Coordination

**Track work in the built-in bboxy task system** (`.bboxy/tasks/`, through the bboxy MCP `knowledge_*`/`task_*` tools or `ctl task`), never in GitKB tasks, even when a seat's swarm selects a GitKB knowledge provider. Create tasks only through `knowledge_write`, `task_delegate` or `ctl task create`, never as hand-written Markdown: the tools set a native status (`open`…`done`) and an owner, which `task_*` lifecycle requires. A task left without an owner is closed by the operator (`ctl task reconcile`); a legacy status goes through `task_import`. Use GitHub for handoffs. **GitKB, ATC and ICQ are retired as workflow tools; never invoke `git-kb` for this repository's tasks or notes.** Only the GitKB plugin and its tests call it. `scripts/land.sh` runs the signed landing gates and records its receipt locally, without messaging. Use it when the brief requires it; use the Make targets above for build/test/install work. Do not recruit, assign, merge or publish outside that brief.

**Close out landed work at once** (operator, 2026-11-02), entirely through MCP, following the hints each step returns:
- `task_accept` once the work is reported and verified. It refuses report commits that are not on `origin/main`.
- `task_close` (accepted → done). Its hints lead to the next two steps.
- `seat_close` the assignee's seat if it holds no other live task.
- `workspace_audit {task_path}`, then `workspace_release` each release-ready worktree. Release refuses unpreserved commits, dirty files or open processes, and never force-removes.

`workspace_audit` with no arguments also lists unmanaged worktrees (no record). Remove those by hand only after `git cherry origin/main HEAD` shows nothing unlanded and `lsof` shows no users. Keep a seat or worktree only while it still has assigned work. Finished seats and stale worktrees are clutter and use disk.

**Skills live in the KB** at `.bboxy/skills/<name>/SKILL.md` (`skill` document type, Oracle-written through `knowledge_write`), and `.agents/skills` links to them so every harness sees the same repository-crafted skills. Find them with MCP `skill_search` and read them with `skill_read`.

The Oracle verifies landed code and logs; the Architect owns interface/spec consistency; Creativity owns the reference lab. Design gaps are written into the relevant spec/interface, not answered only in a message. Operator instructions outrank defaults.

## Oracle merge and publish authorization (operator, 2026-10-31)

The Oracle may merge PRs into `main` and publish the toolkit crates as **bboxy450**, autonomously. Operator: "you can merge if executor did enough automatic testing, its important to write correct specs, write correct code, do formal verification, do unit - integration - smoke testing".

The Oracle checks each of these itself, not from the Executor's report. A merge needs all of them:

- the change matches its written spec or ICD
- formal verification is green: `make test-formal`, with any changed stateful contract carrying its updated model and a broken control
- unit, integration and smoke tests for the affected crates are green under `make land` (`make land-full` when the brief requires the full suite), run unattended
- the diff has been read
- every commit is signed
- the landing gates pass through `make land`

If any of these is missing, the PR goes back to its Executor. Crate releases follow the signed release order in `~/.agents/AGENTS.md`.
