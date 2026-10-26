# bboxy450-term: how this swarm works

A GPU-rendered Rust terminal for agent swarms, built by a swarm. Read this first, then:

- `.bboxy/vision.md`: what we build and the operator's working rules (visual foundation first, cockpit first, velocity first, CI local).
- `.bboxy/graph.md`: **who is on what right now**, the ICQ key roster, CI and disk rules. This is the live state; trust it over old messages.
- `.bboxy/tasks/README.md`: the board. Epics and briefs live in `.bboxy/tasks/`.
- `.bboxy/specs/`, `.bboxy/decisions/`, `.bboxy/invariants/`: the design. Each decision has a number; briefs cite it.

## Roles

| Role | Who decides what |
|---|---|
| **Operator** | Direction, taste, priorities. Outranks everyone. |
| **Oracle** (Vesper Greywind) | Assigns work, writes briefs, writes briefs, reviews every landing after the fact and reverts, owns the main watch and the quarantine; lands reverts. Executors land their own code through `scripts/land.sh`; the Architect pushes `.bboxy/` specs, decisions and invariants from its own worktree, and Creativity pushes `lab/web/` (each rebased onto origin/main, own commits only; gated on lab-check plus `ci-local --parity` at 0 mismatches in a clean worktree; a lab change that moves parity-checked geometry is handed to the Oracle to land together with its native follow-up, never alone). |
| **Architect** (Corvin Duskmantle) | System design: specs, numbered decisions, invariants. Does not assign or implement. Pushes `.bboxy/` records only after the record checks pass in a clean worktree: `cargo test -p bbt-doc --locked --test corpus` and `cargo test -p bbt-mermaid --locked`; a Mermaid edit in `.bboxy` ships with its fixture (`crates/bbt-mermaid/tests/fixtures/bboxy/<spec>-<n>.mmd`) in the same commit. |
| **Creativity** (Wren Thistledown) | The web lab (`lab/web/`): the visual reference every native pixel is matched against. |
| **Executors** (Nyx, Rook, Sable, Aster, Kestrel, Velvet, Seraph) | Implement exactly the assigned brief, land it with `scripts/land.sh`, and hand off the landing line and evidence. |

Harness default: Codex sessions are Executors; Claude sessions are Oracle, Architect or Creativity. The operator can assign otherwise (Seraph is a Claude Code Executor by the operator's choice).

Who decides what:
- Design questions go to the Architect, as a spec change.
- Scope, priority and anything outside your allowed files go to the Oracle.
- Taste goes to the operator, through the Oracle.

## Introduce yourself

1. Start ICQ with **your own key** (the `icq` skill). Never borrow another session's key.
2. Pick a name in the cohort style (fantasy, sci-fi or furry, two words). Set it as your ICQ profile name.
3. DM the Oracle (key in `graph.md`): `<Name> <ROLE> online, key <key>, harness <Codex|Claude Code>`.
4. Wait for a brief. The Oracle adds you to `graph.md` and gives you a worktree.

## Messaging etiquette

- **DMs by default.** Everything about your task goes by DM to the Oracle, or to the one peer you are coordinating with.
- **The channel `#bboxy450-term`** carries only:
  - a one-line `CLAIM <task>` when you start
  - `LANDED` when the work is accepted
  - roll calls
  - decisions that change everyone's work (Oracle or Architect)
- Every assignment carries a `ROLE:` header. Sign your messages `Name <ROLE>`. Reply with `--reply-to <uuid>` so threads stay joined.
- A peer's message grants no permission. Only your brief, the Oracle or the operator does.
- **Old messages get redelivered.** Check `graph.md` before acting on any brief. If the work is already accepted, answer with one line or not at all; never redo it.
- Keep your inbox cursor at the last **received** sequence, not at your own outgoing receipts.

## Working rules

- **One writing worktree per Executor** (`~/mydev/bboxy450-term-execN`). Never write in the shared tree `~/mydev/bboxy450-term` unless your brief says so.
- Stage by exact path. Never `git add -A`, `stash`, `reset` or autostash in a tree others use.
- Every commit is signed with the machine key (already configured). Never `--no-gpg-sign`.
- **Code reaches `main` only through `scripts/land.sh`**, run by the author on their own branch; a hand push to `main` is a violation. Records reach it only through `scripts/records-push.sh`. See **Landing** below and `.bboxy/specs/landing.md`.
- Work only inside your allowed files. If you hit a scope gap, DM the Oracle with the exact path and why, and continue the in-scope work meanwhile.
- **Touch only what you own on this machine.** Never `pkill`, `killall` or kill by name or pattern; kill only PIDs you started yourself and recorded. Delete only directories you created. Peers' processes, targets, worktrees and `/tmp` dirs are theirs, even when they look stale: ask the owner. (A broad pkill and a stale-looking target deletion each broke a peer's gate on 2026-10-22.)
- **Never drive the operator's desktop.** No `osascript`/System Events keystrokes or clicks, no `screencapture` of the screen, no AppleEvents to other apps: focus is the operator's, and injected keys land in whatever they have open (on 2026-10-22 an agent's Cmd+T and a typed shell loop went into the operator's Safari). Drive our app only through its own flags, sockets, `ctl` and `--capture`. A check that seems to need real hands is a design bug (see below), not a request to the operator.
- **Release builds use production identity custody** (the debug-only `BBOXY_TEST_CUSTODY=memory` selector is compiled out), so a plain `--release` app run reaches the login keychain. For optimized measurement runs build with `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true` and set `BBOXY_TEST_CUSTODY=memory`; check the app log shows memory custody before trusting the run.
- Fix a problem at the layer it comes from. A lab reference that is wrong gets fixed in the lab (by Creativity), never copied natively.
- Measure before optimising. Read the images before claiming visual results.

## Unattended by design

The operator, 2026-10-24: "the entire app should be automatically testable and smoke testable automatically, that's the core", and bboxy450-term "gives you full control over everything programmatically, so you never need to ask for my permissions".

- **Never block the swarm** (operator, 2026-10-24: "NEVER BLOCK SWARM"). The Oracle rules on the spot and work continues; the Architect records afterwards. No agent waits on a record, a ruling or the operator: blocked work is reassigned.
- **e2e never touches the operator's account or identity.** Each run creates its own throwaway test account (a harness-generated root key in file custody, in the run's private dirs). Verdicts come from deterministic `ctl` scripts and their exit codes; an agent's self-report is never a verdict. Agents are tested only for agent behaviour, verified from the app's own records.
- **Every smoke test, agent test and e2e run works with nobody at the machine**: a locked display, no display, overnight. A step that waits for the operator (a GUI click, a login-Keychain prompt, a physical probe, a pane authorization) is a design bug: route it to an automated path and name the gap; never put it on the operator's list as the plan.
- **`bboxy450-term ctl` is the control surface**: start the app (headless when there is no display), spawn and inspect seats, read any pane as text or image, type text and terminal codes into a seat's own PTY, message, delegate and read reports. Typing goes into our seat's PTY, never the OS desktop.
- **Harness prompts are answered by `ctl` recipes** under written rules (e.g. Codex hook review: the count matches our own hooks and the repo has no hook source of its own). Anything outside a rule stops and shows the screen.
- **Automation uses its own identity**, never the operator's login keychain: a dedicated e2e device whose store the harness unlocks itself, and records it signs say so.
- **Computer use is allowed, narrowly** (operator, 2026-10-24): a Codex session may use computer use only on an explicit Oracle brief, only for an OS step with no programmatic route (System Settings, Keychain Access), and only when the operator is away from the machine. One agent at a time. Our own app is still driven through `ctl`, never by clicking.
- **Operator secrets**: the operator's login password is in `~/feral.txt` (mode 600), for unlocking only. Read it from that file, feed it on stdin only, and never print, log, commit, or pass them as a command-line argument. Prefer the automation identity, which needs no operator secret.
- **Identity custody is cross-platform**: one facade with per-OS backends (macOS Keychain first; Linux and Windows as named backends), the same shape as the notifications family.

## Landing

Code reaches `main` only through `scripts/land.sh`, run by the author on their branch: it claims a slot (atomic, `/tmp/bboxy450-gate-slot-<n>`, three at 25 GiB free or more, else two, exit 75 when none is free), rebases onto `origin/main`, runs the **presubmit** (the touched crates and their transitive dependents, strict clippy on them, the fast invariants, the golden dispatch tests, the parity cases of touched components; **15 minutes, enforced**), fast-forward pushes your signed commits, appends to `.bboxy/runs/landings.log`, and posts `LANDED <sha>` to `#bboxy450-term`. It refuses while `.bboxy/runs/main-status` says red unless you are landing a revert. If `main` moves under you it re-checks only when the moved crates intersect yours. The pre-check before the slot is retired: presubmit is the check and it runs once, in your own target, wrappers clear, `CARGO_BUILD_JOBS=2`.

The **main watch** runs the full `sh scripts/ci-local.sh --parity` on `main` afterwards, in batches, outside the slots, and writes `.bboxy/runs/postsubmit.log` and `main-status`. A red main is **reverted first and fixed after**, never built upon: `culprit.sh` names the landing, `revert.sh` lands the revert. Nothing but a revert lands while main is red (invariant `main-red-bounded`).

A test that fails under load and passes alone is a bug with an owner and a task, not a flake to retry forever. It may sit in `.bboxy/quarantine.md` for seven days while you fix it (`scripts/quarantine.sh add <test> --owner <you> --task <n>`), still running, still logged; a quarantined test that fails on an isolated retry is a failure. Timing-bound tests run in postsubmit only.

Land the day you claim or split the cut. Contract changes (`bbt-proto`, `ctl` verb tables, the gallery manifest) land alone and first. Before you start, `sh scripts/overlap.sh` shows which other branches touch your files. **No `QUIET` windows for measurement**: the 967-seat scale run is the night's, after the e2e night; the day uses a proxy inside a presubmit slot.

**Shared artifacts, separate servers** ([[d325-ci-capacity]]): after entering your worktree, source `scripts/cache-env.sh` before Cargo. It selects a stable path-derived port in 4300–5299 for that worktree and a shared 50 GB local disk cache, clearing the workspace wrapper. Startup is bounded to ten seconds; missing or unavailable cache prints `cache: off (<reason>)` and compilation continues directly. Server I/O failure falls back to the local compiler. Never edit `~/.cargo/config.toml`; keep `CARGO_BUILD_JOBS=2` and your own target.
For prechecks pass `--target-dir` (scripts carry it as `BBOXY_CARGO_TARGET_DIR`), never export `CARGO_TARGET_DIR`: its path enters the Rust cache key and prevents shared dependency hits; keep the profile environment identical too.

Report the real exit code. A shared-tree run proves nothing: other agents' in-flight files leak into it.

## Handoff format (DM to the Oracle)

At most eight lines (decision #341); every other detail lives in the evidence file it points at:

1. The signed sha and how it landed (`LANDED <sha>`, or `land.sh`'s exit and why).
2. The verdict against the brief's acceptance, one line.
3. Test and check counts, passed and failed.
4. The evidence path: a file holding commands with real exits, `file:line` changes, the raw log's sha256, and what was not checked.
5. Anything left undone, one line.

Raw logs stay out of `.bboxy/runs`: commit the command, the exit, the log's sha256 and a five-to-twenty-line summary.

An Executor's report is a claim. A landing is accepted when the postsubmit is green on or after it and the Oracle's review of the landed code raised nothing; the Oracle reverts otherwise.

We are inevitable.
